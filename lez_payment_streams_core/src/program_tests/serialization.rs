//! Vault and stream account payload roundtrips.
//! Use deterministic [`crate::test_helpers::create_keypair`] outputs in layout tests.
//!
//! Use local byte constants separate from [`crate::harness_seeds`] so stream state experiments stay isolated.

use std::mem::size_of;

use nssa_core::account::AccountId;

use crate::test_helpers::create_keypair;
use crate::{
    StreamConfig, StreamState, Timestamp, VaultConfig, VaultHolding, VaultId, VaultPrivacyTier,
    VersionId,
};

/// Seed constants for layout tests only.
/// Distinct from [`crate::harness_seeds`] integration values.
const SERIALIZATION_SEED_KEYPAIR: u8 = 7;
const SERIALIZATION_SEED_PROVIDER: u8 = 5;

#[test]
fn test_keypair_is_deterministic_for_seed() {
    for &seed in &[SERIALIZATION_SEED_KEYPAIR, SERIALIZATION_SEED_PROVIDER] {
        let (_, first) = create_keypair(seed);
        let (_, second) = create_keypair(seed);
        assert_eq!(first, second);
    }
}

#[test]
fn test_vault_config_roundtrip_serialization_succeeds() {
    let vault_config = VaultConfig::new(
        AccountId::new([43; 32]),
        VaultId::from(34u64),
        None::<VersionId>,
        None::<VaultPrivacyTier>,
    );
    let serialized = borsh::to_vec(&vault_config).unwrap();
    let deserialized = borsh::from_slice::<VaultConfig>(&serialized).unwrap();
    assert_eq!(vault_config, deserialized);
}

#[test]
fn test_vault_config_roundtrip_pseudonymous_funder_tier_succeeds() {
    let vault_config = VaultConfig::new(
        AccountId::new([44; 32]),
        VaultId::from(35u64),
        None::<VersionId>,
        Some(VaultPrivacyTier::PseudonymousFunder),
    );
    let serialized = borsh::to_vec(&vault_config).unwrap();
    let deserialized = borsh::from_slice::<VaultConfig>(&serialized).unwrap();
    assert_eq!(vault_config, deserialized);
}

#[test]
fn test_vault_config_from_bytes_wrong_len_fails() {
    let vault_config = VaultConfig::new(
        AccountId::new([43; 32]),
        VaultId::from(34u64),
        None::<VersionId>,
        None::<VaultPrivacyTier>,
    );
    let bytes = borsh::to_vec(&vault_config).unwrap();
    let short = &bytes[..bytes.len() - 1];
    assert!(borsh::from_slice::<VaultConfig>(short).is_err());
    let mut long = bytes.clone();
    long.push(0);
    assert!(borsh::from_slice::<VaultConfig>(&long).is_err());
}

#[test]
fn test_vault_holding_roundtrip_serialization_succeeds() {
    let vault_holding = VaultHolding::new(None::<VersionId>);
    let serialized = borsh::to_vec(&vault_holding).unwrap();
    let deserialized = borsh::from_slice::<VaultHolding>(&serialized).unwrap();
    assert_eq!(vault_holding, deserialized);
}

#[test]
fn test_vault_holding_from_bytes_wrong_len_fails() {
    let vault_holding = VaultHolding::new(None::<VersionId>);
    let bytes = borsh::to_vec(&vault_holding).unwrap();
    let short = &bytes[..bytes.len() - 1];
    assert!(borsh::from_slice::<VaultHolding>(short).is_err());
    let mut long = bytes.clone();
    long.push(0);
    assert!(borsh::from_slice::<VaultHolding>(&long).is_err());
}

#[test]
fn test_stream_config_roundtrip_serialization_succeeds() {
    let (_, provider) = create_keypair(SERIALIZATION_SEED_PROVIDER);
    let s_original = StreamConfig::new(7, provider, 10, 200, 12_345, None::<VersionId>);
    let serialized = borsh::to_vec(&s_original).unwrap();
    let deserialized = borsh::from_slice::<StreamConfig>(&serialized).unwrap();
    assert_eq!(s_original, deserialized);
}

#[test]
fn test_stream_config_from_bytes_wrong_len_fails() {
    let (_, provider) = create_keypair(SERIALIZATION_SEED_PROVIDER);
    let s_original = StreamConfig::new(7, provider, 10, 200, 12_345, None::<VersionId>);
    let bytes = borsh::to_vec(&s_original).unwrap();
    let short = &bytes[..bytes.len() - 1];
    assert!(borsh::from_slice::<StreamConfig>(short).is_err());
    let mut long = bytes.clone();
    long.push(0);
    assert!(borsh::from_slice::<StreamConfig>(&long).is_err());
}

#[test]
fn test_stream_config_from_bytes_invalid_stream_state_fails() {
    // Pick `highest_defined_discriminant + 1` so the byte is outside every [`StreamState`] variant without a magic constant. Update the table when adding states.
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

    let (_, provider) = create_keypair(SERIALIZATION_SEED_PROVIDER);
    let s_valid = StreamConfig::new(0, provider, 1, 1, 1, None::<VersionId>);
    let mut serialized = borsh::to_vec(&s_valid).unwrap();
    let timestamp_field_size = size_of::<Timestamp>();
    let stream_state_byte_index = serialized.len() - timestamp_field_size - 1;
    serialized[stream_state_byte_index] = undefined_discriminant_after_max;
    assert!(borsh::from_slice::<StreamConfig>(&serialized).is_err());
}
