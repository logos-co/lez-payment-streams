//! [`VaultConfig`], [`VaultHolding`], and vault `total_allocated` bookkeeping helpers.

use core::mem::size_of;

use serde::{Deserialize, Serialize};

use nssa_core::account::{AccountId, Balance};

use crate::error_codes::{
    ERR_ALLOCATION_EXCEEDS_UNALLOCATED, ERR_TOTAL_ALLOCATED_OVERFLOW, ERR_TOTAL_ALLOCATED_UNDERFLOW,
};
use crate::{StreamId, VaultId, VersionId};

/// Compute the next [`VaultConfig::total_allocated`] after adding `increase_total_allocated_by`,
/// capped by unallocated liquidity. Pure helper (guest persists the value).
///
/// Unallocated is `vault_holding_balance.saturating_sub(vault_total_allocated)`.
/// [`crate::Instruction::CreateStream`] applies the new stream allocation.
/// [`crate::Instruction::TopUpStream`] applies the top-up increment.
///
/// Zero `increase_total_allocated_by` is invalid where [`crate::ERR_ZERO_STREAM_ALLOCATION`] or [`crate::ERR_ZERO_TOP_UP_AMOUNT`] apply.
///
/// [`ERR_TOTAL_ALLOCATED_OVERFLOW`] guards `checked_add` (defensive given the unallocated bound).
pub fn checked_total_allocated_after_add(
    vault_holding_balance: Balance,
    vault_total_allocated: Balance,
    increase_total_allocated_by: Balance,
) -> Result<Balance, u32> {
    let unallocated = vault_holding_balance.saturating_sub(vault_total_allocated);
    if increase_total_allocated_by > unallocated {
        return Err(ERR_ALLOCATION_EXCEEDS_UNALLOCATED);
    }
    vault_total_allocated
        .checked_add(increase_total_allocated_by)
        .ok_or(ERR_TOTAL_ALLOCATED_OVERFLOW)
}

/// Compute the next [`VaultConfig::total_allocated`] after subtracting `decrease_total_allocated_by` (`claim`, `close`, …).
/// Pure helper (guest persists the value).
///
/// Zero decrease: return the input unchanged (e.g. close with nothing left to release).
/// [`ERR_TOTAL_ALLOCATED_UNDERFLOW`] when the decrease exceeds current `total_allocated`.
pub fn checked_total_allocated_after_release(
    vault_total_allocated: Balance,
    decrease_total_allocated_by: Balance,
) -> Result<Balance, u32> {
    if decrease_total_allocated_by == 0 {
        return Ok(vault_total_allocated);
    }
    vault_total_allocated
        .checked_sub(decrease_total_allocated_by)
        .ok_or(ERR_TOTAL_ALLOCATED_UNDERFLOW)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultConfig {
    pub version: VersionId,
    pub owner: AccountId,
    pub vault_id: VaultId,
    pub next_stream_id: StreamId,
    pub total_allocated: Balance,
}

impl VaultConfig {
    pub const SIZE: usize = size_of::<VersionId>()
        + size_of::<AccountId>()
        + size_of::<VaultId>()
        + size_of::<StreamId>()
        + size_of::<Balance>();

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(self.owner.value());
        buf.extend_from_slice(&self.vault_id.to_le_bytes());
        buf.extend_from_slice(&self.next_stream_id.to_le_bytes());
        buf.extend_from_slice(&self.total_allocated.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() != Self::SIZE {
            return None;
        }
        // extract fields
        // version
        let mut offset = 0;
        let size = size_of::<VersionId>();
        let version = VersionId::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        // owner
        let size = size_of::<AccountId>();
        let owner = AccountId::new(data[offset..offset + size].try_into().ok()?);
        offset += size;

        // vault_id
        let size = size_of::<VaultId>();
        let vault_id = VaultId::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        // next_stream_id
        let size = size_of::<StreamId>();
        let next_stream_id = StreamId::from_le_bytes(data[offset..offset + size].try_into().ok()?);
        offset += size;

        // total_allocated
        let size = size_of::<Balance>();
        let total_allocated = Balance::from_le_bytes(data[offset..offset + size].try_into().ok()?);

        Some(Self {
            version,
            owner,
            vault_id,
            next_stream_id,
            total_allocated,
        })
    }

    pub fn new(owner: AccountId, vault_id: VaultId) -> Self {
        Self::new_with_version(owner, vault_id, crate::DEFAULT_VERSION)
    }

    pub fn new_with_version(owner: AccountId, vault_id: VaultId, version: VersionId) -> Self {
        Self {
            version,
            owner,
            vault_id,
            next_stream_id: StreamId::MIN,
            total_allocated: Balance::MIN,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultHolding {
    pub version: VersionId,
}

impl VaultHolding {
    pub const SIZE: usize = size_of::<VersionId>();

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() != Self::SIZE {
            return None;
        }
        // extract fields (one field - version - only)
        let version = VersionId::from_le_bytes(data[..Self::SIZE].try_into().ok()?);

        Some(Self { version })
    }

    pub fn new() -> Self {
        Self::new_with_version(crate::DEFAULT_VERSION)
    }

    pub fn new_with_version(version: VersionId) -> Self {
        Self { version }
    }
}

#[cfg(test)]
mod checked_total_allocated_after_add_tests {
    use nssa_core::account::Balance;

    use super::checked_total_allocated_after_add;
    use crate::error_codes::ERR_ALLOCATION_EXCEEDS_UNALLOCATED;

    #[test]
    fn add_succeeds_within_unallocated() {
        let next_vault_total_allocated = checked_total_allocated_after_add(500, 200, 100).unwrap();
        assert_eq!(next_vault_total_allocated, 300 as Balance);
    }

    #[test]
    fn add_rejects_exceeds_unallocated() {
        assert_eq!(
            checked_total_allocated_after_add(500, 400, 200),
            Err(ERR_ALLOCATION_EXCEEDS_UNALLOCATED)
        );
    }
}

#[cfg(test)]
mod checked_total_allocated_after_release_tests {
    use nssa_core::account::Balance;

    use super::checked_total_allocated_after_release;
    use crate::error_codes::ERR_TOTAL_ALLOCATED_UNDERFLOW;

    #[test]
    fn release_succeeds() {
        let next_vault_total_allocated = checked_total_allocated_after_release(300, 100).unwrap();
        assert_eq!(next_vault_total_allocated, 200 as Balance);
    }

    #[test]
    fn release_noop_when_decrease_total_allocated_by_zero() {
        let next_vault_total_allocated = checked_total_allocated_after_release(300, 0).unwrap();
        assert_eq!(next_vault_total_allocated, 300 as Balance);
    }

    #[test]
    fn release_rejects_underflow() {
        assert_eq!(
            checked_total_allocated_after_release(100, 200),
            Err(ERR_TOTAL_ALLOCATED_UNDERFLOW)
        );
    }
}
