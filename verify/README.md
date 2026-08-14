Operational scripts for E2E, testnet bootstrap, wallet `.lgx`, and hygiene.

Flag detail and artifacts: [docs/reference/verification-matrix.md](../docs/reference/verification-matrix.md).
Orchestrated recipes: [docs/reproduce/store-eligibility.md](../docs/reproduce/store-eligibility.md).
Manual protocol path: [docs/reproduce/payment-streams.md](../docs/reproduce/payment-streams.md).

## MODE × network

| MODE | Network | Command |
| --- | --- | --- |
| `module` | local | `MODE=module ./verify/e2e.sh local run` |
| `module` | testnet | `MODE=module ./verify/e2e.sh testnet run` |
| `store` | local | `MODE=store ./verify/e2e.sh local run` (or `./verify/e2e.sh local run`) |
| `store` | testnet | `MODE=store ./verify/e2e.sh testnet run` |

Privacy: `OWNER_PRIVACY=1` (PseudonymousFunding vault owner), `PROVIDER_PRIVACY=1` (shielded claim).
`PRIVACY=1` is still accepted as an alias for `OWNER_PRIVACY=1` when `OWNER_PRIVACY` is unset.
Each `run` performs prepare, orchestration, and teardown unless `SKIP_TEARDOWN=1`.

```bash
./verify/e2e.sh <local|testnet> <prepare|run|teardown>
./verify/e2e.sh build
```

Make aliases: `verify-module-local`, `verify-module-testnet`, `verify-store-local`, `verify-store-testnet`.
Terminology: `make check-terminology`.

## Environment

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

On-chain confirmation: [verification-matrix.md](../docs/reference/verification-matrix.md#on-chain-confirmation-principle).

## Cold start

Full checklist: [verification-matrix.md — Cold start](../docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine).

## Inventory

Root files are what a human types from README Testing, `docs/reproduce/payment-streams.md`, or `make help`.
Subdirs are ownership, not one taxonomy.
`verify/testnet/submit/` is the testnet write helper for vault-ensure, bootstrap, and Store E2E.

Root:

| File | Caller |
| --- | --- |
| `e2e.sh` | README Testing, Make `verify-*` |
| `lifecycle.sh` | `e2e.sh` |
| `fixture.sh` | `e2e.sh` |
| `module-e2e.sh` | `e2e.sh` when `MODE=module` |
| `module-close-negatives.sh` | Make `verify-module-local-close-negatives` only (`e2e.sh` does not exec it) |
| `repro-reset.sh` | `docs/reproduce/payment-streams.md` |
| `repro-shell.sh` | same |
| `repro-lgs-setup.sh` | same |
| `repro-install.sh` | same |
| `repro-auth-transfer.sh` | same |
| `check-terminology.sh` | Make `check-terminology` |

`lib/`:

| File | Caller |
| --- | --- |
| `common.sh`, `auth_transfer.sh`, `chain_poll.sh`, `vault_scan.sh`, `repro-env.sh` | sourced by root scripts |
| `await_tx.py` | `chain_poll.sh` |
| `ensure-scaffold-lez-layout.sh` | `seed.sh` |
| `auth-transfer-ensure.sh` | `repro-auth-transfer.sh`, Store E2E |
| `build-wallet-lgx.sh` | `e2e.sh`, Make `wallet-lgx` |
| `test_fixture_vault.sh` | Make `test-fixture-vault` (tests `fixture.sh` / `vault_scan`, not Store) |

`store/`:

| File | Caller |
| --- | --- |
| `run_e2e.py` | `e2e.sh` when `MODE=store` |
| `test_run_e2e_pure.py` | colocated unit test (`import run_e2e`) |
| `seed_provider_acceptance.py` | `run_e2e.py` |
| `continuation-owner-topup.sh` | `run_e2e.py` |
| `store-lifecycle.sh` | Make `verify-store-local-lifecycle` |
| `.gitignore` | Store Python cache ignore |

`testnet/`:

| File | Caller |
| --- | --- |
| `bootstrap-testnet.sh`, `bootstrap-testnet-module.sh`, `deploy-testnet.sh` | Make |
| `fund-testnet-accounts.sh` | `docs/reproduce/store-eligibility.md`, Store E2E, module-e2e hints (no Make target) |
| `prepare-testnet-privacy-seed.sh` | `docs/reproduce/store-eligibility.md`, Store E2E (no Make target) |
| `ensure-testnet-vault.sh` | Store testnet E2E |
| `testnet-common.sh`, `fund_testnet.sh` | sourced by testnet operators |
| `testnet_rpc.py` | `testnet-common.sh`, `run_e2e.py` |
| `sequencer_latency_probe.py` | Make `debug-sequencer-latency` |
| `submit/` (src, `Cargo.toml`, `Cargo.lock`, `flake.nix`, `flake.lock`) | `ensure-testnet-vault.sh`, bootstrap |

`seed/`: `seed.sh`, `Cargo.toml`, `Cargo.lock`, seed and bootstrap bins. Make `seed-fixture`.

`verify/fixtures/`: committed examples plus `testnet-module.json` and the wallet-config example.
Generated `localnet.json` / `testnet.json` are gitignored.

`archive/`: non-runnable historical source. Do not chmod from living Make recipes.

JSON-lines artifacts live under `.scaffold/e2e/artifacts/` (see verification matrix).
