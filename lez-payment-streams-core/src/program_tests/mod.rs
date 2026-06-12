//! Integration tests against the guest in an in-process [`nssa::V03State`].
//! Test vault setup, deposits, streams, and serialization helpers.
//! Transparent (“public ladder”) tests are always built with `cargo test --lib`.
//! Privacy-preserving (PP) tests live behind the `pp-program-tests` Cargo feature and must be run with
//! `RISC0_DEV_MODE=1` (see README); the harness refuses other `RISC0_DEV_MODE` values.
//!
//! Most in-process `#[test]` functions use
//! `#[ignore = common::IGNORE_LEZ_GUEST_ON_NSSA_HARNESS]` while the guest targets LEZ 491 and the
//! harness still models NSSA v0.1.2 builtins. Re-enable after SPEL-on-LEE or a LEE executor.

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
