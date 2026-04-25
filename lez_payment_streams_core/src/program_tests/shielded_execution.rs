//! Privacy-preserving (`execute_and_prove` plus `transition_from_privacy_preserving_transaction`) tests.
//!
//! NSSA rejects PP messages when both `new_commitments` and `new_nullifiers` are empty, so flows
//! use mixed visibility (public PDAs plus at least one private `2` slot where applicable).

use nssa::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies, message::Message, witness_set::WitnessSet,
        PrivacyPreservingTransaction,
    },
    program::Program,
    V03State,
};
use nssa_core::{
    account::{Account, AccountId, AccountWithMetadata, Balance, Data, Nonce},
    encryption::{EphemeralPublicKey, Scalar, ViewingPublicKey},
    program::InstructionData,
    BlockId, Commitment, EncryptionScheme, MembershipProof, NullifierPublicKey, NullifierSecretKey,
    SharedSecretKey,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, derive_stream_pda, derive_vault_pdas, force_clock_account_monotonic,
        load_guest_program, patch_vault_config, state_with_initialized_vault,
        state_with_initialized_vault_with_privacy_tier, transfer_native_balance_for_tests,
        VaultFixture,
    },
    StreamConfig, StreamId, StreamState, Timestamp, TokensPerSecond, VaultConfig, VaultHolding,
    VaultPrivacyTier, CLOCK_01_PROGRAM_ACCOUNT_ID,
};

use super::common::TEST_PUBLIC_TX_TIMESTAMP;

const RECIPIENT_NSK: NullifierSecretKey = [0x5a; 32];
const RECIPIENT_VSK: Scalar = [0x6b; 32];
const EPK_SCALAR: Scalar = [3u8; 32];

fn recipient_npk() -> NullifierPublicKey {
    NullifierPublicKey::from(&RECIPIENT_NSK)
}

fn recipient_vpk() -> ViewingPublicKey {
    ViewingPublicKey::from_scalar(RECIPIENT_VSK)
}

fn withdraw_instruction_data(vault_id: u64, amount: Balance) -> InstructionData {
    Program::serialize_instruction(Instruction::Withdraw { vault_id, amount })
        .expect("withdraw instruction serializes")
}

fn account_meta(state: &V03State, id: AccountId, is_authorized: bool) -> AccountWithMetadata {
    AccountWithMetadata {
        account: state.get_account_by_id(id),
        is_authorized,
        account_id: id,
    }
}

/// [`VaultPrivacyTier::Public`] vault funded via a public `Deposit` (standard non-PP ladder).
fn vault_fixture_public_tier_funded_via_deposit() -> VaultFixture {
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
fn vault_fixture_pseudonymous_funder_funded_via_native_transfer() -> VaultFixture {
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

/// PP-transition artifacts a caller may further assert on (for example, decrypt the private
/// post-state using `shared_secret`).
struct PpWithdrawReceipt {
    tx: PrivacyPreservingTransaction,
    shared_secret: SharedSecretKey,
}

/// Build and submit a PP `withdraw` from `fx` to an arbitrary private recipient (vis-2 slot).
///
/// The vault's public owner (`fx.owner_account_id`) signs as vis-0; the new private recipient
/// is created at vis-2.  Returns a receipt containing the raw tx (for commitment inspection)
/// and the shared secret used to encrypt the recipient's private post-state.
fn fund_private_account_via_pp_withdraw(
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
fn run_pp_withdraw_to_private_recipient(
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

#[test]
fn test_withdraw_private_recipient_pp_transition_succeeds() {
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 3 as BlockId;

    let mut fx = vault_fixture_public_tier_funded_via_deposit();
    let holding_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let owner_before = fx.state.get_account_by_id(fx.owner_account_id);

    let receipt = run_pp_withdraw_to_private_recipient(&mut fx, withdraw_amount, block_withdraw);

    assert_eq!(
        fx.state
            .get_account_by_id(fx.vault_holding_account_id)
            .balance,
        holding_before - withdraw_amount
    );
    let owner_after = fx.state.get_account_by_id(fx.owner_account_id);
    assert_eq!(owner_after.balance, owner_before.balance);
    let mut expected_nonce = owner_before.nonce;
    expected_nonce.public_account_nonce_increment();
    assert_eq!(owner_after.nonce, expected_nonce);

    let cfg = VaultConfig::from_bytes(&fx.state.get_account_by_id(fx.vault_config_account_id).data)
        .expect("vault");
    assert_eq!(cfg.total_allocated, 0u128);

    assert_eq!(receipt.tx.message().new_commitments.len(), 1);
    let commitment = receipt.tx.message().new_commitments[0].clone();
    let ciphertext = &receipt.tx.message().encrypted_private_post_states[0].ciphertext;
    let decrypted = EncryptionScheme::decrypt(ciphertext, &receipt.shared_secret, &commitment, 0)
        .expect("decrypt private withdraw_to post-state");
    assert_eq!(decrypted.balance, withdraw_amount);
}

#[test]
fn test_pp_withdraw_private_recipient_pseudonymous_funded_vault_succeeds() {
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 3 as BlockId;

    let mut fx = vault_fixture_pseudonymous_funder_funded_via_native_transfer();
    let holding_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
    let owner_before = fx.state.get_account_by_id(fx.owner_account_id);

    let _receipt = run_pp_withdraw_to_private_recipient(&mut fx, withdraw_amount, block_withdraw);

    assert_eq!(
        fx.state
            .get_account_by_id(fx.vault_holding_account_id)
            .balance,
        holding_before - withdraw_amount
    );
    let owner_after = fx.state.get_account_by_id(fx.owner_account_id);
    assert_eq!(owner_after.balance, owner_before.balance);
    let mut expected_nonce = owner_before.nonce;
    expected_nonce.public_account_nonce_increment();
    assert_eq!(owner_after.nonce, expected_nonce);

    let cfg = VaultConfig::from_bytes(&fx.state.get_account_by_id(fx.vault_config_account_id).data)
        .expect("vault");
    assert_eq!(cfg.privacy_tier, VaultPrivacyTier::PseudonymousFunder);
    assert_eq!(cfg.total_allocated, 0u128);
}

// ---- Phase 1: visibility-1 private signer (provider / authority) ---- //

const PP_T0: Timestamp = 1;
const PP_T1: Timestamp = 6;
const PP_STREAM_RATE: TokensPerSecond = 10;
const PP_STREAM_ALLOCATION: Balance = 100;
const PP_WITHDRAW_AMOUNT: Balance = 50;
// accrual at T1: rate * (T1 - T0) = 10 * 5 = 50
const PP_CLAIM_PAYOUT: Balance = PP_STREAM_RATE as Balance * (PP_T1 - PP_T0) as Balance;

fn claim_instruction_data_for_pp(vault_id: u64, stream_id: StreamId) -> InstructionData {
    Program::serialize_instruction(Instruction::Claim { vault_id, stream_id })
        .expect("claim instruction serializes")
}

fn close_stream_instruction_data_for_pp(vault_id: u64, stream_id: StreamId) -> InstructionData {
    Program::serialize_instruction(Instruction::CloseStream { vault_id, stream_id })
        .expect("close_stream instruction serializes")
}

/// Shared state for Phase 1 PP claim and close tests.
///
/// Ladder: funded vault → PP withdraw 50 to RECIPIENT → create stream with RECIPIENT as
/// provider.  Clock is at `PP_T0` after this setup; tests advance it to `PP_T1`.
struct PpClaimCloseSetup {
    fx: VaultFixture,
    stream_id: StreamId,
    stream_pda: AccountId,
    /// RECIPIENT's committed account state after the PP withdraw (their private balance = 50).
    provider_committed_account: Account,
}

/// Build the shared state for [`test_pp_claim_private_provider_succeeds`] and
/// [`test_pp_close_stream_private_provider_authority_succeeds`].
fn pp_claim_close_setup() -> PpClaimCloseSetup {
    // Block 1: initialize_vault, Block 2: deposit 400 (inside fixture)
    let mut fx = vault_fixture_public_tier_funded_via_deposit();

    // Block 3: PP withdraw 50 to RECIPIENT — creates RECIPIENT commitment
    let receipt = run_pp_withdraw_to_private_recipient(&mut fx, PP_WITHDRAW_AMOUNT, 3 as BlockId);

    // Decrypt RECIPIENT's committed account from the PP withdraw ciphertext
    let commitment_from_withdraw = &receipt.tx.message().new_commitments[0];
    let provider_committed_account = EncryptionScheme::decrypt(
        &receipt.tx.message().encrypted_private_post_states[0].ciphertext,
        &receipt.shared_secret,
        commitment_from_withdraw,
        0,
    )
    .expect("decrypt provider state from PP withdraw");

    // Write clock T0 so create_stream has a valid accrual start time
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    force_clock_account_monotonic(&mut fx.state, clock_id, 1, PP_T0);

    // Block 4, Nonce(3): create_stream with RECIPIENT as provider
    // Nonce 3 = after init(0) + deposit(1) + PP-withdraw public-signer increment(2→3)
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

#[test]
fn test_pp_claim_private_provider_succeeds() {
    let PpClaimCloseSetup {
        mut fx,
        stream_id,
        stream_pda,
        provider_committed_account,
    } = pp_claim_close_setup();

    // Advance clock to T1 so the stream has accrued PP_CLAIM_PAYOUT tokens
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    force_clock_account_monotonic(&mut fx.state, clock_id, 2, PP_T1);

    let guest_program = load_guest_program();
    assert_eq!(guest_program.id(), fx.program_id);

    let provider_npk = recipient_npk();
    let provider_id = AccountId::from(&provider_npk);
    let provider_commitment = Commitment::new(&provider_npk, &provider_committed_account);
    let membership_proof = fx
        .state
        .get_proof_for_commitment(&provider_commitment)
        .expect("provider commitment not found in state after PP withdraw");

    // claim accounts: vault_config(0), vault_holding(1), stream_config(2),
    //                 owner(3), provider(4, vis-1), clock(5)
    let pre_states = vec![
        account_meta(&fx.state, fx.vault_config_account_id, false),
        account_meta(&fx.state, fx.vault_holding_account_id, false),
        account_meta(&fx.state, stream_pda, false),
        account_meta(&fx.state, fx.owner_account_id, false),
        AccountWithMetadata {
            account: provider_committed_account.clone(),
            is_authorized: true,
            account_id: provider_id,
        },
        account_meta(&fx.state, clock_id, false),
    ];

    let provider_shared_secret = SharedSecretKey::new(&EPK_SCALAR, &recipient_vpk());
    let provider_epk = EphemeralPublicKey::from_scalar(EPK_SCALAR);

    let (output, proof) = execute_and_prove(
        pre_states,
        claim_instruction_data_for_pp(fx.vault_id, stream_id),
        vec![0u8, 0, 0, 0, 1, 0],
        vec![(provider_npk.clone(), provider_shared_secret)],
        vec![RECIPIENT_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(guest_program),
    )
    .expect("execute_and_prove claim");

    let holding_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;

    let message = Message::try_from_circuit_output(
        vec![
            fx.vault_config_account_id,
            fx.vault_holding_account_id,
            stream_pda,
            fx.owner_account_id,
            clock_id,
        ],
        vec![],
        vec![(provider_npk, recipient_vpk(), provider_epk)],
        output,
    )
    .expect("try_from_circuit_output for claim");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    fx.state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("claim PP transition");

    // Vault holding decreased by payout
    assert_eq!(
        fx.state
            .get_account_by_id(fx.vault_holding_account_id)
            .balance,
        holding_before - PP_CLAIM_PAYOUT
    );

    // Stream config: accrued cleared, allocation reduced by payout
    let stream_after =
        StreamConfig::from_bytes(&fx.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(stream_after.accrued, 0);
    assert_eq!(stream_after.allocation, PP_STREAM_ALLOCATION - PP_CLAIM_PAYOUT);
    assert_eq!(stream_after.state, StreamState::Active);

    // Vault config: total_allocated reduced by payout
    let vault_after =
        VaultConfig::from_bytes(&fx.state.get_account_by_id(fx.vault_config_account_id).data)
            .expect("vault");
    assert_eq!(
        vault_after.total_allocated,
        PP_STREAM_ALLOCATION - PP_CLAIM_PAYOUT
    );

    // Provider's new commitment decryptable to updated balance
    assert_eq!(tx.message().new_commitments.len(), 1);
    assert_eq!(tx.message().encrypted_private_post_states.len(), 1);
    let new_commitment = &tx.message().new_commitments[0];
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &provider_shared_secret,
        new_commitment,
        0,
    )
    .expect("decrypt provider post-state after claim");
    assert_eq!(decrypted.balance, PP_WITHDRAW_AMOUNT + PP_CLAIM_PAYOUT);
}

#[test]
fn test_pp_close_stream_private_provider_authority_succeeds() {
    let PpClaimCloseSetup {
        mut fx,
        stream_id,
        stream_pda,
        provider_committed_account,
    } = pp_claim_close_setup();

    // Advance clock to T1 so close_at_time sees accrual
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    force_clock_account_monotonic(&mut fx.state, clock_id, 2, PP_T1);

    let guest_program = load_guest_program();
    assert_eq!(guest_program.id(), fx.program_id);

    let authority_npk = recipient_npk();
    let authority_id = AccountId::from(&authority_npk);
    let authority_commitment = Commitment::new(&authority_npk, &provider_committed_account);
    let membership_proof = fx
        .state
        .get_proof_for_commitment(&authority_commitment)
        .expect("authority commitment not found in state after PP withdraw");

    // close_stream accounts: vault_config(0), vault_holding(1), stream_config(2),
    //                        owner(3), authority(4, vis-1), clock(5)
    let pre_states = vec![
        account_meta(&fx.state, fx.vault_config_account_id, false),
        account_meta(&fx.state, fx.vault_holding_account_id, false),
        account_meta(&fx.state, stream_pda, false),
        account_meta(&fx.state, fx.owner_account_id, false),
        AccountWithMetadata {
            account: provider_committed_account.clone(),
            is_authorized: true,
            account_id: authority_id,
        },
        account_meta(&fx.state, clock_id, false),
    ];

    let authority_shared_secret = SharedSecretKey::new(&EPK_SCALAR, &recipient_vpk());
    let authority_epk = EphemeralPublicKey::from_scalar(EPK_SCALAR);

    let vault_total_allocated_before = VaultConfig::from_bytes(
        &fx.state
            .get_account_by_id(fx.vault_config_account_id)
            .data,
    )
    .expect("vault config")
    .total_allocated;

    let (output, proof) = execute_and_prove(
        pre_states,
        close_stream_instruction_data_for_pp(fx.vault_id, stream_id),
        vec![0u8, 0, 0, 0, 1, 0],
        vec![(authority_npk.clone(), authority_shared_secret)],
        vec![RECIPIENT_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(guest_program),
    )
    .expect("execute_and_prove close_stream");

    let message = Message::try_from_circuit_output(
        vec![
            fx.vault_config_account_id,
            fx.vault_holding_account_id,
            stream_pda,
            fx.owner_account_id,
            clock_id,
        ],
        vec![],
        vec![(authority_npk, recipient_vpk(), authority_epk)],
        output,
    )
    .expect("try_from_circuit_output for close_stream");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    fx.state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("close_stream PP transition");

    // Stream is now closed; allocation trimmed to accrued amount
    let stream_after =
        StreamConfig::from_bytes(&fx.state.get_account_by_id(stream_pda).data).expect("stream");
    assert_eq!(stream_after.state, StreamState::Closed);
    let accrued_at_t1 = PP_STREAM_RATE as Balance * (PP_T1 - PP_T0) as Balance;
    assert_eq!(stream_after.allocation, accrued_at_t1);
    assert_eq!(stream_after.accrued, accrued_at_t1);

    // total_allocated reduced by unaccrued amount released back to vault
    let unaccrued = PP_STREAM_ALLOCATION - accrued_at_t1;
    let vault_after =
        VaultConfig::from_bytes(&fx.state.get_account_by_id(fx.vault_config_account_id).data)
            .expect("vault");
    assert_eq!(
        vault_after.total_allocated,
        vault_total_allocated_before - unaccrued
    );

    // Authority's committed balance unchanged (close does not pay out to authority)
    assert_eq!(tx.message().new_commitments.len(), 1);
    assert_eq!(tx.message().encrypted_private_post_states.len(), 1);
    let new_commitment = &tx.message().new_commitments[0];
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &authority_shared_secret,
        new_commitment,
        0,
    )
    .expect("decrypt authority post-state after close_stream");
    assert_eq!(decrypted.balance, PP_WITHDRAW_AMOUNT);
}

// ---- Phase 2: PP deposit ---- //

const OWNER_NSK: NullifierSecretKey = [0x7c; 32];
const OWNER_VSK: Scalar = [0x8d; 32];
const OWNER_FUND_EPK_SCALAR: Scalar = [4u8; 32];
const PP_DEPOSIT_EPK_SCALAR: Scalar = [5u8; 32];
const PP_OWNER_FUND_AMOUNT: Balance = 80;
const PP_DEPOSIT_AMOUNT: Balance = 30;

fn owner_npk() -> NullifierPublicKey {
    NullifierPublicKey::from(&OWNER_NSK)
}

fn owner_vpk() -> ViewingPublicKey {
    ViewingPublicKey::from_scalar(OWNER_VSK)
}

fn load_payment_streams_with_auth_transfer() -> ProgramWithDependencies {
    let payment_streams = load_guest_program();
    let auth_transfer = Program::authenticated_transfer_program();
    ProgramWithDependencies::new(
        payment_streams,
        [(auth_transfer.id(), auth_transfer)].into(),
    )
}

fn deposit_instruction_data(vault_id: crate::VaultId, amount: Balance) -> InstructionData {
    Program::serialize_instruction(crate::Instruction::Deposit {
        vault_id,
        amount,
        authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
    })
    .expect("deposit instruction serializes")
}

#[test]
fn test_pp_deposit_private_owner_succeeds() {
    // Step 1: vault_A — public tier, funded via standard deposit.
    let mut fx_a = vault_fixture_public_tier_funded_via_deposit();

    // Step 2: PP auth_transfer from the public genesis sender to owner_npk (vis-2) so the
    // resulting commitment is owned by authenticated_transfer_program.  A PP payment_streams
    // withdraw would give the new account program_owner = payment_streams, but auth_transfer can
    // only debit accounts it owns, so auth_transfer must be the claiming program here.
    let owner_npk = owner_npk();
    let owner_id = AccountId::from(&owner_npk);
    let owner_fund_shared_secret = SharedSecretKey::new(&OWNER_FUND_EPK_SCALAR, &owner_vpk());
    let owner_fund_epk = EphemeralPublicKey::from_scalar(OWNER_FUND_EPK_SCALAR);

    let fx_a_sender_before = fx_a.state.get_account_by_id(fx_a.owner_account_id);

    let auth_transfer_program = Program::authenticated_transfer_program();
    let pre_states_fund = vec![
        account_meta(&fx_a.state, fx_a.owner_account_id, true), // vis-0 public sender
        AccountWithMetadata {
            account: Account::default(),
            is_authorized: false,
            account_id: owner_id, // vis-2 new private recipient
        },
    ];

    let (fund_output, fund_proof) = execute_and_prove(
        pre_states_fund,
        Program::serialize_instruction(PP_OWNER_FUND_AMOUNT)
            .expect("serialize auth_transfer amount"),
        vec![0u8, 2],
        vec![(owner_npk.clone(), owner_fund_shared_secret)],
        vec![],
        vec![None::<MembershipProof>],
        &ProgramWithDependencies::from(auth_transfer_program),
    )
    .expect("execute_and_prove: fund owner via PP auth_transfer");

    let fund_message = Message::try_from_circuit_output(
        vec![fx_a.owner_account_id],
        vec![fx_a_sender_before.nonce],
        vec![(owner_npk.clone(), owner_vpk(), owner_fund_epk)],
        fund_output,
    )
    .expect("try_from_circuit_output: fund owner");

    let fund_witness =
        WitnessSet::for_message(&fund_message, fund_proof, &[&fx_a.owner_private_key]);
    let fund_tx = PrivacyPreservingTransaction::new(fund_message, fund_witness);

    fx_a.state
        .transition_from_privacy_preserving_transaction(
            &fund_tx,
            3 as BlockId,
            TEST_PUBLIC_TX_TIMESTAMP,
        )
        .expect("transition: fund owner via PP auth_transfer");

    let owner_commitment = &fund_tx.message().new_commitments[0];
    let owner_committed_account = EncryptionScheme::decrypt(
        &fund_tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_fund_shared_secret,
        owner_commitment,
        0,
    )
    .expect("decrypt owner state from PP withdraw");
    assert_eq!(owner_committed_account.balance, PP_OWNER_FUND_AMOUNT);

    // Step 3: Force-insert vault_B accounts (NPK-derived owner, PseudonymousFunder tier).
    // vault_B uses a distinct vault_id so its PDAs don't collide with vault_A's.
    let vault_b_id: crate::VaultId = 2;
    let (vault_config_b_id, vault_holding_b_id) =
        derive_vault_pdas(fx_a.program_id, owner_id, vault_b_id);

    let vault_config_b = Account {
        program_owner: fx_a.program_id,
        balance: 0,
        data: Data::try_from(
            VaultConfig::new(owner_id, vault_b_id, None, Some(VaultPrivacyTier::PseudonymousFunder))
                .to_bytes(),
        )
        .expect("vault_config_b data fits"),
        ..Account::default()
    };
    fx_a.state.force_insert_account(vault_config_b_id, vault_config_b);

    let vault_holding_b = Account {
        program_owner: fx_a.program_id,
        balance: 0,
        data: Data::try_from(VaultHolding::new(None).to_bytes())
            .expect("vault_holding_b data fits"),
        ..Account::default()
    };
    fx_a.state.force_insert_account(vault_holding_b_id, vault_holding_b);

    // Step 4: PP deposit on vault_B with private owner at vis-1.
    let owner_commitment_obj =
        Commitment::new(&owner_npk, &owner_committed_account);
    let membership_proof = fx_a
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment not in state after PP withdraw");

    let deposit_shared_secret = SharedSecretKey::new(&PP_DEPOSIT_EPK_SCALAR, &owner_vpk());
    let deposit_epk = EphemeralPublicKey::from_scalar(PP_DEPOSIT_EPK_SCALAR);

    let pre_states_deposit = vec![
        account_meta(&fx_a.state, vault_config_b_id, false),
        account_meta(&fx_a.state, vault_holding_b_id, false),
        AccountWithMetadata {
            account: owner_committed_account.clone(),
            is_authorized: true,
            account_id: owner_id,
        },
    ];

    let (deposit_output, deposit_proof) = execute_and_prove(
        pre_states_deposit,
        deposit_instruction_data(vault_b_id, PP_DEPOSIT_AMOUNT),
        vec![0u8, 0, 1],
        vec![(owner_npk.clone(), deposit_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &load_payment_streams_with_auth_transfer(),
    )
    .expect("execute_and_prove: PP deposit");

    let holding_b_before = fx_a.state.get_account_by_id(vault_holding_b_id).balance;

    let deposit_message = Message::try_from_circuit_output(
        vec![vault_config_b_id, vault_holding_b_id],
        vec![],
        vec![(owner_npk, owner_vpk(), deposit_epk)],
        deposit_output,
    )
    .expect("try_from_circuit_output: deposit");

    let deposit_witness = WitnessSet::for_message(&deposit_message, deposit_proof, &[]);
    let deposit_tx = PrivacyPreservingTransaction::new(deposit_message, deposit_witness);

    fx_a.state
        .transition_from_privacy_preserving_transaction(
            &deposit_tx,
            4 as BlockId,
            TEST_PUBLIC_TX_TIMESTAMP,
        )
        .expect("PP deposit transition");

    assert_eq!(
        fx_a.state.get_account_by_id(vault_holding_b_id).balance,
        holding_b_before + PP_DEPOSIT_AMOUNT
    );

    assert_eq!(deposit_tx.message().new_commitments.len(), 1);
    assert_eq!(deposit_tx.message().encrypted_private_post_states.len(), 1);
    let new_commitment = &deposit_tx.message().new_commitments[0];
    let decrypted = EncryptionScheme::decrypt(
        &deposit_tx.message().encrypted_private_post_states[0].ciphertext,
        &deposit_shared_secret,
        new_commitment,
        0,
    )
    .expect("decrypt owner post-state after deposit");
    assert_eq!(decrypted.balance, PP_OWNER_FUND_AMOUNT - PP_DEPOSIT_AMOUNT);
}

// ---- Phase 3: PP owner-signer instructions ---- //
//
// The vault owner is a private vis-1 account.  Owner balance never decreases (only vault holding
// does), so program_owner of the owner commitment does not matter for validate_execution rule 5.
// Phase 3 funds the owner via `fund_private_account_via_pp_withdraw` (payment_streams withdraw
// → owner becomes payment_streams-owned), which is fine here.

const PP3_OWNER_FUND_EPK_SCALAR: Scalar = [6u8; 32];
const PP3_SIGNER_EPK_SCALAR: Scalar = [7u8; 32];
const PP3_RECIPIENT_NSK: NullifierSecretKey = [0x9e; 32];
const PP3_RECIPIENT_VSK: Scalar = [0xaf; 32];
const PP3_RECIPIENT_EPK_SCALAR: Scalar = [8u8; 32];
const PP3_VAULT_B_BALANCE: Balance = 400;
const PP3_OWNER_FUND_AMOUNT: Balance = 80;
const PP3_STREAM_RATE: TokensPerSecond = 5;
const PP3_STREAM_ALLOCATION: Balance = 100;
const PP3_TOP_UP_AMOUNT: Balance = 50;
const PP3_T0: Timestamp = 100;
const PP3_T1: Timestamp = 105; // accrued = 5 * (105 - 100) = 25
const PP3_WITHDRAW_AMOUNT: Balance = 30;

fn pp3_recipient_npk() -> NullifierPublicKey {
    NullifierPublicKey::from(&PP3_RECIPIENT_NSK)
}

fn pp3_recipient_vpk() -> ViewingPublicKey {
    ViewingPublicKey::from_scalar(PP3_RECIPIENT_VSK)
}

/// Shared state for Phase 3 PP owner-signer tests.
struct PpOwnerSetup {
    fx: VaultFixture,
    vault_b_id: u64,
    vault_config_b_id: AccountId,
    vault_holding_b_id: AccountId,
    /// Owner's committed account after the funding PP withdraw (private balance = PP3_OWNER_FUND_AMOUNT).
    owner_committed_account: Account,
    owner_npk: NullifierPublicKey,
}

/// Build the shared state for all Phase 3 PP owner-signer tests.
///
/// Ladder: funded vault_A (public tier) → PP withdraw 80 to owner private account →
/// force-insert vault_B (PseudonymousFunder, holding = PP3_VAULT_B_BALANCE) →
/// clock at PP3_T0.
fn pp_owner_setup() -> PpOwnerSetup {
    // Blocks 1–2: initialize_vault + deposit (inside fixture)
    let mut fx = vault_fixture_public_tier_funded_via_deposit();

    // Block 3: PP withdraw from vault_A to fund the owner's private account (vis-2)
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

    // Force-insert vault_B: owner = NPK-derived id, PseudonymousFunder tier, funded holding
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

#[test]
fn test_pp_create_stream_private_owner_succeeds() {
    let mut setup = pp_owner_setup();
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let stream_id = 0u64;
    let stream_pda = derive_stream_pda(setup.fx.program_id, setup.vault_config_b_id, stream_id);
    let provider_id = AccountId::from(&recipient_npk());

    let owner_commitment_obj = Commitment::new(&setup.owner_npk, &setup.owner_committed_account);
    let membership_proof = setup
        .fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after PP withdraw");

    let owner_shared_secret = SharedSecretKey::new(&PP3_SIGNER_EPK_SCALAR, &owner_vpk());
    let owner_epk = EphemeralPublicKey::from_scalar(PP3_SIGNER_EPK_SCALAR);

    // create_stream: vault_config(0), vault_holding(1), stream_config(2), owner(3,vis-1), clock(4)
    let pre_states = vec![
        account_meta(&setup.fx.state, setup.vault_config_b_id, false),
        account_meta(&setup.fx.state, setup.vault_holding_b_id, false),
        account_meta(&setup.fx.state, stream_pda, false),
        AccountWithMetadata {
            account: setup.owner_committed_account.clone(),
            is_authorized: true,
            account_id: AccountId::from(&setup.owner_npk),
        },
        account_meta(&setup.fx.state, clock_id, false),
    ];

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(Instruction::CreateStream {
            vault_id: setup.vault_b_id,
            stream_id,
            provider: provider_id,
            rate: PP3_STREAM_RATE,
            allocation: PP3_STREAM_ALLOCATION,
        })
        .expect("create_stream instruction serializes"),
        vec![0u8, 0, 0, 1, 0],
        vec![(setup.owner_npk.clone(), owner_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP create_stream");

    let message = Message::try_from_circuit_output(
        vec![
            setup.vault_config_b_id,
            setup.vault_holding_b_id,
            stream_pda,
            clock_id,
        ],
        vec![],
        vec![(setup.owner_npk.clone(), owner_vpk(), owner_epk)],
        output,
    )
    .expect("try_from_circuit_output: create_stream");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    setup
        .fx
        .state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("create_stream PP transition");

    let stream =
        StreamConfig::from_bytes(&setup.fx.state.get_account_by_id(stream_pda).data)
            .expect("stream config after create_stream");
    assert_eq!(stream.state, StreamState::Active);
    assert_eq!(stream.rate, PP3_STREAM_RATE);
    assert_eq!(stream.allocation, PP3_STREAM_ALLOCATION);
    assert_eq!(stream.provider, provider_id);

    let vault =
        VaultConfig::from_bytes(&setup.fx.state.get_account_by_id(setup.vault_config_b_id).data)
            .expect("vault config after create_stream");
    assert_eq!(vault.total_allocated, PP3_STREAM_ALLOCATION);
    assert_eq!(vault.next_stream_id, 1);

    // Owner commitment refreshed; balance unchanged
    assert_eq!(tx.message().new_commitments.len(), 1);
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_shared_secret,
        &tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after create_stream");
    assert_eq!(decrypted.balance, PP3_OWNER_FUND_AMOUNT);
}

#[test]
fn test_pp_pause_stream_private_owner_succeeds() {
    let mut setup = pp_owner_setup();
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let stream_id = 0u64;
    let stream_pda = derive_stream_pda(setup.fx.program_id, setup.vault_config_b_id, stream_id);
    let provider_id = AccountId::from(&recipient_npk());

    // Force-insert Active stream at PP3_T0 so pause can fold accrual at PP3_T1
    let stream_config = StreamConfig::new(
        stream_id,
        provider_id,
        PP3_STREAM_RATE,
        PP3_STREAM_ALLOCATION,
        PP3_T0,
        None,
    );
    let stream_account = Account {
        program_owner: setup.fx.program_id,
        balance: 0,
        data: Data::try_from(stream_config.to_bytes()).expect("stream config fits"),
        ..Account::default()
    };
    setup.fx.state.force_insert_account(stream_pda, stream_account);

    patch_vault_config(&mut setup.fx.state, setup.vault_config_b_id, |cfg| {
        cfg.next_stream_id = 1;
        cfg.total_allocated = PP3_STREAM_ALLOCATION;
    });

    // Advance clock to PP3_T1; stream accrues 5 * 5 = 25 tokens
    force_clock_account_monotonic(&mut setup.fx.state, clock_id, 5, PP3_T1);

    let owner_commitment_obj = Commitment::new(&setup.owner_npk, &setup.owner_committed_account);
    let membership_proof = setup
        .fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after PP withdraw");

    let owner_shared_secret = SharedSecretKey::new(&PP3_SIGNER_EPK_SCALAR, &owner_vpk());
    let owner_epk = EphemeralPublicKey::from_scalar(PP3_SIGNER_EPK_SCALAR);

    let pre_states = vec![
        account_meta(&setup.fx.state, setup.vault_config_b_id, false),
        account_meta(&setup.fx.state, setup.vault_holding_b_id, false),
        account_meta(&setup.fx.state, stream_pda, false),
        AccountWithMetadata {
            account: setup.owner_committed_account.clone(),
            is_authorized: true,
            account_id: AccountId::from(&setup.owner_npk),
        },
        account_meta(&setup.fx.state, clock_id, false),
    ];

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(Instruction::PauseStream {
            vault_id: setup.vault_b_id,
            stream_id,
        })
        .expect("pause_stream instruction serializes"),
        vec![0u8, 0, 0, 1, 0],
        vec![(setup.owner_npk.clone(), owner_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP pause_stream");

    let message = Message::try_from_circuit_output(
        vec![
            setup.vault_config_b_id,
            setup.vault_holding_b_id,
            stream_pda,
            clock_id,
        ],
        vec![],
        vec![(setup.owner_npk.clone(), owner_vpk(), owner_epk)],
        output,
    )
    .expect("try_from_circuit_output: pause_stream");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    setup
        .fx
        .state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("pause_stream PP transition");

    let stream =
        StreamConfig::from_bytes(&setup.fx.state.get_account_by_id(stream_pda).data)
            .expect("stream config after pause");
    assert_eq!(stream.state, StreamState::Paused);
    let expected_accrued = PP3_STREAM_RATE as Balance * (PP3_T1 - PP3_T0) as Balance;
    assert_eq!(stream.accrued, expected_accrued);
    assert_eq!(stream.allocation, PP3_STREAM_ALLOCATION);

    assert_eq!(tx.message().new_commitments.len(), 1);
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_shared_secret,
        &tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after pause_stream");
    assert_eq!(decrypted.balance, PP3_OWNER_FUND_AMOUNT);
}

#[test]
fn test_pp_resume_stream_private_owner_succeeds() {
    let mut setup = pp_owner_setup();
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let stream_id = 0u64;
    let stream_pda = derive_stream_pda(setup.fx.program_id, setup.vault_config_b_id, stream_id);
    let provider_id = AccountId::from(&recipient_npk());
    let accrued = PP3_STREAM_RATE as Balance * (PP3_T1 - PP3_T0) as Balance; // 25

    // Force-insert Paused stream with 25 accrued (simulates a prior pause at PP3_T1)
    let mut stream_config = StreamConfig::new(
        stream_id,
        provider_id,
        PP3_STREAM_RATE,
        PP3_STREAM_ALLOCATION,
        PP3_T0,
        None,
    );
    stream_config.state = StreamState::Paused;
    stream_config.accrued = accrued;
    let stream_account = Account {
        program_owner: setup.fx.program_id,
        balance: 0,
        data: Data::try_from(stream_config.to_bytes()).expect("stream config fits"),
        ..Account::default()
    };
    setup.fx.state.force_insert_account(stream_pda, stream_account);

    patch_vault_config(&mut setup.fx.state, setup.vault_config_b_id, |cfg| {
        cfg.next_stream_id = 1;
        cfg.total_allocated = PP3_STREAM_ALLOCATION;
    });

    let owner_commitment_obj = Commitment::new(&setup.owner_npk, &setup.owner_committed_account);
    let membership_proof = setup
        .fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after PP withdraw");

    let owner_shared_secret = SharedSecretKey::new(&PP3_SIGNER_EPK_SCALAR, &owner_vpk());
    let owner_epk = EphemeralPublicKey::from_scalar(PP3_SIGNER_EPK_SCALAR);

    let pre_states = vec![
        account_meta(&setup.fx.state, setup.vault_config_b_id, false),
        account_meta(&setup.fx.state, setup.vault_holding_b_id, false),
        account_meta(&setup.fx.state, stream_pda, false),
        AccountWithMetadata {
            account: setup.owner_committed_account.clone(),
            is_authorized: true,
            account_id: AccountId::from(&setup.owner_npk),
        },
        account_meta(&setup.fx.state, clock_id, false),
    ];

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(Instruction::ResumeStream {
            vault_id: setup.vault_b_id,
            stream_id,
        })
        .expect("resume_stream instruction serializes"),
        vec![0u8, 0, 0, 1, 0],
        vec![(setup.owner_npk.clone(), owner_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP resume_stream");

    let message = Message::try_from_circuit_output(
        vec![
            setup.vault_config_b_id,
            setup.vault_holding_b_id,
            stream_pda,
            clock_id,
        ],
        vec![],
        vec![(setup.owner_npk.clone(), owner_vpk(), owner_epk)],
        output,
    )
    .expect("try_from_circuit_output: resume_stream");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    setup
        .fx
        .state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("resume_stream PP transition");

    let stream =
        StreamConfig::from_bytes(&setup.fx.state.get_account_by_id(stream_pda).data)
            .expect("stream config after resume");
    assert_eq!(stream.state, StreamState::Active);
    assert_eq!(stream.accrued, accrued);
    assert_eq!(stream.allocation, PP3_STREAM_ALLOCATION);

    assert_eq!(tx.message().new_commitments.len(), 1);
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_shared_secret,
        &tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after resume_stream");
    assert_eq!(decrypted.balance, PP3_OWNER_FUND_AMOUNT);
}

#[test]
fn test_pp_top_up_stream_private_owner_succeeds() {
    let mut setup = pp_owner_setup();
    let clock_id = CLOCK_01_PROGRAM_ACCOUNT_ID;
    let stream_id = 0u64;
    let stream_pda = derive_stream_pda(setup.fx.program_id, setup.vault_config_b_id, stream_id);
    let provider_id = AccountId::from(&recipient_npk());
    let depleted_allocation: Balance = PP3_TOP_UP_AMOUNT; // 50

    // Force-insert a fully depleted Paused stream (allocation == accrued == 50)
    let mut stream_config = StreamConfig::new(
        stream_id,
        provider_id,
        PP3_STREAM_RATE,
        depleted_allocation,
        PP3_T0,
        None,
    );
    stream_config.state = StreamState::Paused;
    stream_config.accrued = depleted_allocation;
    let stream_account = Account {
        program_owner: setup.fx.program_id,
        balance: 0,
        data: Data::try_from(stream_config.to_bytes()).expect("stream config fits"),
        ..Account::default()
    };
    setup.fx.state.force_insert_account(stream_pda, stream_account);

    patch_vault_config(&mut setup.fx.state, setup.vault_config_b_id, |cfg| {
        cfg.next_stream_id = 1;
        cfg.total_allocated = depleted_allocation;
    });

    let owner_commitment_obj = Commitment::new(&setup.owner_npk, &setup.owner_committed_account);
    let membership_proof = setup
        .fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after PP withdraw");

    let owner_shared_secret = SharedSecretKey::new(&PP3_SIGNER_EPK_SCALAR, &owner_vpk());
    let owner_epk = EphemeralPublicKey::from_scalar(PP3_SIGNER_EPK_SCALAR);

    let pre_states = vec![
        account_meta(&setup.fx.state, setup.vault_config_b_id, false),
        account_meta(&setup.fx.state, setup.vault_holding_b_id, false),
        account_meta(&setup.fx.state, stream_pda, false),
        AccountWithMetadata {
            account: setup.owner_committed_account.clone(),
            is_authorized: true,
            account_id: AccountId::from(&setup.owner_npk),
        },
        account_meta(&setup.fx.state, clock_id, false),
    ];

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(Instruction::TopUpStream {
            vault_id: setup.vault_b_id,
            stream_id,
            vault_total_allocated_increase: PP3_TOP_UP_AMOUNT,
        })
        .expect("top_up_stream instruction serializes"),
        vec![0u8, 0, 0, 1, 0],
        vec![(setup.owner_npk.clone(), owner_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP top_up_stream");

    let message = Message::try_from_circuit_output(
        vec![
            setup.vault_config_b_id,
            setup.vault_holding_b_id,
            stream_pda,
            clock_id,
        ],
        vec![],
        vec![(setup.owner_npk.clone(), owner_vpk(), owner_epk)],
        output,
    )
    .expect("try_from_circuit_output: top_up_stream");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    setup
        .fx
        .state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("top_up_stream PP transition");

    let stream =
        StreamConfig::from_bytes(&setup.fx.state.get_account_by_id(stream_pda).data)
            .expect("stream config after top_up");
    assert_eq!(stream.state, StreamState::Active);
    assert_eq!(stream.allocation, depleted_allocation + PP3_TOP_UP_AMOUNT);
    assert_eq!(stream.accrued, depleted_allocation);

    let vault =
        VaultConfig::from_bytes(&setup.fx.state.get_account_by_id(setup.vault_config_b_id).data)
            .expect("vault config after top_up");
    assert_eq!(vault.total_allocated, depleted_allocation + PP3_TOP_UP_AMOUNT);

    assert_eq!(tx.message().new_commitments.len(), 1);
    let decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_shared_secret,
        &tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after top_up_stream");
    assert_eq!(decrypted.balance, PP3_OWNER_FUND_AMOUNT);
}

#[test]
fn test_pp_withdraw_private_owner_succeeds() {
    let mut setup = pp_owner_setup();

    let recipient_npk_val = pp3_recipient_npk();
    let recipient_id = AccountId::from(&recipient_npk_val);

    let owner_commitment_obj = Commitment::new(&setup.owner_npk, &setup.owner_committed_account);
    let membership_proof = setup
        .fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after PP withdraw");

    let owner_shared_secret = SharedSecretKey::new(&PP3_SIGNER_EPK_SCALAR, &owner_vpk());
    let owner_epk = EphemeralPublicKey::from_scalar(PP3_SIGNER_EPK_SCALAR);
    let recipient_shared_secret =
        SharedSecretKey::new(&PP3_RECIPIENT_EPK_SCALAR, &pp3_recipient_vpk());
    let recipient_epk = EphemeralPublicKey::from_scalar(PP3_RECIPIENT_EPK_SCALAR);

    let holding_before = setup.fx.state.get_account_by_id(setup.vault_holding_b_id).balance;

    // withdraw: vault_config(0), vault_holding(1), owner(2,vis-1), withdraw_to(3,vis-2)
    let pre_states = vec![
        account_meta(&setup.fx.state, setup.vault_config_b_id, false),
        account_meta(&setup.fx.state, setup.vault_holding_b_id, false),
        AccountWithMetadata {
            account: setup.owner_committed_account.clone(),
            is_authorized: true,
            account_id: AccountId::from(&setup.owner_npk),
        },
        AccountWithMetadata {
            account: Account::default(),
            is_authorized: false,
            account_id: recipient_id,
        },
    ];

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(Instruction::Withdraw {
            vault_id: setup.vault_b_id,
            amount: PP3_WITHDRAW_AMOUNT,
        })
        .expect("withdraw instruction serializes"),
        vec![0u8, 0, 1, 2],
        vec![
            (setup.owner_npk.clone(), owner_shared_secret),
            (recipient_npk_val.clone(), recipient_shared_secret),
        ],
        vec![OWNER_NSK],
        vec![Some(membership_proof), None],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP withdraw private owner");

    let message = Message::try_from_circuit_output(
        vec![setup.vault_config_b_id, setup.vault_holding_b_id],
        vec![],
        vec![
            (setup.owner_npk.clone(), owner_vpk(), owner_epk),
            (recipient_npk_val.clone(), pp3_recipient_vpk(), recipient_epk),
        ],
        output,
    )
    .expect("try_from_circuit_output: withdraw private owner");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    setup
        .fx
        .state
        .transition_from_privacy_preserving_transaction(&tx, 5 as BlockId, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("withdraw private owner PP transition");

    assert_eq!(
        setup.fx.state.get_account_by_id(setup.vault_holding_b_id).balance,
        holding_before - PP3_WITHDRAW_AMOUNT
    );

    assert_eq!(tx.message().new_commitments.len(), 2);
    assert_eq!(tx.message().encrypted_private_post_states.len(), 2);

    // Owner commitment refreshed; balance unchanged
    let owner_decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[0].ciphertext,
        &owner_shared_secret,
        &tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after withdraw");
    assert_eq!(owner_decrypted.balance, PP3_OWNER_FUND_AMOUNT);

    // Recipient receives PP3_WITHDRAW_AMOUNT (output_index=1: second private account)
    let recipient_decrypted = EncryptionScheme::decrypt(
        &tx.message().encrypted_private_post_states[1].ciphertext,
        &recipient_shared_secret,
        &tx.message().new_commitments[1],
        1,
    )
    .expect("decrypt recipient post-state after withdraw");
    assert_eq!(recipient_decrypted.balance, PP3_WITHDRAW_AMOUNT);
}

// ---- Phase 4: PP initialize_vault ---- //

const PP4_FUND_EPK_SCALAR: Scalar = [9u8; 32];
const PP4_INIT_EPK_SCALAR: Scalar = [10u8; 32];
const PP4_OWNER_FUND_AMOUNT: Balance = 50;

#[test]
fn test_pp_initialize_vault_private_owner_succeeds() {
    // Ladder:
    // Blocks 1-2: vault_A initialized and deposited (inside fixture).
    // Block 3:    PP withdraw from vault_A → owner private commitment (vis-2 slot).
    // Block 4:    PP initialize_vault_B with private owner at vis-1; PDAs created as vis-0.

    // Step 1: vault_A — public tier, funded via standard deposit.
    let mut fx = vault_fixture_public_tier_funded_via_deposit();

    // Step 2: Fund the owner's private account via PP withdraw from vault_A.
    let owner_npk = owner_npk();
    let owner_id = AccountId::from(&owner_npk);
    let fund_receipt = fund_private_account_via_pp_withdraw(
        &mut fx,
        &owner_npk,
        owner_vpk(),
        PP4_FUND_EPK_SCALAR,
        PP4_OWNER_FUND_AMOUNT,
        3 as BlockId,
    );
    let owner_committed_account = EncryptionScheme::decrypt(
        &fund_receipt.tx.message().encrypted_private_post_states[0].ciphertext,
        &fund_receipt.shared_secret,
        &fund_receipt.tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner state from funding PP withdraw");
    assert_eq!(owner_committed_account.balance, PP4_OWNER_FUND_AMOUNT);

    // Step 3: Derive vault_B PDAs.  vault_B does not yet exist in state.
    let vault_b_id: crate::VaultId = 2;
    let (vault_config_b_id, vault_holding_b_id) =
        derive_vault_pdas(fx.program_id, owner_id, vault_b_id);

    let owner_commitment_obj = Commitment::new(&owner_npk, &owner_committed_account);
    let membership_proof = fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after fund");

    let init_shared_secret = SharedSecretKey::new(&PP4_INIT_EPK_SCALAR, &owner_vpk());
    let init_epk = EphemeralPublicKey::from_scalar(PP4_INIT_EPK_SCALAR);

    // initialize_vault: vault_config(0, vis-0 init), vault_holding(1, vis-0 init), owner(2, vis-1)
    let pre_states = vec![
        AccountWithMetadata {
            account: Account::default(),
            is_authorized: false,
            account_id: vault_config_b_id,
        },
        AccountWithMetadata {
            account: Account::default(),
            is_authorized: false,
            account_id: vault_holding_b_id,
        },
        AccountWithMetadata {
            account: owner_committed_account.clone(),
            is_authorized: true,
            account_id: owner_id,
        },
    ];

    let (output, proof) = execute_and_prove(
        pre_states,
        Program::serialize_instruction(Instruction::initialize_vault(
            vault_b_id,
            VaultPrivacyTier::PseudonymousFunder,
        ))
        .expect("initialize_vault instruction serializes"),
        vec![0u8, 0, 1],
        vec![(owner_npk.clone(), init_shared_secret)],
        vec![OWNER_NSK],
        vec![Some(membership_proof)],
        &ProgramWithDependencies::from(load_guest_program()),
    )
    .expect("execute_and_prove: PP initialize_vault");

    let message = Message::try_from_circuit_output(
        vec![vault_config_b_id, vault_holding_b_id],
        vec![],
        vec![(owner_npk, owner_vpk(), init_epk)],
        output,
    )
    .expect("try_from_circuit_output: initialize_vault");

    let witness_set = WitnessSet::for_message(&message, proof, &[]);
    let init_tx = PrivacyPreservingTransaction::new(message, witness_set);

    fx.state
        .transition_from_privacy_preserving_transaction(
            &init_tx,
            4 as BlockId,
            TEST_PUBLIC_TX_TIMESTAMP,
        )
        .expect("PP initialize_vault transition");

    // vault_config_b created with correct owner and tier.
    let vault_config_after =
        VaultConfig::from_bytes(&fx.state.get_account_by_id(vault_config_b_id).data)
            .expect("vault_config_b created");
    assert_eq!(vault_config_after.owner, owner_id);
    assert_eq!(vault_config_after.vault_id, vault_b_id);
    assert_eq!(vault_config_after.privacy_tier, VaultPrivacyTier::PseudonymousFunder);
    assert_eq!(vault_config_after.total_allocated, 0);
    assert_eq!(vault_config_after.next_stream_id, 0);

    // vault_holding_b created.
    assert!(
        VaultHolding::from_bytes(&fx.state.get_account_by_id(vault_holding_b_id).data).is_some()
    );

    // Owner commitment refreshed; balance unchanged (owner only signs, balance not debited).
    assert_eq!(init_tx.message().new_commitments.len(), 1);
    assert_eq!(init_tx.message().encrypted_private_post_states.len(), 1);
    let decrypted = EncryptionScheme::decrypt(
        &init_tx.message().encrypted_private_post_states[0].ciphertext,
        &init_shared_secret,
        &init_tx.message().new_commitments[0],
        0,
    )
    .expect("decrypt owner post-state after initialize_vault");
    assert_eq!(decrypted.balance, PP4_OWNER_FUND_AMOUNT);
}
