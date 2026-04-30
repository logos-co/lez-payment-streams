# lez-payment-streams

LEZ implementation of payment streams.
A SPEL program built with [spel-framework](https://github.com/logos-co/spel).
Protocol semantics are defined in [LIP-155](https://lip.logos.co/ift-ts/raw/payment-streams.html).
For the rationale behind design choices and a suggested reading order, see [architecture.md](architecture.md).

## Code Map

| Path | Concern |
|---|---|
| `methods/guest/src/bin/lez_payment_streams.rs` | Guest program: `#[lez_program]` module, `#[instruction]` handlers, account attributes |
| `lez_payment_streams_core/src/` | Shared types and pure logic: `VaultConfig`, `VaultHolding`, `StreamConfig`, `Instruction`, error codes, accrual math |
| `lez_payment_streams_core/src/program_tests/` | In-process `V03State` tests, one module per instruction plus `common.rs`, `pp_common.rs`, `invariants.rs`, `serialization.rs`, `privacy_tier_policy.rs` |
| `lez_payment_streams_core/src/test_helpers.rs` | Test harness helpers: keypairs, state setup, guest deployment, transaction builders |
| `examples/src/bin/` | IDL generator and CLI wrapper |


## Running Tests

After any change to the guest binary or to types shared with the guest,
rebuild the guest ELF before testing:

```bash
cargo risczero build --manifest-path methods/guest/Cargo.toml
```

Run tests:

```bash
# Fast local loop (no ZK proof generation)
RISC0_DEV_MODE=1 cargo test -p lez_payment_streams_core --lib

# Narrower filter when not touching other unit tests
RISC0_DEV_MODE=1 cargo test -p lez_payment_streams_core --lib program_tests
```

`RISC0_DEV_MODE=1` skips ZK proof generation and is the standard mode for both
local development and CI test runs.
Full proof generation is reserved for release or dedicated proving jobs.

## Prerequisites (Integration Only)

- Rust + [risc0 toolchain](https://dev.risczero.com/api/zkvm/install)
- [LSSA wallet CLI](https://github.com/logos-blockchain/lssa) (`wallet` binary)
- A running sequencer

Local `cargo test` does not require a wallet or sequencer.


## Quick Start (Integration)

```bash
# Build guest and IDL
make build
make idl

# Deploy
make deploy

# Show CLI help
make cli ARGS="--help"
```

# License

Licensed under either of:

- MIT License – see [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT
- Apache License 2.0 – see [LICENSE-APACHE-v2](LICENSE-APACHE-v2) or http://www.apache.org/licenses/LICENSE-2.0

at your option.
