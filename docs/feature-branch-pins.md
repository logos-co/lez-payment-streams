# Feature branch pins for wallet integration

This document describes why we pin certain flake inputs to feature branches
and what changed in each repo to make that pin reproducible with Nix.

The overarching goal is to run the payment-streams demo stack against wallet APIs
that are not yet on upstream default branches.

Store query for the demo ships on our `logos-delivery` / `logos-delivery-module` forks
(Steps 15–16, [D2](reference/decisions-and-notes.md#d2-delivery-module-hook-design),
[N6](reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)). Step 15 DoD
tests live in `logos-delivery` (N8 Nim test + C eligibility-hook smoke); delivery commit SHAs
are recorded here when Step 17 needs reproducible installs (wallet-only pins below until then).

## Delivery integration branches (Steps 14–17)

Branch from upstream `master` in each delivery repo; avoid release-tag baselines and the
retired `feat/liblogosdelivery-query-store` branch. Default shared name:
`feat/payment-streams-store-eligibility` on `logos-delivery` (Steps 14–15) and
`logos-delivery-module` (Step 16); alternatives if needed:
`feat/lip155-store-eligibility`, `integration/payment-streams-store`.
Point the module flake's `logos-delivery` input at the fork rev that built `liblogosdelivery`.
When Step 17 adds flake pins here, record branch names and commit SHAs like the wallet table below.
Workflow detail: [integration-index.md](../integration-index.md#delivery-integration-branches).

## Wallet — primary path (510 on main + PR 19)

Chain writes use generic public transactions and program deploy FFI from LEZ 510:

| Layer | Upstream | Role |
| --- | --- | --- |
| LEZ `wallet_ffi` | [`logos-execution-zone` `main`](https://github.com/logos-blockchain/logos-execution-zone) at [PR 510](https://github.com/logos-blockchain/logos-execution-zone/pull/510) merge | Deploy, program ELF helpers, shielded `key_path`, zones API |
| Wallet Qt module | [`logos-execution-zone-module` PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19) | Expose FFI to Logos modules (`Q_INVOKABLE` / LogosAPI) |

Do not pin [PR 429 / PR 16](archive/superseded-wallet-pr-429-16.md) in this integration.

### Flake refs (Step 11d)

- LEZ rev `62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60` in:
  - `scaffold.toml` `[repos.lez].pin`
  - `nix/payment-streams-ffi.nix` (`fetchFromGitHub`)
  - `lez-wallet-ffi-patched/flake.nix` (wallet wrapper input)
- Patched wallet wrapper `upstream` =
  `github:logos-blockchain/logos-execution-zone-module?ref=refs/pull/19/head`
  with `upstream.inputs.logos-execution-zone.follows` the same LEZ input as payment streams.

After PR 19 merges, pin `main` on the wallet module repo and drop pull-request refs.

### Our patch (wrapper flake)

We use the local wrapper flake for payment-streams wallet behavior (guest ELF from env,
`send_generic_public_transaction_json`, `sign_public_payload`) and build fixes (codegen API headers,
`.lgx` metadata for bundler). Logos module id matches upstream PR 19: `logos_execution_zone`.
`wallet-qt-cmake-ffi-include.patch` in the same directory is optional (Qt include propagation);
wire it in `postPatch` if the wallet plugin fails to find `wallet_ffi.h`.
If `nix bundle` fails after a pin bump, adjust
`logos-execution-zone-module-patched/flake.nix` against current PR 19 packages.

Runbook: [`step11d-wallet-510.md`](step11d-wallet-510.md).

### After changing pins

```bash
cd logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched
nix flake update

cd ../../..   # logos-payment-streams-module/
nix flake update logos-execution-zone logos-execution-zone-module
```

Refresh `rev` / `sha256` in `nix/payment-streams-ffi.nix` when the LEZ pin moves
(`fetchFromGitHub` for program-methods symlink).

From repo root after LEZ bump:

```bash
lgs setup
nix build .#payment-streams-ffi
./scripts/build-wallet-lgx.sh
nix build ./logos-payment-streams-module#lgx
```

## Payment streams workspace (`lez-payment-streams`)

### Rust FFI (`nix/payment-streams-ffi.nix`)

`lez-payment-streams-ffi` symlinks LEZ `artifacts/` from the same `logos-execution-zone` revision
as the wallet stack (LEZ `main` / 510 merge).

### Scaffold localnet (`scaffold.toml`)

`[repos.lez].pin` must match the LEZ `rev` in `nix/payment-streams-ffi.nix` (Step 10a / 11d).
After bumping either pin, re-run `lgs setup` from this repo so `wallet` and localnet match LEZ `main`.

### Payment-streams Logos module (`logos-payment-streams-module/flake.nix`)

- `logos_execution_zone` flake input → patched wrapper (PR 19 upstream inside).
- `logos-execution-zone` follows LEZ 510 for `wallet_ffi`.

## Verification commands

```bash
# Payment-streams FFI (repo root)
nix build .#payment-streams-ffi

# Patched wallet lib (wrapper flake)
nix build ./logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched#lib

# Payment-streams Logos module bundle
nix build ./logos-payment-streams-module#lgx

./scripts/verify-step11d-dod.sh
```

For `lgpm`, `logoscore`, and the Step 7+ loop see [`logos-runtime-guide.md`](logos-runtime-guide.md).
