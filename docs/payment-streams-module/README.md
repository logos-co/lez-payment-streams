# Payment streams module

Universal Logos module (`payment_streams_module`) exposing LIP-155 vault and stream lifecycle via
`chainAction`. Assumes familiarity with Logos (logoscore, `.lgx` modules, LEZ wallet).

## Required verification

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

Success: exit 0 and JSON-lines under `.scaffold/e2e/artifacts/` (`module-e2e-*.log`) with phases
`vault_init`, `deposit`, `create_stream`, `claim`, `module_e2e_complete`. Uses isolated wallet
`.scaffold/module-e2e-wallet/` and vault/stream 0 on the live local sequencer (not the Store demo
snapshot).

Prepare only:

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local prepare
```

Orchestrator: [scripts/module-e2e.sh](../../scripts/module-e2e.sh).
Matrix: [reference/verification-matrix.md](../reference/verification-matrix.md).
First machine: [cold start](../reference/verification-matrix.md#cold-start-first-time-on-a-machine)
in the verification matrix.

## Setup

Tooling example:

```bash
nix shell \
  github:logos-co/logos-package-manager \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-module#lm \
  --command bash
```

Scaffold: `lgs init`, `lgs setup`, `lgs localnet start`. `make seed-fixture` for chain seed script.

Build module (no delivery):

```bash
MODE=module CHAIN=local ./scripts/e2e.sh build
# or: nix build ./logos-payment-streams-module#lgx
```

Patched `logos_execution_zone` wallet: [reference/feature-branch-pins.md](../reference/feature-branch-pins.md).

Guest ELF for logoscore:

```bash
export PAYMENT_STREAMS_GUEST_BIN="$REPO/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
cargo risczero build --manifest-path methods/guest/Cargo.toml
```

## Host boundary

One logoscore process loads `logos_execution_zone` and `payment_streams_module`. Store integration
adds `delivery_module` on provider/user hosts — see [store-integration](../store-integration/).

## Manual path (optional)

After prepare, run `logoscore` with wallet + module, `sync_to_block`, then `chainAction`:
`initializeVault`, `deposit`, `createStream`, `pauseStream`, `resumeStream`, `topUpStream`, `claim`.
Shapes mirror [module-e2e.sh](../../scripts/module-e2e.sh).

## Recovery

[archive/operator/localnet-recovery.md](../archive/operator/localnet-recovery.md).

## Out of scope

- Module verification on testnet (unsupported)
- Store eligibility — [store-integration](../store-integration/)
