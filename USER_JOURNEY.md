# User Journey: Operate Payment Streams via Logos Core Module

## What the user achieves

A user creates and manages payment streams on the Logos Execution Zone, enabling continuous micropayments to service providers with per-second granularity.

## Why it matters

Payment streams replace discrete invoices with continuous value flow, enabling real-time compensation for ongoing services. This journey demonstrates direct operation of the LIP-155 protocol through the Logos Core module interface, independent of any specific service integration.

## Key components

* `payment_streams_module` — Universal Logos Core module exposing LIP-155 vault and stream lifecycle
* `logos_execution_zone` — Wallet module for chain interaction and transaction signing
* `lez-payment-streams` — On-chain SPEL guest program implementing LIP-155 semantics
* `logoscore` — Logos Core CLI for module loading and method invocation

## Repository

https://github.com/logos-co/lez-payment-streams (FILL IN: exact repository URL)

## Runtime target

local LEZ (module only, single-host)

## Prerequisites

* OS: Linux (Ubuntu 22.04+) or macOS 14+
* Hardware: 2 GB RAM, ~5 GB free disk
* Tools: Nix with flakes enabled; Rust toolchain with RISC Zero (for guest builds)
* Network: Internet access for Nix flakes

Cold start: [docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine](docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine)

## Commands and expected outputs

The journey exercises the complete payment stream lifecycle: create vault, deposit funds, open a stream to a provider, pause and resume operations, add more funds, then close and claim accrued value.

### 1. Environment setup

Enter a shell with required tools and initialize:

```bash
nix shell --accept-flake-config \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-package-manager \
  --command bash

# From repo root
lgs init
lgs setup
lgs localnet start

# Build the guest ELF (one-time)
cargo risczero build --manifest-path methods/guest/Cargo.toml
```

### 2. One-command verification (recommended)

Run the complete module lifecycle automatically:

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

This orchestrates: wallet creation, vault initialization, deposit, stream creation with a provider, pause/resume/top-up operations, and provider claim.

Expected output (`.scaffold/e2e/artifacts/module-e2e-*.log`):

```jsonl
{"phase":"wallet_open","ok":true}
{"phase":"vault_init","ok":true}
{"phase":"deposit","ok":true}
{"phase":"create_stream","ok":true}
{"phase":"pause_stream","ok":true}
{"phase":"resume_stream","ok":true}
{"phase":"topup_stream","ok":true}
{"phase":"claim","ok":true}
{"phase":"module_e2e_complete","ok":true}
```

### 3. Manual path (step-by-step)

For explicit control, run commands directly through `logoscore`.

#### Start logoscore and load modules

```bash
# Terminal 1: Start daemon
logoscore -D -m .scaffold/e2e/user/modules &

# Load modules (wallet first)
logoscore load-module logos_execution_zone
logoscore load-module payment_streams_module

# Create and open wallet
logoscore call logos_execution_zone create_new \
  .scaffold/wallet/wallet_config.json \
  .scaffold/wallet/storage.json \
  "your-wallet-password"
```

#### Create accounts and fund them

```bash
# Create owner (who pays) and provider (who receives)
OWNER=$(logoscore call logos_execution_zone create_account_public | tail -1 | jq -r '.result.account_id')
PROVIDER=$(logoscore call logos_execution_zone create_account_public | tail -1 | jq -r '.result.account_id')

# Fund for gas (requires scaffold wallet)
lgs wallet topup --address "Public/$OWNER"
lgs wallet topup --address "Public/$PROVIDER"
```

#### Vault operations

```bash
# Initialize vault for owner
logoscore call payment_streams_module chainAction initializeVault \
  '{"signer":"'$OWNER'","vault_id":0}'

# Deposit funds (100 units)
logoscore call payment_streams_module chainAction deposit \
  '{"signer":"'$OWNER'","vault_id":0,"amount_lo":100,"amount_hi":0}'

# Check vault status
logoscore call payment_streams_module chainAction getVaultStatus \
  '{"owner":"'$OWNER'","vault_id":0}'
```

#### Stream lifecycle

```bash
# Create stream to provider (rate 10/s, allocation 80)
logoscore call payment_streams_module chainAction createStream \
  '{"signer":"'$OWNER'","vault_id":0,"stream_id":0,"provider":"'$PROVIDER'","rate":10,"allocation_lo":80,"allocation_hi":0}'

# Pause stream (temporarily stop accrual)
logoscore call payment_streams_module chainAction pauseStream \
  '{"signer":"'$OWNER'","vault_id":0,"stream_id":0}'

# Resume stream
logoscore call payment_streams_module chainAction resumeStream \
  '{"signer":"'$OWNER'","vault_id":0,"stream_id":0}'

# Top up stream (add 1 more unit)
logoscore call payment_streams_module chainAction topUpStream \
  '{"signer":"'$OWNER'","vault_id":0,"stream_id":0,"increase_lo":1,"increase_hi":0}'

# Check stream status
logoscore call payment_streams_module chainAction getStreamStatus \
  '{"owner":"'$OWNER'","vault_id":0,"stream_id":0}'
```

#### Provider claim

```bash
# Wait for value to accrue (5+ seconds)
sleep 5

# Provider claims accrued funds
logoscore call payment_streams_module chainAction claim \
  '{"provider":"'$PROVIDER'","vault_id":0,"stream_id":0}'
```

#### Close stream (optional)

```bash
# Owner closes the stream (no more accrual)
logoscore call payment_streams_module chainAction closeStream \
  '{"signer":"'$OWNER'","vault_id":0,"stream_id":0}'
```

#### Shutdown

```bash
logoscore stop
```

## Success command

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

## Expected result

Exit code 0. JSON-lines artifact at `.scaffold/e2e/artifacts/module-e2e-*.log` with all phases reporting `"ok":true`, ending with `module_e2e_complete`.

## Configuration details

### Environment variables

* `PAYMENT_STREAMS_GUEST_BIN`: Path to compiled guest ELF
* `VAULT_ID`: Vault identifier (default: 0)
* `STREAM_ID`: Stream identifier (default: 0)
* `DEPOSIT`: Initial deposit amount (default: 100)
* `RATE`: Stream accrual rate per second (default: 10)
* `ALLOCATION`: Amount allocated to stream (default: 80)

### Module requirements

Single-host configuration:
* `logos_execution_zone` — wallet and chain interface
* `payment_streams_module` — LIP-155 operations

No `delivery_module` needed (this is the module-only flow).

## Failure modes and limits

| Symptom | Cause | Fix |
|---------|-------|-----|
| `account data missing` after operation | Module submits async; read before inclusion | Retry after 3–5 seconds |
| `NO_ELIGIBLE_VAULT` | Vault not initialized or wrong ID | Run `initializeVault` first |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | `topUpStream` or `closeStream` then create new |
| `claim` returns 0 | No time elapsed for accrual | Wait longer between create and claim |
| Wallet open fails | Wrong password or missing config | Check `wallet_config.json` path |

### Limits

* Testnet claim is not reliable (future work) — localnet only for this flow
  (LEZ v0.2.0 localnet claim is fixed: the provider must be
  auth-transfer-initialized before the first signer tx; see
  [step-27](docs/plan/completed/step-27-claim-fix-verification.md).
  Public testnet re-test is pending the testnet v0.2.0 upgrade.)
* Stream IDs are per-vault sequential integers
* Paused streams do not accrue; resumed streams continue from pause point

## GitHub handle

@FILL_IN

## Discord handle

FILL_IN

## Existing docs or specs

* **LIP-155 (Payment Streams)**: https://lip.logos.co/anoncomms/raw/payment-streams.html
* **Payment streams module**: [docs/payment-streams-module/README.md](docs/payment-streams-module/)
* **Module E2E script**: [scripts/module-e2e-local.sh](scripts/module-e2e-local.sh)
* **Verification matrix**: [docs/reference/verification-matrix.md](docs/reference/verification-matrix.md)

## Estimated time to complete

* Cold start: 20–40 minutes
* Subsequent runs: 2–5 minutes

## Security notes

* Test keys in fixtures — never reuse for production
* Wallet password protects local storage; private keys never leave wallet module
* Provider can claim without owner approval (designed behavior) but cannot withdraw more than accrued
