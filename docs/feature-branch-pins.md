# Feature branch pins for wallet integration

This document describes why we pin certain flake inputs to feature branches
and what changed in each repo to make that pin reproducible with Nix.

The overarching goal is to run the payment-streams demo stack against wallet APIs
that are not yet on upstream default branches.

Store querying through `delivery_module` is not pinned here.
Upstream is implementing Store access on their roadmap with a different design
than our earlier `logosdelivery_query_store` / `queryStore` PRs.
Integration work waits for that functionality on `logos-delivery-module` `master`
and does not use branch `feat/liblogosdelivery-query-store` or local forks of those PRs.

We still pin wallet FFI for arbitrary public transactions
(`wallet_ffi_send_public_transaction`)
and the corresponding wallet module surface (`send_public_transaction`)
until those PRs merge.

Until wallet merges happen,
flakes pin Git refs that correspond to open PRs
(or equivalently named branches that carry the same commits).

Links used when pinning:

- Universal public transaction signing (execution zone) —
  [`logos-blockchain/logos-execution-zone` PR 429](https://github.com/logos-blockchain/logos-execution-zone/pull/429).
  The flake input used by sibling repos may appear as `logos-blockchain/lssa`
  with `refs/pull/429/head` because module flakes historically named that input against `lssa`.
  Commit resolved by Nix matches PR head (`6721d8d96e71566f072bab2ededcf56d29b002b0`).
- Companion wallet module changes —
  [`logos-blockchain/logos-execution-zone-module` PR 16](https://github.com/logos-blockchain/logos-execution-zone-module/pull/16).

## Logos Delivery module (Store query — not used)

Earlier experiments opened PRs to expose `logosdelivery_query_store` and
`delivery_module.queryStore`. That path is retired for this integration:
do not point `logos-delivery-module` flakes at `feat/liblogosdelivery-query-store`
or maintain a parallel `queryStore` in our forks.

When upstream merges Store query support on `master`, consume the released API
only (see N6 in `integration-plan-v2.md`).

## Payment streams workspace (`lez-payment-streams`)

This repo ties together Rust FFI (`payment-streams-ffi`),
the payment-streams Logos module (`logos-payment-streams-module`),
and integration docs.

### Rust FFI (`nix/payment-streams-ffi.nix`)

Goal.

`lez-payment-streams-ffi` must symlink LEZ program-methods artifacts from the same tree
that understands current NSSA layouts used by the crate graph.

What we changed.

- `fetchFromGitHub` for `logos-blockchain/logos-execution-zone`
  was bumped to PR 429 head
  (`rev = 6721d8d96e71566f072bab2ededcf56d29b002b0`,
  `sha256 = sha256-t0SsUY2+gusYfvTZP1yUORIhlDiQWagV6pUUwCplEew=`).

That aligns vendored LEZ sources used during `nix build .#payment-streams-ffi`
with the wallet FFI branch.

### Payment-streams Logos module (`logos-payment-streams-module/flake.nix`)

Goal.

The module declares `lez_wallet_module` as a flake dependency (`mkLogosModule` `flakeInputs`).
That dependency must be the wallet PR 16 branch
built against LEZ PR 429,
not older LEZ revisions bundled inside an unlocked submodule flake lock.

What we changed.

1. Pin LEZ (PR 429).

   - Root flake input `logos-execution-zone.url` =
     `github:logos-blockchain/lssa?ref=refs/pull/429/head`.
   - Child flake follows —
     `logos-execution-zone-module.inputs.logos-execution-zone.follows = "logos-execution-zone"`.

2. Pin wallet module (PR 16) behind a thin wrapper.

   Direct upstream PR 16 triggers several mismatches with the stock toolchain:

   - CMake did not put `wallet_ffi.h` on the compile line for the Qt plugin target
     (`INTERFACE_INCLUDE_DIRECTORIES` on an `IMPORTED` target did not propagate as expected).
   - Upstream PR 16 metadata names the module `liblogos_execution_zone_wallet_module`,
     while `logos-module-builder` / `logos-cpp-generator` historically assume `lez_wallet_module`
     for dependency APIs and plugin filenames (`lez_wallet_module_plugin.so`).

   So instead of pointing `mkLogosModule` straight at the upstream PR flake,
   we added a local wrapper flake
   `logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`:

   - Input `upstream` =
     `github:logos-blockchain/logos-execution-zone-module?ref=refs/pull/16/head`.
   - Input `upstream.inputs.logos-execution-zone.follows = "logos-execution-zone"`.
   - Applies `cmake-wallet-ffi-include.patch`
     (adds `target_include_directories(... "${LOGOS_EXECUTION_ZONE_WALLET_INCLUDE}")`).
   - `postPatch` on the wallet derivation substitutes `metadata.json`
     so manifest name matches `lez_wallet_module`.
   - `postInstall` installs `metadata.json` into `$out/` and adds
     `$out/lib/lez_wallet_module_plugin.so` symlink to the real Qt plugin `.so`.
   - PR 16 ships plain CMake packages (no `mkLogosModule`), so the wallet store path
     never gained `include/lez_wallet_module_api.{h,cpp}` that `logos-plugin-qt`
     copies into dependents before `--general-only`. The wrapper adds a
     `runCommand` pass that runs `logos-cpp-generator` with `--module-only` on
     `lez_wallet_module_plugin.so`, installs those files under `include/`, and
     reuses `logos-cpp-sdk` / `nixpkgs` from `logos-module-builder` (wired via
     follows in `logos-payment-streams-module/flake.nix`). After copying the base
     derivation into the new store path, `chmod -R u+w` is required so generated
     headers can be installed (store paths copied with `cp -a` are read-only).

   The parent flake sets:

   - `logos-execution-zone-module.url = "path:./nix/flakes/logos-execution-zone-module-patched"`.
   - `logos-execution-zone-module.inputs.logos-cpp-sdk.follows = "logos-module-builder/logos-cpp-sdk"`.
   - `logos-execution-zone-module.inputs.nixpkgs.follows = "logos-module-builder/nixpkgs"`.

Nix requirement.

Flakes referenced with `path:` must live under the Git tree Nix evaluates,
and untracked paths are invisible until `git add`.
Track the wrapper directory when locking or building from a dirty repo.

Lockfile.

`logos-payment-streams-module/flake.lock` encodes the resolved revisions and follows edges.
After changing inputs on the patched flake, run `nix flake update logos-execution-zone-module`
from `logos-payment-streams-module/` so new follows nodes are recorded.

## Verification commands

Workspace layout uses two flakes.
The repo root flake only exposes `payment-streams-ffi`.
The Logos Qt module (`lgx`) is built from `logos-payment-streams-module/` (that flake inputs `path:..`, so the root flake cannot forward `#lgx` without a circular lock).

```bash
# Payment-streams FFI (from lez-payment-streams repo root)
nix build .#payment-streams-ffi

# Payment-streams Logos module bundle (from logos-payment-streams-module subflake only)
nix build ./logos-payment-streams-module#lgx
```

Adjust the system attribute for non-Linux hosts.

For installing `.lgx` files with `lgpm` and running `logoscore`,
see [`docs/logos-operator-install-basics.md`](docs/logos-operator-install-basics.md).
