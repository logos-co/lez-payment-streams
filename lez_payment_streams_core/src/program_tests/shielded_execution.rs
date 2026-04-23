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
    account::{Account, AccountId, AccountWithMetadata, Balance, Nonce},
    encryption::{EphemeralPublicKey, Scalar, ViewingPublicKey},
    program::InstructionData,
    BlockId, EncryptionScheme, MembershipProof, NullifierPublicKey, NullifierSecretKey,
    SharedSecretKey,
};

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, load_guest_program, state_with_initialized_vault,
        state_with_initialized_vault_with_privacy_tier, transfer_native_balance_for_tests,
        VaultFixture,
    },
    VaultConfig, VaultPrivacyTier,
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

/// Build and submit a PP `withdraw` from `fx` to a private recipient (visibility-`2` slot).
/// Uses the single-npk recipient material at module scope so both tiers share the same private
/// payout identity.
fn run_pp_withdraw_to_private_recipient(
    fx: &mut VaultFixture,
    withdraw_amount: Balance,
    block_withdraw: BlockId,
) -> PpWithdrawReceipt {
    let guest_program = load_guest_program();
    assert_eq!(guest_program.id(), fx.program_id);

    let npk = recipient_npk();
    let withdraw_to_id = AccountId::from(&npk);

    let owner_before = fx.state.get_account_by_id(fx.owner_account_id);

    // `is_authorized` must match what `transition_from_privacy_preserving_transaction` rebuilds
    // for proof verification (signer accounts only, not PDAs).
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

    let instruction_data = withdraw_instruction_data(fx.vault_id, withdraw_amount);
    let visibility_mask = vec![0u8, 0, 0, 2];
    let shared_secret = SharedSecretKey::new(&EPK_SCALAR, &recipient_vpk());
    let private_keys = vec![(npk.clone(), shared_secret)];
    let private_nsks: Vec<NullifierSecretKey> = vec![];
    let membership_proofs = vec![None::<MembershipProof>];

    let (output, proof) = execute_and_prove(
        pre_states,
        instruction_data,
        visibility_mask,
        private_keys,
        private_nsks,
        membership_proofs,
        &ProgramWithDependencies::from(guest_program),
    )
    .expect("execute_and_prove withdraw");

    let epk = EphemeralPublicKey::from_scalar(EPK_SCALAR);
    let message = Message::try_from_circuit_output(
        vec![
            fx.vault_config_account_id,
            fx.vault_holding_account_id,
            fx.owner_account_id,
        ],
        vec![owner_before.nonce],
        vec![(npk, recipient_vpk(), epk)],
        output,
    )
    .expect("try_from_circuit_output");

    let witness_set = WitnessSet::for_message(&message, proof, &[&fx.owner_private_key]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    fx.state
        .transition_from_privacy_preserving_transaction(
            &tx,
            block_withdraw,
            TEST_PUBLIC_TX_TIMESTAMP,
        )
        .expect("transition_from_privacy_preserving_transaction");

    PpWithdrawReceipt { tx, shared_secret }
}

#[test]
fn test_withdraw_private_recipient_pp_transition_succeeds() {
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 3 as BlockId;

    let mut fx = vault_fixture_public_tier_funded_via_deposit();
    let holding_before = fx.state.get_account_by_id(fx.vault_holding_account_id).balance;
    let owner_before = fx.state.get_account_by_id(fx.owner_account_id);

    let receipt = run_pp_withdraw_to_private_recipient(&mut fx, withdraw_amount, block_withdraw);

    assert_eq!(
        fx.state.get_account_by_id(fx.vault_holding_account_id).balance,
        holding_before - withdraw_amount
    );
    let owner_after = fx.state.get_account_by_id(fx.owner_account_id);
    assert_eq!(owner_after.balance, owner_before.balance);
    let mut expected_nonce = owner_before.nonce;
    expected_nonce.public_account_nonce_increment();
    assert_eq!(owner_after.nonce, expected_nonce);

    let cfg = VaultConfig::from_bytes(
        &fx.state.get_account_by_id(fx.vault_config_account_id).data,
    )
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
    let holding_before = fx.state.get_account_by_id(fx.vault_holding_account_id).balance;
    let owner_before = fx.state.get_account_by_id(fx.owner_account_id);

    let _receipt = run_pp_withdraw_to_private_recipient(&mut fx, withdraw_amount, block_withdraw);

    assert_eq!(
        fx.state.get_account_by_id(fx.vault_holding_account_id).balance,
        holding_before - withdraw_amount
    );
    let owner_after = fx.state.get_account_by_id(fx.owner_account_id);
    assert_eq!(owner_after.balance, owner_before.balance);
    let mut expected_nonce = owner_before.nonce;
    expected_nonce.public_account_nonce_increment();
    assert_eq!(owner_after.nonce, expected_nonce);

    let cfg = VaultConfig::from_bytes(
        &fx.state.get_account_by_id(fx.vault_config_account_id).data,
    )
    .expect("vault");
    assert_eq!(cfg.privacy_tier, VaultPrivacyTier::PseudonymousFunder);
    assert_eq!(cfg.total_allocated, 0u128);
}
