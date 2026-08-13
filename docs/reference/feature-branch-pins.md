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

Step 45 freezes a deliberate split between operator-stack LEZ and program-graph LEZ.
The operator vs program-graph LEZ split stays.
Unification was [Step 48](../plan/wontfix/step-48-program-graph-lez-unify.md)
(wontfix).
Pins SSOT for the freeze packet:
[step-45-dependencies-and-patches.md](plan/completed/step-45-dependencies-and-patches.md).

## LIP-155 spec (Step 49 pin)

Single citation pin for living docs and implementation comments.
`logos-lips` `master` at a tip that contains Step 40 (`435a6f18`,
[logos-lips#397](https://github.com/logos-co/logos-lips/pull/397)) and Step 41
(`f09f9e9e`, [logos-lips#379](https://github.com/logos-co/logos-lips/pull/379)).

| Artifact | Ref | Locked rev |
| --- | --- | --- |
| `logos-lips` / `rfc-index` `docs/anoncomms/raw/payment-streams.md` | `master` | `32d7da4e` |

Local clone: `lez-related/rfc-index`.
Historical Step 19 work used `feat/payment-streams-onchain-part` at `345c8eef`.

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
Step 23 hosted provider uses the same delivery pins. Wallet and LEZ pins follow the split tables
below. Workflow detail: [index.md](plan/index.md#delivery-integration-branches).

### Delivery flake lock (logos-delivery-module)

| Artifact | Branch ref | Locked rev (2026-08-12 Step 45 Phase 2) |
| --- | --- | --- |
| `logos-delivery` flake input | `feat/payment-streams-store-eligibility` | `c101e31a` (D45.19 Jul-30 ABI parent + eligibility; org remote `logos-messaging/logos-delivery`) |
| `logos-delivery-module` integration branch | `feat/payment-streams-store-eligibility` | `49eb6c0` on personal fork `s-tikhomirov/logos-delivery-module` (D45.20/D45.21 Lp async trampoline; D45.10) |

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

Module repo: same branch name on `logos-delivery-module`.
Durable remote SSOT is the personal fork
[`s-tikhomirov/logos-delivery-module`](https://github.com/s-tikhomirov/logos-delivery-module)
branch `feat/payment-streams-store-eligibility`
(tip recorded in the table above; includes Step 47 `user_peer_id`).
The operator account does not have push access to org
`logos-co/logos-delivery-module`, so the eligibility branch is not mirrored there
(D45.10). E2E defaults to a sibling checkout of that fork.
`logos-delivery` eligibility stays on the org remote
(`logos-messaging/logos-delivery`) because the module flake URL pins that ref
and push access exists there.

Re-run `nix flake update logos-delivery` in `logos-delivery-module` after pushing new commits to
the delivery eligibility branch, then commit the updated `flake.lock`. Steps 17–18 E2E may pin this rev explicitly in
scripts; until then the branch ref in `flake.nix` plus a committed lock is the source of truth.

## Split LEZ pin policy (Step 45)

Two LEZ pins on purpose. Do not force them equal.
Host and guest unification (and dropping the AT hex override) was Step 48
(wontfix).

| Layer | LEZ | Spel | What pins it |
| --- | --- | --- | --- |
| Operator stack | `v0.2.4` (`47eba256479f6f785acbd138834340703cd03401`) | `v0.6.0` (`0cb7e0980535af619482cf1c823f4d394b3ebd61`) in `scaffold.toml` | `scaffold.toml` `[repos.lez]`, Nix wallet / `lez-wallet-ffi-patched`, PATH `wallet` via scaffold cache (`ps_lez_cache` / `lez_scaffold_cache_dir`), `tools/lez-testnet-submit`, `scripts/lib/testnet-common.sh` `LEZ_OP_REV` |
| Program graph | `v0.2.0` (`a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a`) | same stock `v0.6.0` tag | `lez-payment-streams-core`, `-ffi`, `methods/guest`, `examples`, `nix/payment-streams-ffi.nix` |

Also locked:

| Artifact | Pin |
| --- | --- |
| LEZ-module (wallet wrapper upstream) | `549cf1159f20fa0c3fe8e88a5ab71de68a5aa34b` |
| Spel | stock `v0.6.0` (`0cb7e098…`) — no `vendor/spel-*`, no path `[patch]` |
| Testnet marketing name `v0.2.1` | not a software pin |

Do not pin LEZ `dev` tip.
Retired rule: `scaffold.toml` `[repos.lez].pin` must equal `nix/payment-streams-ffi.nix` rev.
Those pins now diverge by design (operator vs program graph).

Reference-only SHAs (not freeze pins): LEZ `v0.2.1` `15144ddb…`, `v0.2.2` `d6e4ae69…`,
`v0.2.3` `43b66b15…`.

### Operator stack (LEZ v0.2.4)

Chain writes from the wallet module, PATH `wallet`, scaffold localnet tooling, and the
testnet submit helper use operator LEZ `v0.2.4` (`47eba256…`).

| Layer | Upstream | Role |
| --- | --- | --- |
| LEZ `wallet_ffi` | [`logos-execution-zone`](https://github.com/logos-blockchain/logos-execution-zone) @ `47eba256…` (`v0.2.4`) | Deploy, program ELF helpers, public/private tx signing for the live operator surface |
| Wallet module | [`logos-execution-zone-module`](https://github.com/logos-blockchain/logos-execution-zone-module) @ `549cf115…` | Expose FFI to Logos modules (std::string / LogosAPI) |
| PATH `wallet` | scaffold cache under `~/.cache/logos-scaffold/repos/lez/${LEZ_OP_REV}/target/release/wallet` | Prefer over `~/.cargo/bin/wallet` (`ps_prepend_lez_wallet_path` / `lez_wallet_bin`) |
| `tools/lez-testnet-submit` | same LEZ rev in crate Cargo.toml + lockfile | Required testnet bootstrap (AT id + ELF source) |
| `scripts/lib/testnet-common.sh` | `LEZ_OP_REV` default `47eba256…` | Live shared helpers (moved out of `scripts/archive/`) |

Do not pin [PR 429 / PR 16](archive/superseded-wallet-pr-429-16.md) in this integration.

Flake refs:

- `scaffold.toml` `[repos.lez].pin` = `47eba256479f6f785acbd138834340703cd03401`
- `scaffold.toml` `[repos.spel].pin` = `0cb7e0980535af619482cf1c823f4d394b3ebd61`
- `lez-wallet-ffi-patched` input already at `47eba256…`
  (`logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/lez-wallet-ffi-patched/`)
- Patched wallet wrapper `upstream` =
  `github:logos-blockchain/logos-execution-zone-module/549cf1159f20fa0c3fe8e88a5ab71de68a5aa34b`
- `upstream.inputs.logos-execution-zone.follows` the operator LEZ input (`lez-wallet-ffi-patched`)

Retired shims: `wallet-v021.sh` and the wallet-shim directory are deleted.
Keep dual `NSSA_WALLET_HOME_DIR` + `LEE_WALLET_HOME_DIR` exports.

### Program graph (LEZ v0.2.0)

Guest ImageID, PDAs, core types, and payment-streams FFI artifacts stay on LEZ `v0.2.0`
(`a58fbce2…` / tag `v0.2.0`):

- `lez-payment-streams-core`, `lez-payment-streams-ffi`, `methods/guest`, `examples`
- `nix/payment-streams-ffi.nix` (`fetchFromGitHub` rev `a58fbce2…`)

Spel is stock `v0.6.0` tag-only in core, guest, and examples (no vendored tree).
Stock macros expand to `nssa_core::`; crates alias `nssa_core` to package `lee_core`
from program-graph LEZ `v0.2.0`.

### Authenticated transfer hex under the split

Keep `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` as documented testnet config (D45.14).
Under the split, program-graph FFI embeds the `v0.2.0` AT id, while the live sequencer
(and operator LEZ `v0.2.2+`) use `fe96c422…`. Dropping the override was Step 48
(wontfix).

`tools/lez-testnet-submit` is the AT id and ELF source for `bootstrap_testnet_fixture` /
`ensure-testnet-vault` (no ELF override flag today). Rebuild the release binary on
operator LEZ `v0.2.4` after any helper pin bump, then assert:

```text
auth-transfer-program-id-hex == fe96c422…   # live / operator AT id
```

A stale `tools/lez-testnet-submit/target/release` silently keeps an old AT id.
Record the helper LEZ rev in the Step 45 gate-log Pins column.

### Our patch (wrapper flake)

We use the local wrapper flake for payment-streams wallet behavior
(`send_generic_public_transaction_json`, `sign_public_payload`, private counterparts)
and build fixes (codegen API headers, `.lgx` metadata for bundler). Logos module id
matches upstream shipping identity: `logos_execution_zone`. The wallet module is
Universal (std::string/std::vector).

Patches live under
`logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`
and are applied in `postPatch` (list the directory; do not assume a fixed count):

- `wallet-qt-sign-public-payload.patch`
- `wallet-qt-sign-private-payload.patch`
- `wallet-qt-send-generic-public-transaction-json.patch`
- `wallet-qt-send-generic-private-transaction-json.patch`
- `wallet-qt-private-json-elf-path.patch`
- `wallet-qt-transfer-shielded-amount-prefix.patch`
- `wallet-qt-fix-authenticated-transfer-elf.patch` (skipped with `|| true` when already applied upstream)
- `wallet-qt-cmake-module-name.patch`
- `wallet-qt-metadata-module-name.patch`

`lez-wallet-ffi-patched/` also carries LEZ-side Rust patches
(`lez-rust-sign-public-payload.patch`, `lez-rust-sign-private-payload.patch`) for the
operator `wallet_ffi` input.

Upstream module identity is `lez_core`; the two identity patches plus wrapper
`preInstall`/`postInstall` hooks (bridge eval-time-templated `lez_core_plugin.so`
to `logos_execution_zone_plugin.so`) and the `lidl` rename keep
`payment_streams_module` call sites, `metadata.json` dependency, Qt client name,
and codegen header unchanged.

Step 30 (static dependency migration) lists `logos_execution_zone` in
`payment_streams_module`'s `metadata.json` `"dependencies"` and migrates the
wallet-call surface to codegen-emitted typed `modules().logos_execution_zone`
wrappers. Complex-type methods (`sign_public_payload`,
`send_generic_public_transaction_json`, private/JSON variants,
`authenticated_transfer_elf`) stay on a minimal dynamic-dispatch fallback through
`modules().api`; see
[step-30-static-dependency-migration.md](../plan/completed/step-30-static-dependency-migration.md#findings).

Runbook: [`archive/steps/wallet-510-runbook.md`](archive/steps/wallet-510-runbook.md).

### After changing pins

Operator stack (wallet / LEZ-module / helper):

```bash
cd logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched
nix flake update

cd ../../..   # logos-payment-streams-module/
nix flake update logos-execution-zone logos-execution-zone-module

# Rebuild submit helper after operator LEZ bump; assert AT id
cd ../tools/lez-testnet-submit && cargo build --release
# auth-transfer-program-id-hex must equal live fe96c422…
```

Program graph only (guest / FFI / core) — bump `nix/payment-streams-ffi.nix` and
Cargo tags/revs together; do not move `scaffold.toml` `[repos.lez]` unless the
operator pin is intentionally changing:

```bash
# Refresh rev / sha256 in nix/payment-streams-ffi.nix when the program-graph LEZ pin moves
nix build .#payment-streams-ffi
```

From repo root after an operator LEZ bump:

```bash
lgs setup
./scripts/archive/build-wallet-lgx.sh
nix build ./logos-payment-streams-module#lgx-portable
```

After a program-graph or guest ImageID change, rebuild the funded localnet snapshot:
`make full-reset-localnet`
([step-17b-localnet-snapshot-restore.md](plan/completed/step-17b-localnet-snapshot-restore.md)).
Do not require scaffold LEZ pin to match `payment-streams-ffi.nix` before reset;
snapshot `lez_pin` tracks the operator scaffold pin.

## Payment streams workspace

### Rust FFI

`nix/payment-streams-ffi.nix` builds `lez-payment-streams-ffi` and symlinks LEZ
`artifacts/` from the program-graph pin (`v0.2.0` / `a58fbce2…`), not the
operator scaffold pin.

### Scaffold

`scaffold.toml` `[repos.lez].pin` is the operator pin (`v0.2.4` / `47eba256…`).
`[repos.spel].pin` is stock `v0.6.0` (`0cb7e098…`).
After bumping the operator pin, re-run `lgs setup` so PATH `wallet` and localnet
tooling match `LEZ_OP_REV`.

### Payment-streams Logos module

- `logos_execution_zone` flake input → patched wrapper (upstream at `549cf115…`).
  Listed in `metadata.json` `"dependencies"` since Step 30; wallet calls use
  codegen-emitted typed wrappers with a minimal dynamic-dispatch fallback for
  patched methods.
- `logos-execution-zone` follows the operator LEZ pin (`v0.2.4`) for `wallet_ffi`
  via `lez-wallet-ffi-patched`.

## Public testnet under the split

Local E2E module writes and public testnet operator tooling share operator LEZ
`v0.2.4` (`47eba256…`). Guest ImageID and program-graph types stay on `v0.2.0`.
See [archive/steps/public-sequencer-store-runbook.md](archive/steps/public-sequencer-store-runbook.md)
and [step-18b-rc5-unify-handoff.md](plan/completed/step-18b-rc5-unify-handoff.md).

| Artifact | Pin / ref | Role |
| --- | --- | --- |
| `logos_execution_zone` .lgx | `47eba256` (v0.2.4) + module `549cf115` | Local E2E + testnet reads/writes via patched wallet |
| `lez-testnet-submit` | `47eba256` (v0.2.4) | Required bootstrap; AT id/ELF source; assert vs live `fe96c422…` |
| PATH `wallet` CLI | scaffold cache @ `47eba256` | `make deploy-testnet`, bootstrap, Piñata |
| Guest / FFI / core | `a58fbce2` (v0.2.0) | ImageID, PDAs, program-graph types |
| `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` | documented config → live `fe96c422…` | Required under the split (Step 48 wontfix) |

Guest `program_id_hex` on testnet: org deploy recorded in step packet; example in
`fixtures/testnet.json.example`.

Guest release profile: `methods/guest/Cargo.toml` ships `[profile.release]` with
`debug = 0; strip = "symbols"` (matches the `lez-programs` convention). The
ImageID is computed over the release-stripped binary, so this profile is part of
program identity. A rebuild with a different profile produces a different
`program_id_hex` and invalidates every PDA derived from it; after a guest
rebuild, re-derive `program_id_hex` and the vault/stream-config/clock PDAs.

As of Step 26, module `chainUsesTestnetSubmit()` always returns `false` and all
writes route through `submitGenericPublicViaFfi`. The helper remains required for
testnet bootstrap (AT id + ELF), not as an in-module write path. Removing the
helper (WalletCore port) is Tier C / later and must add AT hex + ELF inject flags
first (D45.16).

Runbook: [archive/steps/public-sequencer-store-runbook.md](archive/steps/public-sequencer-store-runbook.md).

## Verification commands

```bash
# Payment-streams FFI (repo root; program-graph LEZ)
nix build .#payment-streams-ffi

# Patched wallet lib (wrapper flake; operator LEZ)
nix build ./logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched#lib

# Payment-streams Logos module bundle
nix build ./logos-payment-streams-module#lgx-portable

# Submit helper (operator LEZ); rebuild before asserting AT id
(cd tools/lez-testnet-submit && cargo build --release)
```

Confirm PATH `wallet` resolves under the scaffold operator cache
(`~/.cache/logos-scaffold/repos/lez/47eba256…/target/release/wallet`) unless a
Phase 0 schema exception is recorded in the Step 45 gate log.

Step 11 DoD scripts under `scripts/archive/verify-step11*-dod.sh` are pinned
to rc5 and retained as historical checks; they fail on current LEZ pins and are not
run as gates.

`#lgx-portable` (not `#lgx`) is required for `lgpm` 0.2.0 / `logoscore`,
which reject `linux-amd64-dev` variants. The wallet bundle uses
`nix-bundle-lgx#portable` for the same reason
([Step 31](plan/completed/step-31-dependencies-upgrade.md)).

For `lgpm`, `logoscore`, and the Step 7+ loop see [`logos-runtime-guide.md`](logos-runtime-guide.md).
