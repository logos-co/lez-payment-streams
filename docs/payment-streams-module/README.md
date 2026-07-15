# Payment streams module

Universal Logos module (`payment_streams_module`) exposing LIP-155 vault and stream lifecycle via
`chainAction`. Assumes familiarity with Logos (logoscore, `.lgx` modules, LEZ wallet).

## Required verification

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

Testnet:

```bash
make bootstrap-testnet-module   # one-time
make verify-module-testnet
```

Success: exit code 0 and JSON-lines under `.scaffold/e2e/artifacts/` (`module-e2e-*.log`) with phases
`vault_init`, `deposit`, `create_stream`, `claim`, `module_e2e_complete`. Localnet module E2E uses
`e2e/user/wallet-local`; testnet uses `e2e/testnet-wallet`. Layout:
[naming-conventions.md](../reference/naming-conventions.md#scaffold-layout).

Recipes: [journeys/E2E.md](../journeys/E2E.md).
Hands-on testnet walkthrough: [journeys/USER_JOURNEY.md](../journeys/USER_JOURNEY.md).

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

Set `PAYMENT_STREAMS_GUEST_BIN` on the logoscore daemon process before writes.

## Host boundary

One logoscore process loads `logos_execution_zone` and `payment_streams_module`. Store integration
adds `delivery_module` on provider/user hosts — see [store-integration](../store-integration/).

## chainAction catalogue

SSOT for module I/O. Invoke:

```bash
logoscore call payment_streams_module chainAction <operation> '<paramsJson>'
```

`paramsJson` is a compact JSON object. Writes return submit JSON (`status`, `tx_hash`, …); callers
sync via `logos_execution_zone sync_to_block` and poll status ops below.
Historical step runbook: [archive/steps/module-chain-writes-runbook.md](../archive/steps/module-chain-writes-runbook.md)
(points here).

Legend: **UJ** = exercised in [USER_JOURNEY.md](../journeys/USER_JOURNEY.md) testnet walkthrough.

### Writes

| operation | JSON keys | Semantics | UJ |
| --- | --- | --- | --- |
| `initializeVault` | `signer`, `vault_id` | Create empty vault PDA for signer | yes |
| `deposit` | `signer`, `vault_id`, `amount_lo`, `amount_hi` | Credit vault from signer balance | yes |
| `withdraw` | `signer`, `vault_id`, `amount_lo`, `amount_hi`, optional `withdraw_to` | Debit vault to signer or `withdraw_to` | no |
| `createStream` | `signer`, `vault_id`, `stream_id`, `provider`, `rate`, `allocation_lo`, `allocation_hi` | Open stream to payee (`provider` base58) | yes |
| `pauseStream` | `signer`, `vault_id`, `stream_id` | Pause accrual | no |
| `resumeStream` | `signer`, `vault_id`, `stream_id` | Resume paused stream | no |
| `topUpStream` | `signer`, `vault_id`, `stream_id`, `increase_lo`, `increase_hi` | Increase stream allocation | no |
| `closeStream` | `signer`, `vault_id`, `stream_id`, optional `authority` | Close stream; unaccrued returns to vault; omit `authority` to sign as `signer` | yes |
| `claim` | `owner`, `provider`, `vault_id`, `stream_id` | Payee (`provider`) claims accrued on stream | yes |

### Reads (via chainAction)

| operation | JSON keys | Semantics | UJ |
| --- | --- | --- | --- |
| `getVaultStatus` | `owner`, `vault_id` | Vault holding balance hex + config (e.g. `total_allocated_lo`) | yes |
| `getStreamStatus` | `owner`, `vault_id`, `stream_id` | `accrued_lo`, `unaccrued_lo`, `stream_state` (0 Active, 1 Paused, 2 Closed) | yes |

### Low-level decode helpers (separate invokables)

| Method | Purpose |
| --- | --- |
| `readVaultConfigDecoded` | Decode vault config PDA by base58 account id |
| `readVaultHoldingDecoded` | Decode vault holding PDA |
| `readStreamConfigDecoded` | Decode stream config PDA |
| `readClockDecoded` | Clock PDA |
| `readClock10Decoded` | Default clock-10 account from fixture |

## Recovery

[archive/operator/localnet-recovery.md](../archive/operator/localnet-recovery.md).

## Out of scope

- Store eligibility — [store-integration](../store-integration/)
