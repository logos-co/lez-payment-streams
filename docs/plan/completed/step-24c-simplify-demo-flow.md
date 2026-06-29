# Step 24c — simplify demo flow (fresh stream per run)

Completed packet. Index: [program-index.md](../../development-map/program-index.md).
Prerequisites: Step 17/17b scripts; Step 24b (rc5 guest + tooling).
Related: [step-17b-localnet-snapshot-restore.md](step-17b-localnet-snapshot-restore.md),
[archive/steps/local-store-dual-host-runbook.md](../../archive/steps/local-store-dual-host-runbook.md),
[step-18-public-testnet-demo.md](../completed/step-18-public-testnet-demo.md),
[archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md).

## Status (2026-06-28)

24c is complete. The per-run lifecycle (fresh stream per run, explicit prepare id,
provider-signed close, optional claim) is implemented and green on localnet, and the
testnet demo path runs end to end through the paid Store query and close.

| Area | State |
| --- | --- |
| Local Track A E2E (Step 17) | Green: `SKIP_BUILD=1 make verify-step17`, `make verify-step17-back-to-back` |
| Step 10a / 12 / 13 DoD on localnet | Green (default and strict Step 12 paths; see verification matrix) |
| Same-chain Step 13 then `REQUIRE_STREAM_PROOF=1` Step 12 | Green after clock sync + chain settle + lifecycle teardown |
| Testnet-faithful localnet (clock, sizing, honest seed CLI) | Implemented in tree; existing funded snapshots may still hold old deposit totals until `make full-reset-localnet` |
| Verify lifecycle teardown scripts | In tree (`demo-stream-teardown-localnet.sh`, `sync-seed-wallet-after-logoscore.sh`, `wait-clock-synced.sh`, `wait-chain-settle.sh`) |
| Testnet demo create to fundable to paid Store query to close | Green via Clock01 (depletion gate cleared; close via seed path) |
| Testnet claim | Optional in the demo; not reliably confirming on testnet — [archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md) |

Local 24c orchestration (E2E per-run create, dual prepare methods, provider-signed seed close,
back-to-back) is done. Verify scripts match the E2E lifecycle on a continuously running localnet:
funded vault from snapshot reuse, per-run stream id, teardown close, no restore between Step 13
and strict Step 12. Testnet chain I/O is owned by Step 18 Part B; the 24c demo flow runs on it
with claim treated as optional.

---

## Operator sequence (same chain, no restore between 13 and 12)

```text
./scripts/lifecycle.sh snapshot restore funded
make prepare-localnet
make verify-step13
REQUIRE_STREAM_PROOF=1 make verify-step12
```

This sequence passes on localnet after the testnet-faithful harness updates.

Equivalent Make targets: `make verify-step13`, then `REQUIRE_STREAM_PROOF=1 make verify-step12`.

---

## Testnet-faithful localnet harness

### Clock after snapshot restore

Restoring RocksDB rewinds the on-chain demo clock. `CreateStream` sets `accrued_as_of` from
that clock at create time; creating before the clock catches up causes a one-shot fold over
snapshot age (fold jump). The demo clock is `Clock01` (refreshes every block); see the testnet
outcome section for why the demo moved off `Clock10`.

| Piece | Role |
| --- | --- |
| `seed_localnet_fixture wait-clock-synced` | Poll the demo clock until wall skew is at most `MAX_CLOCK_SKEW_S` (default 5) |
| `scripts/archive/wait-clock-synced.sh` | Calls the subcommand; nudges block production via pinata top-ups when the sequencer is idle (clock only advances in blocks) |
| [`scripts/lifecycle.sh`](../../../scripts/lifecycle.sh) `snapshot restore` | Clock sync after restore (via archived restore helper or e2e prepare) |
| `scripts/archive/create-localnet-stream-fixture.sh` | Clock sync again before seed create (defense in depth) |

Retired: snapshot age rebuild (`SNAPSHOT_MAX_AGE_S` prefund) and E2E
`refresh_stream_checkpoint_if_clock_drifted` top-up workaround. Use `wait-clock-synced.sh`
after restore instead.

### Chain settle before seed submit

| Piece | Role |
| --- | --- |
| `scripts/archive/wait-chain-settle.sh` | Waits for owner nonce / blocks after logoscore smokes so seed create/close does not reuse a stale committed nonce |

Wired before seed create and close in `scripts/archive/create-localnet-stream-fixture.sh` and
`scripts/archive/demo-stream-teardown-localnet.sh`.

### Conservative vault sizing (defaults)

| Env / constant | Default | Notes |
| --- | --- | --- |
| `SEED_DEPOSIT_AMOUNT` | 1000 | One-time prefund / pinata baseline |
| `SEED_STREAM_ALLOCATION` | 200 | About 20% of deposit; enough for back-to-back runs after close returns unaccrued |
| `SEED_STREAM_RATE` | 1 | Minimum per demo policy |

Pinata rounds in `prefund-localnet.sh` scale from deposit:
`(DEPOSIT_AMOUNT + 149) / 150 + 4`. Rebuild the funded snapshot after changing defaults
(`make full-reset-localnet` or `FULL_RESET=1` prepare).

Testnet funding must be sized so a run completes without claim recycling funds; see
[archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md) (Funding must be sufficient
without claim).

### Honest seed CLI (stream vs vault)

| Command | Parameters |
| --- | --- |
| `create-stream-onchain` | Stream only: owner, provider, vault/stream ids, rate, allocation. No `--deposit-amount`. Manifest `demo_deposit_amount` is read from on-chain vault holding balance. Client preflight errors if `allocation > unallocated`. |
| `prefund-onchain`, `deposit-onchain` | Vault: `--deposit-amount` (actual `Deposit` instruction) |
| `close-stream-onchain`, `claim-onchain` | Stream teardown: provider is the authority/signer (Clock01) |
| `seed-onchain` | Combined prefund + create (convenience) |

---

## Lifecycle scripts (verify parity with E2E)

| File | Role |
| --- | --- |
| `scripts/demo-stream-teardown-localnet.sh` | `logoscore stop`, optional wallet sync, seed `close-stream-onchain` (provider signer), strip stream fields from manifest. Skip via `SKIP_VERIFY_STREAM_TEARDOWN=1`. |
| `scripts/sync-seed-wallet-after-logoscore.sh` | Brief logoscore: open scaffold wallet, `sync_to_block`, `close` — seed wallet nonce after smokes. |
| `scripts/verify-step13-dod.sh` | Combined `EXIT` trap (teardown + cleanup); `logos_execution_zone close` before `logoscore stop` in smoke. |
| `scripts/step12-topup-and-prepare.sh` | `trap teardown_verify_stream EXIT` after create. |
| `scripts/create-localnet-stream-fixture.sh` | `logoscore stop`, `wait-chain-settle`, `wait-clock-synced`, then seed create. |

Default teardown: `VERIFY_TEARDOWN_CLAIM=0` (skip claim when accrued is zero). Teardown clears
manifest stream fields after close attempt; confirm `Confirmed` in logs for on-chain close.

Close on chain: `close_stream` authority is the provider; seed close signs with the provider key
in `.scaffold/wallet`. E2E success criterion: `stream_state == Closed`.

---

## Goal (unchanged)

Per-run lifecycle: vault baseline from snapshot to create at `next_stream_id` to proof prepare
with explicit `stream_id` to teardown close then optional claim. No reuse of stream ids across
runs; vault is reused.

Dual prepare methods (universal module — two names, not overloads):

| Method | Args | Output |
| --- | --- | --- |
| `prepareEligibilityProofWithStreamProposalForStoreQuery` | n8, provider_peer | `stream_proposal` |
| `prepareEligibilityProofWithStreamProofForStoreQuery` | n8, provider_peer, stream_id | `stream_proof` |

Baseline manifest (v2): no `stream_id` / `stream_config_account_id`. Per-run manifest adds them
after create.

---

## Implemented in tree (summary)

Module: dual prepare methods; fold clock fixes in FFI/C++; single demo clock `Clock01` for every
instruction and for the module now read used by the fundability fold.

Orchestration: `run_local_e2e.py` strips stream fields on load; create at `next_stream_id`; close
default `E2E_CLOSE_VIA=seed` with provider authority on both localnet and testnet; claim via the
seed path with `chainAction` fallback and optional on testnet
(`E2E_CLAIM_OPTIONAL`); continuation leg pinata + `ensure_continuation_vault_funded`; artifacts
`create_demo_stream`, `demo_close_stream`, `demo_claim`.

Scripts: `make prepare-localnet` / [`scripts/e2e.sh`](../../../scripts/e2e.sh) `local prepare` (vault-only baseline; per-run stream in orchestrator);
[`scripts/archive/create-localnet-stream-fixture.sh`](../../../scripts/archive/create-localnet-stream-fixture.sh) with `CREATE_FORCE` / `E2E_PER_RUN_STREAM`; seed
`close-stream-onchain`, `claim-onchain`, `deposit-onchain`, `top-up-stream-onchain`,
`wait-clock-synced`.

Makefile: `full-reset-localnet`, `verify-step17-back-to-back` (leg 2 `SKIP_SEED=1`, continuation
topup).

Docs touched: [archive/steps/local-store-dual-host-runbook.md](../../archive/steps/local-store-dual-host-runbook.md),
[program-index.md](../../development-map/program-index.md),
[integration-contracts.md](../../reference/integration-contracts.md),
[archive/operator/localnet-recovery.md](../../archive/operator/localnet-recovery.md),
[archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md).

---

## Verification matrix

| Gate | Command | Last known result |
| --- | --- | --- |
| Step 10a | `make verify-step10a` after prepare | Green |
| Step 12 default | `make verify-step12` | Green |
| Step 12 strict proof | `REQUIRE_STREAM_PROOF=1 make verify-step12` after Step 13 on same chain | Green |
| Step 13 | `make verify-step13` | Green; teardown close confirmed in logs |
| Step 17 | `SKIP_BUILD=1 make verify-step17` | Green |
| Back-to-back | `SKIP_BUILD=1 make verify-step17-back-to-back` | Green |
| Step 18 testnet | `make verify-step18` | Green through create, fundable, paid Store query, close; claim optional (see known issue) |
| `create-stream-onchain --help` | no `--deposit-amount` | Confirmed |

---

## Known limits

| Topic | Notes |
| --- | --- |
| Stale funded snapshot balance | Snapshots taken before the 1000/200 defaults still hold the old vault deposit until `make full-reset-localnet`. Clock reuse is fine; rebalance only when you need the new sizing on disk. |
| Clock sync without txs | Idle sequencer does not fold blocks; `wait-clock-synced.sh` uses pinata nudges. Set `SKIP_CLOCK_SYNC=1` only when debugging. |
| Owner-only close from user wallet | Module default authority = signer breaks close metas; use provider authority until product change. |
| `chainAction` vs seed on testnet | Testnet `chainAction` is unreliable (`RPC_FAILED`); the demo uses the direct-submit seed path for create/close/claim, with `chainAction` as opt-in fallback. |
| Testnet claim not confirming | Claim is optional in the demo; tracked in [archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md). |
| Many closed stream PDAs | Expected on long-lived localnet/testnet; each run uses the next id. |

---

## Non-goals

Guest accrual math; Track B UI; auto-closing all historical streams; retiring
`tools/lez-testnet-submit`; fixing testnet claim confirmation (tracked separately under Step 18).

---

## Completion checklist

| Item | Done |
| --- | --- |
| Dual prepare methods + delivery bridge | yes |
| Vault-only manifest + per-run stream fields | yes |
| Step 10a vault-only verify | yes |
| E2E per-run create + provider seed close + back-to-back | yes |
| `findActiveStreamForProvider` removed | yes |
| Verify scripts close stream after smokes (lifecycle) | yes |
| `REQUIRE_STREAM_PROOF=1` after Step 13 same chain | yes |
| Clock sync after restore (testnet-faithful time) | yes |
| Honest `create-stream-onchain` (no deposit flag) + unallocated preflight | yes |
| Conservative default deposit / allocation | yes (new prefund; old snapshots optional reset) |
| Testnet demo create to fundable to paid Store query to close | yes (Clock01) |
| Testnet claim | optional in demo; open issue tracked under Step 18 |

---

## Testnet outcome (2026-06-28)

This work took the 24c per-run lifecycle onto public testnet and cleared the create-time
depletion gate. Testnet chain I/O remains Step 18 Part B; the items below are the durable record.

### Demo rewired to Clock01

The demo uses `Clock01` (refreshes every block, base58
`4BdcjoXkq786TMWcBGGHqcxeLYMZmn17rL4eM9ZyRWNU`) instead of `Clock10`
(`...RWSs`, every 10 blocks). On testnet (about 90 to 120 seconds idle block time) `Clock10`
jumped roughly 600 seconds per update, fully accruing a `rate=1` stream the moment it was
created. `Clock01` reduces the create-time fold gap to a single block (about 120 seconds
observed), which clears the post-create fundability gate.

Rewire points (Clock10 to Clock01):

- `examples/src/bin/seed_localnet_fixture.rs`, `examples/src/bin/bootstrap_testnet_fixture.rs`:
  constant `CLOCK_10_PROGRAM_ACCOUNT_ID` to `CLOCK_01_PROGRAM_ACCOUNT_ID` (instruction
  accounts, manifest value written, and `wait-clock-synced` read).
- `logos-payment-streams-module/src/payment_streams_ffi_bridge_writes.c`:
  `ps_ffi_fixed_clock_10_account_id` selects `CLOCK_ACCOUNT_CHOICE_CLOCK01`, the single clock
  source for every module instruction and the module now read used by the fundability fold.
- `logos-payment-streams-module/src/payment_streams_module_impl.cpp` and `..._writes.cpp`:
  hardcoded fallback `...RWSs` to `...RWNU`.
- Manifests (`fixtures/testnet.json`, `*.json.example`, `localnet-debug.json`): the
  `clock_10_account_id` field value to the Clock01 id. The field name is kept as a legacy key;
  the value is authoritative.
- Scripts `verify-step10a-dod.sh` (expected value) and `verify-step18-testnet-read-smoke.sh`
  (fallback) to the Clock01 id.

The FFI unit test `clock_fixture_matches_known_clock10_literal` still pins the `Clock10`
selector to its own id and is intentionally unchanged. Identifiers that still carry `10`
(`ps_ffi_fixed_clock_10_account_id`, `kDefaultClock10Base58`, manifest key `clock_10_account_id`,
module RPC `readClock10Decoded`) are legacy names now pointing at Clock01; renaming them would
ripple into the module RPC surface and is left as a separate cleanup.

### Verified on testnet (vault 0, owner `DkT97...`, provider `BhyL...`)

`SKIP_BUILD=1 make verify-step18` with the rebuilt module cleared the depletion gate:

- `create_demo_stream`: stream 3, `rate=1`, `allocation=1000`, `fold_gap_seconds=120`
  (one Clock01 block, was 601 with Clock10).
- `wait_stream_fundable`: pass, `unaccrued_lo=880`, `min_unaccrued_lo=250`.
- `store_query_success`: 75 messages, status 200 (paid Store query succeeded).
- `demo_close_stream`: streams 3, 4, 5 reached `stream_state=2` via the seed close path.

### Close and claim teardown on testnet

Testnet `chainAction` is unreliable (`RPC_FAILED`), so the teardown uses the direct-submit seed
path for both close and claim, with `chainAction` kept as fallback (`E2E_CLOSE_VIA=chainaction`
forces the old path).

- `seed_localnet_fixture` gained a `claim-onchain` subcommand (mirrors `close-stream-onchain`;
  provider is the claim authority/signer, `Instruction::Claim`, Clock01).
- `run_local_e2e.py`: testnet close and claim use the seed path; both skip the fast localnet
  poll overrides and use a 900 second subprocess timeout because testnet confirmation is slow and
  highly variable (observed 69 seconds to over 900 seconds). Post-close and post-claim checks
  poll up to 12 by 20 seconds so a tx that confirms after the subprocess returns or times out is
  still detected. `subprocess.TimeoutExpired` from the seed subprocess is caught.
- The close path previously used the localnet `.lez_payment_streams-state` `SIGNER_ID` as the
  signer; on testnet that is a stale account and made the close revert and hang. Fixed to use the
  manifest owner on testnet.

Claim is optional in the demo and is not reliably confirming on testnet; see
[archive/operator/testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md) for the observed behavior,
the next diagnostic step, and the funding-without-claim policy.
