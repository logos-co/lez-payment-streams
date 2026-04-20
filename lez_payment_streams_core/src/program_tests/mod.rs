//! Guest-backed integration tests using in-process [`nssa::V03State`].
//! Covers vault flows, streams, and layout or harness primitives (see submodules).

mod accrual;
mod close_stream;
mod common;
mod seeds;
mod create_stream;
mod deposit;
mod initialize;
mod pause_resume;
mod serialization;
mod top_up;
mod withdraw;
