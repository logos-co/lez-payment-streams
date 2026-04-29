//! `initialize_vault` PDAs and vault bytes.

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_vault_pdas,
    },
    StreamId, VaultConfig, VaultHolding, VaultId, VaultPrivacyTier, DEFAULT_VERSION,
};
use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use super::common::{DEFAULT_OWNER_GENESIS_BALANCE, TEST_PUBLIC_TX_TIMESTAMP};
use crate::harness_seeds::SEED_OWNER;

#[test]
fn test_initialize_vault_then_reinitialize_fails() {
    let owner_genesis_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let initial_accounts_data = vec![(owner_account_id, owner_genesis_balance)];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect(
            "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
        );
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let block_init = 1 as BlockId;
    let block_reinit = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_reinit = Nonce(1);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    // One nonce per signer account.
    let instruction_init = Instruction::initialize_vault_public(vault_id);
    let tx_init = build_signed_public_tx(
        program_id,
        instruction_init,
        &account_ids,
        &[nonce_init],
        &[&owner_private_key],
    );

    let result =
        state.transition_from_public_transaction(&tx_init, block_init, TEST_PUBLIC_TX_TIMESTAMP);
    assert!(result.is_ok(), "initialize_vault tx failed: {:?}", result);
    let vault_config_account = state.get_account_by_id(vault_config_account_id);
    let vault_config =
        borsh::from_slice::<VaultConfig>(&vault_config_account.data).expect("valid vault config bytes");
    assert_eq!(vault_config.version, DEFAULT_VERSION);
    assert_eq!(vault_config.owner, owner_account_id);
    assert_eq!(vault_config.vault_id, vault_id);
    assert_eq!(vault_config.next_stream_id, StreamId::MIN);
    assert_eq!(vault_config.total_allocated, 0 as Balance);
    assert_eq!(vault_config.privacy_tier, crate::VaultPrivacyTier::Public);
    let vault_holding_account = state.get_account_by_id(vault_holding_account_id);
    let vault_holding =
        borsh::from_slice::<VaultHolding>(&vault_holding_account.data).expect("valid vault holding bytes");
    assert_eq!(vault_holding.version, DEFAULT_VERSION);

    // Second `init` hits SPEL validation before program `ERR_*` strings. Expect `is_err()` only.
    let instruction_reinit = Instruction::initialize_vault_public(vault_id);
    let tx_reinit = build_signed_public_tx(
        program_id,
        instruction_reinit,
        &account_ids,
        &[nonce_reinit],
        &[&owner_private_key],
    );
    let result = state.transition_from_public_transaction(
        &tx_reinit,
        block_reinit,
        TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result.is_err(),
        "repeated initialize_vault tx succeeded: {:?}",
        result
    );
}

#[test]
fn test_initialize_vault_wrong_signer_witness_fails() {
    let owner_genesis_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let (_, owner_account_id) = create_keypair(SEED_OWNER);
    let (alt_private_key, _) = create_keypair(crate::harness_seeds::SEED_ALT_SIGNER);
    let initial_accounts_data = vec![(owner_account_id, owner_genesis_balance)];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect(
            "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
        );
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let block_init = 1 as BlockId;
    let nonce_init = Nonce(0);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    let tx_wrong_signer = build_signed_public_tx(
        program_id,
        Instruction::initialize_vault_public(vault_id),
        &account_ids,
        &[nonce_init],
        &[&alt_private_key],
    );
    let result = state.transition_from_public_transaction(
        &tx_wrong_signer,
        block_init,
        TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result.is_err(),
        "initialize_vault with non-owner witness should fail: {result:?}"
    );
}

#[test]
fn test_initialize_vault_pseudonymous_funder_succeeds() {
    let owner_genesis_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let (owner_private_key, owner_account_id) = create_keypair(SEED_OWNER);
    let initial_accounts_data = vec![(owner_account_id, owner_genesis_balance)];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect(
            "guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok",
        );
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id,
    ];

    let tx = build_signed_public_tx(
        program_id,
        Instruction::initialize_vault(vault_id, VaultPrivacyTier::PseudonymousFunder),
        &account_ids,
        &[Nonce(0)],
        &[&owner_private_key],
    );
    let result =
        state.transition_from_public_transaction(&tx, 1 as BlockId, TEST_PUBLIC_TX_TIMESTAMP);
    assert!(result.is_ok(), "{result:?}");
    let vc = borsh::from_slice::<VaultConfig>(&state.get_account_by_id(vault_config_account_id).data)
        .expect("vault config");
    assert_eq!(vc.privacy_tier, VaultPrivacyTier::PseudonymousFunder);
}

// ---- PP tests ---- //

use nssa::{
    execute_and_prove,
    privacy_preserving_transaction::{
        circuit::ProgramWithDependencies,
        message::Message,
        witness_set::WitnessSet,
        PrivacyPreservingTransaction,
    },
    program::Program,
};
use nssa_core::{
    account::{Account, AccountId, AccountWithMetadata},
    encryption::EphemeralPublicKey,
    Commitment, EncryptionScheme, SharedSecretKey,
};
use crate::test_helpers::load_guest_program;
use super::pp_common::{
    fund_private_account_via_pp_withdraw, owner_npk, owner_vpk,
    vault_fixture_public_tier_funded_via_deposit,
    OWNER_NSK, PP4_FUND_EPK_SCALAR, PP4_INIT_EPK_SCALAR, PP4_OWNER_FUND_AMOUNT,
};

#[test]
fn test_pp_initialize_vault_private_owner_succeeds() {
    let mut fx = vault_fixture_public_tier_funded_via_deposit();

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

    let vault_b_id = 2u64;
    let (vault_config_b_id, vault_holding_b_id) =
        derive_vault_pdas(fx.program_id, owner_id, vault_b_id);

    let owner_commitment_obj = Commitment::new(&owner_npk, &owner_committed_account);
    let membership_proof = fx
        .state
        .get_proof_for_commitment(&owner_commitment_obj)
        .expect("owner commitment in state after fund");

    let init_shared_secret = SharedSecretKey::new(&PP4_INIT_EPK_SCALAR, &owner_vpk());
    let init_epk = EphemeralPublicKey::from_scalar(PP4_INIT_EPK_SCALAR);

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

    let vault_config_after =
        borsh::from_slice::<VaultConfig>(&fx.state.get_account_by_id(vault_config_b_id).data)
            .expect("vault_config_b created");
    assert_eq!(vault_config_after.owner, owner_id);
    assert_eq!(vault_config_after.vault_id, vault_b_id);
    assert_eq!(vault_config_after.privacy_tier, VaultPrivacyTier::PseudonymousFunder);
    assert_eq!(vault_config_after.total_allocated, 0);
    assert_eq!(vault_config_after.next_stream_id, 0);

    assert!(
        borsh::from_slice::<VaultHolding>(&fx.state.get_account_by_id(vault_holding_b_id).data).is_ok()
    );

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
