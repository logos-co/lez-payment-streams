use nssa_core::{
    account::{Balance, Nonce},
    program::BlockId,
};
use crate::{
    DEFAULT_VERSION, StreamId,
    VaultConfig, VaultHolding, VaultId,
    test_helpers::{build_signed_public_tx, create_keypair, create_state_with_guest_program, derive_vault_pdas},
};
use crate::Instruction;

use super::common::DEFAULT_OWNER_GENESIS_BALANCE;

#[test]
fn test_initialize_vault_then_reinitialize_fails() {
    let owner_genesis_balance = DEFAULT_OWNER_GENESIS_BALANCE;
    let (owner_private_key, owner_account_id) = create_keypair(1);
    let initial_accounts_data = vec![(owner_account_id, owner_genesis_balance)];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data)
        .expect("guest image present (cargo build -p lez_payment_streams-methods) and state genesis ok");
    let program_id = guest_program.id();

    let vault_id = VaultId::from(1u64);
    let block_init = 1 as BlockId;
    let block_reinit = 2 as BlockId;
    let nonce_init = Nonce(0);
    let nonce_reinit = Nonce(1);
    let (vault_config_account_id, vault_holding_account_id) =
        derive_vault_pdas(program_id, owner_account_id, vault_id);
    let account_ids = [vault_config_account_id, vault_holding_account_id, owner_account_id];

    // one nonce per _signer_ account (not every account!)
    let instruction_init = Instruction::InitializeVault { vault_id };
    let tx_init = build_signed_public_tx(
        program_id,
        instruction_init,
        &account_ids,
        &[nonce_init],
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
    assert_eq!(vault_config.next_stream_id, StreamId::MIN);
    assert_eq!(vault_config.total_allocated, Balance::MIN);
    let vault_holding_account = state.get_account_by_id(vault_holding_account_id);
    assert_eq!(vault_holding_account.data.len(), VaultHolding::SIZE);
    let vault_holding = VaultHolding::from_bytes(&vault_holding_account.data).expect("valid vault holding bytes");
    assert_eq!(vault_holding.version, DEFAULT_VERSION);

    // negative test: re-initialization must fail (SPEL reports init-on-existing during account
    // validation; the host sees an opaque transaction error, not a structured SpelError here).
    let instruction_reinit = Instruction::InitializeVault { vault_id };
    let tx_reinit = build_signed_public_tx(
        program_id,
        instruction_reinit,
        &account_ids,
        &[nonce_reinit],
        &[&owner_private_key],
    );
    let result = state.transition_from_public_transaction(&tx_reinit, block_reinit);
    assert!(result.is_err(), "repeated initialize_vault tx succeeded: {:?}", result);

}
