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

For the payment-streams-only path (no Store), see [User Journey](USER_JOURNEY.md) (`MODE=module`, `scripts/module-e2e.sh`).

## Repository

https://github.com/logos-co/lez-payment-streams (FILL IN: exact repository URL)

## Runtime target

Local LEZ (dual-host Store demo) is the primary verification target. TestNet v0.2 is supported for the same Store orchestrator with a gitignored `fixtures/testnet.json` baseline (`make bootstrap-testnet`).

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

Store integration also requires sibling checkouts `logos-delivery` and `logos-delivery-module` at pins in [feature-branch-pins.md](docs/reference/feature-branch-pins.md).

## Commands and expected outputs

The verification runs a per-run stream (Step 24c): user creates vault/stream (or continues from fixture), registers the provider peer mapping, performs a paid Store query with a stream proof, checks rejection without proof, then teardown closes the stream and claims accrued value to the provider when non-zero.

Integrators must learn the provider’s libp2p `PeerId` out of band to call `registerProviderMapping` ([N18](docs/reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)); the E2E script uses manifest-backed ids.

### One-command verification (recommended, localnet)

```bash
make verify-store-local
```

Equivalent:

```bash
./scripts/e2e.sh local run
```

Expected: exit 0; artifact `.scaffold/e2e/artifacts/e2e-*.log` with headline gates:

```jsonl
{"phase":"store_query_success","ok":true}
{"phase":"store_query_missing_proof","ok":true}
```

Teardown (same run, end of core phase) also records close/claim lines such as `demo_close_stream_verify` and `demo_claim` when accrued balance is non-zero.

### Testnet verification (advanced)

One-time:

```bash
make bootstrap-testnet
```

After a public sequencer relaunch, redeploy the guest if needed:

```bash
make deploy-testnet
```

Run:

```bash
make verify-store-testnet
```

Equivalent:

```bash
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

LEZ v0.2.0 requires owner and provider accounts to be registered under the
`authenticated_transfer` program before deposit/claim debits and credits settle.
`make bootstrap-testnet`, `scripts/fixture.sh prefund`, and Store E2E call
`scripts/auth-transfer-ensure.sh` (shared `ps_auth_transfer_ensure`) with strict
on-chain verify against the `authenticated_transfer` ImageID. Testnet wallet
home: `.scaffold/e2e/testnet-wallet` (manifest owner and provider must be
signable from that storage).

Settlement is narrated as **Close** then **Claim**: `close_stream` and
`close_state`, then `claim` and optional `claim_balance` when residual accrued
after close is greater than zero. Seed submit for close and claim defaults via
`E2E_CLOSE_VIA=seed` (same env governs both phases).

Claim in teardown uses an explicit stream **owner** in `chainAction` JSON (not only
`signer` on the provider host), matching [integration-contracts — chain teardown](docs/reference/integration-contracts.md#chain-teardown-step-24c-local-e2e):

```json
{"owner":"<owner_account_id>","provider":"<provider_account_id>","vault_id":0,"stream_id":<id>}
```

The orchestrator prefers `seed_localnet_fixture claim-onchain` on testnet; `chainAction claim` is the fallback. By default **`E2E_CLAIM_OPTIONAL=1` on testnet**, so a claim that does not confirm on chain may still yield exit 0; set **`E2E_CLAIM_OPTIONAL=0`** to fail the run when claim does not settle. Product verification treats claim as required on both chains ([verification matrix](docs/reference/verification-matrix.md)); see [testnet-claim-known-issue](docs/archive/operator/testnet-claim-known-issue.md) (resolved on v0.2.0).

### Step-by-step path (manual)

For explicit commands without the orchestrator, see [docs/store-integration/README.md#step-by-step-path](docs/store-integration/README.md#step-by-step-path) and the archived runbook [docs/archive/steps/local-store-dual-host-runbook.md](docs/archive/steps/local-store-dual-host-runbook.md).

Provider host: `verifyEligibilityForStoreQuery`. User host: `initializeVault`, `deposit`, `createStream`, `registerProviderMapping`, `prepareEligibilityProofWithStreamProofForStoreQuery`, then `delivery_module.storeQuery`. Teardown: provider-signed `closeStream` (authority = provider account) and `claim` with **`owner`** set to the vault owner account id.

## Success command

```bash
make verify-store-local
```

## Expected result

Exit code 0. JSON-lines artifact at `.scaffold/e2e/artifacts/e2e-*.log` with
`store_query_success` and `store_query_missing_proof` reporting `"ok":true`,
`auth_init_owner` / `auth_init_provider`, `close_state` before `claim`, and when
teardown accrual is non-zero, `claim` with `"ok":true` (canonical; `demo_claim`
mirrors the same payload until D3 follow-up). Skipped claim uses
`"reason":"zero_accrued"`.

## Configuration details

### Fixture manifest

Default local fixture: `fixtures/localnet.json` with `owner_account_id`, `provider_account_id`, `service_id` (`/vac/waku/store-query/3.0.0`), demo policy fields (`min_rate`, `min_allocation`, …). Testnet: gitignored `fixtures/testnet.json` from `make bootstrap-testnet`.

### Key environment variables

* `PAYMENT_STREAMS_GUEST_BIN`: Path to compiled guest ELF
* `MODE`: `store` (Store integration, default) or `module` (User Journey only)
* `CHAIN`: `local` or `testnet`
* `SKIP_BUILD=1`: Skip `.lgx` rebuilds on subsequent runs
* `E2E_CLAIM_OPTIONAL`: On testnet defaults to `1`; set `0` to require confirming claim in teardown
* `FIXTURE_MANIFEST`: Override fixture path

### Module dependencies

Store integration requires `logos_execution_zone`, `payment_streams_module`, and `delivery_module`. See [docs/reference/feature-branch-pins.md](docs/reference/feature-branch-pins.md) for sibling repo pins.

## Failure modes and limits

| Failure | Cause | Resolution |
|---------|-------|------------|
| `NO_ELIGIBLE_VAULT` | No vault initialized or insufficient deposit | Run `initializeVault` and `deposit` first |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | `topUpStream` or create a new stream |
| `PROOF_INVALID` | Eligibility proof verification failed | Ensure stream is active; check N8 canonical payload |
| `STREAM_NOT_ACTIVE` | Stream closed (or not yet active) | Create a new stream; pause/resume are not part of this demo |
| Claim succeeds in module E2E but fails in Store testnet teardown | Provider not AT-initialized or wrong fixture provider | Re-run bootstrap auth-transfer init; fix `provider_account_id` |
| Store query dial failures | Provider not reachable on libp2p | Check provider node multiaddr and peer id in manifest |

Full error reference: [docs/reference/integration-contracts.md](docs/reference/integration-contracts.md)

Recovery: [docs/archive/operator/localnet-recovery.md](docs/archive/operator/localnet-recovery.md)

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
* **Step 20 plan packet**: [docs/plan/upcoming/step-20-developer-journey.md](docs/plan/upcoming/step-20-developer-journey.md)

## Additional context

### Sibling repositories

Store integration requires patched forks:

* `logos-delivery`: Store protocol with eligibility hooks
* `logos-delivery-module`: Module packaging for delivery

Pins: [docs/reference/feature-branch-pins.md](docs/reference/feature-branch-pins.md)

### Estimated time to complete

* Cold start (first time): 20–40 minutes (Nix deps, guest build, scaffold init, delivery siblings)
* Subsequent local Store runs: about 3–8 minutes (`make verify-store-local`)
* Testnet Store runs: often 10–20+ minutes (inclusion and libp2p); module-only testnet is shorter — see [User Journey](USER_JOURNEY.md)

## Security notes

* Fixture manifests contain test keys; never use for mainnet
* Private keys stay in `wallet_module`; proofs are signed attestations
* Privacy-preserving (PP) mode uses circuits; this journey covers transparent mode
