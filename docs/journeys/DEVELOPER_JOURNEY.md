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

The verification runs a per-run stream (Step 24c): user creates vault/stream (or continues from fixture), registers the provider peer mapping, performs a paid Store query with a stream proof, checks rejection without proof, then settlement **Close** then **Claim** (residual accrued on the closed stream when non-zero).

After both logoscore wallets open, the orchestrator runs
`scripts/auth-transfer-ensure.sh` (same `ps_auth_transfer_ensure` as module E2E and
`scripts/fixture.sh prefund`) and records `auth_init_owner` / `auth_init_provider`
with an `extra` object (`account_id`, `already_initialized`, `via`, `verify`, optional
`tx_hash`). `./scripts/e2e.sh` sets `FIXTURE_MANIFEST`, `WALLET_CONFIG`, and
`WALLET_STORAGE` from `CHAIN` on each run so `CHAIN=testnet` always uses
`.scaffold/e2e/testnet-wallet` and the testnet fixture path.

Integrators must learn the provider's libp2p `PeerId` out of band to call `registerProviderMapping` ([N18](docs/reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)); the E2E script uses manifest-backed ids.

### Fresh vault per run (Step 33)

Each Store run ensures a **fresh vault** on chain instead of reusing a shared
vault 0. The orchestrator scans vault ids upward from 0 using an empty-config
probe (`vault_config_is_empty`) and picks the first id whose vault config
account has no data. It then runs `fixture.sh vault ensure <id>` (localnet) or
`scripts/e2e/ensure-testnet-vault.sh` (testnet) to initialize and deposit,
and creates **stream id 0** on that vault. The fixture baseline
(`fixtures/localnet.json`, `fixtures/testnet.json`) carries identity and
policy fields only (owner, provider, program id, `allocation`, `stream_rate`);
`vault_id`, `vault_config_account_id`, and `vault_holding_account_id` are
written by the orchestrator after ensure.

Set `VAULT_ID=<id>` to skip the scan and pin a specific vault. Set
`E2E_REUSE_BASELINE_VAULT=1` to restore the legacy vault-0 reuse path used by
`make verify-store-local-lifecycle`.

Testnet Store sizing defaults (Step 33): `SEED_ALLOCATION=400`,
`SEED_DEPOSIT_AMOUNT=500`, `E2E_CREATE_VIA=chainaction`. Override via env. The
provider calls `rediscoverStreams` after stream creation and before the first
paid Store query so the eligibility verifier sees the new stream.

Phase ordering (D3): vault ensure -> environment setup -> AT ensure -> stream
creation -> publish Store messages during accrual wait -> eligibility proof ->
paid Store query -> close then claim.

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

Settlement (same run, after Store gates) uses module-shaped lines plus legacy
`demo_*` aliases until D3 follow-up:

```jsonl
{"phase":"auth_init_owner","ok":true,"extra":{…}}
{"phase":"auth_init_provider","ok":true,"extra":{…}}
{"phase":"close_stream","ok":true,"extra":{"stream_id":0,"via":"seed_close_stream_onchain"}}
{"phase":"close_state","ok":true,"extra":{"vault_balance":…,"stream_accrued":…,"stream_state":"Closed"}}
{"phase":"claim","ok":true,"extra":{"stream_id":0,"via":"seed_claim_onchain"}}
{"phase":"claim_balance","ok":true,"extra":{…}}
```

Legacy lines `demo_close_stream`, `demo_close_stream_verify`, and `demo_claim`
mirror the same steps for older log parsers. Skipped claim records
`{"phase":"claim","ok":true,"extra":{"skipped":true,"reason":"zero_accrued",…}}`.

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

Provider host: `verifyEligibilityForStoreQuery`. User host: `initializeVault`, `deposit`, `createStream`, `registerProviderMapping`, `prepareEligibilityProofWithStreamProofForStoreQuery`, then `delivery_module.storeQuery`. Settlement: provider-signed `closeStream` (authority = provider account), then `claim` with **`owner`** set to the vault owner account id on the **closed** stream.

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
* `FIXTURE_MANIFEST`: Override fixture path (`e2e.sh` defaults from `CHAIN`)
* `E2E_CLOSE_VIA`: `seed` (default) or `chainaction` for close/claim submit path
* `PS_AT_LOGOSCORE_WALLET_HANDOFF`: Set by Store E2E when releasing logoscore wallet before standalone wallet CLI for seed close/claim
* `VAULT_ID`: Pin a specific vault id for Store runs (default: scan for first empty config)
* `E2E_REUSE_BASELINE_VAULT=1`: Use legacy vault-0 reuse path (lifecycle regression)
* `SEED_ALLOCATION`: CreateStream allocation in lo (testnet Store default: 400)
* `SEED_DEPOSIT_AMOUNT`: Vault deposit in lo (testnet Store default: 500)
* `E2E_CREATE_VIA`: `seed` or `chainaction` for stream create path (testnet Store default: `chainaction`)

### Module dependencies

Store integration requires `logos_execution_zone`, `payment_streams_module`, and `delivery_module`. See [docs/reference/feature-branch-pins.md](docs/reference/feature-branch-pins.md) for sibling repo pins.

## Failure modes and limits

| Failure | Cause | Resolution |
|---------|-------|------------|
| `NO_ELIGIBLE_VAULT` | No vault initialized or insufficient deposit | Run `initializeVault` and `deposit` first |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | `topUpStream` or create a new stream |
| `PROOF_INVALID` | Eligibility proof verification failed | Ensure stream is active; check N8 canonical payload |
| `STREAM_NOT_ACTIVE` | Stream closed (or not yet active) | Create a new stream; pause/resume are not part of this demo |
| Claim succeeds in module E2E but fails in Store testnet teardown | Provider not AT-initialized or wrong fixture provider | Re-run `ps_auth_transfer_ensure` / bootstrap; fix `provider_account_id` |
| `create_demo_stream` / vault unallocated | Testnet vault holding depleted for fixture owner | `deposit-onchain` or `make bootstrap-testnet` with `FIXTURE_MANIFEST=fixtures/testnet.json` and testnet wallet home |
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
* **Step 32 (AT unify, close-then-claim)**: [docs/plan/upcoming/step-32-auth-transfer-unify-store-claim.md](docs/plan/upcoming/step-32-auth-transfer-unify-store-claim.md)
* **Step 33 (fresh vault, testnet sizing)**: [docs/plan/upcoming/step-33-store-e2e-fresh-vault.md](docs/plan/upcoming/step-33-store-e2e-fresh-vault.md)

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
