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

5. Store integration only: sibling checkouts at default paths
   `../logos-delivery-module` and `../logos-delivery` on the branch in
   [feature-branch-pins.md](feature-branch-pins.md). Module verification does not need them.
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
- Claim is required on both chains for the module (User Journey). The v0.2.0
  testnet upgrade resolved the previous claim reliability issue; see
  [archive/operator/testnet-claim-known-issue.md](../archive/operator/testnet-claim-known-issue.md).
- Developer Journey Store testnet teardown keeps default `E2E_CLAIM_OPTIONAL=1`
  until Step 32 D3 gate passes; strict runs use `E2E_CLAIM_OPTIONAL=0`.
  Artifact parsers treat phase `claim` as canonical (`demo_claim` is a
  transitional alias).

## Commands (canonical)

```bash
# Module verification — Required, localnet
MODE=module CHAIN=local ./scripts/e2e.sh local run

# Module verification — Required, testnet
make bootstrap-testnet-module   # one-time
MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run

# Store integration — Required, localnet
./scripts/e2e.sh local run

# Store integration — Required, testnet
make bootstrap-testnet   # one-time
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

Make convenience aliases (same commands): `verify-module-local`, `verify-module-testnet`,
`verify-store-local`, `verify-store-testnet`. Legacy names `verify-step17` / `verify-step18`
still work.

Maintainer-only (not integrator gates): `make verify-store-local-lifecycle` or
[`scripts/archive/verify-store-local-lifecycle.sh`](../../scripts/archive/verify-store-local-lifecycle.sh).

## Notes

- Localnet prepare restores or seeds a funded vault baseline for Store flow;
  module flow only ensures localnet is up and skips `delivery_module` build.
- Artifacts: `.scaffold/e2e/artifacts/` JSON-lines logs.
  Module: `module-e2e-*.log` (`vault_init`, `deposit`, `create_stream`, `claim`, …).
  Store: `e2e-*.log` (`store_query_success`, `store_query_missing_proof`, `claim`, …).
