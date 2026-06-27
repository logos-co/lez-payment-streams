# Step 24c — simplify demo flow (fresh stream per run)

Normative handoff for agents. Index: [integration-index.md](../../../integration-index.md).
Prerequisites: Step 17/17b scripts exist; Step 24b complete (rc5 guest + tooling).
Related: [step-17b-localnet-snapshot-restore.md](../completed/step-17b-localnet-snapshot-restore.md),
[step17-e2e-local.md](../../step17-e2e-local.md),
[step-18-public-testnet-demo.md](step-18-public-testnet-demo.md),
[N15](../../reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19),
[N17](../../reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06).

Status: **complete** for Step 24c local gate (`SKIP_BUILD=1 make verify-step17` and `make verify-step17-back-to-back` green on 2026-06-28 after close-signer fix).

### Update 2026-06-28 (close + back-to-back gate)

Close on chain (root cause)

- `close_stream` uses a six-account layout: owner binding slot plus a distinct **authority** signer (vault owner or stream provider). Owner-initiated close via module defaulted `authority` to owner, which duplicates the owner account id in the account list; txs never landed on the debug sequencer.
- Seed close must plan **provider** as authority and sign with the **provider** key (same `.scaffold/wallet` as prefund). `demo_teardown` `chainAction` fallback uses `cfg_provider` with `"authority": provider_account_id`.
- E2E close success is `stream_state == Closed` on chain (not `total_allocated_lo < allocation`; accrued residual can keep `total_allocated_lo` high until claim).

Verified on this machine

- `/tmp/verify-b2b-final7.log`: leg 1 teardown `total_allocated_lo: 0`; leg 2 seed close + claim after accrual; both `run_total` ok.

### Update 2026-06-27 (continuation leg)

Back-to-back leg 2 (current behavior in tree)

- Leg 1 teardown: seed `close-stream-onchain` (provider signs) then `claim` when accrued > 0.
  After a full close+claim, `vault_unallocated_lo` is often low (~200 on a 2000-deposit vault); leg 2
  may call `ensure_continuation_vault_funded` (owner pinata + seed `deposit-onchain`) when accrued > 0
  left vault liquidity tight.
- Between legs: `make verify-step17-back-to-back` sleeps 45s, runs
  `scripts/e2e/continuation-owner-topup.sh`, then leg 2 with `SKIP_SEED=1` / `RESTORE_LOCALNET=0`.
- Leg 2 create: default `E2E_CREATE_VIA=seed` (`create-localnet-stream-fixture.sh`) with optional
  precreate after user wallet sync; seed top-up for large clock fold gaps (seed first, `chainAction`
  fallback on seed failure).
- Prefund deposit default **2000** (`scripts/prefund-localnet.sh`); pinata rounds scale with deposit.
  A 5000-deposit snapshot experiment was reverted (insufficient pinata headroom on typical localnets).

## Handoff summary (2026-06-28)

Local Phase 6 gates are green on this machine (`SKIP_BUILD=1` after modules installed). Remaining
24c scope is testnet alignment (Step 18), optional doc sweep, and re-running Step 12/13 DoD scripts
after dual prepare naming — not blockers for the local demo lifecycle.

### What has been done

Module and API

- Removed `findActiveStreamForProvider` from prepare paths.
- Public prepare split: `prepareEligibilityProofWithStreamProposalForStoreQuery` (2 args) and
  `prepareEligibilityProofWithStreamProofForStoreQuery` (3 args); single-line declarations in impl.h for universal glue.
- `logos-delivery-module` updated to call the proposal method name (Step 16).
- FFI: `chain_timestamp_to_fold_seconds` applied to proposal deadlines and fold `as_of`; clock decode
  normalizes ms to seconds. JSON diagnostics add `accrued_as_of_seconds` on stream decode.
- C++: `foldClockForPolicy` on clock reads used for proposal deadlines; `readClock10Timestamp` applies it.

Orchestration (`scripts/e2e/run_local_e2e.py`, `demo-e2e-local.sh`)

- Per run: strip `stream_id` / `stream_config_account_id` from manifest on load; `next_stream_id` from
  `getVaultStatus` only (no manifest fallback).
- Reset `payment_streams_state.json` under user/provider persist at start of `core` / `all`.
- Local create: `seed_create_stream_onchain` via `create-localnet-stream-fixture.sh` with
  `CREATE_FORCE=1` / `E2E_PER_RUN_STREAM=1`. Continuation legs (`SKIP_SEED=1` /
  `RESTORE_LOCALNET=0`): default `E2E_CREATE_VIA=seed`; optional precreate after wallet sync.
- Seed failure on local: fallback `chainAction createStream` via logoscore (user host).
- Teardown in `core`: **close then claim** — local default `E2E_CLOSE_VIA=seed`
  (`close-stream-onchain`, provider authority + provider signer); `chainAction` fallback on
  provider host with `"authority": provider_account_id`. Close verified via on-chain
  `stream_state == Closed`. Claim skipped when accrued is 0.
- Prepare uses proof method with explicit manifest `stream_id`.
- Artifact phases: `plan_demo_stream`, `baseline_before_create`, `checkpoint_after_create`,
  `refresh_stream_checkpoint` (topUp when clock fold gap is large), `create_demo_stream`,
  `wait_stream_fundable`, `demo_close_stream`, `demo_claim`.
- Removed misleading fundability shortcut that compared raw on-chain `accrued_lo` without fold.

Prepare and fixture scripts

- `demo-localnet-prepare.sh`: vault-only baseline (`SKIP_STREAM_CREATE=1`); stream created in E2E only.
- `create-localnet-stream-fixture.sh`: `--force` when `CREATE_FORCE` or `E2E_PER_RUN_STREAM`.
- `step12-topup-and-prepare.sh`: always strip stream fields from manifest, then create at chain
  `next_stream_id` with force (no “reuse manifest stream_id” branch).
- `write-vault-manifest.sh` / seed `write-vault-manifest` for baseline v2.

Snapshot and verify entrypoints

- `localnet_snapshot_stale_for_restore`: if `snapshot.json` `created_at` is older than
  `SNAPSHOT_MAX_AGE_S` (default 1800), `demo-localnet-prepare` runs `prefund-localnet.sh` before
  restore (avoids restored ledger clock pinned at snapshot time while wall clock has moved).
- Makefile: `make full-reset-localnet`, `make verify-step17-back-to-back` (run 1 restore, run 2
  `SKIP_SEED=1` + 45s pause), `RESTORE_LOCALNET=0` forces `SKIP_SEED=1`.
- Docs: [step17-e2e-local.md](../../step17-e2e-local.md), [integration-index.md](../../../integration-index.md)
  updated for back-to-back gate and prepare policy.

Harness timing and wallet poll (2026-06-27, not part of 24c protocol scope)

- [step17-e2e-local.md](../../step17-e2e-local.md): run duration table, fast-iteration env vars.
- `scripts/e2e/run_local_e2e.py`: `run_config`, `timing_mark`, `run_total`, `E2E_SUBPROC_TIMEOUT_S`,
  seed wallet copy applies `E2E_WALLET_POLL_MAX_DELAY` / `E2E_WALLET_POLL_MAX_ATTEMPTS`.
- `scripts/e2e/sequencer_latency_probe.py` + `make debug-sequencer-latency`.
- LEZ `TxPoller` exponential backoff (250 ms → cap `seq_poll_timeout`) in sibling
  `logos-execution-zone/lez/wallet/src/poller.rs` — requires rebuilding `wallet` into the scaffold
  LEZ pin before seed create uses it; E2E config overrides alone do not change the binary.

- `examples/src/bin/seed_localnet_fixture.rs`: `deposit-onchain`, `close-stream-onchain`
  (provider signer), `top-up-stream-onchain`.
- `scripts/e2e/run_local_e2e.py`: continuation vault funding, `release_logoscore_wallet` around
  seed deposit/create/close/top-up, `vault_liquidity_*` artifact phases, strict close verify on
  stream PDA state.

### What works (verified locally, 2026-06-28)

- `make full-reset-localnet` — vault-only snapshot, deposit **2000**, no stream in snapshot.
- `SKIP_BUILD=1 make verify-step17` — per-run stream at `next_stream_id`, Store DoD, seed close,
  claim or skip.
- `SKIP_BUILD=1 make verify-step17-back-to-back` — leg 1 restore + stream 0; leg 2 stream 1 on
  same ledger (pinata between legs, continuation deposit when needed). Reference log:
  `/tmp/verify-b2b-final7.log`.

### Known limits (not local gate failures)

| Topic | Notes |
| --- | --- |
| Stale snapshot restore | Large clock vs `accrued_as_of` fold gap can virtual-deplete a new stream; use fresh snapshot, `SNAPSHOT_MAX_AGE_S` auto-prefund, or `make full-reset-localnet`. |
| Owner-only close from user wallet | Module default `authority = signer` duplicates owner in close account metas; E2E uses **provider** as authority. Product change needed for user-wallet-only close. |
| `chainAction` inclusion | Wallet may report `success` before `getTransaction` sees the tx; seed paths preferred locally; set `E2E_STRICT_SEQUENCER_TX_WAIT=1` to tighten waits. |
| Step 12 / 13 DoD scripts | Dual prepare naming landed; full `verify-step12-dod.sh` / `verify-step13-dod.sh` not re-run as part of 24c local gate. |

### Root cause note (clock vs fold, not ms/sec)

E2E “depleted stream right after create” on **old snapshot restore** was traced to **fold gap**
(clock fold seconds minus `accrued_as_of` fold seconds), not broken ms→s conversion in FFI fold.
Fresh prefund aligns snapshot clock with wall time; stale restore without rebuild reproduces large gap.
Artifacts: compare `baseline_before_create.clock_fold_seconds`, `checkpoint_after_create.fold_gap_seconds`,
and `refresh_stream_checkpoint`.

## Goal

Replace “reuse stream 0 until depleted” demo logic with a **deterministic lifecycle**:

1. **Reuse the vault** (snapshot baseline: init + deposit, no stream in the snapshot).
2. **Create a new on-chain stream every demo run** at `vault_config.next_stream_id`.
3. **Bind eligibility to that stream explicitly** — no scanning or guessing stream ids in the module.
4. **Teardown after the demo**: **close** the run’s stream (return unaccrued to vault unallocated),
   then **claim** accrued funds to the provider (tighter than leaving the stream open while claiming).

This step is **demo orchestration + module API correctness**, not a protocol change. Track A E2E
(Steps 17, 18, 20) should stop depending on `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF`,
`ensure_fresh_demo_stream` heuristics, and `skip_if_initialized` stream reuse.

## Problem statement (current behavior)

| Issue | Where it shows up |
| --- | --- |
| Stream 0 treated as a long-lived fixture | `create-localnet-stream-fixture.sh`, prepare with `SKIP_STREAM_CREATE`, manifest always `stream_id: 0` |
| Depletion over wall clock | Same stream accrues; reruns fail unless top-up, bypass env, or full reset |
| `ensure_fresh_demo_stream` is not “usable stream” | `E2E_LATE_STREAM_CREATE=0` no-op; otherwise `allocation_available` on manifest id only; late create sets `STREAM_ID` but shell seed still defaults to stream 0 |
| Prepare picks wrong stream | `findActiveStreamForProvider` returns **lowest** matching provider stream id, ignoring manifest and skipping Closed only **after** selection via `eligibilityErrorForStreamState` |
| Inventory vs chain | `listMyStreams` iterates module inventory; stale ids vs fresh `next_stream_id` |

Operators should not need recovery docs for “stream 0 depleted again.”

## Target demo contract

### Per-run lifecycle (local and testnet)

```text
[baseline vault on chain]  ← snapshot restore OR existing testnet vault
        │
        ▼
  read next_stream_id
        │
        ▼
  create_stream(rate, allocation)  →  update FIXTURE_MANIFEST (per-run stream_id + stream PDA)
        │
        ▼
  rediscoverStreams(vault_id)  →  inventory includes this stream_id only for this run’s proof path
        │
        ▼
  publish + prepareEligibilityProofWithStreamProofForStoreQuery(..., stream_id) + storeQuery + missing-proof check
        │
        ▼
  closeStream(run_stream_id)     →  unaccrued → vault holding (unallocated)
        │
        ▼
  claim(run_stream_id)           →  accrued → provider (skip if accrued is 0)
```

**Order: close then claim.** Close first so the owner’s unaccrued remainder returns to the vault
holding immediately; the stream’s allocation is trimmed to accrued. Claim then pays the provider
the accrued balance on that **closed** stream (supported by core/guest). Avoid claim-while-open,
which leaves a window for further accrual before close.

**Do not reuse streams across runs.** Older stream PDAs may remain on chain (Closed); that is
fine. Each run uses a **new** id and an updated manifest.

**Do reuse the vault.** Step 17b snapshot stays “funded vault, no stream.” Testnet bootstrap
keeps owner, provider, vault 0, and deposit; only the per-run stream id changes.

### Prepare API — dual method names (locked)

Universal modules (`interface: universal`) do **not** export C++ overloads with the same name on
the Logos wire (`logos-cpp-generator` registers one entry per method name). Use **two method names**
instead of two Qt overloads of `prepareEligibilityForStoreQuery`. Remove **`findActiveStreamForProvider`**
from prepare entry points entirely.

| Method | Args | Behavior |
| --- | --- | --- |
| `prepareEligibilityProofWithStreamProposalForStoreQuery(n8_hex, provider_peer_id)` | **Two** | **Proposal path only:** vault holding + owner sign → JSON `"kind":"stream_proposal"`. Must **not** scan streams or return `stream_proof`. |
| `prepareEligibilityProofWithStreamProofForStoreQuery(n8_hex, provider_peer_id, stream_id)` | **Three** | **Proof path only:** `readStreamAtId(vault, stream_id)`, provider must match mapping, `eligibilityErrorForStreamState` → `"kind":"stream_proof"`. Must **not** fall back to proposal or another stream id. |

Implementation: two public **single-line** method declarations in `payment_streams_module_impl.h`
(universal `logos-cpp-generator` skips multi-line declarations); shared logic via internal helpers.

Documentation (required in [integration-contracts.md](../../integration-contracts.md), [step12-user-eligibility.md](../../step12-user-eligibility.md), [step17-e2e-local.md](../../step17-e2e-local.md)):

- Explicitly state that proposal and proof are **different LogosAPI methods**, not arity overloads.
- Step 12 DoD: `prepareEligibilityProofWithStreamProposalForStoreQuery` on vault-only manifest; after create,
  `prepareEligibilityProofWithStreamProofForStoreQuery` with fixture `stream_id`.
- Track A E2E (Steps 17–18, 20): **proof method only** after per-run `create_stream`.
- Wrong stream on proof method → existing error codes (`STREAM_DEPLETED`, etc.), no silent proposal.

Verify path unchanged: proof bytes embed `stream_id`; provider verify reads proof only.

**Script rule:** Orchestrator writes manifest `stream_id` after create, passes the same integer as the **third** prepare argument and into teardown `closeStream` / `claim`.

### Fixture manifest — baseline vs per-run (locked)

Two layers; do **not** treat bootstrap/snapshot JSON as owning a canonical stream.

| Layer | When | Required fields | Stream fields |
| --- | --- | --- | --- |
| **Vault baseline** | After prefund snapshot, testnet bootstrap, `demo-localnet-prepare` | `schema_version`, sequencer, `program_id_hex`, owner, provider, `vault_id`, vault + holding PDAs, `clock_10`, deposit / default rate & allocation for **create** | **Omit** `stream_id`, `stream_config_account_id` (or document as absent / null in schema v2) |
| **Per-run manifest** | After orchestrator `create_stream` for this demo run | All baseline fields **plus** run’s `stream_id`, `stream_config_account_id`, and create params used | **Present** until next run overwrites file |

So: **you do not store a durable “fixture stream id”** in the vault baseline file. Stream id is **run-scoped** state written when the stream is created (local or testnet). The module still reads `stream_id` from the manifest **during** a run for vault id and PDA helpers; that value is not a long-lived operator pin like owner or vault 0.

Implement: extend `seed_localnet_fixture` with **`write-vault-manifest`** (or equivalent) that emits vault-only JSON; **`create-stream-onchain`** continues to write the full per-run manifest. Update `fixtures/testnet.json.example` to vault-only shape.

### Explicit stream id for eligibility (module change)

Replace the old “required stream_id / scan” table with the dual-method API above. Remove
`findActiveStreamForProvider` and any remaining references to a single overloaded prepare name.

## Implementer defaults (normative)

Treat this section as binding for implementation unless a row is marked **in doubt**. It closes
the former “open decisions” on manifest schema and inventory, and expands dual prepare methods,
teardown, and Step 18 coordination.

### Manifest schema and inventory

| Topic | Default |
| --- | --- |
| `schema_version` | Use **2** for vault-only baseline. Readers accept **v1 with full stream fields** (legacy) and **v2 without** `stream_id` / `stream_config_account_id`. Per-run writes use v2 with populated stream fields. |
| `rediscoverStreams` | **E2E always** calls it after `create_stream`. **Do not** auto-inventory in the module on create in 24c. **In doubt:** optional later optimization only. |
| Fixture state vs manifest | `payment_streams_state.json` schema (module persisted state) is separate from `FIXTURE_MANIFEST`; do not conflate them in docs. |

### Runtime dispatch — universal module glue (resolved)

Investigation (2026-06-27): `payment_streams_module` uses `interface: universal`. Method metadata and
`callMethod` dispatch come from **`logos-cpp-generator`**, not Qt MOC overload resolution on the impl.
**Same C++ method name with different arity is not exported twice** — only the first overload appears
in glue / `getMethods()` / `lm methods`.

| Layer | Behavior |
| --- | --- |
| **`logoscore call`** | Forwards tokens after `<module> <method>` unchanged. Method name selects the entry point. |
| **Universal glue** | One registered name per public method; proof and proposal must use **different names**. |
| **`lm methods`** | Lists each registered name and signature (expect both prepare* methods after 24c). |

Normative implication: **`prepareEligibilityProofWithStreamProposalForStoreQuery`** and
**`prepareEligibilityProofWithStreamProofForStoreQuery`** (see [Prepare API](#prepare-api--dual-method-names-locked)).
Smoke after impl: `lm methods` lists both; proposal vs proof `logoscore call` uses the matching name.

### LogosAPI wire rules (dual prepare methods)

| Rule | Default |
| --- | --- |
| Proposal only | `logoscore call payment_streams_module prepareEligibilityProofWithStreamProposalForStoreQuery '<n8>' '<peer>'` |
| Proof | `logoscore call payment_streams_module prepareEligibilityProofWithStreamProofForStoreQuery '<n8>' '<peer>' <stream_id>` (third = unsigned `stream_id`, `0` valid) |
| Delivery bridge | Step 16 invokes **`prepareEligibilityProofWithStreamProposalForStoreQuery`** only; Track A E2E uses proof method from orchestrator. |

Copy the table above (plus example commands) into [integration-contracts.md](../../integration-contracts.md) under **Prepare methods (Step 24c)** in Phase 5.

### `findActiveStreamForProvider`

| Default | Detail |
| --- | --- |
| Today | Only referenced from legacy single-name prepare (removed in 24c). |
| 24c | **Remove** after dual-method refactor; delete the helper if no callers remain. |

### Prepare paths — negotiation and inventory

| Path | Default |
| --- | --- |
| **3-arg proof** | **Skip** proposal negotiation block (`PROPOSAL_PENDING`, eviction tied to proposal flow). Pending proposal is a 2-arg concern only. **In doubt:** explicit “no proof while proposal pending” on 3-arg is optional later, not required for 24c. |
| **2-arg proposal** | Keep existing negotiation + eviction logic unchanged. |
| **3-arg chain read** | **Required:** `readStreamAtId` + provider match + `eligibilityErrorForStreamState`. Prepare must **not** depend on inventory listing the stream if the chain account exists. |
| Session keys | Keep `sessionKeysForVaultProvider` / generate + persist as today. |
| Inventory side effect | Keep `addInventory` on successful proof. |
| `rediscoverStreams` | **E2E:** call after create, before prepare. **Step 12/13:** document “after create, call `rediscoverStreams` once” in the proof case; not required for proposal case. |

### Step 12 / Step 13 verify scripts

| Default | Detail |
| --- | --- |
| Restructure DoD | **Case A:** Vault-only manifest + **proposal method** → expect `"kind":"stream_proposal"`. **Case B:** After `create-stream-onchain` + **proof method** with that id → expect `"kind":"stream_proof"`. |
| Case A precondition | Run Case A after **`make prepare-localnet`** (or explicit `write-vault-manifest`), not on a post-E2E manifest that still contains stream fields. |
| Remove | Any test that expects `stream_proof` from the **proposal** method because streams exist on chain (old scan behavior). |
| Top-up | Keep top-up only on paths that **create** a stream for the proof case, not as a substitute for 3-arg. |
| Proposal smoke | **Yes** — one proposal case on a vault **without** creating a stream in that test session. Track A happy path does not use it; Step 12 DoD still covers negotiation. |
| Step 13 | Same as Step 12 for prepare: **proof method** with seeded stream id before `verifyEligibilityForStoreQuery` in the proof path. |

Phase 6 “green” = updated `verify-step12-dod.sh` and `verify-step13-dod.sh` match dual prepare methods.

### Teardown (E2E)

| Topic | Default |
| --- | --- |
| RPC shapes | Reuse Step 11b [chainAction](../../step11b-chain-writes.md): owner **`closeStream`** — `{ "signer": "<owner_base58>", "vault_id", "stream_id" }`; provider **`claim`** — `{ "provider", "vault_id", "stream_id" }` (same as today’s E2E `claim` in `run_local_e2e.py`). |
| Order | **Close** (local seed: provider authority signs; `chainAction` on provider host with `"authority": provider_account_id`) → sync + `rediscoverStreams` → read **`accrued_lo`** → **claim** if accrued > 0, else `demo_claim` with `skipped: true`, `reason: zero_accrued`. |
| When | After **successful `core`** (store query + missing-proof). **`all`** includes the same teardown (no second claim pass). |
| E2E phases | Move teardown into **`core`**; remove duplicate **`claim`** phase or make `--phase claim` a no-op with deprecation note so **`E2E_PHASE=core`** still runs teardown. Default `demo-e2e-local.sh` uses `E2E_PHASE=all` — must not run close+claim twice. |
| Failure policy | Teardown failure **fails the run** (non-zero exit, artifact `ok: false`). Back-to-back `verify-step17` depends on close recycling unallocated. |
| Abort / partial | No teardown if orchestrator exited before core success. **`SKIP_VERIFY` / read-smoke** — out of 24c scope; no teardown. |
| Accrued = 0 | Normal on fast local runs. **Close still required**; skip claim only. |

### Manifest files (on disk)

| Default | Detail |
| --- | --- |
| Single file | One **`FIXTURE_MANIFEST`** (default `fixtures/localnet.json`, operator `fixtures/testnet.json`). |
| Lifecycle | **After prepare:** vault-only v2. **After per-run create:** overwrite same file with v2 + `stream_id`, `stream_config_account_id`, rate/allocation used. |
| Dual file | **No** overlay in 24c. |
| After teardown | Do **not** require stripping stream fields back to vault-only; next run overwrites on create. Optional post-teardown vault-only rewrite is **not required**. |

### `seed_localnet_fixture` writers

| Default | Detail |
| --- | --- |
| Add | **`write-vault-manifest`** → v2, no stream fields. |
| Keep | **`create-stream-onchain`** → full per-run manifest (stream fields populated). |
| `write-manifest` | **Deprecate for demos:** alias vault-only or delete demo use; stop `demo-e2e-local.sh` stub `write-manifest` without chain. Do not leave two writers that emit conflicting stream ids without docs. |
| Rust JSON | Make stream fields **optional** in the fixture struct for v2 baseline writes. |

### Testnet bootstrap

| Default | Detail |
| --- | --- |
| Target | **`bootstrap_testnet_fixture`** matches local: vault init + deposit only, manifest **vault-only v2** (no stream create). |
| Idempotency | Keep **`TESTNET_REUSE_FIXTURE=1`** owner/provider reuse; vault txs **`skip_if_initialized`** where seed already supports it — do not recreate stream 0 on every bootstrap. |
| `next_stream_id` | Increments only via **E2E create** each run; bootstrap does not pin stream id. |
| **In doubt** | Exact bootstrap binary flags after stream create is removed — mirror `prefund-onchain` + vault manifest writer; validate with `TESTNET_REUSE_FIXTURE=1` smoke. |

### E2E artifact names

| Old (remove/rename) | New (DoD) |
| --- | --- |
| `late_create_stream`, `late_create_stream_ready` | **`create_demo_stream`** (ok + `stream_id`) |
| (none) | **`demo_close_stream`** |
| `claim` (phase artifact) | **`demo_claim`** (include `skipped` when accrued 0) |

Reviewers use **only** the three new names for 24c artifact gates.

### Step 18 and Makefile (same PR)

| Default | Detail |
| --- | --- |
| Landing | **24c PR** updates [step-18-public-testnet-demo.md](step-18-public-testnet-demo.md) and [step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md) — drop “must keep `E2E_LATE_STREAM_CREATE=0`” guard; non-regression = **`make verify-step17`** with unified per-run create. |
| Makefile | Remove `E2E_LATE_STREAM_CREATE=0` from the `verify-step17` target. |

### Track A vs proposal ([N18](../../reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06))

| Default | Detail |
| --- | --- |
| Track A (17–18–20 default scripts) | **create → proof method only**; no `stream_proposal` on happy path. |
| Proposal | Remains Step 12 DoD + Step 20 tier-2 manual / negotiation journey. |

### Hygiene (same PR)

- Update **`create-localnet-stream-fixture.sh`** header comment to “per-run stream id”, not “stream 0”.

## Implementation plan

### Phase 1 — Module API and behavior

1. Add **`prepareEligibilityProofWithStreamProposalForStoreQuery`** and **`prepareEligibilityProofWithStreamProofForStoreQuery`**
   (proposal-only vs proof-only). Share implementation via internal helpers.
2. Proof method: `readStreamAtId` at supplied id; provider match; `eligibilityErrorForStreamState`.
3. Proposal method: unchanged vault logic; **no** stream scan before proposal.
4. Audit `verifyEligibilityForStoreQuery` — proof stream id only.
5. Update Step 12/13 dod: proposal method on vault-only; proof method with id from seeded stream.
6. Update `logos-delivery-module` to call proposal method name for Step 16 registration + invoke.
7. Run `verify-step12-dod.sh`, `verify-step13-dod.sh`.

### Phase 2 — Seed and fixture scripts

1. **`create-localnet-stream-fixture.sh`**
   - Accept `STREAM_ID` (default: `getVaultStatus.next_stream_id`).
   - Pass `--stream-id` to `seed_localnet_fixture create-stream-onchain`.
   - Writes **per-run** manifest (includes `stream_id` + stream PDA).

2. **Vault-only manifest writer** (new seed subcommand or script)
   - Emitted after prefund / bootstrap / prepare — **no** `stream_id`, **no** `stream_config_account_id`.
   - Used for Step 10a vault fixture check and operator baseline.

3. **`demo-localnet-prepare.sh`**
   - Restore snapshot only; write or refresh **vault-only** manifest.
   - Drop `SKIP_STREAM_CREATE` / `E2E_LATE_STREAM_CREATE` stream-creation coupling.

4. **`prefund-localnet.sh`**
   - Unchanged on-chain; snapshot + vault-only manifest ([N15](../../reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19)).

5. **Step 10a — vault fixture only (locked)**
   - Change [`verify-step10a-dod.sh`](../../../scripts/verify-step10a-dod.sh) and
     [step10a-local-chain-fixture.md](../../step10a-local-chain-fixture.md): verify sequencer,
     wallet health, program id, **vault_config** + **vault_holding** PDAs from manifest.
   - **Remove** requirement that **stream_config** PDA in manifest is initialized after prepare.
   - Stream PDA checks move to post-create path (E2E after `create_demo_stream`) or Step 11/12
     fixtures that create a stream explicitly.

### Phase 3 — E2E orchestrator (`run_local_e2e.py`, `demo-e2e-local.sh`)

1. **Remove** `ensure_fresh_demo_stream` (or reduce to a thin `create_demo_stream_for_run`):
   - After user/provider daemons up and wallet synced, **always** (local + testnet):
     - `getVaultStatus` → `next_stream_id`
     - invoke create-stream script with that id
     - reload manifest; `rediscoverStreams(vault_id)`
     - poll until stream Active and `unaccrued_lo` > 0 (or module prepare dry-check if added)

2. **Remove** dependence on:
   - `E2E_LATE_STREAM_CREATE`
   - `allocation_available` / `min_unaccrued_lo_for_proof` heuristics for gating (optional keep as
     logging only)
   - `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF` for default demo paths (document test-only use)

3. **`user_prepare_proof`**
   - Call `prepareEligibilityProofWithStreamProofForStoreQuery` with manifest `stream_id` after per-run create.
   - Drop top-up retry loops as **default** (optional env for stress tests).

4. **Teardown** — per [Implementer defaults (normative)](#teardown-e2e): run at end of successful
   **`core`**; collapse duplicate **`claim`** phase; artifacts `demo_close_stream`, `demo_claim`

5. **`demo-e2e-local.sh`**
   - Stop writing stub manifest via `write-manifest` without chain create.
   - `ensure_fixture`: local = restore baseline only (or skip if snapshot + localnet already up);
     testnet = require manifest with vault fields, stream created in orchestrator.

6. **`Makefile` `verify-step17`**
   - Drop `E2E_LATE_STREAM_CREATE=0` special case; single code path.

### Phase 4 — Testnet (Step 18)

1. Align `bootstrap-testnet` with **vault-only** manifest (no pinned stream 0).
2. E2E creates stream at `next_stream_id` each run; per-run manifest update on disk.
3. Update [step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md) — remove bypass
   fence as default; document close+claim at end.
4. `make verify-step18` DoD: passes with `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF=0`.
5. Update Step 18 packet Makefile guard in the **same PR** as 24c (see implementer defaults).

### Phase 5 — Documentation

1. [step17-e2e-local.md](../../step17-e2e-local.md) — new lifecycle section; delete stream-0 reuse /
   late-create / depletion recovery as primary path.
2. [demo-localnet-recovery.md](../../demo-localnet-recovery.md) — simplify: recovery = restore vault
   snapshot or redeposit; not “un-deplete stream 0.”
3. [integration-contracts.md](../../integration-contracts.md) — **Prepare methods (Step 24c)**:
   dual method names; `logoscore call` examples; delivery uses proposal method only.
4. [step-20-developer-journey.md](step-20-developer-journey.md) — tier-1/tier-2 commands include
   explicit stream id and teardown (Step 20 should follow 24c or document interim behavior).

### Phase 6 — Verification (definition of done)

| Gate | Command / check |
| --- | --- |
| Module | `./scripts/verify-step12-dod.sh`, `./scripts/verify-step13-dod.sh` |
| Local fixture | `make prepare-localnet` → vault-only manifest; Step 10a green without stream PDA |
| Local E2E ×2 back-to-back | `make verify-step17-back-to-back` — green locally 2026-06-28 (`SKIP_BUILD=1` after module install) |
| Testnet read smoke | `make verify-step18-testnet-read-smoke` (unchanged) |
| Testnet E2E | `make verify-step18` with `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF=0` — **not verified in 24c local gate** |
| Artifacts | JSON-lines include `create_demo_stream`, `demo_close_stream`, `demo_claim` (names exact in impl) |

Record completion date and any manifest/schema bump in this packet’s status line when merged.

## Non-goals

- Changing guest accrual math or `close_stream` / `claim` semantics on chain.
- Track B UI (Steps 21–22) except noting explicit stream id in API docs.
- Retiring `tools/lez-testnet-submit` (Step 24b out of scope item).
- Auto-closing all historical streams on vault (only the current run’s id).

## Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Vault runs out of unallocated holding after many runs without teardown | Default teardown close+claim; document `SEED_DEPOSIT_AMOUNT` / manual deposit |
| Many closed stream accounts on localnet | Acceptable; optional future “vault id increment” for long dev sessions |
| API break for external callers of prepare | Version note in contracts; Step 12 tests updated in same PR |
| Testnet tx latency on create+close+claim | Reuse pollers; testnet chain-action timeouts only. Local seed: exponential `TxPoller` + E2E `E2E_WALLET_POLL_*`; long runs usually prefund or failed poll retries, not block time ([step17-e2e-local.md](../../step17-e2e-local.md)) |

## Resolved decisions (2026-06-26)

| Topic | Decision |
| --- | --- |
| Prepare API | **Dual methods:** `prepareEligibilityProofWithStreamProposalForStoreQuery` (proposal), `prepareEligibilityProofWithStreamProofForStoreQuery` (proof); no scan |
| Runtime dispatch | Universal glue: **one LogosAPI name per method**; overloads not supported on wire |
| Fixture baseline | **Vault-only JSON** — no durable `stream_id` / stream PDA in bootstrap or post-prepare manifest |
| Per-run manifest | Orchestrator adds `stream_id` + stream PDA **after** `create_stream` for that run only |
| Teardown | On **`core`** success: close then claim; **skip claim** if accrued is 0 |
| Step 10a | **Vault fixture only** — drop stream_config PDA check from post-prepare verify |

Expanded normative detail: [Implementer defaults (normative)](#implementer-defaults-normative) (2026-06-27).

## Remaining open questions

Resolve during implementation (smoke test or product call); do not block starting Phase 1 unless noted.

1. **Prepare method smoke** — After Phase 1, confirm `lm methods` lists **both** prepare method names and distinct signatures.

2. **Testnet bootstrap cutover** — After `bootstrap_testnet_fixture` stops creating a stream, exact CLI flags and `skip_if_initialized` behavior for vault-only bootstrap should be validated once with `TESTNET_REUSE_FIXTURE=1` and a fresh operator manifest.

3. **Proof while proposal pending (optional product rule)** — 24c default: proof method **skips** negotiation gates. If product later requires rejecting proof when a pending proposal exists for the same provider, add an explicit error on the proof path only; not in scope unless requested.

### Retired question (resolved 2026-06-27)

**Same-name overloads on universal modules** — Resolved: `logos-cpp-generator` exports one wire method per name; use **`prepareEligibilityProofWithStreamProposalForStoreQuery`** and **`prepareEligibilityProofWithStreamProofForStoreQuery`** instead of Qt-style arity overloads of a single name.

## Completion checklist (fill on merge)

| Item | Done |
| --- | --- |
| Dual prepare methods + contracts doc | yes (impl + delivery; Step 20 journey doc may lag) |
| Vault-only manifest + per-run stream fields | yes (prepare + orchestrator) |
| Step 10a vault-only verify | yes (`make prepare-localnet` + verify-step10a green 2026-06-28) |
| Scripts create at `next_stream_id` every run | yes (E2E + step12 strip/create) |
| `ensure_fresh_demo_stream` removed or replaced | yes → per-run create in orchestrator |
| Close then claim teardown (claim skip if zero accrued) | yes; provider-signed close on local seed path |
| verify-step17-back-to-back | yes (local, 2026-06-28 post-commit run) |
| E2E timing / poll diagnostics | yes (orchestrator + step17 runbook) |
| `findActiveStreamForProvider` removed | yes |
| Step 18 testnet E2E with unified lifecycle | open (Phase 4) |
| verify-step12-dod / verify-step13-dod re-run | both green 2026-06-28 |
