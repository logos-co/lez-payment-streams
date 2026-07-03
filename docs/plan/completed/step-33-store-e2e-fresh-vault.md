# Step 33 — Store E2E fresh vault and testnet sizing

Index: [index.md](../index.md). Status: **complete** — testnet gate in
[step-33-testnet-gate-log.md](step-33-testnet-gate-log.md) (2026-07-03).

Prerequisites: [Step 24c](../completed/step-24c-simplify-demo-flow.md) (per-run stream,
vault baseline model), [Step 28](../completed/step-28-user-journey-testnet.md) (module
testnet patterns), [Step 32](../upcoming/step-32-auth-transfer-unify-store-claim.md) (AT ensure,
close-then-claim). Related UX: [Step 29](step-29-e2e-script-ux.md) (phase tables updated with
orchestrator changes).

## Summary

Refactor Developer Journey E2E (`MODE=store`, `run_local_e2e.py`) so automated runs can
use a **fresh vault id per run** with **init + deposit + createStream(stream_id=0)** sized
for **`allocation`** and testnet wall-clock latency, instead of relying on a long-lived
**vault 0** on a shared fixture whose **unallocated** balance and stream history cause
intermittent `verify-store-testnet` failures.

User Journey (`MODE=module`) already uses a **new `VAULT_ID` per run** to demonstrate full
vault lifecycle. Developer Journey does **not** need to teach vault creation in the
narrative, but verification should not depend on manual vault hygiene on public testnet.

## Why this step exists

### Journey goals differ

| Journey | Primary proof | Vault in the story |
| --- | --- | --- |
| User (`module-e2e.sh`) | LIP-155 lifecycle end to end | Create vault, deposit, stream, close, claim |
| Developer (Store E2E) | Paid Store + eligibility proof | Infrastructure; integration is Store + proof |

Step 24c optimized Developer Journey for **localnet**: restore a **funded vault 0**
snapshot, create a **new stream id each run**, teardown close (optional claim). That
matches fast local iteration and **`verify-store-local-lifecycle`** (one vault, many
streams on one ledger).

### What broke on testnet (evidence)

Store testnet runs showed **two failure classes** (not AT or Store wire):

1. **Create rejected** — `vault unallocated X < requested allocation Y`. Owner LEZ balance
   was fine; **program vault unallocated** on shared **vault 0** was depleted by prior
   streams and runs. Pinata does not fix this because it credits the owner account,
   not the program vault's unallocated balance.

2. **Create succeeded, fundability gate failed** — `stream_fully_accrued_or_depleted`:
   small demo **`allocation`** (e.g. 80), rate 1/s, long dual-host setup before Store,
   and `wait_stream_fundable` requiring **`min_unaccrued_lo`** (e.g. 64 for allocation 80)
   left **unaccrued 0** before `storeQuery`.

Module testnet passed with **fresh `VAULT_ID`** and the Step 32 **`e2e.sh` module path**
wallet/fixture rebind (commit `ea0f89a`). Store testnet still used **shared vault 0** in
`fixtures/testnet.json`.

Root cause is **E2E economics and shared state**, not a missing Store feature. Timing
matters for failure class 2; **shared vault liquidity** matters for class 1.

## Target behavior (implementer)

After this step (defaults as decided in the [Decision log](#decision-log)):

1. **Pick `VAULT_ID`** for the run (env or auto; see safe defaults).
2. **Strip per-run stream fields** from manifest (existing 24c helper); set **`vault_id`**.
3. **Vault ensure** — for localnet use `fixture.sh vault ensure <VAULT_ID>`; for testnet
   use the dedicated testnet helper. Both run `initialize-vault-onchain` + `deposit-onchain`
   with chain-appropriate **`SEED_DEPOSIT_AMOUNT`** (must cover **`allocation`** on an empty
   vault).
4. **Refresh vault PDAs** in manifest (`vault_config_account_id`, `vault_holding_account_id`).
5. Dual-host flow: AT ensure, then **create stream 0** on that vault, then publish Store
   messages while waiting for the stream to become fundable. (See D3.)
6. Existing gates: fundability → `store_query_success` / `store_query_missing_proof` →
   close then claim (Step 32).

Fixture baseline (`fixtures/testnet.json`, local `localnet.json`) is **identity +
policy** only (owner, provider, program id, **`allocation`**, `stream_rate`, service ids).
It does not encode a fixed vault or vault PDAs.

## Scope

| In | Out |
| --- | --- |
| `run_local_e2e.py` fresh-vault path, artifacts (`plan_demo_vault`, `vault_ensure`) | User Journey / `module-e2e.sh` vault selection (already fresh id) |
| Chain-appropriate vault ensure helpers (`fixture.sh` for localnet, dedicated testnet helper) | Guest program or LIP-155 rule changes |
| `e2e.sh` env defaults for Store + testnet sizing | Replacing Python orchestrator with bash |
| Docs: DEVELOPER_JOURNEY, verification-matrix, bootstrap runbook notes | Step 21 UI |
| `E2E_REUSE_BASELINE_VAULT=1` legacy reuse-vault mode (lifecycle / back-to-back) | Full bootstrap archive rewrite (D6-style defer unless needed) |

## Safe defaults (no further discussion)

Implement these unless an open question explicitly overrides.

| Topic | Default |
| --- | --- |
| Manifest field for stream size | **`allocation`** only ([Step 32 terminology](../completed/step-32-auth-transfer-unify-store-claim.md) / fc351ca) |
| Stream id on new vault | **`0`** (first stream on that vault) |
| `VAULT_ID` | Honor **`VAULT_ID` env** if set; else scan owner vault ids upward from **0** using an **empty-config probe** (not `vault_is_funded`) until the vault config account has no data (first free id) |
| Deposit vs allocation | **`SEED_DEPOSIT_AMOUNT` ≥ `allocation` + 100** lo buffer on empty vault; testnet Store run sets **`SEED_ALLOCATION`** from manifest **`allocation`** unless **`SEED_ALLOCATION` env** overrides |
| Testnet allocation default | **`400`** lo in manifest example / Store testnet profile unless manifest specifies (raises fundability headroom vs 80) |
| Testnet deposit default | **`500`** lo when using allocation 400 (override via env) |
| Fundability formula | Unchanged: **`min_unaccrued_lo = max(64, min(allocation // 4, 50_000))`** |
| Testnet vault ensure I/O | Via dedicated testnet helper using `LEZ_TESTNET_SUBMIT` + rc3 wallet poll (same pattern as `create-testnet-stream-fixture.sh`) |
| Testnet create path | **`E2E_CREATE_VIA=chainaction`** default for **`CHAIN=testnet`** only; local Store keeps seed default unless env set |
| New vault init | Always **init + deposit** for chosen id (do not skip deposit because vault 0 exists elsewhere) |
| Teardown | Close stream **0** on run vault; **leave vault on chain** (no vault delete) |
| Testnet claim gate | Keep **`E2E_CLAIM_OPTIONAL=1`** until Step 32 D3 gate log says otherwise |
| Owner funding | No new pinata in Store path; assume bootstrap / operator funded owner; preflight requires balance >= `SEED_DEPOSIT_AMOUNT` per run, and `2 * SEED_DEPOSIT_AMOUNT` (1000 lo) for the two-pass testnet gate |
| Manifest writes | Continue updating **`FIXTURE_MANIFEST`** on disk for seed compatibility; strip **`stream_*`** after run start; update **`vault_id`** and vault PDAs each run |

**Notes on safe defaults:**

- **Reuse mode:** `E2E_REUSE_BASELINE_VAULT=1` selects the legacy vault-0 reuse path.
  Default is fresh-vault per run.
- **`min_unaccrued_lo` for allocation 400:** 100 lo (`max(64, min(400 // 4, 50_000))`).
- **`demo_deposit_amount`:** a manifest field written by the seed/bootstrap tools to
  reflect the actual on-chain vault holding balance. It is not the control input.
  The env `SEED_DEPOSIT_AMOUNT` controls the deposit.
- **Testnet owner balance floor:** at least `2 * SEED_DEPOSIT_AMOUNT` (1000 lo) for two
  consecutive `verify-store-testnet` passes.

## Implementation sequence

1. **Helpers** — resolve `VAULT_ID` (env + scan) using an empty-config probe;
   `ensure_fresh_vault_for_store_run(manifest, artifact)` wrapping the chain-appropriate
   vault ensure helper (`fixture.sh` for localnet, dedicated helper for testnet) + PDA
   refresh.
2. **`run_local_e2e.py`** — fresh-vault path is the default; call ensure before stream
   create; force **`STREAM_ID=0`** for fresh vault; support `E2E_REUSE_BASELINE_VAULT=1` for
   legacy lifecycle mode; log phases.
3. **`e2e.sh`** — export testnet Store sizing env; document **`VAULT_ID`** optional.
4. **Local path** — make fresh-vault the default for `make verify-store-local`; gate
   `fixture.sh vault ensure 0` on `E2E_REUSE_BASELINE_VAULT=1`. Update
   `make verify-store-local-lifecycle` to set the flag for both legs.
5. **Docs** — DEVELOPER_JOURNEY testnet section (vault ensure, sizing, no manual vault 0
   hygiene for verify); verification-matrix row; short operator note in
   `docs/archive/operator/` if needed.
6. **Verify** — `make verify-store-testnet` twice consecutively without manual
   `deposit-onchain`; `make verify-store-local` (+ lifecycle target via `E2E_REUSE_BASELINE_VAULT=1`).

Coordinate [Step 29](step-29-e2e-script-ux.md) phase tables when adding console steps for
vault ensure.

## Verification strategy

The goal is to catch regressions quickly without running the full testnet E2E after
every change. Use a tiered approach.

### 1. Baseline

Before changing code, run the current green baseline and record the artifacts:

```bash
make verify-store-local
make verify-store-local-lifecycle
```

Save the artifact logs and the manifest state so regressions are obvious.

### 2. Fast localized checks (seconds)

Run after every commit or before each targeted E2E:

- `make fmt` / shellcheck on edited scripts.
- Python unit tests for pure functions in `scripts/e2e/run_local_e2e.py`.
- Bash component tests for `scripts/fixture.sh` vault helpers.

### 3. Component tests (minutes)

Run after touching a specific helper:

- `fixture.sh vault ensure <id>` on a fresh localnet to confirm init + deposit and
  idempotency.
- `seed_localnet_fixture write-vault-manifest` to confirm it produces an identity-only
  manifest.
- `ensure-testnet-vault.sh --dry-run` to confirm argument parsing and manifest output.
- `e2e.sh local prepare` with `E2E_PREPARE_DRY_RUN=1` to confirm `cmd_prepare_local`
  does not call `fixture.sh vault ensure 0` in default mode and does call it with
  `E2E_REUSE_BASELINE_VAULT=1`.

### 4. Targeted local E2E (tens of minutes)

Run after each major milestone:

- `make verify-store-local` with the fresh-vault default.
- `make verify-store-local` with `VAULT_ID=3` to exercise the scanner.
- `E2E_REUSE_BASELINE_VAULT=1 make verify-store-local-lifecycle` to exercise the reuse
  path.

### 5. Full testnet gate

Run only after local E2E is green:

```bash
make verify-store-testnet
```

Repeat twice as specified in the verification table.

### New tests to add

- `tests/scripts/test_fixture_vault.sh` — tests `vault_config_is_empty`, `vault_is_funded`,
  and the `VAULT_ID` scanner.
- `tests/e2e/test_run_local_e2e_pure.py` — unit tests for pure functions in
  `run_local_e2e.py`.
- `--dry-run` / `--verify-only` flag on `ensure-testnet-vault.sh`.
- `E2E_PREPARE_DRY_RUN=1` support in `e2e.sh` for inspecting `cmd_prepare_local` without
  running the orchestrator.

## Implementation notes and blockers

The following decisions are now recorded.

- **Testnet vault-ensure I/O.** Use a dedicated testnet helper. The orchestrator
  calls `scripts/e2e/ensure-testnet-vault.sh` (or equivalent) for `CHAIN=testnet`.
  The helper's entry point is `bootstrap_testnet_fixture` with
  `--rc3-wallet-config`, `--rc3-wallet-storage`, and
  `--submit-helper $(lez_testnet_submit_bin)`; it drives `lez-testnet-submit`
  for inclusion polling. This reuses the proven path from
  `scripts/bootstrap-testnet-module.sh`. It is a Store-only testnet bootstrap
  helper, not a regression of the Step 26 FFI direction, which retired
  `lez-testnet-submit` dispatch from the module itself.

- **Stub signature for `ensure-testnet-vault.sh`.**

  ```bash
  scripts/e2e/ensure-testnet-vault.sh \
    --manifest <path> \
    --vault-id <id> \
    --deposit-amount <lo> \
    --wallet-config <path> \
    --wallet-storage <path> \
    --sequencer-url <url> \
    --program-id-hex <hex> \
    --program-bin <path> \
    [--submit-helper <path>]
  ```

  The helper writes `vault_id`, `vault_config_account_id`, and
  `vault_holding_account_id` back to the manifest and is idempotent.

- **VAULT_ID scan semantics.** Implement a new `vault_config_is_empty(owner, vault_id)`
  probe separate from `vault_is_funded`. The scan starts at 0 and picks the first
  id whose vault config account has no data. `STREAM_ID=0` is only valid for a vault
  that the probe reports as empty.

- **Reuse-mode toggle.** `E2E_REUSE_BASELINE_VAULT=1` is the canonical flag.
  - Default path: do not pre-create a vault in `cmd_prepare_local`; the orchestrator
    scans and ensures a fresh vault per run.
  - Reuse path: `cmd_prepare_local` restores the `funded` snapshot and calls
    `fixture.sh vault ensure 0`.
  - `make verify-store-local-lifecycle` sets this flag for both legs.
  - `cmd_prepare_local`'s continuation branch (`SKIP_SEED=1 / RESTORE_LOCALNET=0`)
    only calls `fixture.sh vault ensure 0` when the flag is set.

- **Snapshot migration.** Regenerate the local `funded` snapshot without a program
  vault. Update `prefund-localnet.sh` and `localnet-snapshot-common.sh` to stop
  writing `vault_id` and PDAs into the snapshot metadata.

- **Deposit / allocation numbers.** Allocation 400 gives `min_unaccrued_lo = 100`.
  `SEED_DEPOSIT_AMOUNT` controls the deposit; `demo_deposit_amount` is diagnostic.

- **Option A ordering.** After stream creation, the provider must call
  `rediscoverStreams` before the first paid Store query. Verify the testnet
  `chainAction` create path produces stream id 0 on a fresh vault.

- **Owner funding on testnet.** Preflight requires at least `SEED_DEPOSIT_AMOUNT`
  per run; the two-pass verification gate requires at least
  `2 * SEED_DEPOSIT_AMOUNT` (1000 lo). Vaults are left on chain; operators top up
  the owner manually or use reuse mode for long-running CI.

- **Fixture baseline cleanup.** `fixtures/testnet.json.example` and tracked
  `fixtures/testnet.json` now omit fixed vault fields and `reserved_for_step_*`.
  `fixtures/localnet.json.example` and `seed_localnet_fixture write-vault-manifest`
  are updated the same way. The run orchestrator writes all vault/stream fields
  after ensure/create.

## Additional clarifications

### PR boundaries and dependencies

- Step 29 in the same PR. Step 33 only needs the Step 29 phase-table changes that are
  directly caused by Step 33: vault ensure phase, sizing values, and messaging moved
  into the accrual phase. Broader Step 29 work is not a dependency. If Step 29 has not
  landed, the new phases run without narrative polish, but JSON-lines artifacts remain
  unchanged.
- Testnet gate log. Create the sibling gate log at the start of testnet verification and
  append a pass row after each green run. The definition of done is two consecutive green
  passes.

### Testnet vault ID scan and ensure

- Where the scan runs. The empty-config probe and scanner live in a shared helper used
  by both `fixture.sh` and `ensure-testnet-vault.sh` / `run_local_e2e.py`. The canonical
  read is the same RPC path used by `vault_is_funded` (e.g., `getAccount` /
  `getVaultStatus`). The scanner checks whether the vault config account exists and has
  non-zero data.
- Exact meaning of `vault_config_is_empty`. An empty vault is one whose config account
  does not exist on chain or has zero bytes. If a prior run partially initialized the
  vault, the scanner will skip that id and the chosen id will be fresh. The ensure step
  then always init + deposit for the chosen id.
- Idempotency of `ensure-testnet-vault.sh`. The helper is idempotent for the chosen id:
  init if the config account is missing, deposit if the holding account is below the
  target. It does not fail fast on an existing underfunded vault, and it does not force a
  new scanned id.
- Scan upper bound. The scan is unbounded for now. CI and documented operator flows should
  set `VAULT_ID` once a baseline run has established a free id, to avoid scanning many
  old vaults.

### Script inventory and reuse

- `create-testnet-stream-fixture.sh`. Keep it for non-Store manual testnet flows. The
  Store orchestrator replaces it with `ensure-testnet-vault.sh` + create in
  `run_local_e2e.py`.
- `bootstrap_testnet_fixture` entry point. A Store-only helper is acceptable for the first
  pass. If the same submit/poll logic is needed elsewhere, factor it into
  `scripts/lib/testnet_submit.sh`.

### Local snapshot migration (D2)

- Regenerating the `funded` snapshot. The PR changes `prefund-localnet.sh` and
  `localnet-snapshot-common.sh` so the next `prepare-localnet` or
  `full-reset-localnet` regenerates an identity-only snapshot. The PR does not commit
  snapshot bytes. A one-time maintainer note documents deleting an old `funded` snapshot
  if it contains a vault.
- First-run local ergonomics. On a clean clone, `make verify-store-local` runs
  `e2e.sh local prepare`, which starts localnet and lets the orchestrator ensure a fresh
  vault. No extra bootstrap step is required for the default path. Reuse mode requires a
  pre-existing `funded` snapshot.

### Lifecycle / reuse mode behavior

- `verify-store-local-lifecycle` with `E2E_REUSE_BASELINE_VAULT=1`. Both legs reuse vault 0.
  The contract is one vault, new stream id per leg (Step 24c): leg 1 creates stream 0,
  leg 2 creates stream 1, etc.

### Acceptance vs code changes

- D3 ordering and `rediscoverStreams`. Provider-side `rediscoverStreams` before the first
  paid Store query is a code change in scope for Step 33, not just a verification
  requirement. If the current `chainAction` path already satisfies it, the item becomes a
  verification-only check.
- `E2E_CREATE_VIA=chainaction` for testnet only. Confirmed: local Store default remains
  `seed` unless `E2E_CREATE_VIA` is set. No Makefile or `e2e.sh` override currently
  conflicts.
- Python unit test surface. Extract and unit-test: the vault scanner /
  `vault_config_is_empty`, `strip_snapshot_stream_fields`, `min_unaccrued_lo_for_proof`,
  `manifest_allocation_lo`, and the PDA-computation core of `refresh_manifest_pdas`.
- `E2E_PREPARE_DRY_RUN=1`. New behavior; nothing similar exists today.

## Verification

| Gate | Command |
| --- | --- |
| Store local | `make verify-store-local` |
| Store testnet | `make verify-store-testnet` (two consecutive passes, no manual deposit) |
| Lifecycle (reuse mode) | `make verify-store-local-lifecycle` |
| Module regression (unchanged) | `make verify-module-local`, `make verify-module-testnet` (module mode keeps its own fresh `VAULT_ID` resolver; it does not use the Store resolver) |

Create the sibling gate log at the start of testnet verification and append a pass
row after each green run. The definition of done is two consecutive green passes.

## Decision log

| Id | Topic | Outcome |
| --- | --- | --- |
| D1 | Q1 parity local vs testnet | Fresh-vault per run is the default for both local and testnet. `make verify-store-local-lifecycle` sets `E2E_REUSE_BASELINE_VAULT=1` for both legs to keep the reuse mode for back-to-back regression. |
| D2 | Q2 snapshot / prepare-localnet | Snapshot is ledger + owner/provider + AT-ready accounts only; no program vaults. Default `cmd_prepare_local` no longer calls `fixture.sh vault ensure 0`. |
| D3 | Q3 stream create ordering | Create stream immediately after daemons/AT ensure; publish Store messages during the accrual wait. |
| D4 | Q4 bootstrap / fixture baseline | Store bootstrap is identity-only. `fixtures/testnet.json.example` and tracked `fixtures/testnet.json` drop fixed `vault_id`, `vault_config_account_id`, and `vault_holding_account_id`; `reserved_for_step_*` comments are removed. |

These decisions are mutually consistent: D1 requires D2, D3 ensures the fresh
vault is fundable before the Store query, and D4 removes the fixed `vault_id:
0` contract that the other decisions no longer need. They also align with the
safe defaults above (fresh-id scan, `allocation` 400, deposit 500, always init +
deposit for the chosen id).
