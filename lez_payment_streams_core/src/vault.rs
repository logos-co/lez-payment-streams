//! [`VaultConfig`], [`VaultHolding`], and vault `total_allocated` bookkeeping helpers.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use nssa_core::account::{AccountId, Balance};

use crate::error_codes::ErrorCode;
use crate::{StreamId, VaultId, VersionId};

/// Execution-mode intent stored on [`VaultConfig`], immutable at creation.
/// The guest stores this field but cannot determine execution mode at runtime;
/// the wallet enforces shielded-only policy for [`VaultPrivacyTier::PseudonymousFunder`] vaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum VaultPrivacyTier {
    Public = 0,
    PseudonymousFunder = 1,
}

impl VaultPrivacyTier {
    pub const fn as_wire_byte(self) -> u8 {
        self as u8
    }

    pub const fn from_wire_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Public),
            1 => Some(Self::PseudonymousFunder),
            _ => None,
        }
    }
}

impl Serialize for VaultPrivacyTier {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.as_wire_byte())
    }
}

impl<'de> Deserialize<'de> for VaultPrivacyTier {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let b = <u8 as Deserialize>::deserialize(deserializer)?;
        Self::from_wire_byte(b).ok_or_else(|| DeError::custom("unknown vault privacy tier"))
    }
}

#[cfg(test)]
mod vault_privacy_tier_wire_tests {
    use super::VaultPrivacyTier;

    /// Wire byte with no [`VaultPrivacyTier`] variant (reserved on the `InitializeVault` wire).
    const UNDEFINED_PRIVACY_TIER_WIRE_BYTE: u8 = 99;

    #[test]
    fn from_wire_byte_rejects_undefined_privacy_tier_wire_byte() {
        assert!(VaultPrivacyTier::from_wire_byte(UNDEFINED_PRIVACY_TIER_WIRE_BYTE).is_none());
    }
}

/// Compute the next [`VaultConfig::total_allocated`] after adding `increase_total_allocated_by`,
/// capped by unallocated liquidity. Pure helper (guest persists the value).
///
/// Unallocated is `vault_holding_balance.saturating_sub(vault_total_allocated)`.
/// [`crate::Instruction::CreateStream`] applies the new stream allocation.
/// [`crate::Instruction::TopUpStream`] applies the top-up increment.
///
/// Zero `increase_total_allocated_by` is invalid where [`crate::error_codes::ErrorCode::ZeroStreamAllocation`] or [`crate::error_codes::ErrorCode::ZeroTopUpAmount`] apply.
///
/// [`crate::error_codes::ErrorCode::TotalAllocatedOverflow`] guards `checked_add` (defensive given the unallocated bound).
pub fn checked_total_allocated_after_add(
    vault_holding_balance: Balance,
    vault_total_allocated: Balance,
    increase_total_allocated_by: Balance,
) -> Result<Balance, ErrorCode> {
    let unallocated = vault_holding_balance.saturating_sub(vault_total_allocated);
    if increase_total_allocated_by > unallocated {
        return Err(ErrorCode::AllocationExceedsUnallocated);
    }
    vault_total_allocated
        .checked_add(increase_total_allocated_by)
        .ok_or(ErrorCode::TotalAllocatedOverflow)
}

/// Compute the next [`VaultConfig::total_allocated`] after subtracting `decrease_total_allocated_by` (`claim`, `close`, …).
/// Pure helper (guest persists the value).
///
/// Zero decrease: return the input unchanged (e.g. close with nothing left to release).
/// [`crate::error_codes::ErrorCode::TotalAllocatedUnderflow`] when the decrease exceeds current `total_allocated`.
pub fn checked_total_allocated_after_release(
    vault_total_allocated: Balance,
    decrease_total_allocated_by: Balance,
) -> Result<Balance, ErrorCode> {
    if decrease_total_allocated_by == 0 {
        return Ok(vault_total_allocated);
    }
    vault_total_allocated
        .checked_sub(decrease_total_allocated_by)
        .ok_or(ErrorCode::TotalAllocatedUnderflow)
}

#[spel_framework_macros::account_type]
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct VaultConfig {
    pub version: VersionId,
    pub owner: AccountId,
    pub vault_id: VaultId,
    pub next_stream_id: StreamId,
    pub total_allocated: Balance,
    pub privacy_tier: VaultPrivacyTier,
}

impl VaultConfig {
    pub fn to_bytes(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("VaultConfig borsh serialization is infallible")
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        borsh::from_slice(data).ok()
    }

    /// Fresh vault config: `next_stream_id` at [`StreamId::MIN`], `total_allocated` at zero.
    ///
    /// Use `None` for `version` to pick [`crate::DEFAULT_VERSION`], and `None` for `privacy_tier`
    /// to pick [`VaultPrivacyTier::Public`].
    pub fn new(
        owner: AccountId,
        vault_id: VaultId,
        version: Option<VersionId>,
        privacy_tier: Option<VaultPrivacyTier>,
    ) -> Self {
        Self {
            version: version.unwrap_or(crate::DEFAULT_VERSION),
            owner,
            vault_id,
            next_stream_id: StreamId::MIN,
            total_allocated: Balance::MIN,
            privacy_tier: privacy_tier.unwrap_or(VaultPrivacyTier::Public),
        }
    }
}

#[spel_framework_macros::account_type]
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct VaultHolding {
    pub version: VersionId,
}

impl VaultHolding {
    pub fn to_bytes(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("VaultHolding borsh serialization is infallible")
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        borsh::from_slice(data).ok()
    }

    /// Use `None` for `version` to pick [`crate::DEFAULT_VERSION`].
    pub fn new(version: Option<VersionId>) -> Self {
        Self {
            version: version.unwrap_or(crate::DEFAULT_VERSION),
        }
    }
}

#[cfg(test)]
mod checked_total_allocated_after_add_tests {
    use nssa_core::account::Balance;

    use super::checked_total_allocated_after_add;
    use crate::error_codes::ErrorCode;

    #[test]
    fn add_within_unallocated_succeeds() {
        let next_vault_total_allocated = checked_total_allocated_after_add(500, 200, 100).unwrap();
        assert_eq!(next_vault_total_allocated, 300 as Balance);
    }

    #[test]
    fn add_exceeds_unallocated_fails() {
        assert_eq!(
            checked_total_allocated_after_add(500, 400, 200),
            Err(ErrorCode::AllocationExceedsUnallocated)
        );
    }
}

#[cfg(test)]
mod checked_total_allocated_after_release_tests {
    use nssa_core::account::Balance;

    use super::checked_total_allocated_after_release;
    use crate::error_codes::ErrorCode;

    #[test]
    fn release_decrease_succeeds() {
        let next_vault_total_allocated = checked_total_allocated_after_release(300, 100).unwrap();
        assert_eq!(next_vault_total_allocated, 200 as Balance);
    }

    #[test]
    fn release_zero_decrease_noop_succeeds() {
        let next_vault_total_allocated = checked_total_allocated_after_release(300, 0).unwrap();
        assert_eq!(next_vault_total_allocated, 300 as Balance);
    }

    #[test]
    fn release_underflow_fails() {
        assert_eq!(
            checked_total_allocated_after_release(100, 200),
            Err(ErrorCode::TotalAllocatedUnderflow)
        );
    }
}
