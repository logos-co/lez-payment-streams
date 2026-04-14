//! Layout roundtrips for vault and stream account payloads,
//! plus deterministic test harness keys (`create_keypair`).

use std::mem::size_of;

use nssa_core::account::AccountId;

use crate::test_helpers::create_keypair;
use crate::{StreamConfig, StreamState, Timestamp, VaultConfig, VaultHolding, VaultId};

#[test]
fn test_keypair_is_deterministic_for_seed() {
    let (_, first) = create_keypair(7);
    let (_, second) = create_keypair(7);
    assert_eq!(first, second);
}

#[test]
fn test_vault_config_roundtrip_serialization() {
    let vault_config = VaultConfig::new(AccountId::new([43; 32]), VaultId::from(34u64));
    let serialized = vault_config.to_bytes();
    let deserialized = VaultConfig::from_bytes(&serialized);
    assert_eq!(Some(vault_config), deserialized);
}

#[test]
fn test_vault_config_from_bytes_wrong_len_returns_none() {
    let vault_config = VaultConfig::new(AccountId::new([43; 32]), VaultId::from(34u64));
    let bytes = vault_config.to_bytes();
    let short = &bytes[..bytes.len() - 1];
    assert!(VaultConfig::from_bytes(short).is_none());
    let mut long = bytes.clone();
    long.push(0);
    assert!(VaultConfig::from_bytes(&long).is_none());
}

#[test]
fn test_vault_holding_roundtrip_serialization() {
    let vault_holding = VaultHolding::new();
    let serialized = vault_holding.to_bytes();
    let deserialized = VaultHolding::from_bytes(&serialized);
    assert_eq!(Some(vault_holding), deserialized);
}

#[test]
fn test_vault_holding_from_bytes_wrong_len_returns_none() {
    let vault_holding = VaultHolding::new();
    let bytes = vault_holding.to_bytes();
    let short = &bytes[..bytes.len() - 1];
    assert!(VaultHolding::from_bytes(short).is_none());
    let mut long = bytes.clone();
    long.push(0);
    assert!(VaultHolding::from_bytes(&long).is_none());
}

#[test]
fn test_stream_config_roundtrip_serialization() {
    let (_, provider) = create_keypair(5);
    let stream_config = StreamConfig::new(7, provider, 10, 200, 12_345);
    let serialized = stream_config.to_bytes();
    let deserialized = StreamConfig::from_bytes(&serialized);
    assert_eq!(Some(stream_config), deserialized);
}

#[test]
fn test_stream_config_from_bytes_wrong_len_returns_none() {
    let (_, provider) = create_keypair(5);
    let stream_config = StreamConfig::new(7, provider, 10, 200, 12_345);
    let bytes = stream_config.to_bytes();
    let short = &bytes[..bytes.len() - 1];
    assert!(StreamConfig::from_bytes(short).is_none());
    let mut long = bytes.clone();
    long.push(0);
    assert!(StreamConfig::from_bytes(&long).is_none());
}

#[test]
fn test_stream_config_from_bytes_invalid_stream_state_returns_none() {
    // Use one past the largest defined `StreamState` discriminant so the byte is never a valid
    // variant, without hard-coding a magic invalid value. Keep the list in sync when adding variants.
    let highest_defined_discriminant = [
        StreamState::Active,
        StreamState::Paused,
        StreamState::Closed,
    ]
    .map(|state| state as u8)
    .into_iter()
    .max()
    .expect("StreamState should have variants");
    assert!(
        highest_defined_discriminant < u8::MAX,
        "need a gap above the highest defined discriminant for this test"
    );
    let undefined_discriminant_after_max = highest_defined_discriminant + 1;

    let (_, provider) = create_keypair(5);
    let valid_config = StreamConfig::new(0, provider, 1, 1, 1);
    let mut serialized = valid_config.to_bytes();
    let timestamp_field_size = size_of::<Timestamp>();
    let stream_state_byte_index = serialized.len() - timestamp_field_size - 1;
    serialized[stream_state_byte_index] = undefined_discriminant_after_max;
    assert!(StreamConfig::from_bytes(&serialized).is_none());
}
