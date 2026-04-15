//! Guest-backed integration tests using in-process [`nssa::V03State`].
//! Covers vault flows, streams, and layout or harness primitives (see submodules).

mod accrual;
mod common;
mod create_stream;
mod deposit;
mod initialize;
mod serialization;
mod withdraw;
