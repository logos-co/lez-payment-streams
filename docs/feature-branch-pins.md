# Feature branch pins for wallet integration

This document describes why we pin certain flake inputs to feature branches
and what changed in each repo to make that pin reproducible with Nix.

The overarching goal is to run the payment-streams demo stack against wallet APIs
that are not yet on upstream default branches.

Store querying through `delivery_module`: integration plan N6 and Step 6 (not pinned here).

## Wallet — primary path (491 + 19)

Chain writes use generic public transactions:

| Layer | Upstream PR | Role |
| --- | --- | --- |
| LEZ `wallet_ffi` | [`logos-execution-zone` PR 491](https://github.com/logos-blockchain/logos-execution-zone/pull/491) | Resolve accounts, serialize instruction words, send with program ELF bundle |
| Wallet Qt module | [`logos-execution-zone-module` PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19) | Expose 491 to Logos modules (`Q_INVOKABLE` / LogosAPI) |

PR 491 supersedes [PR 429](https://github.com/logos-blockchain/logos-execution-zone/pull/429).
PR 19 supersedes [PR 16](https://github.com/logos-blockchain/logos-execution-zone-module/pull/16) (429 JSON wrapper).

Do not pin 429 or 16 in this integration.

### Flake refs (until merge)

- `logos-execution-zone.url` =
  `github:logos-blockchain/lssa?ref=refs/pull/491/head`
  (flake input name may be `logos-execution-zone` or `lssa` depending on flake).
- Patched wallet wrapper `upstream` =
  `github:logos-blockchain/logos-execution-zone-module?ref=refs/pull/19/head`
  with `upstream.inputs.logos-execution-zone.follows` the same LEZ input as payment streams.

After both PRs merge, pin `main` on both repos and drop pull-request refs.

### Our patch (wrapper flake)

We still use the local wrapper flake when upstream metadata names differ from `lez_wallet_module`
or when we add `sign_public_payload` before upstream does.
PR 19 uses `mkLogosModule` (not the old plain-CMake PR 16 layout); the wrapper may only need
metadata/codegen overrides — if `nix bundle` fails after a pin bump, adjust
`logos-execution-zone-module-patched/flake.nix` against current PR 19 packages.

### After changing pins

```bash
cd logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched
nix flake update

cd ../../..   # logos-payment-streams-module/
nix flake update logos-execution-zone logos-execution-zone-module
```

Refresh `rev` / `sha256` in `nix/payment-streams-ffi.nix` when the LEZ pin moves
(`fetchFromGitHub` for program-methods symlink).

## Payment streams workspace (`lez-payment-streams`)

### Rust FFI (`nix/payment-streams-ffi.nix`)

`lez-payment-streams-ffi` symlinks LEZ `artifacts/` from the same `logos-execution-zone` revision
as the wallet stack (491 head until merge).

### Payment-streams Logos module (`logos-payment-streams-module/flake.nix`)

- `lez_wallet_module` flake input → patched wrapper (PR 19 upstream inside).
- `logos-execution-zone` follows PR 491 for LEZ + `wallet_ffi`.

## Verification commands

```bash
# Payment-streams FFI (repo root)
nix build .#payment-streams-ffi

# Payment-streams Logos module bundle
nix build ./logos-payment-streams-module#lgx
```

For `lgpm`, `logoscore`, and the Step 7+ loop see
[`logos-runtime-guide.md`](logos-runtime-guide.md)
and [`logos-runtime-guide.md`](logos-runtime-guide.md).
