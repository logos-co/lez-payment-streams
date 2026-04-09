use serde::{Deserialize, Serialize};

use nssa_core::account::{
    AccountId,
    Balance,
};

use core::mem::size_of;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod vault_tests;

// ---- Type aliases ---- //

pub type VersionId = u8;
pub type VaultId = u64;
pub type StreamId = u64;

// ---- Version ---- //

pub const DEFAULT_VERSION: VersionId = 1;

// ---- VaultConfig ---- //

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultConfig {
    pub version: VersionId,
    pub owner: AccountId,
    pub vault_id: VaultId,
    pub next_stream_id: StreamId,
    pub total_allocated: Balance,
}

impl VaultConfig {

    pub const SIZE: usize = 
        size_of::<VersionId>() +
        size_of::<AccountId>() +
        size_of::<VaultId>() +
        size_of::<StreamId>() +
        size_of::<Balance>();

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
        Self::new_with_version(owner, vault_id, DEFAULT_VERSION)
    }

    pub fn new_with_version(owner: AccountId, vault_id: VaultId, version: VersionId) -> Self {
        Self {
            version,
            owner,
            vault_id,
            next_stream_id: 0,
            total_allocated: 0,
        }
    }

}



// ---- VaultHolding ---- //

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
        Self::new_with_version(DEFAULT_VERSION)
    }

    pub fn new_with_version(version: VersionId) -> Self {
        Self { version }
    }
}