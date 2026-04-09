use nssa_core::account::AccountId;

use crate::{VaultConfig, VaultHolding, test_helpers::create_keypair};

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