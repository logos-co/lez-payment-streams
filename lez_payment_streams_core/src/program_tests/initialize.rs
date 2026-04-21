//! `initialize_vault` PDAs and vault bytes.

use crate::Instruction;
use crate::{
    test_helpers::{
        build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_vault_pdas,
    },
    StreamId, VaultConfig, VaultHolding, VaultId, DEFAULT_VERSION,
};
use nssa_core::{
    account::{Balance, Nonce},
    BlockId,
};

use super::common::DEFAULT_OWNER_GENESIS_BALANCE;
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
    let instruction_init = Instruction::InitializeVault { vault_id };
    let tx_init = build_signed_public_tx(
        program_id,
        instruction_init,
        &account_ids,
        &[nonce_init],
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(
        &tx_init,
        block_init,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(result.is_ok(), "initialize_vault tx failed: {:?}", result);
    let vault_config_account = state.get_account_by_id(vault_config_account_id);
    assert_eq!(vault_config_account.data.len(), VaultConfig::SIZE);
    let vault_config =
        VaultConfig::from_bytes(&vault_config_account.data).expect("valid vault config bytes");
    assert_eq!(vault_config.version, DEFAULT_VERSION);
    assert_eq!(vault_config.owner, owner_account_id);
    assert_eq!(vault_config.vault_id, vault_id);
    assert_eq!(vault_config.next_stream_id, StreamId::MIN);
    assert_eq!(vault_config.total_allocated, 0 as Balance);
    let vault_holding_account = state.get_account_by_id(vault_holding_account_id);
    assert_eq!(vault_holding_account.data.len(), VaultHolding::SIZE);
    let vault_holding =
        VaultHolding::from_bytes(&vault_holding_account.data).expect("valid vault holding bytes");
    assert_eq!(vault_holding.version, DEFAULT_VERSION);

    // Second `init` hits SPEL validation before program `ERR_*` strings. Expect `is_err()` only.
    let instruction_reinit = Instruction::InitializeVault { vault_id };
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
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
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
        Instruction::InitializeVault { vault_id },
        &account_ids,
        &[nonce_init],
        &[&alt_private_key],
    );
    let result = state.transition_from_public_transaction(
        &tx_wrong_signer,
        block_init,
        crate::program_tests::common::TEST_PUBLIC_TX_TIMESTAMP,
    );
    assert!(
        result.is_err(),
        "initialize_vault with non-owner witness should fail: {result:?}"
    );
}
