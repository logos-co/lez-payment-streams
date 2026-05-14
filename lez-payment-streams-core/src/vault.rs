//! [`VaultConfig`], [`VaultHolding`], and vault `total_allocated` bookkeeping helpers.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use nssa_core::account::{AccountId, Balance};

use crate::error_codes::ErrorCode;
use crate::{StreamId, VaultId, VersionId};

/// Execution-mode intent stored on [`VaultConfig`], immutable at creation.
/// The guest stores this field but cannot determine execution mode at runtime;
/// the wallet enforces shielded-only policy for [`VaultPrivacyTier::PseudonymousFunder`] vaults.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
#[borsh(use_discriminant = true)]
#[serde(into = "u8", try_from = "u8")]
#[repr(u8)]
pub enum VaultPrivacyTier {
    Public = 0,
    PseudonymousFunder = 1,
}

impl From<VaultPrivacyTier> for u8 {
    fn from(tier: VaultPrivacyTier) -> u8 {
        tier as u8
    }
}

impl TryFrom<u8> for VaultPrivacyTier {
    type Error = &'static str;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Public),
            1 => Ok(Self::PseudonymousFunder),
            _ => Err("unknown vault privacy tier"),
        }
    }
}

#[cfg(test)]
mod vault_privacy_tier_tests {
    use super::VaultPrivacyTier;

    #[test]
    fn try_from_unknown_byte_fails() {
        assert!(VaultPrivacyTier::try_from(99u8).is_err());
    }
}

/// Compute the next [`VaultConfig::total_allocated`] after adding `increase_total_allocated_by`,
/// capped by unallocated liquidity. Pure helper (guest persists the value).
///
/// Unallocated is `vault_holding_balance.saturating_sub(vault_total_allocated)`.
///
/// This helper protects the vault-side half of the accounting invariants during `create_stream`
/// and `top_up_stream`:
/// 1. `vault_holding.balance >= vault_config.total_allocated`
/// 2. `vault_config.total_allocated` tracks the sum of all stream `allocation` values
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

/// Compute the next [`VaultConfig::total_allocated`] after subtracting `decrease_total_allocated_by`
/// (`claim`, `close`, …). Pure helper (guest persists the value).
///
/// Zero decrease: return the input unchanged (e.g. close with nothing left to release).
///
/// Callers must pass exactly the amount by which some stream's `allocation` shrank:
/// - `close_stream` releases only the stream's unaccrued remainder back to the vault
/// - `claim` releases the full payout amount because `claim_at_time` reduces `allocation` by payout
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
