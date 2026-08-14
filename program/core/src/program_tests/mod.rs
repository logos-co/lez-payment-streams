//! Integration tests against the guest in an in-process [`lee::V03State`].
//! Test vault setup, deposits, streams, and serialization helpers.
//! Transparent (“public ladder”) tests are always built with `cargo test --lib`.
//! Privacy-preserving (PP) tests live behind the `pp-program-tests` Cargo feature and must be run with
//! `RISC0_DEV_MODE=1` (see README); the harness refuses other `RISC0_DEV_MODE` values.

// Test fixtures deliberately use `.unwrap()` on known-good inputs and fixed
// arithmetic to assert state transitions; the workspace `deny` policy for
// `unwrap_used` / `arithmetic_side_effects` / `indexing_slicing` is relaxed
// here, matching how `lez-programs` allows these in its integration_tests crate.
#![allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "test fixtures use known-good inputs and fixed arithmetic to assert state transitions"
)]

mod claim;
mod close_stream;
pub(crate) mod common;
mod create_stream;
mod deposit;
mod initialize;
mod invariants;
mod pause_stream;
#[cfg(feature = "pp-program-tests")]
pub(crate) mod pp_common;
mod privacy_tier_policy;
mod resume_stream;
mod serialization;
mod top_up;
mod withdraw;
