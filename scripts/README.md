# Scripts — E2E and verification

Canonical interface: [`e2e.sh`](e2e.sh). Documentation:
[docs/reference/verification-matrix.md](../docs/reference/verification-matrix.md).

## Cold start (optional, first machine)

Full checklist: [verification-matrix.md — Cold start](../docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine).

Minimal sequence from repo root:

```bash
# Tooling shell (logoscore + lgpm)
nix shell --accept-flake-config \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-package-manager \
  --command bash

# Inside that shell (lgs on PATH, e.g. export PATH="$HOME/.cargo/bin:$PATH")
lgs init    # once, if no .scaffold/
lgs setup   # once, if scaffold.toml missing
cargo risczero build --manifest-path methods/guest/Cargo.toml   # once, if guest .bin missing

# Store flow: ../logos-delivery-module + ../logos-delivery per feature-branch-pins.md

MODE=module CHAIN=local ./scripts/e2e.sh local run   # or ./scripts/e2e.sh local run for Store
```

Use `SKIP_BUILD=1` on later runs when modules under `.scaffold/e2e/user/modules` are already
installed. Path layout: [naming-conventions.md](../docs/reference/naming-conventions.md#scaffold-layout).
`make verify-module-local` / `make verify-store-local` are the same commands but still require
`logoscore` and `lgpm` on `PATH` (use the nix shell above).

## External verification (three paths)

```bash
# Module verification — Required, localnet
MODE=module CHAIN=local ./scripts/e2e.sh local run

# Store integration — Required, localnet (MODE=store is default)
./scripts/e2e.sh local run

# Store integration — Required, testnet (after bootstrap)
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run

# Module verification — Required, testnet (after bootstrap)
MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run
```

Each `run` performs prepare, orchestration, and teardown unless `SKIP_TEARDOWN=1`.

## Entry point

```bash
./scripts/e2e.sh <local|testnet> <prepare|run|teardown>
./scripts/e2e.sh build
```

| Variable | Default | Role |
| --- | --- | --- |
| `MODE` | `store` | `module` = module verification; `store` = Store integration |
| `CHAIN` | set by subcommand | `local` or `testnet` |
| `SKIP_BUILD` | `0` on prepare | Skip `.lgx` build when `1` |
| `SKIP_SEED` | `0` | Continuation legs (maintainer only) |
| `RESTORE_LOCALNET` | `1` | Snapshot restore for Store prepare |
| `FULL_RESET` | `0` | Rebuild funded snapshot when `1` |
| `E2E_PHASE` | `all` | Store Python: `core`, `claim`, or `all` |

`MODE=module` with `CHAIN=testnet` is fully supported.

## Components

| Script | Role |
| --- | --- |
| [e2e.sh](e2e.sh) | Prepare / run / teardown |
| [lifecycle.sh](lifecycle.sh) | Localnet, snapshots, testnet wallet |
| [fixture.sh](fixture.sh) | Prefund, vault, stream CLI (Store prepare) |
| [module-e2e.sh](module-e2e.sh) | Module verification orchestrator (local or testnet) |
| [e2e/run_local_e2e.py](e2e/run_local_e2e.py) | Store integration dual-host orchestrator |

## Make aliases (optional)

Same as `e2e.sh`: `verify-module-local`, `verify-module-testnet`, `verify-store-local`, `verify-store-testnet`.
Legacy: `verify-step17`, `verify-step18`.

## Maintainer only

[`archive/verify-store-local-lifecycle.sh`](archive/verify-store-local-lifecycle.sh) —
two Store runs on one local ledger (not an external integrator gate).

Historical DoD scripts under `archive/`; see
[docs/plan/index.md](../docs/plan/index.md).

## Artifacts

JSON-lines under `.scaffold/e2e/artifacts/` (see verification matrix).
