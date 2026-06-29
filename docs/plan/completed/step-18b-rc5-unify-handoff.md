# Step 18b — rc5 tooling unification (complete)

Handoff packet for Step 18b. Canonical operator docs:
[step-18-public-testnet-demo.md](step-18-public-testnet-demo.md),
[step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md),
[N16](../../reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06).

Status: **complete** on `master`. Operational LEZ pin is **`v0.2.0-rc5`**
(`27360cb7d6ccb2bfbcca7d171bab8a3938490264`) for local Step 17, module `.lgx`, testnet scripts,
and `tools/lez-testnet-submit`. Local verification: `make verify-step17`; testnet path:
read smoke, `make bootstrap-testnet`, `make verify-step18`.

## Audience and goal (achieved)

One LEZ pin for wallet, module, local E2E, and public testnet chain I/O. Dual-pin (510 reads +
rc3 writes) is retired from docs and scripts.

| Field | Value |
| --- | --- |
| Sequencer | `https://testnet.lez.logos.co/` |
| Org `program_id_hex` | `79b1dd5c441caede8f9f82c30de637aba465f94cc43817b1105c8c48c77d0fc9` |
| Deploy tx hash | `1787368626484789a2976a2aa8631d2b5b39c415c0a74b5a345474d1415f79b1` |
| Deploy block | 3284 |

Read order: [`AGENTS.md`](../../../AGENTS.md) → Step 18 plan →
[step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md) →
[program-index.md](../../development-map/program-index.md).

## Why dual-pin was abandoned (historical)

Step 18 WIP used pin `62d9ba10` for module reads and rc3 (`cf3639d8`) for deploy and
`lez-testnet-submit`. Live testnet expects LEE v0.3 public message hashing and 510-lineage
builtin ids; rc3 signing and `check-health` failed against the public sequencer. Unification
target **`v0.2.0-rc5`** matches testnet and includes `lez/wallet-ffi` plus generic public tx
flows. See recon summary in git history on `feat/lez-unify-v0.2.0-rc5`.

## Execution summary (Path C)

1. WIP snapshot on `feat/step18-public-testnet`.
2. `feat/lez-unify-v0.2.0-rc5` from `master` — rc5 pins, helper on rc5, testnet script fixes.
3. Merge to `master`; rebase Step 18 branch; Part B DoD on unified tooling.

Harness note: `lez-payment-streams-core` `program_tests` may stay on PR 510 until Step 24
harness bump; that is not the operational pin.

## Key files

| Area | Path |
| --- | --- |
| Submit helper (Phase 9 retirement) | `tools/lez-testnet-submit/` |
| Bootstrap | `scripts/bootstrap-testnet.sh`, `scripts/testnet-common.sh` |
| Deploy | `scripts/deploy-testnet.sh` |
| Verify | `scripts/verify-step18.sh`, `scripts/verify-step18-testnet-read-smoke.sh` |
| LEZ pin | `scaffold.toml`, `nix/payment-streams-ffi.nix`, module wallet flake |
| Pins doc | `docs/feature-branch-pins.md` |

## Environment (testnet)

| Variable | Role |
| --- | --- |
| `CHAIN=testnet` | Testnet sequencer and manifest |
| `FIXTURE_MANIFEST=fixtures/testnet.json` | After bootstrap |
| `WALLET_CONFIG` / `WALLET_STORAGE` | `.scaffold/e2e/testnet-wallet/` (rc5) |
| `LEE_WALLET_HOME_DIR` | Same directory as testnet wallet home for CLI |
| `TESTNET_SKIP_PINATA=1` | Reuse manifest owner; owner must have balance |
| `LEZ_TESTNET_SUBMIT` | Optional helper path until Phase 9 |

Phase 9: remove helper and `CHAIN=testnet` C++ submit branch when module `chainAction` works
on testnet without subprocess.

## Operator defaults (recorded)

- Single rc5 PR merged to `master` before canonical Step 18 Part B docs.
- CI default remains `make verify-step17`.
- Regenerate gitignored `fixtures/testnet.json` after pin changes via `make bootstrap-testnet`.
