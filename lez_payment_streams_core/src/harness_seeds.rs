//! Deterministic [`crate::test_helpers::create_keypair`] seeds for integration tests.
//!
//! # Bands
//!
//! - `0x01`–`0x0F`: core identities (owner, alternate signer).
//! - `0x10`–`0x1F`: default harness accounts — mock timestamp clock, stream providers (A/B),
//!   withdraw recipient. Reuse these for any test that needs one clock and one or two providers;
//!   each test gets a fresh [`nssa::V03State`], so distinct byte values per file are unnecessary.
//!
//! Add new `pub(crate) const` values in `0x20`+ only when a **single** test needs more distinct
//! accounts than this band provides (e.g. a third provider).
//!
//! New tests should use names from this module instead of raw numeric literals.
//! [`crate::program_tests::serialization`] keeps its own literals for determinism checks.

// ---- 0x01–0x0F core ---- //

pub(crate) const SEED_OWNER: u8 = 0x01;
pub(crate) const SEED_ALT_SIGNER: u8 = 0x02;

// ---- 0x10–0x1F default harness extras ---- //

pub(crate) const SEED_MOCK_CLOCK: u8 = 0x10;
pub(crate) const SEED_PROVIDER: u8 = 0x11;
pub(crate) const SEED_PROVIDER_B: u8 = 0x12;
pub(crate) const SEED_RECIPIENT: u8 = 0x13;
