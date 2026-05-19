# lez-payment-streams

LEZ implementation of payment streams.
A SPEL program built with [spel-framework](https://github.com/logos-co/spel).
Protocol semantics are defined in [LIP-155](https://lip.logos.co/ift-ts/raw/payment-streams.html).
For the rationale behind design choices and a suggested reading order, see [architecture.md](architecture.md).

## Code Map

| Path | Concern |
|---|---|
| `methods/guest/src/bin/lez_payment_streams.rs` | Guest program: `#[lez_program]` module, `#[instruction]` handlers, account attributes |
| `lez-payment-streams-core/src/` | Shared types and pure logic: `VaultConfig`, `VaultHolding`, `StreamConfig`, `Instruction`, error codes, accrual math |
| `lez-payment-streams-core/src/program_tests/` | In-process `V03State` tests: transparent flows always-on; PP flows behind `--features pp-program-tests` |

| `lez-payment-streams-core/src/test_helpers.rs` | Test harness helpers: keypairs, state setup, guest deployment, transaction builders |
| `examples/src/bin/` | IDL generator and CLI wrapper |

The workspace directories use hyphenated Cargo package names (`lez-payment-streams-core`, `lez-payment-streams-ffi`);
Rust code still imports `lez_payment_streams_core` for the protocol library crate.

For semantics review, one distinction matters early:
`close_stream` may be initiated by the vault owner or the provider,
while `claim` is provider-specific.
Closing releases only the unaccrued remainder.
Claiming pays out accrued funds.


## Logos module package (Nix)

The workspace root `flake.nix` exposes `packages.<system>.payment-streams-ffi` only.
To build the payment-streams Logos Core plugin bundle (`.lgx`), use the nested flake:

```bash
nix build ./logos-payment-streams-module#lgx
```

(Run from the `lez-payment-streams` repo root, or `cd logos-payment-streams-module` and use `nix build .#lgx`.)

Operator setup (`lgpm`, `logoscore`, wallet `.lgx` bundling, shared `modules/` directory)
is summarized in [`docs/logos-operator-install-basics.md`](docs/logos-operator-install-basics.md)
(integration plan Step 6c).


## Running Tests

After any change to the guest binary or to types shared with the guest,
rebuild the guest ELF before testing:

```bash
cargo risczero build --manifest-path methods/guest/Cargo.toml
```

Run tests:

```bash
# Default (transparent program_tests only). Uses RISC0 dev proving for any guest proofs.
RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --lib

# Narrow filter when iterating on harness cases
RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --lib program_tests
```

`RISC0_DEV_MODE=1` skips zk proof generation in the RISC Zero zkVM harness and should be used for all routine test runs.

Optional privacy-preserving program tests compile only with `--features pp-program-tests`; they intentionally panic unless `RISC0_DEV_MODE=1` so local or CI jobs never regress into full zk proving accidentally.

```bash
RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --lib \
  --features pp-program-tests pp_program_tests
```

Full zk proving stays reserved for release or dedicated proving jobs.

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
