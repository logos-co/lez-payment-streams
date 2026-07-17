# Verification matrix (flow x chain)

Canonical entry point: [`scripts/e2e.sh`](../../scripts/e2e.sh) with `MODE` and `CHAIN`.
Each `local run` / `testnet run` performs prepare, run, and teardown unless `SKIP_TEARDOWN=1`.

See [naming-conventions.md](naming-conventions.md) for `MODE` values vs Journey names.

## Cold start (first time on a machine)

Optional one-time setup before the commands below. `local run` calls prepare, which builds
modules unless `SKIP_BUILD=1`.

1. Host: [Nix](https://nixos.org/download/) with flakes enabled; Rust + RISC Zero toolchain for
   the guest ELF (`cargo risczero build --manifest-path methods/guest/Cargo.toml`, or `make build`
   in this repo).
2. Logos scaffold CLI `lgs` on `PATH` (install per your Logos workspace; many setups use
   `~/.cargo/bin` after installing `lgs`).
3. Run verification inside a shell that provides `logoscore` and `lgpm`:

```bash
nix shell --accept-flake-config \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-package-manager \
  --command bash
```

4. From the repo root (still in that shell, plus `lgs` on `PATH`):

```bash
lgs init      # if .scaffold/ is missing
lgs setup     # if scaffold.toml / layout is missing
```

5. Store integration only: clone `logos-delivery-module` at the default path
   `../logos-delivery-module` (or set `DELIVERY_MODULE_ROOT`) on the branch in
   [feature-branch-pins.md](feature-branch-pins.md).
   E2E does not clone it; build fails if the directory is missing.
   A `../logos-delivery` sibling is optional (local `liblogosdelivery` overlay only; Nix
   fetches the locked delivery input when building the module).
   Module verification (`MODE=module`) does not need delivery checkouts.
6. First local run (builds `.lgx` via Nix, starts localnet, installs modules). Expect a long
   first build; later runs can use `SKIP_BUILD=1` when `.scaffold/e2e/*/modules` are already
   populated.

`e2e.sh` sets `PAYMENT_STREAMS_GUEST_BIN` to the guest path under `methods/guest/target/...`
when the file exists; build the guest before Store prepare if seed/fixture steps fail.

Recovery: [archive/operator/localnet-recovery.md](../archive/operator/localnet-recovery.md).

## Terminology

- `MODE=module` — User Journey (module verification).
  Single-host path through `payment_streams_module` `chainAction` (vault, stream, claim).
  No Store or eligibility.
- `MODE=store` (default) — Developer Journey (Store integration verification).
  Dual-host paid Store query with LIP-155 eligibility proof;
  orchestrator [`scripts/e2e/run_local_e2e.py`](../../scripts/e2e/run_local_e2e.py).

## The matrix

|  | Localnet (`CHAIN=local`) | Testnet (`CHAIN=testnet`) |
| --- | --- | --- |
| User Journey — module | Required | Required |
| Developer Journey — Store | Required | Required |

## Support tiers

- Required — both journeys on both chains.
  Localnet needs no fixture; clone and verify on your machine.
  Testnet needs `fixtures/testnet.json` (one-time `make bootstrap-testnet`); module-only
  users can use `fixtures/testnet-module.json` (one-time `make bootstrap-testnet-module`).
  Store runs use a fresh vault per run (Step 33); set `VAULT_ID` to pin a vault id
  or `E2E_REUSE_BASELINE_VAULT=1` for the legacy vault-0 lifecycle path.
- Claim is required on both chains for the module (User Journey). The v0.2.0
  testnet upgrade resolved the previous claim reliability issue; see
  [archive/operator/testnet-claim-known-issue.md](../archive/operator/testnet-claim-known-issue.md).
- Developer Journey Store testnet teardown keeps default `E2E_CLAIM_OPTIONAL=1`
  until Step 32 D3 gate passes; strict runs use `E2E_CLAIM_OPTIONAL=0`.
  Artifact parsers treat phase `claim` as canonical (`demo_claim` is a
  transitional alias).

## Commands (canonical)

Per-cell prepare, bootstrap one-liners, verbosity flags, and expected artifacts:
[journeys/E2E.md](../journeys/E2E.md).

Make convenience aliases: `verify-module-local`, `verify-module-testnet`,
`verify-store-local`, `verify-store-testnet`. Legacy names `verify-step17` / `verify-step18`
still work.

Maintainer-only (not integrator gates): `make verify-store-local-lifecycle` or
[`scripts/archive/verify-store-local-lifecycle.sh`](../../scripts/archive/verify-store-local-lifecycle.sh).

## Notes

- Store local prepare restores a funded snapshot (identity + policy only, no
  program vault) and writes `fixtures/localnet.json` from owner/provider markers.
  The orchestrator scans for a fresh vault id and ensures it (init + deposit)
  before stream creation. Set `E2E_REUSE_BASELINE_VAULT=1` to use the legacy
  vault-0 reuse path (used by `verify-store-local-lifecycle`).
- On-chain confirmation principle: every `chainAction` op whose next step reads
  the state it writes is verified on-chain (sequencer inclusion + state poll),
  not by the wallet submit acknowledgement. See
  [journeys/E2E.md#on-chain-confirmation-principle](../journeys/E2E.md#on-chain-confirmation-principle).
  This applies to both `MODE=module` and `MODE=store` orchestrators.
- Module flow only ensures localnet is up and skips `delivery_module` build.
- Testnet gate: two consecutive green passes (Store + Module) on the public
  sequencer are recorded in
  [step-33-testnet-gate-log.md](../plan/completed/step-33-testnet-gate-log.md).
  Module testnet uses `VAULT_ID` to pin a fresh vault (default fixture vault 0
  accumulates stale streams across runs).
- Artifacts: `.scaffold/e2e/artifacts/` JSON-lines logs.
  Layout: [naming-conventions.md#scaffold-layout](naming-conventions.md#scaffold-layout).
  Module: `module-e2e-*.log` (`vault_init`, `deposit`, `create_stream`, `claim`, …).
  Owner-privacy optional gate:
  `MODE=module CHAIN=local OWNER_PRIVACY=1 ./scripts/e2e.sh local run`
  (or `make verify-module-local-privacy`; `PRIVACY=1` alias still works).
  Phases include `pre_shield` when pre-shielding runs.
  Provider-privacy optional gate:
  `MODE=module CHAIN=local PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run`
  (or `make verify-module-local-provider-privacy`). Claim confirms via
  `vault_holding` drop when the provider is private.
  Store × privacy is [Step 38](../plan/upcoming/step-38-store-privacy-e2e.md).
  Store: `e2e-*.log` (`store_query_success`, `store_query_missing_proof`, `claim`, …).
