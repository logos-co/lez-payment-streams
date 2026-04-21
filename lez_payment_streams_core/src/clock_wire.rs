//! System clock account ids and Borsh wire format matching [`clock_core::ClockAccountData`].
//!
//! Duplicated here so the guest and host tests do not need a separate `clock_core` manifest entry
//! (Cargo disallows `git` + `path` for that crate in this workspace).

use borsh::{BorshDeserialize, BorshSerialize};
use nssa_core::account::AccountId;

pub const CLOCK_01_PROGRAM_ACCOUNT_ID: AccountId =
    AccountId::new(*b"/LEZ/ClockProgramAccount/0000001");

pub const CLOCK_10_PROGRAM_ACCOUNT_ID: AccountId =
    AccountId::new(*b"/LEZ/ClockProgramAccount/0000010");

pub const CLOCK_50_PROGRAM_ACCOUNT_ID: AccountId =
    AccountId::new(*b"/LEZ/ClockProgramAccount/0000050");

pub const CLOCK_PROGRAM_ACCOUNT_IDS: [AccountId; 3] = [
    CLOCK_01_PROGRAM_ACCOUNT_ID,
    CLOCK_10_PROGRAM_ACCOUNT_ID,
    CLOCK_50_PROGRAM_ACCOUNT_ID,
];

/// Borsh layout matches LEZ `clock_core::ClockAccountData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ClockAccountData {
    pub block_id: u64,
    pub timestamp: crate::Timestamp,
}

impl ClockAccountData {
    #[must_use]
    pub fn to_bytes(self) -> Vec<u8> {
        borsh::to_vec(&self).expect("ClockAccountData serialization should not fail")
    }

    #[must_use]
    pub fn from_bytes_slice(bytes: &[u8]) -> Option<Self> {
        borsh::from_slice(bytes).ok()
    }
}
