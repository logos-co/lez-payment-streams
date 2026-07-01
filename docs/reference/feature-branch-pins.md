# Feature branch pins for wallet integration

This document describes why we pin certain flake inputs to feature branches
and what changed in each repo to make that pin reproducible with Nix.

The overarching goal is to run the payment-streams demo stack against wallet APIs
that are not yet on upstream default branches.

Store query for the demo ships on our `logos-delivery` / `logos-delivery-module` forks
(Steps 15–16, [D2](reference/integration-decisions.md#d2-delivery-module-hook-design),
[N6](reference/integration-decisions.md#n6-delivery-module-store-query-exposure)). Step 15 is
complete on the delivery fork (verify commands in
[step-15-normative.md](plan/completed/step-15-normative.md)). Step 16 (eligibility bridge,
async `storeQuery`) is complete on the module fork. The module flake pins `logos-delivery` to
the integration branch; locked revs in the table below.

## LIP-155 spec (Step 19, complete)

Integration treats the on-chain LIP as done on the spec feature branch (merge to `main` optional).

| Artifact | Branch ref | Locked rev |
| --- | --- | --- |
| `logos-lips` / `rfc-index` `docs/anoncomms/raw/payment-streams.md` | `feat/payment-streams-onchain-part` | `345c8eef` |

Local clone: `lez-related/rfc-index`. Step 20 doc packets should link this branch/rev until
`main` catches up.

## Delivery integration branches (Steps 14–18)

Branch from upstream `master` in each delivery repo; avoid release-tag baselines and the
retired `feat/liblogosdelivery-query-store` branch. Branch name priority:
[index.md](plan/index.md#delivery-integration-branches).
Point the module flake's `logos-delivery` input at the integration branch (same name on
`logos-messaging/logos-delivery`). Configured in `logos-delivery-module/flake.nix`:

```nix
logos-delivery.url =
  "git+https://github.com/logos-messaging/logos-delivery?ref=feat/payment-streams-store-eligibility&submodules=1";
```

Commit `flake.lock` after changing the input; the lock file records the resolved `rev` at update
time (branch tip moves until you re-lock). Steps 17–18 E2E cite locked revs where needed; optional
Step 23 hosted provider uses the same delivery pins. Wallet pins follow the table in the next section.
Workflow detail: [index.md](plan/index.md#delivery-integration-branches).

### Delivery flake lock (logos-delivery-module)

| Artifact | Branch ref | Locked rev (2026-07-01) |
| --- | --- | --- |
| `logos-delivery` flake input | `feat/payment-streams-store-eligibility` | `64593368` (rebased onto upstream `master` post api-shape phase2; `eligibility_api.nim` adapted to `FFIContext[LogosDelivery]`) |
| `logos-delivery-module` integration branch | `feat/payment-streams-store-eligibility` | `f8a76ba` on fork `s-tikhomirov/logos-delivery-module` (rebased onto upstream `master`; preserves eligibility bridge alongside `collectOpenMetricsText()`) |

After each push to `logos-delivery`, run `nix flake update logos-delivery` in
`logos-delivery-module` and commit the lock. Record the resolved `rev` in this table when
Step 17 E2E is re-verified nix-only.

### Step 17 delivery install and `liblogosdelivery` overlay

E2E installs `delivery_module` via `nix build "$DELIVERY_MODULE_ROOT#lgx"` and
`lgpm install` (same as payment streams and wallet modules). Bundled
`liblogosdelivery.so` comes from the locked `logos-delivery` flake input inside
`logos-delivery-module`.

The outbound-proof bug (clearing `eligibilityProof` after JSON parse) is fixed at
`logos-delivery` rev `39b467ec` and above ([N13](../reference/decisions-historical.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18)).
Symptom on an older library: paid `storeQuery` → provider `BAD_REQUEST`, empty inbound proof.

Optional overlay (default when sibling repo exists): unless
`SKIP_LIBLOGOSDELIVERY_OVERLAY=1`, the demo script runs `make liblogosdelivery` in
`LOGOS_DELIVERY_ROOT` (default `../logos-delivery`) and copies `build/liblogosdelivery.so`
into each `delivery_module/` install — useful while editing `logos-delivery` without re-locking
the module flake.

Hermetic verification (no overlay): `SKIP_LIBLOGOSDELIVERY_OVERLAY=1 make verify-step17` with
`DELIVERY_MODULE_ROOT` pointing at a module checkout whose `flake.lock` resolves
`logos-delivery` to `64593368` or newer. Full checklist:
[archive/steps/local-store-dual-host-runbook.md](archive/steps/local-store-dual-host-runbook.md#hermetic-run-hand-off). Verified 2026-07-01.

Remove the overlay step from the script once every operator relies on hermetic installs only.

Pin table dates are when the row was last updated. Decision subsection titles in
[integration-decisions.md](reference/integration-decisions.md) use their own `(YYYY-MM-DD)` record dates;
those need not match the pin table calendar day.

Module repo: same branch name on `logos-delivery-module` (`flake.nix` + `flake.lock` at
`f8a76ba…` for the `logos-delivery` input). Push target may be org fork or personal fork
(`s-tikhomirov/logos-delivery-module`) until the integration branch lands on
`logos-co/logos-delivery-module`.

Re-run `nix flake update logos-delivery` in `logos-delivery-module` after pushing new commits to
that branch, then commit the updated `flake.lock`. Steps 17–18 E2E may pin this rev explicitly in
scripts; until then the branch ref in `flake.nix` plus a committed lock is the source of truth.

## Wallet — primary path (v0.2.0 operational + `main` module)

Chain writes use generic public transactions and program deploy FFI from LEZ `v0.2.0`
(operational pin; public testnet compatible):

| Layer | Upstream | Role |
| --- | --- | --- |
| LEZ `wallet_ffi` | [`logos-execution-zone`](https://github.com/logos-blockchain/logos-execution-zone) @ `a58fbce…` (`v0.2.0`) | Deploy, program ELF helpers, LEE v0.3 public tx signing |
| Wallet module | [`logos-execution-zone-module`](https://github.com/logos-blockchain/logos-execution-zone-module) @ `main` (Universal, `b555cd5…`) | Expose FFI to Logos modules (std::string / LogosAPI) |

Do not pin [PR 429 / PR 16](archive/superseded-wallet-pr-429-16.md) in this integration.

### Flake refs (Step 26)

- LEZ rev `a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a` (`v0.2.0`) in:
  - `scaffold.toml` `[repos.lez].pin`
  - `nix/payment-streams-ffi.nix` (`fetchFromGitHub`)
  - `lez-wallet-ffi-patched/flake.nix` (wallet wrapper input)
  - `tools/lez-testnet-submit` (testnet submit helper; same pin)
- `lez-payment-streams-core`, guest, FFI, and `examples/` use LEZ `v0.2.0` (`a58fbce…`) — same rev as operational pin.
- Patched wallet wrapper `upstream` =
  `github:logos-blockchain/logos-execution-zone-module` (plain `main`,
  post-PR 19 merge). `upstream.inputs.logos-execution-zone.follows` the
  same LEZ input as payment streams.

### Our patch (wrapper flake)

We use the local wrapper flake for payment-streams wallet behavior
(`send_generic_public_transaction_json`, `sign_public_payload`) and build
fixes (codegen API headers, `.lgx` metadata for bundler). Logos module id
matches upstream: `logos_execution_zone`. The wallet module on `main` is
Universal (std::string/std::vector); the Qt patches were rewritten in
Step 26 against that surface and the 4-argument
`send_generic_public_transaction(account_ids, signing_requirements,
instruction, program_id_hex)` signature. The payload now carries
`program_id_hex` (not `program_elf_hex` / `program_dependencies_hex`).
`wallet-qt-cmake-ffi-include.patch` in the same directory is optional
(Qt include propagation); wire it in `postPatch` if the wallet plugin
fails to find `wallet_ffi.h`.
If `nix bundle` fails after a pin bump, adjust
`logos-execution-zone-module-patched/flake.nix` against current `main` packages.

Step 30 (static dependency migration) lists `logos_execution_zone` in
`payment_streams_module`'s `metadata.json` `"dependencies"` and migrates the
wallet-call surface to codegen-emitted typed `modules().logos_execution_zone`
wrappers (Qt-free `lp` API style; `QString` ↔ `std::string` at the boundary).
Three repo-local / complex-type methods (`sign_public_payload`,
`send_generic_public_transaction_json`, `authenticated_transfer_elf`) stay on
a minimal dynamic-dispatch fallback through `modules().api`; see
[step-30-static-dependency-migration.md](../plan/completed/step-30-static-dependency-migration.md#findings).
D6's revisit condition is closed.

Runbook: [`archive/steps/wallet-510-runbook.md`](archive/steps/wallet-510-runbook.md).

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
./scripts/archive/build-wallet-lgx.sh
nix build ./logos-payment-streams-module#lgx-portable
```

## Payment streams workspace (`lez-payment-streams`)

### Rust FFI (`nix/payment-streams-ffi.nix`)

`lez-payment-streams-ffi` symlinks LEZ `artifacts/` from the same `logos-execution-zone` revision
as the wallet stack (operational pin `v0.2.0`).

### Scaffold localnet (`scaffold.toml`)

`[repos.lez].pin` must match the LEZ `rev` in `nix/payment-streams-ffi.nix` (Step 10a / 11d / 18b).
After bumping either pin, re-run `lgs setup` from this repo so `wallet` and localnet match the operational LEZ rev.
Rebuild the Step 17b funded snapshot after a LEZ pin or guest ImageID change:
`make full-reset-localnet` ([step-17b-localnet-snapshot-restore.md](plan/completed/step-17b-localnet-snapshot-restore.md)).

### Payment-streams Logos module (`logos-payment-streams-module/flake.nix`)

- `logos_execution_zone` flake input → patched wrapper (`main` upstream inside).
  Listed in `metadata.json` `"dependencies"` since Step 30 (static dependency
  migration); wallet calls use codegen-emitted typed wrappers with a
  minimal dynamic-dispatch fallback for patched methods.
- `logos-execution-zone` follows the operational LEZ pin (`v0.2.0`) for `wallet_ffi`.

## Step 18 public testnet (single v0.2.0 pin)

Local E2E, module `.lgx`, and public testnet share LEZ `v0.2.0` (`a58fbce…`). See
[archive/steps/public-sequencer-store-runbook.md](archive/steps/public-sequencer-store-runbook.md) and
[step-18b-rc5-unify-handoff.md](plan/completed/step-18b-rc5-unify-handoff.md).

| Artifact | Pin / ref | Role |
| --- | --- | --- |
| `logos_execution_zone` .lgx | `a58fbce` (v0.2.0) | Local E2E + testnet reads/writes |
| `lez-testnet-submit` | `a58fbce` (v0.2.0) | Retained as fallback; not dispatched from module as of Step 26 |
| `wallet` CLI | `a58fbce` (v0.2.0) | `make deploy-testnet`, bootstrap, Piñata |

Guest `program_id_hex` on testnet: org deploy recorded in step packet; example in
`fixtures/testnet.json.example`.

Guest release profile: `methods/guest/Cargo.toml` ships `[profile.release]` with
`debug = 0; strip = "symbols"` (matches the `lez-programs` convention). The
ImageID is computed over the release-stripped binary, so this profile is part of
program identity. A rebuild with a different profile produces a different
`program_id_hex` and invalidates every PDA derived from it; after a guest
rebuild, re-derive `program_id_hex` and the vault/stream-config/clock PDAs.

Retirement (Phase 9): when module `chainAction` works on testnet without
the helper on the FFI path, delete `tools/lez-testnet-submit`,
`chainUsesTestnetSubmit`, `submitGenericPublicViaTestnetHelper`, and the
`LEZ_TESTNET_SUBMIT` plumbing. As of Step 26, `chainUsesTestnetSubmit()`
always returns `false` and all writes route through
`submitGenericPublicViaFfi`; the helper is retained in-tree as a manual
operator fallback and is no longer invoked from the module.

Runbook: [archive/steps/public-sequencer-store-runbook.md](archive/steps/public-sequencer-store-runbook.md).

## Verification commands

```bash
# Payment-streams FFI (repo root)
nix build .#payment-streams-ffi

# Patched wallet lib (wrapper flake)
nix build ./logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched#lib

# Payment-streams Logos module bundle
nix build ./logos-payment-streams-module#lgx-portable
```

Step 11 DoD scripts under `scripts/archive/verify-step11*-dod.sh` are pinned
to rc5 and retained as historical checks; they fail on v0.2.0 and are not
run as gates.

`#lgx-portable` (not `#lgx`) is required for `lgpm` 0.2.0 / `logoscore`,
which reject `linux-amd64-dev` variants. The wallet bundle uses
`nix-bundle-lgx#portable` for the same reason
([Step 31](plan/upcoming/step-31-dependencies-upgrade.md)).

For `lgpm`, `logoscore`, and the Step 7+ loop see [`logos-runtime-guide.md`](logos-runtime-guide.md).
