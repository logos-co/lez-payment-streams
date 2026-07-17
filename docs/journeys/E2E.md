# E2E verification recipes

Maintainer and integrator run recipes for the mode × chain matrix.
Tier definitions, cold start, and artifact locations stay in
[verification-matrix.md](../reference/verification-matrix.md).
This file is the SSOT for per-cell commands, bootstrap one-liners, and expected outcomes.

Entry point: [`scripts/e2e.sh`](../../scripts/e2e.sh) with `MODE` and `CHAIN`.
Each `local run` / `testnet run` performs prepare, run, and teardown unless `SKIP_TEARDOWN=1`.

Automated module verification narrative and JSONL phases: [`scripts/module-e2e.sh`](../../scripts/module-e2e.sh).
Hands-on testnet commands for learning LIP-155 without scripts:
[USER_JOURNEY.md](USER_JOURNEY.md).

## Doc boundary

| Document | Role |
| --- | --- |
| [verification-matrix.md](../reference/verification-matrix.md) | Required tiers, cold start, maintainer notes, artifact paths |
| E2E.md (this file) | Per-cell prepare/bootstrap and run recipes, plus privacy profile overlays |
| [USER_JOURNEY.md](USER_JOURNEY.md) | End-user testnet CLI walkthrough (module only, no Store) |
| [DEVELOPER_JOURNEY.md](DEVELOPER_JOURNEY.md) | Protocol-agnostic eligibility integration guide (Store as worked example) |
| [PRIVACY_ENHANCED_JOURNEY.md](PRIVACY_ENHANCED_JOURNEY.md) | Hands-on owner and provider privacy narrative |

## Shared prepare

Build guest ELF and module `.lgx` artifacts (first run is slow):

```bash
./scripts/e2e.sh build
```

Cold start (Nix shell, `lgs init`, `lgs setup`, delivery checkout for Store):
[verification-matrix.md#cold-start-first-time-on-a-machine](../reference/verification-matrix.md#cold-start-first-time-on-a-machine).

Module runs set `PAYMENT_STREAMS_GUEST_BIN` to the built guest under `methods/guest/target/...`
when the file exists (see `scripts/e2e.sh`).

Scaffold paths (modules, wallets, artifacts): [naming-conventions.md](../reference/naming-conventions.md#scaffold-layout).

## Module × localnet (User Journey)

Required tier. Single host, no Store.

```bash
SKIP_BUILD=1 MODE=module CHAIN=local ./scripts/e2e.sh local run --verbosity verbose
```

Make alias: `make verify-module-local`.

Expected: exit code 0; console ends with `E2E COMPLETE: All phases succeeded`.
Artifact: `.scaffold/e2e/artifacts/module-e2e-*.log` with phases including
`vault_init`, `deposit`, `create_stream`, `accrual`, `close_stream`, `claim`, `module_e2e_complete`.

Optional top-up phase:

```bash
MODULE_E2E_TOPUP=1 SKIP_BUILD=1 MODE=module CHAIN=local ./scripts/e2e.sh local run
```

## Module × localnet (owner privacy)

Optional privacy profile overlay on the User Journey module cell.
Owner privacy and provider privacy are independent choices; this cell covers
payer unlinkability only (`PseudonymousFunder` vault, public provider).

```bash
SKIP_BUILD=1 MODE=module CHAIN=local OWNER_PRIVACY=1 ./scripts/e2e.sh local run
```

Make alias: `make verify-module-local-privacy`.

`PRIVACY=1` is accepted as an alias for `OWNER_PRIVACY=1` when `OWNER_PRIVACY` is unset.

Expected: exit code 0; same lifecycle phases as the public module cell, plus
`pre_shield` before vault init. `OWNER_PRIVACY=1` defaults pause/resume and
top-up on. Artifact: `.scaffold/e2e/artifacts/module-e2e-*.log`.

## Module × localnet (provider privacy)

Optional privacy profile overlay for payee receiver privacy (private provider
account, shielded claim). Independent of `OWNER_PRIVACY`.

```bash
SKIP_BUILD=1 MODE=module CHAIN=local PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run
```

Make alias: `make verify-module-local-provider-privacy`.

Expected: exit code 0; public vault owner; private provider; claim confirms via
`vault_holding` drop (destination shielded). AT-init covers the public owner
only. Combo with owner privacy:

```bash
SKIP_BUILD=1 MODE=module CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run
```

Store × privacy profiles remain
[Step 38](../plan/upcoming/step-38-store-privacy-e2e.md).

Narrative walkthrough: [PRIVACY_ENHANCED_JOURNEY.md](PRIVACY_ENHANCED_JOURNEY.md).

## Module × testnet (User Journey)

Required tier. One-time bootstrap on the machine:

```bash
make bootstrap-testnet-module
```

Creates wallet layout and `fixtures/testnet-module.json` (sequencer URL, program id, payer/payee
account ids). Does not replace per-run funding; fund accounts before or during the run.

Pre-fund fixture accounts (recommended before repeated demos):

```bash
./scripts/fund-testnet-accounts.sh
```

Run:

```bash
SKIP_BUILD=1 MODULE_E2E_SKIP_FUND=1 MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run --verbosity verbose
```

Make alias: `make verify-module-testnet`.

Expected: exit code 0 and the same phase names as localnet in `module-e2e-*.log`.
The script auto-resolves a fresh empty `vault_id` under the fixture payer unless `VAULT_ID` is pinned.

Sizing SSOT for docs and fixture: `demo_deposit_amount` 500, `allocation` 80, `stream_rate` 1 in
[`fixtures/testnet-module.json`](../../fixtures/testnet-module.json).
`module-e2e.sh` env overrides (`DEPOSIT`, `ALLOCATION`, …) may differ; the fixture fields are what
[USER_JOURNEY.md](USER_JOURNEY.md) teaches.

Note: `module-e2e.sh` still passes the payee as `closeStream` `authority` until
[e2e-close-payer-authority.md](../plan/raw-todos/e2e-close-payer-authority.md) lands.
[USER_JOURNEY.md](USER_JOURNEY.md) documents payer-only close (omit `authority`).

## Store × localnet (Developer Journey)

Required tier. Dual-host Store query with eligibility proof.

```bash
SKIP_BUILD=1 E2E_VERBOSITY=verbose MODE=store CHAIN=local ./scripts/e2e.sh local run
```

Make alias: `make verify-store-local`.

Expected: exit code 0; artifact `.scaffold/e2e/artifacts/e2e-*.log` includes Store query phases
(for example `store_query_success`, `store_query_missing_proof`) and settlement phases per the
orchestrator in [`scripts/e2e/run_local_e2e.py`](../../scripts/e2e/run_local_e2e.py).

Maintainer lifecycle regression (not an integrator gate):

```bash
make verify-store-local-lifecycle
```

## Store × testnet (Developer Journey)

Required tier, but this recipe section stays minimal until Step 32 D3 gate passes.

One-time Store bootstrap:

```bash
make bootstrap-testnet
```

Run (prepare + orchestrator):

```bash
SKIP_BUILD=1 E2E_VERBOSITY=verbose MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

Make alias: `make verify-store-testnet`.

Expected when green: exit code 0 and `e2e-*.log` with paid Store query and claim-related phases.
Teardown keeps default `E2E_CLAIM_OPTIONAL=1` until Step 32 D3 gate passes; strict runs use
`E2E_CLAIM_OPTIONAL=0`. See
[verification-matrix.md](../reference/verification-matrix.md) and
[step-32-testnet-gate-log.md](../plan/completed/step-32-testnet-gate-log.md).

Gate history: [step-33-testnet-gate-log.md](../plan/completed/step-33-testnet-gate-log.md).

## Configuration

### Key environment variables

- `PAYMENT_STREAMS_GUEST_BIN`: Path to compiled guest ELF.
- `MODE`: `store` (default) or `module` (single-host module E2E only).
- `CHAIN`: `local` or `testnet`.
- `OWNER_PRIVACY`: `0` (default) or `1` for PseudonymousFunder vault owner (module mode).
- `PROVIDER_PRIVACY`: `0` (default) or `1` for private provider claim (Step 37; module mode).
- `PRIVACY`: alias for `OWNER_PRIVACY=1` when `OWNER_PRIVACY` is unset.
- `SKIP_BUILD=1`: Skip `.lgx` rebuilds on subsequent runs.
- `E2E_CLAIM_OPTIONAL`: Testnet claim strictness (default `1`; use `0` for strict).
- `FIXTURE_MANIFEST`: Override fixture path.
- `E2E_CLOSE_VIA`: `seed` (default) or `chainaction` for close/claim submit path.
- `VAULT_ID`: Pin vault id (default: scan for first empty config).
- `E2E_REUSE_BASELINE_VAULT=1`: Vault-0 reuse path (lifecycle regression).
- `SEED_ALLOCATION`: `createStream` allocation in lo (testnet Store default: 400).
- `SEED_DEPOSIT_AMOUNT`: Vault deposit in lo (testnet Store default: 500).
- `E2E_CREATE_VIA`: `seed` or `chainaction` for stream create (testnet default: `chainaction`).
- `SKIP_TEARDOWN=1`: Skip teardown phase in `local run` / `testnet run`.

### Module dependencies

At runtime the Store demo loads `logos_execution_zone`, `payment_streams_module`, and
`delivery_module`. Module-only verification (`MODE=module`) does not need delivery
checkouts.

### Verbosity

Console output level via `./scripts/e2e.sh --verbosity quiet|normal|verbose` or
`E2E_VERBOSITY`:

- `quiet` — JSON-lines artifact only.
- `normal` — phase headers, status markers, on-chain values.
- `verbose` — adds concept explanations.

### Demo assumptions

The script is a demo harness, not a production deployment pattern.
Provider libp2p peer id for `registerProviderMapping` comes from the fixture.
On testnet, `E2E_CLAIM_OPTIONAL` defaults to `1`; set `0` for strict claim confirmation.
Each Store run scans vault ids from 0 upward and uses the first unused id.
`VAULT_ID=<id>` pins a vault.
`E2E_REUSE_BASELINE_VAULT=1` selects the vault-0 reuse path for
`make verify-store-local-lifecycle`.

## Failure modes and limits

| Failure | Cause | Resolution |
|---------|-------|------------|
| `NO_ELIGIBLE_VAULT` | Vault missing or insufficient deposit | Run vault ensure / deposit; check vault scan |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | Create a new stream or top up |
| `PROOF_INVALID` | Eligibility proof verification failed | Confirm stream is active; check N8 payload |
| `STREAM_NOT_ACTIVE` | Stream closed or not yet active | Create a new stream on the vault |
| Claim fails on Store testnet teardown | AT or fixture provider | Re-run AT ensure; fix `provider_account_id` |
| Vault unallocated on testnet | Depleted holding for owner | Deposit or re-bootstrap with testnet wallet home |
| Store query dial failures | Provider unreachable on libp2p | Check multiaddr and peer id in manifest |

## API shape for integrators

Module writes and status reads use a single router:
`logoscore call payment_streams_module chainAction <operation> '<json>'`.

Full operation catalogue:
[payment-streams-module/README.md](../payment-streams-module/README.md#chainaction-catalogue).
