# Step 17b — localnet snapshot restore (complete)

Operational addendum to [Step 17](step-17.md). Runnable E2E still gates on
[step17-e2e-local.md](../../step17-e2e-local.md); this step removes pinata-heavy reuse and
stale-stream depletion on back-to-back runs.

## Goal

- **Stage A (once per LEZ pin + guest ImageID):** fund vault `0` without stream `0`, snapshot
  ledger + wallet + scaffold state to `.scaffold/snapshots/funded/`.
- **Per demo run (superseded by Step 24c):** restore vault-only baseline; create at
  `next_stream_id` after clock sync; teardown closes the run’s stream. Step 10a
  verify uses vault PDAs only (no stream in prepare manifest).

## Design

| State | Contents |
| --- | --- |
| Funded baseline | Deployed program, owner balance, vault init + deposit, **no** stream |
| Per-run | New stream at `next_stream_id`; demo clock synced before create ([Step 24c](step-24c-simplify-demo-flow.md)) |

Ledger authority: `~/.cache/logos-scaffold/repos/lez/<pin>/rocksdb/` (stop sequencer before
copy). Validity: `snapshot.json` (`lez_pin`, `program_id_hex`, owner/provider ids, deposit,
rate/allocation).

## Operator commands

Post–Step 24c, localnet prepare and Step 17 E2E go through [`scripts/e2e.sh`](../../../scripts/e2e.sh).
Legacy wrappers live under `scripts/archive/` (see [scripts/README.md](../../../scripts/README.md)).

```bash
# Default prepare (restore if snapshot valid, else prefund once)
make prepare-localnet

# Rebuild snapshot from scratch (pinata + prefund)
make full-reset-localnet

# Full Flow B demo (prepare + dual-host run; see e2e.sh for teardown)
make verify-step17
```

Manual snapshot (equivalent to stage A pieces):

```bash
FULL_RESET=1 ./scripts/e2e.sh local prepare   # prefund + snapshot save
./scripts/lifecycle.sh snapshot restore funded
```

Per-run stream create is owned by [`scripts/e2e/run_local_e2e.py`](../../../scripts/e2e/run_local_e2e.py),
not prepare.

`make verify-step17` runs `./scripts/e2e.sh local run`, which calls `local prepare` then the Python
orchestrator (`FULL_RESET` threads through prepare when set).

## Seed binary

`seed_localnet_fixture` subcommands:

- `prefund-onchain` — `initialize_vault` + `Deposit` only
- `create-stream-onchain` — `CreateStream` + manifest (stream params only; no deposit flag)
- `seed-onchain` — full Step 10a one-shot (unchanged semantics via shared helpers)

## Verification

1. `make full-reset-localnet`
2. `make prepare-localnet` twice (second run must not call pinata)
3. `make verify-step17` back-to-back
4. After guest rebuild (`make build`) or LEZ pin change: restore-only prepare fails until
   `make full-reset-localnet` (vault/program mismatch on the restored ledger). Re-run prefund, snapshot, then
   normal restore path — e.g. after [Step 24](step-24-lee-harness-upgrade.md) harness guest bump.

## Status

Complete (2026-06-19). Step 24c extended restore with clock sync, conservative deposit sizing,
and verify lifecycle teardown on the same chain ([step-24c-simplify-demo-flow.md](step-24c-simplify-demo-flow.md)).

Decision record: [N15](../../reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19).
