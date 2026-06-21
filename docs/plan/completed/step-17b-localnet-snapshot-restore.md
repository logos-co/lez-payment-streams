# Step 17b — localnet snapshot restore (complete)

Operational addendum to [Step 17](step-17.md). Runnable E2E still gates on
[step17-e2e-local.md](../../step17-e2e-local.md); this step removes pinata-heavy reuse and
stale-stream depletion on back-to-back runs.

## Goal

- **Stage A (once per LEZ pin + guest ImageID):** fund vault `0` without stream `0`, snapshot
  ledger + wallet + scaffold state to `.scaffold/snapshots/funded/`.
- **Per demo run:** restore baseline, submit one `create-stream-onchain`, write
  `fixtures/localnet.json`, then Step 17 / 10a verify as today.

## Design

| State | Contents |
| --- | --- |
| Funded baseline | Deployed program, owner balance, vault init + deposit, **no** stream |
| Per-run | Fresh stream `0` (accrual starts at create time) |

Ledger authority: `~/.cache/logos-scaffold/repos/lez/<pin>/rocksdb/` (stop sequencer before
copy). Validity: `snapshot.json` (`lez_pin`, `program_id_hex`, owner/provider ids, deposit,
rate/allocation).

## Operator commands

```bash
# Default prepare (restore if snapshot valid, else prefund once)
./scripts/demo-localnet-prepare.sh

# Rebuild snapshot from scratch (pinata + prefund)
FULL_RESET=1 ./scripts/demo-localnet-prepare.sh

# Same as FULL_RESET prepare (legacy name)
./scripts/demo-localnet-fresh.sh

# Stage A only
./scripts/prefund-localnet.sh funded

# Manual snapshot / restore
./scripts/snapshot-localnet.sh funded
./scripts/restore-localnet.sh funded
./scripts/create-localnet-stream-fixture.sh
```

Step 17 entrypoint `make verify-step17` calls `demo-localnet-prepare` via
`demo-e2e-local.sh` (`FULL_RESET` threads through env).

## Seed binary

`seed_localnet_fixture` subcommands:

- `prefund-onchain` — `initialize_vault` + `Deposit` only
- `create-stream-onchain` — `CreateStream` + manifest
- `seed-onchain` — full Step 10a one-shot (unchanged semantics via shared helpers)

## Verification

1. `FULL_RESET=1 ./scripts/demo-localnet-prepare.sh`
2. `./scripts/demo-localnet-prepare.sh` twice (second run must not call pinata)
3. `make verify-step17` back-to-back
4. After guest rebuild (`make build`) or LEZ pin change: restore-only prepare fails until
   `FULL_RESET=1` (vault/program mismatch on the restored ledger). Re-run prefund, snapshot, then
   normal restore path — e.g. after [Step 24](step-24-lee-harness-upgrade.md) harness guest bump.

## Status

Complete (2026-06-19). Verified: restore + create-stream path, back-to-back
`make verify-step17`, and `FULL_RESET=1` rebuild (including post–guest ImageID change).

Decision record: [N15](../../reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19).
