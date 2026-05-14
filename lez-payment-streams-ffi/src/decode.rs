//! Borsh decoders plus version checks aligned with fixtures in [`lez_payment_streams_core`].

use borsh::BorshDeserialize;

use lez_payment_streams_core::{
    ClockAccountData, StreamConfig, VaultConfig, VaultHolding, VersionId, DEFAULT_VERSION,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum DecodeFault {
    Malformed,
    BadVersion,
}

trait DecodeDefaultVersionWire: Sized + BorshDeserialize {
    fn wire_protocol_version(&self) -> VersionId;
}

impl DecodeDefaultVersionWire for VaultConfig {
    fn wire_protocol_version(&self) -> VersionId {
        self.version
    }
}

impl DecodeDefaultVersionWire for VaultHolding {
    fn wire_protocol_version(&self) -> VersionId {
        self.version
    }
}

impl DecodeDefaultVersionWire for StreamConfig {
    fn wire_protocol_version(&self) -> VersionId {
        self.version
    }
}

fn decode_borsh_checked_default_protocol_version<T: DecodeDefaultVersionWire>(
    data: &[u8],
) -> Result<T, DecodeFault> {
    let value = match borsh::from_slice::<T>(data) {
        Ok(decoded) => decoded,
        Err(_) => return Err(DecodeFault::Malformed),
    };

    if value.wire_protocol_version() != DEFAULT_VERSION {
        return Err(DecodeFault::BadVersion);
    }

    Ok(value)
}

/// Decode vault config account bytes (`version` must match [`DEFAULT_VERSION`]).
pub(crate) fn decode_vault_config(data: &[u8]) -> Result<VaultConfig, DecodeFault> {
    decode_borsh_checked_default_protocol_version(data)
}

pub(crate) fn decode_vault_holding(data: &[u8]) -> Result<VaultHolding, DecodeFault> {
    decode_borsh_checked_default_protocol_version(data)
}

pub(crate) fn decode_stream_config(data: &[u8]) -> Result<StreamConfig, DecodeFault> {
    decode_borsh_checked_default_protocol_version(data)
}

pub(crate) fn decode_clock_account_data(data: &[u8]) -> Result<ClockAccountData, DecodeFault> {
    match borsh::from_slice::<ClockAccountData>(data) {
        Ok(clock_account_data) => Ok(clock_account_data),
        Err(_) => Err(DecodeFault::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nssa_core::account::{AccountId, Balance};

    #[test]
    fn vault_config_roundtrip_core_fixture() {
        const ROUNDTRIP_OWNER_BYTE: u8 = 43;
        const ROUNDTRIP_VAULT_ID: u64 = 34;

        let expected_vault_config = VaultConfig::new(
            AccountId::new([ROUNDTRIP_OWNER_BYTE; 32]),
            ROUNDTRIP_VAULT_ID,
            Some(DEFAULT_VERSION),
            None,
        );
        let bytes = borsh::to_vec(&expected_vault_config).unwrap();
        let decoded_vault_config = decode_vault_config(&bytes).unwrap();
        assert_eq!(expected_vault_config, decoded_vault_config);

        assert_eq!(
            decode_vault_config(&bytes[..bytes.len().saturating_sub(1)]),
            Err(DecodeFault::Malformed)
        );

        assert_eq!(decode_vault_config(&[]), Err(DecodeFault::Malformed));
    }

    #[test]
    fn rejects_future_version_fixture() {
        // Distinct from DEFAULT_VERSION; exercise DecodeFault::BadVersion.
        const UNSUPPORTED_PROTOCOL_VERSION_FOR_DECODE_TEST: u8 = 99;
        const VAULT_OWNER_BYTE: u8 = 1;
        const VAULT_ID: u64 = 1;

        let mut vault_config_with_future_version =
            VaultConfig::new(AccountId::new([VAULT_OWNER_BYTE; 32]), VAULT_ID, None, None);
        vault_config_with_future_version.version = UNSUPPORTED_PROTOCOL_VERSION_FOR_DECODE_TEST;
        let bytes = borsh::to_vec(&vault_config_with_future_version).unwrap();
        assert_eq!(decode_vault_config(&bytes), Err(DecodeFault::BadVersion));
    }

    #[test]
    fn vault_holding_roundtrip() {
        let expected_vault_holding = VaultHolding::new(Some(DEFAULT_VERSION));
        let bytes = borsh::to_vec(&expected_vault_holding).unwrap();
        assert_eq!(
            decode_vault_holding(&bytes).unwrap(),
            expected_vault_holding,
        );

        assert_eq!(
            decode_vault_holding(&bytes[..bytes.len().saturating_sub(1)]),
            Err(DecodeFault::Malformed)
        );
        assert_eq!(decode_vault_holding(&[]), Err(DecodeFault::Malformed));
    }

    #[test]
    fn stream_config_roundtrip() {
        const PROVIDER_BYTE: u8 = 5;
        const STREAM_ID: u64 = 7;
        const RATE_TOKENS_PER_SECOND: u64 = 10;
        const ALLOCATION: Balance = 200;
        const ACCRUED_AS_OF: u64 = 12_345;

        let provider = AccountId::new([PROVIDER_BYTE; 32]);
        let allocation: Balance = ALLOCATION;
        let expected_stream_config = StreamConfig::new(
            STREAM_ID,
            provider,
            RATE_TOKENS_PER_SECOND,
            allocation,
            ACCRUED_AS_OF,
            Some(DEFAULT_VERSION),
        );
        let bytes = borsh::to_vec(&expected_stream_config).unwrap();
        assert_eq!(
            decode_stream_config(&bytes).unwrap(),
            expected_stream_config,
        );

        assert_eq!(
            decode_stream_config(&bytes[..bytes.len().saturating_sub(1)]),
            Err(DecodeFault::Malformed)
        );
        assert_eq!(decode_stream_config(&[]), Err(DecodeFault::Malformed));
    }

    #[test]
    fn clock_roundtrip_fixture() {
        const BLOCK_ID: u64 = 5;
        const TIMESTAMP: u64 = 99;

        let expected_clock_account_data = ClockAccountData {
            block_id: BLOCK_ID,
            timestamp: TIMESTAMP,
        };
        let bytes = borsh::to_vec(&expected_clock_account_data).unwrap();
        assert_eq!(
            decode_clock_account_data(&bytes).unwrap(),
            expected_clock_account_data,
        );

        assert_eq!(
            decode_clock_account_data(&bytes[..bytes.len().saturating_sub(1)]),
            Err(DecodeFault::Malformed)
        );
        assert_eq!(decode_clock_account_data(&[]), Err(DecodeFault::Malformed));
    }
}
