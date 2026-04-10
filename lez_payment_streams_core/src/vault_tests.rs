use nssa_core::{account::{AccountId, Balance, Nonce}, program::BlockId};
use nssa::program::Program;

use crate::{DEFAULT_VERSION, VaultConfig, VaultHolding, VaultId, test_helpers::{build_public_tx, create_keypair, create_state_with_guest_program, derive_vault_pdas}};
use crate::Instruction;


// ---- Key derivation ---- //

#[test]
fn vault_tests_keypair_is_deterministic_for_seed() {
    let (_, first) = create_keypair(7);
    let (_, second) = create_keypair(7);
    assert_eq!(first, second);
}


// ---- Serialization --- //

#[test]
fn vault_config_roundtrip_serialization() {
    let vault_config = VaultConfig::new(AccountId::new([43; 32]), 34u64);
    let serialized = vault_config.to_bytes();
    let deserialized = VaultConfig::from_bytes(&serialized);
    assert_eq!(Some(vault_config), deserialized);
}

// ---- Vault initialization ---- //

#[test]
fn vault_config_from_bytes_wrong_len_returns_none() {
    let vault_config = VaultConfig::new(AccountId::new([43; 32]), 34u64);
    let bytes = vault_config.to_bytes();
    let short = &bytes[..bytes.len() - 1];
    assert!(VaultConfig::from_bytes(short).is_none());
    let mut long = bytes.clone();
    long.push(0);
    assert!(VaultConfig::from_bytes(&long).is_none());
}

#[test]
fn vault_holding_roundtrip_serialization() {
    let vault_holding = VaultHolding::new();
    let serialized = vault_holding.to_bytes();
    let deserialized = VaultHolding::from_bytes(&serialized);
    assert_eq!(Some(vault_holding), deserialized);
}

#[test]
fn vault_holding_from_bytes_wrong_len_returns_none() {
    let vault_holding = VaultHolding::new();
    let bytes = vault_holding.to_bytes();
    let short = &bytes[..bytes.len() - 1];
    assert!(VaultHolding::from_bytes(short).is_none());
    let mut long = bytes.clone();
    long.push(0);
    assert!(VaultHolding::from_bytes(&long).is_none());
}


// ---- Vault functionality ---- //

#[test]
fn test_initialize_vault_then_reinitialize_fails() {
    let (owner_private_key, owner_account_id) = create_keypair(1);
    let initial_accounts_data = vec![(owner_account_id, 1_000 as Balance)];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data).unwrap();
    let program_id = guest_program.id();

    let vault_id: VaultId = 1;
    let block_init = 1 as BlockId;
    let block_reinit = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_reinit = Nonce(1);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];
    
    // one nonce per _signer_ account (not every account!)
    let nonces_init = vec![nonce_init];
    let instruction_init = Instruction::InitializeVault { vault_id };
    let tx_init = build_public_tx(
        program_id,
        &account_ids,
        &nonces_init,
        instruction_init,
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_init, block_init);
    assert!(result.is_ok(), "initialize_vault tx failed: {:?}", result);
    let vault_config_account = state.get_account_by_id(vault_config_account_id);
    assert_eq!(vault_config_account.data.len(), VaultConfig::SIZE);
    let vault_config = VaultConfig::from_bytes(&vault_config_account.data).expect("valid vault config bytes");
    assert_eq!(vault_config.version, DEFAULT_VERSION);
    assert_eq!(vault_config.owner, owner_account_id);
    assert_eq!(vault_config.vault_id, vault_id);
    assert_eq!(vault_config.next_stream_id, 0);
    assert_eq!(vault_config.total_allocated, 0);
    let vault_holding_account = state.get_account_by_id(vault_holding_account_id);
    assert_eq!(vault_holding_account.data.len(), VaultHolding::SIZE);
    let vault_holding = VaultHolding::from_bytes(&vault_holding_account.data).expect("valid vault holding bytes");
    assert_eq!(vault_holding.version, DEFAULT_VERSION);

    // negative test: re-initialization must fail
    let nonces_reinit = vec![nonce_reinit];
    let instruction_reinit = Instruction::InitializeVault { vault_id };
    let tx_reinit = build_public_tx(
        program_id,
        &account_ids,
        &nonces_reinit,
        instruction_reinit,
        &[&owner_private_key],
    );
    let result = state.transition_from_public_transaction(&tx_reinit, block_reinit);
    // TODO: assert error is SpelError::AccountAlreadyInitialized (1002)
    assert!(result.is_err(), "repeated initialize_vault tx succeeded: {:?}", result);

}


// ---- Deposit and withdraw ---- //
#[test]
fn test_deposit() {
    let owner_balance_before = 1_000 as Balance;
    let deposit_amount = 300 as Balance;
    let block_init = 1 as BlockId;
    let block_deposit = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_deposit = Nonce(1);

    let (owner_private_key, owner_account_id) = create_keypair(1);
    let initial_accounts_data = vec![(owner_account_id, owner_balance_before)];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data).unwrap();
    let program_id = guest_program.id();

    // initialize vault
    let vault_id: VaultId = 1;
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];
    let nonces_init = vec![nonce_init];
    let instruction_init = Instruction::InitializeVault { vault_id };
    let tx_init = build_public_tx(
        program_id,
        &account_ids,
        &nonces_init,
        instruction_init,
        &[&owner_private_key],
    );

    let result_init = state.transition_from_public_transaction(&tx_init, block_init);
    assert!(result_init.is_ok(), "initialize_vault tx failed: {:?}", result_init);

    let vault_config_before = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_before =
        VaultConfig::from_bytes(&vault_config_before.data).expect("valid vault config bytes");
    let nonces_deposit = vec![nonce_deposit];
    let instruction_deposit = Instruction::Deposit {
        vault_id,
        amount: deposit_amount,
        authenticated_transfer_program_id: Program::authenticated_transfer_program().id(),
    };
    let tx_deposit = build_public_tx(
        program_id,
        &account_ids,
        &nonces_deposit,
        instruction_deposit,
        &[&owner_private_key],
    );

    let vault_holding_balance_before = state.get_account_by_id(vault_holding_account_id).balance;

    let result_deposit = state.transition_from_public_transaction(&tx_deposit, block_deposit);
    assert!(result_deposit.is_ok(), "deposit tx failed: {:?}", result_deposit);

    let owner_balance_after = state.get_account_by_id(owner_account_id).balance;
    let vault_holding_balance_after = state.get_account_by_id(vault_holding_account_id).balance;
    let vault_config_after = state.get_account_by_id(vault_config_account_id);
    let vault_config_state_after =
        VaultConfig::from_bytes(&vault_config_after.data).expect("valid vault config bytes");
    
    assert_eq!(owner_balance_after, owner_balance_before - deposit_amount);
    assert_eq!(vault_holding_balance_after, vault_holding_balance_before + deposit_amount);
    assert_eq!(
        vault_config_state_after.total_allocated,
        vault_config_state_before.total_allocated
    );
    assert_eq!(
        vault_config_state_after.next_stream_id,
        vault_config_state_before.next_stream_id
    );
    assert_eq!(vault_config_state_after.version, vault_config_state_before.version);
    assert_eq!(vault_config_state_after.owner, vault_config_state_before.owner);
    assert_eq!(vault_config_state_after.vault_id, vault_config_state_before.vault_id);
}