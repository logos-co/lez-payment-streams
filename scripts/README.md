# Scripts — E2E and verification

Documentation for the unified automation stack. Operator gates:
[docs/verification-matrix.md](../docs/verification-matrix.md).

## Entry point

```bash
./scripts/e2e.sh <local|testnet> <prepare|run|teardown>
./scripts/e2e.sh build
```

Environment:

| Variable | Default | Role |
| --- | --- | --- |
| `MODE` | `store` | `module` = Flow A; `store` = Flow B |
| `CHAIN` | set by subcommand | `local` or `testnet` |
| `SKIP_BUILD` | `0` on prepare | Skip `.lgx` build when `1` |
| `SKIP_SEED` | `0` | Flow B continuation legs |
| `RESTORE_LOCALNET` | `1` | Snapshot restore for Flow B prepare |
| `FULL_RESET` | `0` | Rebuild funded snapshot when `1` |
| `FIXTURE_MANIFEST` | `fixtures/localnet.json` | Flow B chain ids |
| `E2E_PHASE` | `all` | Flow B Python: `core`, `claim`, or `all` |

`MODE=module` with `CHAIN=testnet` is rejected (Flow A testnet unsupported).

## Components

| Script | Role |
| --- | --- |
| [e2e.sh](e2e.sh) | Prepare / run / teardown; dispatches by `MODE` |
| [lifecycle.sh](lifecycle.sh) | Localnet, snapshots, testnet wallet, scaffold check |
| [fixture.sh](fixture.sh) | Prefund, vault, stream CLI helpers (Flow B prepare) |
| [module-e2e-local.sh](module-e2e-local.sh) | Flow A single-host happy path |
| [e2e/run_local_e2e.py](e2e/run_local_e2e.py) | Flow B dual-host orchestrator |

Shared helpers: [lib/common.sh](lib/common.sh).

## Makefile mapping

| Target | Matrix cell |
| --- | --- |
| `make verify-module-local` | Flow A × localnet |
| `make verify-step17` | Flow B × localnet |
| `make verify-step17-back-to-back` | Flow B × localnet (lifecycle) |
| `make verify-step18` | Flow B × testnet |
| `make prepare-localnet` | Flow B prepare only |

Historical step DoD targets (`verify-step10a`, …) call archived scripts; see
[development-map/README.md](../docs/development-map/README.md).

## Archived demo wrappers (Step 24c)

These paths remain under `scripts/archive/` for reference and old notes. Prefer the
`e2e.sh` entry points above.

| Archived script | Modern equivalent |
| --- | --- |
| `demo-localnet-prepare.sh` | `make prepare-localnet` or `./scripts/e2e.sh local prepare` |
| `demo-localnet-fresh.sh` | `make full-reset-localnet` (sets `FULL_RESET=1` and runs prepare) |
| `demo-e2e-local.sh` | `make verify-step17` or `./scripts/e2e.sh local run` (`MODE=store`) |

Flow B JSON-lines artifacts use `e2e-<timestamp>.log` (archived wrapper used
`demo-e2e-local-<timestamp>.log`).

## Artifacts

JSON-lines logs under `.scaffold/e2e/artifacts/`:

- Flow A: `module-e2e-*.log` — phases `vault_init`, `deposit`, `create_stream`, `claim`, …
- Flow B: `e2e-*.log` — `store_query_success`, `store_query_missing_proof`, `claim`, …

## Recovery

[docs/demo-localnet-recovery.md](../docs/demo-localnet-recovery.md)

## Related

[docs/reference/naming-conventions.md](../docs/reference/naming-conventions.md)
