Operational scripts for E2E, testnet bootstrap, wallet `.lgx`, and hygiene.

Flag detail and artifacts: [docs/reference/matrix.md](../docs/reference/matrix.md).
Orchestrated recipes: [docs/reproduce/store.md](../docs/reproduce/store.md).
Manual protocol path: [docs/reproduce/module.md](../docs/reproduce/module.md).

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
| `SEED_DEPOSIT_AMOUNT` | `1000` local / Store testnet `500` | Vault deposit lo. Skip the submit when unallocated already covers one `SEED_ALLOCATION`. Abort when the public owner cannot cover the deposit (no tx hash). Private owners skip the public-balance check. |
| `SEED_ALLOCATION` | `200` local / Store testnet `400` | createStream allocation lo; also the funded-vault skip bar. |
| `E2E_VAULT_UNALLOC_BUFFER_LO` | `50` | Extra unallocated lo on continuation top-up (`needed + buffer`). |
| `PS_WALLET_SHIELD_TIMEOUT` | `1800` | Wall-clock seconds for `wallet auth-transfer send` (CPU prove measured ~1020s). Stub receipts never take this path. |
| `PAYMENT_STREAMS_PROGRAM_ID_HEX` | unset local; fixture hex on testnet | Testnet ImageID. Must match the fixture and the pinned ELF. Unset on local so identity comes from the live guest. Do not leak across cells. |
| `LOCALNET_BLOCK_TIME` | unset (`15s` on explicit start) | Sequencer `block_create_timeout`. Use `45s` for local real prove. Applied at localnet start/ensure only. |

On-chain confirmation: [verification-matrix.md](../docs/reference/matrix.md#on-chain-confirmation-principle).

## Cold start

Full checklist: [verification-matrix.md — Cold start](../docs/reference/matrix.md#cold-start-first-time-on-a-machine).

## Inventory

Root files are what a human types from README Testing, `docs/reproduce/module.md`, or `make help`.
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
| `repro-reset.sh` | `docs/reproduce/module.md` |
| `repro-shell.sh` | same |
| `repro-lgs-setup.sh` | same |
| `repro-install.sh` | same |
| `repro-auth-transfer.sh` | same |
| `check-terminology.sh` | Make `check-terminology` |

`lib/`:

| File | Caller |
| --- | --- |
| `common.sh`, `auth_transfer.sh`, `chain_poll.sh`, `vault_scan.sh`, `repro-env.sh` | sourced by root scripts |
| `harness_policy.py` | `common.sh`, `run_e2e.py` |
| `test_harness_policy.py` | colocated unit test (`import harness_policy`) |
| `await_tx.py` | `chain_poll.sh` |
| `ensure-scaffold-lez-layout.sh` | `seed.sh` |
| `auth-transfer-ensure.sh` | `repro-auth-transfer.sh`, Store E2E |
| `build-wallet-lgx.sh` | `e2e.sh`, Make `wallet-lgx` |
| `check-relative-links.sh` | Make `check-links` |
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
| `fund-testnet-accounts.sh` | `docs/reproduce/store.md`, Store E2E, module-e2e hints (no Make target) |
| `prepare-testnet-privacy-seed.sh` | `docs/reproduce/store.md`, Store E2E (no Make target) |
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
