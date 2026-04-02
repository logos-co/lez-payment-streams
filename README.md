# lez-payment-streams

LEZ implementation of payment streams.
A SPEL program built with
[spel-framework](https://github.com/logos-co/spel).

## Prerequisites

- Rust + [risc0 toolchain](https://dev.risczero.com/api/zkvm/install)
- [LSSA wallet CLI](https://github.com/logos-blockchain/lssa)
  (`wallet` binary, integration only)
- A running sequencer (integration only)

## Local testing without wallet or sequencer

Use these commands for local development only.
They do not require `wallet` or a sequencer.

```bash
# Rebuild guest ELF after guest or shared-type changes
cargo risczero build --manifest-path methods/guest/Cargo.toml

# Run local library tests in core crate
RISC0_DEV_MODE=1 cargo test -p lez_payment_streams_core --lib vault_tests
```

`cargo risczero build` rebuilds the guest binary.
`cargo test ... vault_tests` runs only matching local library tests.
Deploy and transaction submission still require `wallet`
and a sequencer.

## Quick Start

This section assumes integration setup
with both `wallet` and a running sequencer.
For local-only checks, use the commands above.

```bash
# Build guest and IDL
make build
make idl

# Deploy (integration)
make deploy

# Show CLI help
make cli ARGS="--help"

# Run an instruction
make cli ARGS="-p methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin \\
  <command> --arg1 value1 --arg2 value2"

# Dry run (no submission)
make cli ARGS="--dry-run -p methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin \\
  <command> --arg1 value1"
```

## Make Targets

| Target | Description |
|--------|-------------|
| `make build` | Build the guest binary (risc0) |
| `make idl` | Generate IDL JSON from program source |
| `make cli ARGS="..."` | Run the IDL-driven CLI |
| `make deploy` | Deploy program to sequencer |
| `make inspect` | Show ProgramId for built binary |
| `make setup` | Create accounts via wallet |
| `make status` | Show saved state and binary info |
| `make clean` | Remove saved state |

## Project Structure

```
lez-payment-streams/
├── lez_payment_streams_core/    # Shared types (used by guest + host)
│   └── src/lib.rs
├── methods/
│   └── guest/            # RISC Zero guest program (runs on-chain)
│       └── src/bin/lez_payment_streams.rs
├── examples/             # CLI tools
│   └── src/bin/
│       ├── generate_idl.rs    # One-liner IDL generator
│       └── lez_payment_streams_cli.rs # Three-line CLI wrapper
├── Makefile
└── lez-payment-streams-idl.json       # Auto-generated IDL
```

## How It Works

The `#[lez_program]` macro in your guest binary defines your on-chain program.
The framework automatically:

1. **Generates an `Instruction` enum** from your function signatures
2. **Generates an IDL** (Interface Description Language) describing your program
3. **Provides a full CLI** for building, inspecting, and submitting transactions

You write the program logic. The framework handles the rest.

# License

Licensed under either of:

- MIT License – see [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT
- Apache License 2.0 – see [LICENSE-APACHE-v2](LICENSE-APACHE-v2) or http://www.apache.org/licenses/LICENSE-2.0

at your option.