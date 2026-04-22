//! Integration tests against the guest in an in-process [`nssa::V03State`].
//! Test vault setup, deposits, streams, and serialization helpers.

mod accrual;
mod claim;
mod close_stream;
mod common;
mod create_stream;
mod deposit;
mod initialize;
mod invariants;
mod pause_stream;
mod resume_stream;
mod serialization;
mod shielded_execution;
mod top_up;
mod withdraw;
