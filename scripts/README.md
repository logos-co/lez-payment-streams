# Scripts — E2E and verification

Canonical interface: [`e2e.sh`](e2e.sh).
Flag detail and artifacts: [docs/reference/verification-matrix.md](../docs/reference/verification-matrix.md).
Orchestrated recipes: [docs/reproduce/store-eligibility.md](../docs/reproduce/store-eligibility.md).
Manual protocol path: [docs/reproduce/payment-streams.md](../docs/reproduce/payment-streams.md).

## Cold start (optional, first machine)

Full checklist: [verification-matrix.md — Cold start](../docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine).

Minimal sequence from repo root:

```bash
# Manual protocol walkthrough — reset, toolchain shell, then docs/reproduce/payment-streams.md
./scripts/user-journey-reset.sh
./scripts/user-journey-shell.sh

# Maintainer E2E tooling shell (portable .lgx / #lgx-portable)
nix shell --accept-flake-config \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-package-manager \
  --command bash

# Inside that shell (lgs on PATH, e.g. export PATH="$HOME/.cargo/bin:$PATH")
lgs init    # once, if no .scaffold/
lgs setup   # once, if scaffold.toml missing
cargo risczero build --manifest-path methods/guest/Cargo.toml   # once, if guest .bin missing

# Store flow: ../logos-delivery-module + ../logos-delivery per feature-branch-pins.md

MODE=module ./scripts/e2e.sh local run   # or ./scripts/e2e.sh local run for Store
```

`user-journey-reset.sh`, `user-journey-shell.sh`, and `lib/user-journey-env.sh` keep historical filenames.
They remain the supported entry for the manual protocol path (pinned logoscore and lgpm flake SHAs).

Use `SKIP_BUILD=1` on later runs when modules under `.scaffold/e2e/user/modules` are already installed.
Path layout: [naming-conventions.md](../docs/reference/naming-conventions.md#scaffold-layout).
`make verify-module-local` / `make verify-store-local` are the same commands.
They still require `logoscore` and `lgpm` on `PATH` (use the nix shell above).

## External verification

```bash
# Module verification — localnet
MODE=module ./scripts/e2e.sh local run

# Owner privacy — PseudonymousFunding lifecycle (private owner, public provider)
MODE=module OWNER_PRIVACY=1 ./scripts/e2e.sh local run
# PRIVACY=1 is still accepted as an alias for OWNER_PRIVACY=1

# Store integration — localnet (MODE=store is default)
./scripts/e2e.sh local run

# Store owner privacy — PseudonymousFunding vault
MODE=store OWNER_PRIVACY=1 ./scripts/e2e.sh local run

# Store provider privacy — private provider claim
MODE=store PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run

# Store full privacy — both flags
MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run

# Store integration — testnet (after bootstrap)
MODE=store ./scripts/e2e.sh testnet run

# Module verification — testnet (after bootstrap)
MODE=module ./scripts/e2e.sh testnet run
```

Each `run` performs prepare, orchestration, and teardown unless `SKIP_TEARDOWN=1`.

On-chain confirmation: [verification-matrix.md](../docs/reference/verification-matrix.md#on-chain-confirmation-principle).

## Entry point

```bash
./scripts/e2e.sh <local|testnet> <prepare|run|teardown>
./scripts/e2e.sh build
```

| Variable | Default | Role |
| --- | --- | --- |
| `MODE` | `store` | `module` = module verification. `store` = Store integration. |
| `CHAIN` | set by subcommand | `local` or `testnet`. Set by `e2e.sh local` / `e2e.sh testnet`. Direct `module-e2e.sh` callers may export it. |
| `OWNER_PRIVACY` | `0` | `1` = PseudonymousFunding vault owner (module and Store). |
| `PROVIDER_PRIVACY` | `0` | `1` = private provider / shielded claim (module and Store). |
| `PRIVACY` | `0` | Alias for `OWNER_PRIVACY=1` when `OWNER_PRIVACY` is unset. |
| `SKIP_BUILD` | `0` on prepare | Skip `.lgx` build when `1`. |
| `SKIP_SEED` | `0` | Continuation legs (maintainer only). |
| `RESTORE_LOCALNET` | `1` | Snapshot restore for Store prepare. |
| `FULL_RESET` | `0` | Rebuild funded snapshot when `1`. |
| `E2E_PHASE` | `all` | Store Python: `core`, `claim`, or `all`. |

`MODE=module ./scripts/e2e.sh testnet run` is fully supported.

## Components

| Script | Role |
| --- | --- |
| [e2e.sh](e2e.sh) | Prepare / run / teardown |
| [lifecycle.sh](lifecycle.sh) | Localnet, snapshots, testnet wallet |
| [fixture.sh](fixture.sh) | Prefund, vault, stream CLI (Store prepare) |
| [module-e2e.sh](module-e2e.sh) | Module verification orchestrator (local or testnet; `OWNER_PRIVACY` / `PROVIDER_PRIVACY` profiles) |
| [module-e2e-privacy.sh](module-e2e-privacy.sh) | Sets `OWNER_PRIVACY=1` and execs `module-e2e.sh` |
| [e2e/run_local_e2e.py](e2e/run_local_e2e.py) | Store integration dual-host orchestrator |
| [check-terminology.sh](check-terminology.sh) | Role-terminology and living-doc journey-name gate |

## Make aliases (optional)

Same as `e2e.sh`: `verify-module-local`, `verify-module-testnet`, `verify-store-local`, `verify-store-testnet`.
Legacy: `verify-step17`, `verify-step18`.
Terminology: `make check-terminology` runs [check-terminology.sh](check-terminology.sh).

## Maintainer only

[`archive/verify-store-local-lifecycle.sh`](archive/verify-store-local-lifecycle.sh) —
two Store runs on one local ledger.

Historical DoD scripts under `archive/`. See [docs/plan/index.md](../docs/plan/index.md).

## Artifacts

JSON-lines under `.scaffold/e2e/artifacts/` (see verification matrix).
