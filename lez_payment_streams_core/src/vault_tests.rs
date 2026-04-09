use nssa_core::{account::{AccountId, Balance, Nonce}, program::BlockId};
use spel_framework_core::pda::{compute_pda, seed_from_str};

use crate::{DEFAULT_VERSION, VaultConfig, VaultHolding, VaultId, test_helpers::{build_public_tx, create_keypair, create_state_with_guest_program}};
use crate::test_helpers::seed_from_u64;
use crate::Instruction;

#[test]
fn vault_tests_keypair_is_deterministic_for_seed() {
    let (_, first) = create_keypair(7);
    let (_, second) = create_keypair(7);
    assert_eq!(first, second);
}


#[test]
fn vault_config_roundtrip_serialization() {
    let vault_config = VaultConfig::new(AccountId::new([43; 32]), 34u64);
    let serialized = vault_config.to_bytes();
    let deserialized = VaultConfig::from_bytes(&serialized);
    assert_eq!(Some(vault_config), deserialized);
}

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

#[test]
fn test_initialize_vault_then_reinitialize_fails() {
    let (owner_private_key, owner_account_id) = create_keypair(1);
    let initial_accounts_data = vec![(owner_account_id, 1_000 as Balance)];
    let (mut state, guest_program) = create_state_with_guest_program(&initial_accounts_data).unwrap();
    let program_id = guest_program.id();

    // vault config PDA:
    // [b"vault_config", owner, vault_id]
    let vault_id: VaultId = 1;
    let vault_config_seed_1 = seed_from_str("vault_config");
    let vault_config_seed_2 = *owner_account_id.value();
    let vault_config_seed_3 = seed_from_u64(vault_id);
    let vault_config_account_id = compute_pda(
        &program_id, &[
            &vault_config_seed_1,
            &vault_config_seed_2,
            &vault_config_seed_3,
        ]
    );

    // vault holding PDA:
    // [b"vault_holding", vault_config_pda, asset_tag]
    // asset_tag is "native" for native balance
    let vault_holding_seed_1 = seed_from_str("vault_holding");
    let vault_holding_seed_2 = *vault_config_account_id.value();
    let vault_holding_seed_3 = seed_from_str("native");
    let vault_holding_account_id = compute_pda(
        &program_id,
        &[
            &vault_holding_seed_1,
            &vault_holding_seed_2,
            &vault_holding_seed_3,
        ]
    );
    let account_ids = [
        vault_config_account_id,
        vault_holding_account_id,
        owner_account_id
    ];
    // one nonce per _signer_ account (not every account!)
    let nonces_init = vec![Nonce(0)];
    let instruction_init = Instruction::InitializeVault { vault_id };
    let tx_init = build_public_tx(
        program_id,
        &account_ids,
        &nonces_init,
        instruction_init,
        &[&owner_private_key],
    );

    let result = state.transition_from_public_transaction(&tx_init, 1 as BlockId);
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
    let nonces_reinit = vec![Nonce(1)];
    let instruction_reinit = Instruction::InitializeVault { vault_id };
    let tx_reinit = build_public_tx(
        program_id,
        &account_ids,
        &nonces_reinit,
        instruction_reinit,
        &[&owner_private_key],
    );
    let result = state.transition_from_public_transaction(&tx_reinit, 2 as BlockId);
    assert!(result.is_err(), "repeated initialize_vault tx succeeded: {:?}", result);

}
