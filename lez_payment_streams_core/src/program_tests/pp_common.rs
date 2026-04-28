//! Shared infrastructure for privacy-preserving test flows across instruction test modules.
//!
//! NSSA rejects PP messages when both `new_commitments` and `new_nullifiers` are empty, so flows
//! use mixed visibility (public PDAs plus at least one private slot where applicable).

use nssa::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies,
        message::Message,
        witness_set::WitnessSet,
        PrivacyPreservingTransaction,
    },
    program::Program,
    V03State,
};
use nssa_core::{
    account::{Account, AccountId, AccountWithMetadata, Balance, Data, Nonce},
    encryption::{EphemeralPublicKey, Scalar, ViewingPublicKey},
    program::InstructionData,
    BlockId, EncryptionScheme, MembershipProof, NullifierPublicKey, NullifierSecretKey,
    SharedSecretKey,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, derive_stream_pda, derive_vault_pdas,
        force_clock_account_monotonic, load_guest_program, state_with_initialized_vault,
        state_with_initialized_vault_with_privacy_tier, transfer_native_balance_for_tests,
        VaultFixture,
    },
    StreamId, Timestamp, TokensPerSecond, VaultConfig, VaultHolding, VaultPrivacyTier,
    CLOCK_01_PROGRAM_ACCOUNT_ID,
};

use super::common::TEST_PUBLIC_TX_TIMESTAMP;

// ---- Shared recipient identity (Phase 1 and withdraw tests) ---- //

pub(crate) const RECIPIENT_NSK: NullifierSecretKey = [0x5a; 32];
pub(crate) const RECIPIENT_VSK: Scalar = [0x6b; 32];
pub(crate) const EPK_SCALAR: Scalar = [3u8; 32];

pub(crate) fn recipient_npk() -> NullifierPublicKey {
    NullifierPublicKey::from(&RECIPIENT_NSK)
}

pub(crate) fn recipient_vpk() -> ViewingPublicKey {
    ViewingPublicKey::from_scalar(RECIPIENT_VSK)
}

fn withdraw_instruction_data(vault_id: u64, amount: Balance) -> InstructionData {
    Program::serialize_instruction(Instruction::Withdraw { vault_id, amount })
        .expect("withdraw instruction serializes")
}

pub(crate) fn account_meta(state: &V03State, id: AccountId, is_authorized: bool) -> AccountWithMetadata {
    AccountWithMetadata {
        account: state.get_account_by_id(id),
        is_authorized,
        account_id: id,
    }
}

// ---- Vault fixtures ---- //

/// [`VaultPrivacyTier::Public`] vault funded via a public `Deposit` (standard non-PP ladder).
pub(crate) fn vault_fixture_public_tier_funded_via_deposit() -> VaultFixture {
    let mut fx = state_with_initialized_vault(1_000 as Balance);
    let account_ids_deposit = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        fx.owner_account_id,
    ];
    let tx_deposit = build_signed_public_tx(
        fx.program_id,
        Instruction::Deposit {
            vault_id: fx.vault_id,
            amount: 400 as Balance,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[Nonce(1)],
        &[&fx.owner_private_key],
    );
    fx.state
        .transition_from_public_transaction(&tx_deposit, 2 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("deposit");
    fx
}

/// [`VaultPrivacyTier::PseudonymousFunder`] vault funded via a test-only native transfer: public
/// `Deposit` is refused for this tier at the harness, so tests bypass it to reach a funded PP
/// `withdraw`.
pub(crate) fn vault_fixture_pseudonymous_funder_funded_via_native_transfer() -> VaultFixture {
    let mut fx = state_with_initialized_vault_with_privacy_tier(
        1_000 as Balance,
        VaultPrivacyTier::PseudonymousFunder,
    );
    transfer_native_balance_for_tests(
        &mut fx.state,
        fx.owner_account_id,
        fx.vault_holding_account_id,
        400 as Balance,
    );
    fx
}

// ---- PP withdraw infrastructure ---- //

/// PP-transition artifacts a caller may further assert on.
pub(crate) struct PpWithdrawReceipt {
    pub(crate) tx: PrivacyPreservingTransaction,
    pub(crate) shared_secret: SharedSecretKey,
}

/// Build and submit a PP `withdraw` from `fx` to an arbitrary private recipient (vis-2 slot).
pub(crate) fn fund_private_account_via_pp_withdraw(
    fx: &mut VaultFixture,
    recipient_npk: &NullifierPublicKey,
    recipient_vpk: ViewingPublicKey,
    epk_scalar: Scalar,
    withdraw_amount: Balance,
    block: BlockId,
) -> PpWithdrawReceipt {
    let guest_program = load_guest_program();
    assert_eq!(guest_program.id(), fx.program_id);

    let withdraw_to_id = AccountId::from(recipient_npk);
    let owner_before = fx.state.get_account_by_id(fx.owner_account_id);

    let pre_states = vec![
        account_meta(&fx.state, fx.vault_config_account_id, false),
        account_meta(&fx.state, fx.vault_holding_account_id, false),
        account_meta(&fx.state, fx.owner_account_id, true),
        AccountWithMetadata {
            account: Account::default(),
            is_authorized: false,
            account_id: withdraw_to_id,
        },
    ];

    let shared_secret = SharedSecretKey::new(&epk_scalar, &recipient_vpk);
    let (output, proof) = execute_and_prove(
        pre_states,
        withdraw_instruction_data(fx.vault_id, withdraw_amount),
        vec![0u8, 0, 0, 2],
        vec![(recipient_npk.clone(), shared_secret)],
        vec![],
        vec![None::<MembershipProof>],
        &ProgramWithDependencies::from(guest_program),
    )
    .expect("execute_and_prove: fund via PP withdraw");

    let epk = EphemeralPublicKey::from_scalar(epk_scalar);
    let message = Message::try_from_circuit_output(
        vec![
            fx.vault_config_account_id,
            fx.vault_holding_account_id,
            fx.owner_account_id,
        ],
        vec![owner_before.nonce],
        vec![(recipient_npk.clone(), recipient_vpk, epk)],
        output,
    )
    .expect("try_from_circuit_output: fund via PP withdraw");

    let witness_set = WitnessSet::for_message(&message, proof, &[&fx.owner_private_key]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);
    fx.state
        .transition_from_privacy_preserving_transaction(&tx, block, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("transition: fund via PP withdraw");

    PpWithdrawReceipt { tx, shared_secret }
}

/// Thin wrapper around [`fund_private_account_via_pp_withdraw`] using the module-scope recipient
/// identity so both tier tests share the same private payout account.
pub(crate) fn run_pp_withdraw_to_private_recipient(
    fx: &mut VaultFixture,
    withdraw_amount: Balance,
    block_withdraw: BlockId,
) -> PpWithdrawReceipt {
    fund_private_account_via_pp_withdraw(
        fx,
        &recipient_npk(),
        recipient_vpk(),
        EPK_SCALAR,
        withdraw_amount,
        block_withdraw,
    )
}

// ---- Phase 1: visibility-1 private signer (claim and close_stream) ---- //

pub(crate) const PP_T0: Timestamp = 1;
pub(crate) const PP_T1: Timestamp = 6;
pub(crate) const PP_STREAM_RATE: TokensPerSecond = 10;
pub(crate) const PP_STREAM_ALLOCATION: Balance = 100;
pub(crate) const PP_WITHDRAW_AMOUNT: Balance = 50;
pub(crate) const PP_CLAIM_PAYOUT: Balance = PP_STREAM_RATE as Balance * (PP_T1 - PP_T0) as Balance;

/// Shared state for Phase 1 PP claim and close tests.
pub(crate) struct PpClaimCloseSetup {
    pub(crate) fx: VaultFixture,
    pub(crate) stream_id: StreamId,
    pub(crate) stream_pda: AccountId,
    /// RECIPIENT's committed account state after the PP withdraw (their private balance = 50).
    pub(crate) provider_committed_account: Account,
}

/// Build the shared state for the PP claim and close_stream tests.
pub(crate) fn pp_claim_close_setup() -> PpClaimCloseSetup {
    let mut fx = vault_fixture_public_tier_funded_via_deposit();

    let receipt = run_pp_withdraw_to_private_recipient(&mut fx, PP_WITHDRAW_AMOUNT, 3 as BlockId);

    let commitment_from_withdraw = &receipt.tx.message().new_commitments[0];
    let provider_committed_account = EncryptionScheme::decrypt(
        &receipt.tx.message().encrypted_private_post_states[0].ciphertext,
        &receipt.shared_secret,
        commitment_from_withdraw,
        0,
    )
    .expect("decrypt provider state from PP withdraw");

    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    force_clock_account_monotonic(&mut fx.state, clock_id, 1, PP_T0);

    let stream_id = 0u64;
    let stream_pda = derive_stream_pda(fx.program_id, fx.vault_config_account_id, stream_id);
    let provider_id = AccountId::from(&recipient_npk());
    let stream_ix_accounts = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        stream_pda,
        fx.owner_account_id,
        clock_id,
    ];
    let tx_create = build_signed_public_tx(
        fx.program_id,
        Instruction::CreateStream {
            vault_id: fx.vault_id,
            stream_id,
            provider: provider_id,
            rate: PP_STREAM_RATE,
            allocation: PP_STREAM_ALLOCATION,
        },
        &stream_ix_accounts,
        &[Nonce(3)],
        &[&fx.owner_private_key],
    );
    fx.state
        .transition_from_public_transaction(&tx_create, 4 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("create_stream in pp_claim_close_setup");

    PpClaimCloseSetup {
        fx,
        stream_id,
        stream_pda,
        provider_committed_account,
    }
}

// ---- Phase 2: PP deposit ---- //

pub(crate) const OWNER_NSK: NullifierSecretKey = [0x7c; 32];
pub(crate) const OWNER_VSK: Scalar = [0x8d; 32];
pub(crate) const OWNER_FUND_EPK_SCALAR: Scalar = [4u8; 32];
pub(crate) const PP_DEPOSIT_EPK_SCALAR: Scalar = [5u8; 32];
pub(crate) const PP_OWNER_FUND_AMOUNT: Balance = 80;
pub(crate) const PP_DEPOSIT_AMOUNT: Balance = 30;

pub(crate) fn owner_npk() -> NullifierPublicKey {
    NullifierPublicKey::from(&OWNER_NSK)
}

pub(crate) fn owner_vpk() -> ViewingPublicKey {
    ViewingPublicKey::from_scalar(OWNER_VSK)
}

pub(crate) fn load_payment_streams_with_auth_transfer() -> ProgramWithDependencies {
    let payment_streams = load_guest_program();
    let auth_transfer = Program::authenticated_transfer_program();
    ProgramWithDependencies::new(
        payment_streams,
        [(auth_transfer.id(), auth_transfer)].into(),
    )
}

// ---- Phase 3: PP owner-signer instructions ---- //

pub(crate) const PP3_OWNER_FUND_EPK_SCALAR: Scalar = [6u8; 32];
pub(crate) const PP3_SIGNER_EPK_SCALAR: Scalar = [7u8; 32];
pub(crate) const PP3_RECIPIENT_NSK: NullifierSecretKey = [0x9e; 32];
pub(crate) const PP3_RECIPIENT_VSK: Scalar = [0xaf; 32];
pub(crate) const PP3_RECIPIENT_EPK_SCALAR: Scalar = [8u8; 32];
pub(crate) const PP3_VAULT_B_BALANCE: Balance = 400;
pub(crate) const PP3_OWNER_FUND_AMOUNT: Balance = 80;
pub(crate) const PP3_STREAM_RATE: TokensPerSecond = 5;
pub(crate) const PP3_STREAM_ALLOCATION: Balance = 100;
pub(crate) const PP3_TOP_UP_AMOUNT: Balance = 50;
pub(crate) const PP3_T0: Timestamp = 100;
pub(crate) const PP3_T1: Timestamp = 105;
pub(crate) const PP3_WITHDRAW_AMOUNT: Balance = 30;

pub(crate) fn pp3_recipient_npk() -> NullifierPublicKey {
    NullifierPublicKey::from(&PP3_RECIPIENT_NSK)
}

pub(crate) fn pp3_recipient_vpk() -> ViewingPublicKey {
    ViewingPublicKey::from_scalar(PP3_RECIPIENT_VSK)
}

/// Shared state for Phase 3 PP owner-signer tests.
pub(crate) struct PpOwnerSetup {
    pub(crate) fx: VaultFixture,
    pub(crate) vault_b_id: u64,
    pub(crate) vault_config_b_id: AccountId,
    pub(crate) vault_holding_b_id: AccountId,
    /// Owner's committed account after the funding PP withdraw.
    pub(crate) owner_committed_account: Account,
    pub(crate) owner_npk: NullifierPublicKey,
}

/// Build the shared state for all Phase 3 PP owner-signer tests.
pub(crate) fn pp_owner_setup() -> PpOwnerSetup {
    let mut fx = vault_fixture_public_tier_funded_via_deposit();

    let owner_npk = owner_npk();
    let receipt = fund_private_account_via_pp_withdraw(
        &mut fx,
        &owner_npk,
        owner_vpk(),
        PP3_OWNER_FUND_EPK_SCALAR,
        PP3_OWNER_FUND_AMOUNT,
        3 as BlockId,
    );

    let owner_commitment = &receipt.tx.message().new_commitments[0];
    let owner_committed_account = EncryptionScheme::decrypt(
        &receipt.tx.message().encrypted_private_post_states[0].ciphertext,
        &receipt.shared_secret,
        owner_commitment,
        0,
    )
    .expect("decrypt owner state from PP withdraw");
    assert_eq!(owner_committed_account.balance, PP3_OWNER_FUND_AMOUNT);

    let owner_id = AccountId::from(&owner_npk);
    let vault_b_id: u64 = 2;
    let (vault_config_b_id, vault_holding_b_id) =
        derive_vault_pdas(fx.program_id, owner_id, vault_b_id);

    let vault_config_b = Account {
        program_owner: fx.program_id,
        balance: 0,
        data: Data::try_from(
            VaultConfig::new(
                owner_id,
                vault_b_id,
                None,
                Some(VaultPrivacyTier::PseudonymousFunder),
            )
            .to_bytes(),
        )
        .expect("vault_config_b data fits"),
        ..Account::default()
    };
    fx.state.force_insert_account(vault_config_b_id, vault_config_b);

    let vault_holding_b = Account {
        program_owner: fx.program_id,
        balance: PP3_VAULT_B_BALANCE,
        data: Data::try_from(VaultHolding::new(None).to_bytes())
            .expect("vault_holding_b data fits"),
        ..Account::default()
    };
    fx.state.force_insert_account(vault_holding_b_id, vault_holding_b);

    force_clock_account_monotonic(&mut fx.state, CLOCK_01_PROGRAM_ACCOUNT_ID, 4, PP3_T0);

    PpOwnerSetup {
        fx,
        vault_b_id,
        vault_config_b_id,
        vault_holding_b_id,
        owner_committed_account,
        owner_npk,
    }
}

// ---- Phase 4: PP initialize_vault ---- //

pub(crate) const PP4_FUND_EPK_SCALAR: Scalar = [9u8; 32];
pub(crate) const PP4_INIT_EPK_SCALAR: Scalar = [10u8; 32];
pub(crate) const PP4_OWNER_FUND_AMOUNT: Balance = 50;
