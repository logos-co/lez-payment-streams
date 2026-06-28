# Step 24c — simplify demo flow (fresh stream per run)

Normative packet for agents. Index: [integration-index.md](../../../integration-index.md).
Prerequisites: Step 17/17b scripts; Step 24b (rc5 guest + tooling).
Related: [step-17b-localnet-snapshot-restore.md](../completed/step-17b-localnet-snapshot-restore.md),
[step17-e2e-local.md](../../step17-e2e-local.md),
[step-18-public-testnet-demo.md](step-18-public-testnet-demo.md).

## Status (2026-06-28)

| Area | State |
| --- | --- |
| Local Track A E2E (Step 17) | Green: `SKIP_BUILD=1 make verify-step17`, `make verify-step17-back-to-back` |
| Step 10a / 12 / 13 DoD on localnet | Green (default and strict Step 12 paths; see verification matrix) |
| Same-chain Step 13 then `REQUIRE_STREAM_PROOF=1` Step 12 | Green after clock sync + chain settle + lifecycle teardown (verified 2026-06-28) |
| Testnet-faithful localnet (clock, sizing, honest seed CLI) | Implemented in tree (see below); existing funded snapshots may still hold old deposit totals until `make full-reset-localnet` |
| Verify lifecycle teardown scripts | In tree (`demo-stream-teardown-localnet.sh`, `sync-seed-wallet-after-logoscore.sh`, `wait-clock-synced.sh`, `wait-chain-settle.sh`) |
| Step 18 testnet E2E unified lifecycle | Not part of local 24c gate; Phase 4 open |
| Packet move to `completed/` | Optional after operator commits harness scripts; Step 18 remains separate |

Local 24c orchestration (E2E per-run create, dual prepare methods, provider-signed seed close, back-to-back) is done. Verify scripts now match E2E lifecycle on a continuously running localnet: funded vault from snapshot reuse, per-run stream id, teardown close, no restore between Step 13 and strict Step 12.

---

## Operator sequence (same chain, no restore between 13 and 12)

```text
./scripts/restore-localnet.sh funded
./scripts/demo-localnet-prepare.sh
./scripts/verify-step13-dod.sh
REQUIRE_STREAM_PROOF=1 ./scripts/verify-step12-dod.sh
```

This sequence passed on localnet after the testnet-faithful harness updates (2026-06-28).

Equivalent Make targets: `make verify-step13`, then `REQUIRE_STREAM_PROOF=1 make verify-step12`.

---

## Testnet-faithful localnet harness (2026-06-28)

### Clock after snapshot restore

Restoring RocksDB rewinds on-chain `Clock10`. `CreateStream` sets `accrued_as_of` from that clock at create time; creating before the clock catches up causes a one-shot fold over snapshot age (“fold jump”).

| Piece | Role |
| --- | --- |
| `seed_localnet_fixture wait-clock-synced` | Poll `Clock10` until wall skew ≤ `MAX_CLOCK_SKEW_S` (default 5) |
| `scripts/wait-clock-synced.sh` | Calls the subcommand; nudges block production via pinata top-ups when the sequencer is idle (clock only advances in blocks) |
| `scripts/restore-localnet.sh` | Runs clock sync after `lgs localnet start` |
| `scripts/create-localnet-stream-fixture.sh` | Clock sync again before seed create (defense in depth) |

Retired: snapshot age rebuild (`SNAPSHOT_MAX_AGE_S` prefund) and E2E
`refresh_stream_checkpoint_if_clock_drifted` top-up workaround. Use `wait-clock-synced.sh`
after restore instead.

### Chain settle before seed submit

| Piece | Role |
| --- | --- |
| `scripts/wait-chain-settle.sh` | Waits for owner nonce / blocks after logoscore smokes so seed create/close does not reuse a stale committed nonce |

Wired before seed create and close in `create-localnet-stream-fixture.sh` and `demo-stream-teardown-localnet.sh`.

### Conservative vault sizing (defaults)

| Env / constant | Default | Notes |
| --- | --- | --- |
| `SEED_DEPOSIT_AMOUNT` | 1000 | One-time prefund / pinata baseline |
| `SEED_STREAM_ALLOCATION` | 200 | ~20% of deposit; enough for back-to-back runs after close returns unaccrued |
| `SEED_STREAM_RATE` | 1 | Minimum per demo policy |

Pinata rounds in `prefund-localnet.sh` scale from deposit: `(DEPOSIT_AMOUNT + 149) / 150 + 4`. Rebuild the funded snapshot after changing defaults (`make full-reset-localnet` or `FULL_RESET=1` prepare).

### Honest seed CLI (stream vs vault)

| Command | Parameters |
| --- | --- |
| `create-stream-onchain` | Stream only: owner, provider, vault/stream ids, rate, allocation. No `--deposit-amount`. Manifest `demo_deposit_amount` is read from on-chain vault holding balance. Client preflight errors if `allocation > unallocated`. |
| `prefund-onchain`, `deposit-onchain` | Vault: `--deposit-amount` (actual `Deposit` instruction) |
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

Default teardown: `VERIFY_TEARDOWN_CLAIM=0` (skip claim when accrued is zero). Teardown clears manifest stream fields after close attempt; confirm `Confirmed` in logs for on-chain close.

Close on chain: `close_stream` authority is the provider; seed close signs with the provider key in `.scaffold/wallet`. E2E success criterion: `stream_state == Closed`.

---

## Goal (unchanged)

Per-run lifecycle: vault baseline from snapshot → create at `next_stream_id` → proof prepare with explicit `stream_id` → teardown close then optional claim. No reuse of stream ids across runs; vault is reused.

Dual prepare methods (universal module — two names, not overloads):

| Method | Args | Output |
| --- | --- | --- |
| `prepareEligibilityProofWithStreamProposalForStoreQuery` | n8, provider_peer | `stream_proposal` |
| `prepareEligibilityProofWithStreamProofForStoreQuery` | n8, provider_peer, stream_id | `stream_proof` |

Baseline manifest (v2): no `stream_id` / `stream_config_account_id`. Per-run manifest adds them after create.

---

## Implemented in tree (summary)

Module: dual prepare methods; fold clock fixes in FFI/C++.

Orchestration: `run_local_e2e.py` strips stream fields on load; create at `next_stream_id`; local close default `E2E_CLOSE_VIA=seed` with provider authority; continuation leg pinata + `ensure_continuation_vault_funded`; artifacts `create_demo_stream`, `demo_close_stream`, `demo_claim`.

Scripts: `demo-localnet-prepare.sh` vault-only (`SKIP_STREAM_CREATE=1`); `create-localnet-stream-fixture.sh` with `CREATE_FORCE` / `E2E_PER_RUN_STREAM`; seed `close-stream-onchain`, `deposit-onchain`, `top-up-stream-onchain`, `wait-clock-synced`.

Makefile: `full-reset-localnet`, `verify-step17-back-to-back` (leg 2 `SKIP_SEED=1`, continuation topup).

Docs touched: [step17-e2e-local.md](../../step17-e2e-local.md), [integration-index.md](../../../integration-index.md), [integration-contracts.md](../../integration-contracts.md), [demo-localnet-recovery.md](../../demo-localnet-recovery.md).

---

## Verification matrix

| Gate | Command | Last known result |
| --- | --- | --- |
| Step 10a | `./scripts/verify-step10a-dod.sh` after prepare | Green |
| Step 12 default | `./scripts/verify-step12-dod.sh` | Green |
| Step 12 strict proof | `REQUIRE_STREAM_PROOF=1 ./scripts/verify-step12-dod.sh` after Step 13 on same chain | Green (2026-06-28) |
| Step 13 | `./scripts/verify-step13-dod.sh` | Green; teardown close confirmed in logs |
| Step 17 | `SKIP_BUILD=1 make verify-step17` | Green |
| Back-to-back | `SKIP_BUILD=1 make verify-step17-back-to-back` | Green |
| Step 18 testnet | `make verify-step18` | Not verified in 24c local work |
| `create-stream-onchain --help` | no `--deposit-amount` | Confirmed |

Optional follow-up: dedicated Makefile target `verify-step13-then-step12-proof` wrapping the operator sequence above (not required for gate if manual sequence stays documented).

---

## Known limits

| Topic | Notes |
| --- | --- |
| Stale funded snapshot balance | Snapshots taken before the 1000/200 defaults still hold the old vault deposit until `make full-reset-localnet`. Clock reuse is fine; rebalance only when you need the new sizing on disk. |
| Clock sync without txs | Idle sequencer does not fold blocks; `wait-clock-synced.sh` uses pinata nudges. Set `SKIP_CLOCK_SYNC=1` only when debugging. |
| Owner-only close from user wallet | Module default authority = signer breaks close metas; use provider authority locally until product change. |
| `chainAction` vs seed | Prefer seed for local create/close; wallet may report success before sequencer sees tx. |
| Many closed stream PDAs | Expected on long-lived localnet; each run uses next id. |

---

## Non-goals

Guest accrual math; Track B UI; auto-closing all historical streams; retiring `tools/lez-testnet-submit`.

---

## Remaining / optional

| Item | Notes |
| --- | --- |
| Rebuild funded snapshot | After deposit default change, run `make full-reset-localnet` once per machine |
| Step 18 testnet | Same lifecycle on public sequencer ([step-18-public-testnet-demo.md](step-18-public-testnet-demo.md)) |
| Makefile target | Optional `verify-step13-then-step12-proof` wrapping the operator sequence |
| Move packet to `completed/` | When team treats local 24c as closed |

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
| Step 18 testnet unified lifecycle | no |
| Move packet to `completed/` | optional |

When the team closes 24c locally, move this file to `docs/plan/completed/` and update [docs/plan/README.md](../README.md) and [docs/AGENT-BRIEF.md](../../AGENT-BRIEF.md).
