//! Deterministic [`crate::test_helpers::create_keypair`] byte seeds for tests.
//!
//! Bands:
//!
//! - `0x01`–`0x0F`: owner and alternate signer.
//! - `0x10`–`0x1F`: stream providers A and B, withdraw recipient.
//! Reuse across tests. Each test run gets a new [`nssa::V03State`], so one value per role is enough.
//!
//! System clock account ids are fixed by genesis ([`clock_core::CLOCK_*_PROGRAM_ACCOUNT_ID`]),
//! not derived from these seeds.
//!
//! Add `0x20`+ only when one test needs extra distinct accounts (e.g. a third provider).
//!
//! Prefer named constants here over raw bytes.
//! [`crate::program_tests::serialization`] keeps its own literals for layout checks.
//! Other program tests import via `use crate::harness_seeds::{...}`.

// ---- 0x01–0x0F core ---- //

pub(crate) const SEED_OWNER: u8 = 0x01;
pub(crate) const SEED_ALT_SIGNER: u8 = 0x02;

// ---- 0x11–0x1F default harness extras ---- //

pub(crate) const SEED_PROVIDER: u8 = 0x11;
pub(crate) const SEED_PROVIDER_B: u8 = 0x12;
pub(crate) const SEED_RECIPIENT: u8 = 0x13;
