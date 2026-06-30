# Developer Journey: Build a Store Query Module with LIP-155 Eligibility

## What the user achieves

A developer builds a Logos Core module that attaches payment stream proofs to Store requests, enabling paid historical message retrieval where providers verify active streams before serving queries.

## Why it matters

Logos networks are self-sustaining: users pay providers for services rather than relying on external subsidies. This example uses Store (a Logos Delivery protocol) to demonstrate paid querying of historical messages through payment streams.

## Key components

* `lez-payment-streams` (on-chain program): SPEL guest implementing LIP-155 payment streams — vaults, streams, deposits, claims. Runs on Logos Execution Zone (LEZ).
* `payment_streams_module`: Universal Logos Core module exposing LIP-155 via `chainAction` and eligibility proof methods.
* `delivery_module`: Logos Delivery module with Store protocol and eligibility hooks.
* `wallet_module` (`logos_execution_zone`): Chain interaction for the payment streams module.
* `scripts/e2e/run_local_e2e.py`: Dual-host orchestrator driving user and provider logoscore instances.

See [docs/reference/integration-contracts.md](docs/reference/integration-contracts.md) for API signatures and [docs/store-integration/README.md](docs/store-integration/) for Store integration details.

## Repository

https://github.com/logos-co/lez-payment-streams (FILL IN: exact repository URL)

## Runtime target

testnet v0.2

## Prerequisites

* OS: Linux (Ubuntu 22.04+) or macOS 14+
* Hardware:
  - Minimum: 2 GB RAM, ~5 GB free disk for Nix store + local LEZ
  - Recommended: 4 GB RAM for dual-host parallel operation
* Network: Internet access for Nix flakes and testnet access (testnet flows only)
* Tools (all provided via Nix):
  - Nix with flakes enabled
  - Rust toolchain with RISC Zero for guest ELF builds
  - Logos scaffold CLI (`lgs`) for localnet management

Cold start setup: [docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine](docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine)

## Commands and expected outputs

The verification tests a complete payment stream lifecycle: user creates a vault and opens a stream to a provider, performs a paid Store query demonstrating the active stream, then closes and claims.

### One-command verification (recommended)

```bash
./scripts/e2e.sh local run
```

Expected output: exit 0; artifact `.scaffold/e2e/artifacts/e2e-*.log` containing:

```jsonl
{"phase": "fixture_prepare", "status": "ok"}
{"phase": "store_query_success", "status": "ok"}
{"phase": "store_query_missing_proof", "status": "ok"}
{"phase": "claim", "status": "ok"}
```

### Testnet verification (advanced)

```bash
make bootstrap-testnet
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

Note: Payee `claim` may be optional on testnet. See [docs/archive/operator/testnet-claim-known-issue.md](docs/archive/operator/testnet-claim-known-issue.md).

### Step-by-step path (manual)

For explicit commands without the orchestrator, see [docs/store-integration/README.md#step-by-step-path](docs/store-integration/README.md#step-by-step-path) and the archived runbook [docs/archive/steps/local-store-dual-host-runbook.md](docs/archive/steps/local-store-dual-host-runbook.md).

## Success command

```bash
./scripts/e2e.sh local run
```

## Expected result

Exit code 0. JSON-lines artifact at `.scaffold/e2e/artifacts/e2e-*.log` with phases `store_query_success`, `store_query_missing_proof`, and `claim` all reporting `"status": "ok"`.

## Configuration details

### Fixture manifest

Default fixture: `fixtures/localnet.json` with `provider_account_id`, `service_id` (`/vac/waku/store-query/3.0.0`), `min_rate: 1`, `min_allocation: 1`.

### Key environment variables

* `PAYMENT_STREAMS_GUEST_BIN`: Path to compiled guest ELF
* `MODE`: `store` (Store integration) or `module` (module only)
* `CHAIN`: `local` or `testnet`
* `SKIP_BUILD=1`: Skip `.lgx` rebuilds on subsequent runs

### Module dependencies

Store integration requires `logos_execution_zone`, `payment_streams_module`, and `delivery_module`. See [docs/reference/feature-branch-pins.md](docs/reference/feature-branch-pins.md) for sibling repo pins.

## Failure modes and limits

| Failure | Cause | Resolution |
|---------|-------|------------|
| `NO_ELIGIBLE_VAULT` | No vault initialized or insufficient deposit | Run `initializeVault` and `deposit` first |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | `topUpStream` to add more allocation |
| `PROOF_INVALID` | Eligibility proof verification failed | Ensure stream is active; check N8 canonical payload |
| `STREAM_NOT_ACTIVE` | Stream paused or closed | `resumeStream` or create new stream |

Full error reference: [docs/reference/integration-contracts.md](docs/reference/integration-contracts.md)

## GitHub handle

@FILL_IN

## Discord handle

FILL_IN

## Existing docs or specs

* **LIP-155 (Payment Streams)**: https://lip.logos.co/anoncomms/raw/payment-streams.html
* **RFC 73 (Store Eligibility)**: https://rfc.vac.dev/spec/73/
* **integration-contracts.md**: [docs/reference/integration-contracts.md](docs/reference/integration-contracts.md)
* **Store integration**: [docs/store-integration/README.md](docs/store-integration/)
* **Verification matrix**: [docs/reference/verification-matrix.md](docs/reference/verification-matrix.md)

## Additional context

### Sibling repositories

Store integration requires patched forks:

* `logos-delivery`: Store protocol with eligibility hooks
* `logos-delivery-module`: Module packaging for delivery

Pins: [docs/reference/feature-branch-pins.md](docs/reference/feature-branch-pins.md)

### Estimated time to complete

* Cold start (first time): 20–40 minutes (Nix deps, guest build, scaffold init)
* Subsequent local runs: 2–5 minutes
* Testnet runs: 5–10 minutes

## Security notes

* Fixture manifests contain test keys; never use for mainnet
* Private keys stay in `wallet_module`; proofs are signed attestations
* Privacy-preserving (PP) mode uses circuits; this journey covers transparent mode
