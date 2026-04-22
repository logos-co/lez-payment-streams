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
    PrivateKey, V03State,
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

/// Public ladder: deploy guest, `initialize_vault`, `deposit` (same shape as `withdraw::test_withdraw_succeeds`).
fn state_after_public_deposit() -> (
    V03State,
    Program,
    PrivateKey,
    AccountId,
    AccountId,
    AccountId,
    u64,
) {
    let owner_balance_start = 1_000 as Balance;
    let deposit_amount = 400 as Balance;
    let block_deposit = 2 as BlockId;
    let nonce_deposit = Nonce(1);

    let mut fx = state_with_initialized_vault(owner_balance_start);
    let program_id = fx.program_id;
    let guest_program = load_guest_program();
    assert_eq!(guest_program.id(), program_id);

    let account_ids_deposit = [
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        fx.owner_account_id,
    ];
    let tx_deposit = build_signed_public_tx(
        program_id,
        Instruction::Deposit {
            vault_id: fx.vault_id,
            amount: deposit_amount,
            authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
        },
        &account_ids_deposit,
        &[nonce_deposit],
        &[&fx.owner_private_key],
    );
    fx.state
        .transition_from_public_transaction(&tx_deposit, block_deposit, TEST_PUBLIC_TX_TIMESTAMP)
        .expect("deposit");

    (
        fx.state,
        guest_program,
        fx.owner_private_key,
        fx.vault_config_account_id,
        fx.vault_holding_account_id,
        fx.owner_account_id,
        fx.vault_id,
    )
}

#[test]
fn test_withdraw_private_recipient_pp_transition_succeeds() {
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 3 as BlockId;

    let (mut state, guest_program, owner_key, vault_cfg, vault_holding, owner_id, vault_id) =
        state_after_public_deposit();

    let npk = recipient_npk();
    let withdraw_to_id = AccountId::from(&npk);

    let holding_before = state.get_account_by_id(vault_holding).balance;
    let owner_before = state.get_account_by_id(owner_id);

    // `is_authorized` must match what `transition_from_privacy_preserving_transaction` rebuilds
    // for proof verification (signer accounts only, not PDAs).
    let pre_states = vec![
        account_meta(&state, vault_cfg, false),
        account_meta(&state, vault_holding, false),
        account_meta(&state, owner_id, true),
        AccountWithMetadata {
            account: Account::default(),
            is_authorized: false,
            account_id: withdraw_to_id,
        },
    ];

    let instruction_data = withdraw_instruction_data(vault_id, withdraw_amount);
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
        vec![vault_cfg, vault_holding, owner_id],
        vec![owner_before.nonce],
        vec![(npk, recipient_vpk(), epk)],
        output,
    )
    .expect("try_from_circuit_output");

    let witness_set = WitnessSet::for_message(&message, proof, &[&owner_key]);
    let tx = PrivacyPreservingTransaction::new(message, witness_set);

    state
        .transition_from_privacy_preserving_transaction(
            &tx,
            block_withdraw,
            TEST_PUBLIC_TX_TIMESTAMP,
        )
        .expect("transition_from_privacy_preserving_transaction");

    assert_eq!(
        state.get_account_by_id(vault_holding).balance,
        holding_before - withdraw_amount
    );
    let owner_after = state.get_account_by_id(owner_id);
    assert_eq!(owner_after.balance, owner_before.balance);
    let mut expected_nonce = owner_before.nonce;
    expected_nonce.public_account_nonce_increment();
    assert_eq!(owner_after.nonce, expected_nonce);

    let cfg = VaultConfig::from_bytes(&state.get_account_by_id(vault_cfg).data).expect("vault");
    assert_eq!(cfg.total_allocated, 0u128);

    assert_eq!(tx.message().new_commitments.len(), 1);
    let commitment = tx.message().new_commitments[0].clone();
    let ciphertext = &tx.message().encrypted_private_post_states[0].ciphertext;
    let decrypted = EncryptionScheme::decrypt(ciphertext, &shared_secret, &commitment, 0)
        .expect("decrypt private withdraw_to post-state");
    assert_eq!(decrypted.balance, withdraw_amount);
}

#[test]
fn test_pp_withdraw_private_recipient_pseudonymous_funded_vault_succeeds() {
    let withdraw_amount = 100 as Balance;
    let block_withdraw = 3 as BlockId;

    let owner_balance_start = 1_000 as Balance;
    let deposit_surrogate = 400 as Balance;
    let mut fx = state_with_initialized_vault_with_privacy_tier(
        owner_balance_start,
        VaultPrivacyTier::PseudonymousFunder,
    );
    transfer_native_balance_for_tests(
        &mut fx.state,
        fx.owner_account_id,
        fx.vault_holding_account_id,
        deposit_surrogate,
    );

    let guest_program = load_guest_program();
    assert_eq!(guest_program.id(), fx.program_id);

    let npk = recipient_npk();
    let withdraw_to_id = AccountId::from(&npk);

    let holding_before = fx
        .state
        .get_account_by_id(fx.vault_holding_account_id)
        .balance;
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

    let cfg = VaultConfig::from_bytes(
        &fx.state
            .get_account_by_id(fx.vault_config_account_id)
            .data,
    )
    .expect("vault");
    assert_eq!(cfg.privacy_tier, VaultPrivacyTier::PseudonymousFunder);
    assert_eq!(cfg.total_allocated, 0u128);
}
