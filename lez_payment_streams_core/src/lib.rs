//! LEZ payment streams.
//!
//! Vault PDAs, stream accrual, and [`Instruction`] types for the guest and tests.

#[cfg(test)]
mod harness_seeds;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod program_tests;

mod error_codes;
mod instruction;
mod mock_timestamp;
mod stream_config;
mod vault;

pub use error_codes::*;
pub use instruction::Instruction;
pub use mock_timestamp::MockTimestamp;
pub use stream_config::{StreamConfig, StreamState};
pub use vault::{
    checked_total_allocated_after_add, checked_total_allocated_after_release, VaultConfig,
    VaultHolding,
};

// ---- Type aliases ---- //

pub type VersionId = u8;
pub type VaultId = u64;
pub type StreamId = u64;
pub type TokensPerSecond = u64;
pub type Timestamp = u64;

// ---- Version ---- //

pub const DEFAULT_VERSION: VersionId = 1;
