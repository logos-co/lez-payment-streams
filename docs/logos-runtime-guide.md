# Logos runtime guide

Build, install, and exercise `lez_wallet_module` and `payment_streams_module` in
`logoscore` (integration plan Steps 7, 9–13). Delivery modules are out of scope.

Related: [`feature-branch-pins.md`](feature-branch-pins.md),
[`step8-universal-legacy-probe-results.md`](step8-universal-legacy-probe-results.md),
[`integration-plan-v2.md`](../integration-plan-v2.md).

## Part 1 — First-time install (Step 7)

Build two `.lgx` packages (`lez_wallet_module`, `payment_streams_module`),
install with `lgpm`, and load them in `logoscore`.

Related in this guide:

- Pin and wrapper flake rationale — [`feature-branch-pins.md`](feature-branch-pins.md)
- Upstream CLI reference — `../logos-tutorial/logos-developer-guide.md`

## Workspace layout

```text
lez-related/
  lez-payment-streams/     # this repository (REPO)
  logos-cli/
    modules/               # MODULES — runtime install tree (do not commit)
```

`lgpm --modules-dir` and `logoscore -m` must use the same absolute `MODULES` path.

## Environment variables

Set these in every shell tab before build, install, or runtime commands.
Use `export` so a later `nix shell` subshell still sees them.

```bash
export REPO="$HOME/Downloads/software/waku/lez-related/lez-payment-streams"
export MODULES="$HOME/Downloads/software/waku/lez-related/logos-cli/modules"
```

Adjust paths if your checkout lives elsewhere.

After each build, refresh the `.lgx` path variables (store paths change when outputs rebuild):

```bash
export WALLET_LGX=$(readlink -f \
  "$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out"/*.lgx)

export PS_LGX=$(readlink -f "$REPO/result"/*.lgx)
```

Glob must be inside the `readlink -f` argument.
Writing `"$(readlink -f …/wallet-lgx-out)"/*.lgx` leaves a literal `*.lgx` in the variable.

Tooling (`lgpm`, `logoscore`, `lm`) is not on your normal `PATH`.
Use the same `nix shell` block in each terminal tab (see below), or prefix one-off commands with
`nix shell … --command`.

---

## Phase 1 — Build

### 1a — Payment streams module `.lgx`

From `REPO`:

```bash
cd "$REPO"
nix build ./logos-payment-streams-module#lgx
```

Check:

```bash
ls -l "$REPO/result"/*.lgx
```

Expected: one symlink `result` → a Nix store directory containing a file like
`logos-payment_streams_module-module-lib.lgx` (exact name follows the derivation).

Root flake note: `nix build .#lgx` at `REPO` does not work (circular flake lock with
`path:..` / `path:./logos-payment-streams-module`). The module flake is
`./logos-payment-streams-module#lgx`.

Optional — FFI only (pulled in automatically by the module build):

```bash
nix build .#payment-streams-ffi
```

### 1b — Wallet module `.lgx`

Wallet module is pinned to upstream
[PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19)
on LEZ
[PR 491](https://github.com/logos-blockchain/logos-execution-zone/pull/491);
bundle via the patched wrapper flake (see [`feature-branch-pins.md`](feature-branch-pins.md)):

```bash
cd "$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched"
nix bundle --bundler github:logos-co/nix-bundle-lgx .#lib -o ./wallet-lgx-out -L
```

Check:

```bash
ls -l ./wallet-lgx-out/*.lgx
```

Expected: one `.lgx` under `wallet-lgx-out/` (symlink to store), e.g.
`logos-execution-zone-module-dev-with-sdk-api-headers.lgx`.
Manifest name inside the package is still `lez_wallet_module`.

Refresh path exports:

```bash
export WALLET_LGX=$(readlink -f \
  "$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out"/*.lgx)
export PS_LGX=$(readlink -f "$REPO/result"/*.lgx)

test -f "$WALLET_LGX" && test -f "$PS_LGX" && echo "artifacts OK"
```

Expected: prints `artifacts OK`.

---

## Phase 2 — Tooling shell

In each terminal tab you use for install or runtime:

```bash
export REPO MODULES   # if not already set in this tab
export WALLET_LGX PS_LGX   # after builds

nix shell \
  github:logos-co/logos-package-manager#cli \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-module#lm
```

Check:

```bash
command -v lgpm logoscore lm
```

Expected: three paths under `/nix/store/.../bin/`.

New Cursor terminal tabs do not inherit an old `nix shell`; run the block again.

---

## Phase 3 — Install with lgpm

Load order: wallet first, then payment streams (empty `dependencies` in PS `metadata.json`; D6).

Before install, `echo "$WALLET_LGX"` and `echo "$PS_LGX"`.
Empty `PS_LGX` makes `lgpm install --file` fail with `install requires --file`
(re-export after each build; glob must be inside `readlink -f`, see above).

```bash
mkdir -p "$MODULES"

echo "$WALLET_LGX"
lgpm --modules-dir "$MODULES" install --file "$WALLET_LGX"

echo "$PS_LGX"
lgpm --modules-dir "$MODULES" install --file "$PS_LGX"
```

Check install layout:

```bash
test -d "$MODULES/lez_wallet_module" && echo "wallet dir OK"
test -f "$MODULES/lez_wallet_module/lez_wallet_module_plugin.so" && echo "wallet .so OK"
test -d "$MODULES/payment_streams_module" && echo "ps dir OK"
test -f "$MODULES/payment_streams_module/payment_streams_module_plugin.so" && echo "ps .so OK"

lgpm --modules-dir "$MODULES" list
```

Expected `list` output: two rows — `lez_wallet_module` and `payment_streams_module`,
both `core` / `blockchain`.

Unsigned local packages may print a warning; install still completes.

---

## Phase 4 — Run logoscore and load modules

`-m "$MODULES"` adds a search path only. It does not load every installed package.
`capability_module` loads with the daemon; you load wallet and payment streams explicitly.

### Tab A — daemon

Inside the tooling `nix shell`:

```bash
logoscore -D -m "$MODULES" -v
```

Expected (abbreviated):

- Log lines for `capability_module` loading
- `Logoscore daemon started (pid …)`
- Prompt returns (daemon runs in background)

### Tab B — client (new terminal)

Repeat Phase 2 (`export` + `nix shell`), then:

```bash
logoscore status
```

Expected: `Status: running`, modules section shows wallet and payment streams as
`not_loaded` initially, `capability_module` loaded.

```bash
logoscore load-module lez_wallet_module
logoscore load-module payment_streams_module
```

Expected: `Loaded module: lez_wallet_module` and `Loaded module: payment_streams_module`.

Check:

```bash
logoscore list-modules --loaded
logoscore status
```

Expected: 3 loaded, 0 crashed, 0 not loaded
(`capability_module`, `lez_wallet_module`, `payment_streams_module`).

Optional — load on daemon startup next time:

```bash
logoscore stop
logoscore -D -m "$MODULES" -l lez_wallet_module,payment_streams_module -v
```

---

## Phase 5 — Verification (Step 9 expectations)

### Offline plugin inspection

```bash
lm methods "$MODULES/payment_streams_module/payment_streams_module_plugin.so"
```

Expected: Universal plugin loads; no payment-streams business API until Steps 10–13
(only framework or codegen symbols until you add Step 10 methods).

```bash
lm methods "$MODULES/lez_wallet_module/lez_wallet_module_plugin.so" | rg list_accounts
```

Expected: a `list_accounts` (or similarly named) invokable method on the wallet plugin.

### Runtime module info

```bash
logoscore module-info payment_streams_module
```

Expected: `Status: loaded`. No custom RPC methods until Step 10.

Cross-module Universal to Legacy wallet calls were validated in Step 8
([`step8-universal-legacy-probe-results.md`](step8-universal-legacy-probe-results.md)).
For operator checks, exercise the wallet directly:

```bash
logoscore call lez_wallet_module list_accounts
```

Expected without LEZ / wallet JSON-RPC: RPC or call failure is normal before localnet.
The host must stay running and `payment_streams_module` must have loaded without crashing.

### Teardown

```bash
logoscore stop
```

Expected: daemon stops; a later `logoscore status` reports not running.

---

## Quick reference

| Package | Build | `.lgx` location |
|---------|--------|-----------------|
| `payment_streams_module` | `nix build ./logos-payment-streams-module#lgx` from `REPO` | `$REPO/result/*.lgx` |
| `lez_wallet_module` | `nix bundle … .#lib -o ./wallet-lgx-out` in patched flake dir | `…/wallet-lgx-out/*.lgx` |

| Variable | Purpose |
|----------|---------|
| `REPO` | `lez-payment-streams` git root |
| `MODULES` | Absolute `lgpm` install + `logoscore -m` directory |
| `WALLET_LGX` / `PS_LGX` | Absolute paths to built `.lgx` files for `lgpm install --file` |



---

## Part 2 — Universal module (Step 9)

Normative decision: integration plan D6.

## Decisions

| Topic | Choice |
|-------|--------|
| Interface | `"interface": "universal"` in `metadata.json` |
| Wallet in metadata | Empty `dependencies` — do not declare `lez_wallet_module` (Issue 31 / typed wrapper crash in sidecars) |
| Wallet calls | `modules().api->getClient("lez_wallet_module")->invokeRemoteMethod(...)` |
| Impl class | `PaymentStreamsModuleImpl` extends `LogosModuleContext` |
| Build | `logos-module-builder` `mkLogosModule` + `lez_payment_streams_ffi` external lib |
| Wallet `.lgx` | Install separately; use patched flake until upstream aligns module id (Part 1) |

Decision record: integration plan D6.

## Layout

```text
logos-payment-streams-module/
  metadata.json          # universal, dependencies []
  flake.nix
  CMakeLists.txt
  src/
    payment_streams_module_impl.h
    payment_streams_module_impl.cpp
```

Do not add `onInit` on the impl class; codegen treats it as a public API method.
Step 10 adds wallet read helpers using the `invokeRemoteMethod` pattern in the section above.

## Cross-module wallet pattern

```cpp
LogosAPI* api = modules().api;
LogosAPIClient* client = api->getClient(QStringLiteral("lez_wallet_module"));
const QVariant result = client->invokeRemoteMethod(
    QStringLiteral("lez_wallet_module"), QStringLiteral("list_accounts"));
```

Load order: `lez_wallet_module` before `payment_streams_module`. Do not call
wallet RPC before the wallet module is loaded.

Pinned SDK: use `invokeRemoteMethod` (same as Legacy modules in practice). Do not
use `LogosModules` typed wrapper from core sidecars.

## Step 9 verification

1. `nix build ./logos-payment-streams-module#lgx` (Part 1 Phase 1a if not already built).
2. Part 1 Phases 3–5: install, load, and Phase 5 checks (`module-info`, wallet `list_accounts`).
3. `metadata.json` has `"interface": "universal"` and empty `dependencies`.
4. Sources under `src/payment_streams_module_impl.{h,cpp}` only (no Legacy plugin shell).

## Later steps (plan Steps 10–11)

Step 10 adds wallet read helpers using the same `invokeRemoteMethod` pattern.
Step 11 adds writes via PR 19 wallet methods. New Universal API methods belong
on the impl class; wallet stays dynamic.

## References

- [`logos-universal-legacy-probe`](../logos-universal-legacy-probe/) — probe template
- [`step1-findings-scaffold-rpc.md`](step1-findings-scaffold-rpc.md) — LEZ localnet and RPC
- [`integration-plan-v2.md`](../integration-plan-v2.md) — step definitions

---

## Part 3 — Dev test loop (Steps 10+)

Steps 14–15 change `logos-delivery` / `liblogosdelivery` (Nim, C ABI smoke tests).
They do not use this Logos host loop; see [Steps 14–15](#steps-14-15-delivery-only) at the end.

---

## What repeats vs one-time

| Frequency | Work |
|-----------|------|
| One-time per machine | Pick `SCAFFOLD_WS`, run `lgs init` / `lgs setup`, deploy `lez_payment_streams`, note program id and test account ids ([`step1-findings-scaffold-rpc.md`](step1-findings-scaffold-rpc.md)) |
| One-time per machine | First `lgpm` install of wallet + PS ([Part 1](#part-1--first-time-install-step-7)) |
| Each new terminal | `export` paths + `nix shell` (tools) |
| Each dev iteration | PS `nix build` → `lgpm install` PS → restart logoscore → `load-module` |
| Each test session | Start LEZ localnet if stopped; point wallet at `http://127.0.0.1:3040` |
| When wallet Qt/FFI changes (Step 11+) | Re-bundle wallet `.lgx`, `lgpm install` wallet, then PS reinstall if needed |

---

## Variables (every terminal)

```bash
export REPO="$HOME/Downloads/software/waku/lez-related/lez-payment-streams"
export MODULES="$HOME/Downloads/software/waku/lez-related/logos-cli/modules"

# Scaffold workspace — not inside REPO (see step1 findings doc)
export SCAFFOLD_WS="$HOME/Downloads/software/waku/lez-related/logos-scaffold-workspace"
```

Adjust paths to match your checkout.

Tooling shell (run in each tab that needs `lgpm`, `logoscore`, or `lm`):

```bash
nix shell \
  github:logos-co/logos-package-manager#cli \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-module#lm
```

Check:

```bash
command -v lgpm logoscore lm
```

---

## One-time — LEZ scaffold and deploy (Step 7+)

Commands, program deploy, and account formats:
[`step1-findings-scaffold-rpc.md`](step1-findings-scaffold-rpc.md).
Record `program_id` and test account ids from that doc before module chain-read tests.

Minimal session check:

```bash
cd "$SCAFFOLD_WS"
lgs localnet status
lgs wallet -- check-health
```

Expected: localnet running when testing Steps 10+ (start via step1 if stopped).

---

## Each test session — LEZ before logoscore (Steps 10–13)

If localnet was stopped:

```bash
cd "$SCAFFOLD_WS"
lgs localnet start
lgs localnet status
lgs wallet -- check-health
```

Expected: localnet running, wallet healthy.

Ensure `lez_wallet_module` inside logoscore can reach the sequencer
(JSON-RPC URL and wallet storage — configure via wallet `open` / `create_new`
through `logoscore call` once you have a stable setup; details in step1 findings
and Step 7 implementor notes in the integration plan).

---

## Repeat loop — after PS or Rust FFI edits (Steps 10–13)

### 1. Edit code

Typical touch points:

- `logos-payment-streams-module/src/*.cpp` — new `Q_INVOKABLE` methods, `invokeRemoteMethod`
- `lez-payment-streams-ffi/` — decoders, instruction builders, proof bytes
- `lez-payment-streams-core/` — domain logic consumed by FFI

Step 11 wallet changes happen in `logos-execution-zone-module` (patched flake), not only here.

### 2. Build payment streams `.lgx`

```bash
cd "$REPO"
nix build ./logos-payment-streams-module#lgx
```

Check:

```bash
ls -l "$REPO/result"/*.lgx
```

Expected: fresh store path (mtime changes when inputs changed).

Optional Rust-only sanity before the module build:

```bash
cd "$REPO"
cargo test -p lez-payment-streams-ffi -p lez-payment-streams-core
```

### 3. Refresh install artifact path

```bash
export PS_LGX=$(readlink -f "$REPO/result"/*.lgx)
echo "$PS_LGX"
test -f "$PS_LGX" && echo "PS_LGX OK"
```

### 4. Reinstall into `MODULES`

In the tooling `nix shell`:

```bash
lgpm --modules-dir "$MODULES" install --file "$PS_LGX"
```

Check:

```bash
lgpm --modules-dir "$MODULES" list
```

Expected: `payment_streams_module` row still present (version may unchanged;
content on disk is replaced).

### 5. Restart logoscore and reload modules

Stop any previous daemon (client tab or same tab):

```bash
logoscore stop
```

Tab A — daemon:

```bash
logoscore -D -m "$MODULES" -l lez_wallet_module,payment_streams_module -v
```

Tab B — client (fresh `nix shell` + exports):

```bash
logoscore status
logoscore list-modules --loaded
```

Expected: 3 loaded, 0 crashed (`capability_module`, `lez_wallet_module`,
`payment_streams_module`).

If you did not use `-l` on startup:

```bash
logoscore load-module lez_wallet_module
logoscore load-module payment_streams_module
```

### 6. Confirm new surface (offline)

After adding methods, plugin metadata should reflect them:

```bash
lm methods "$MODULES/payment_streams_module/payment_streams_module_plugin.so"
```

Compare to Step 9 baseline (no business methods yet) — Step 10+ should list your new
exported method entries.

```bash
logoscore module-info payment_streams_module
```

Expected: `Status: loaded`, methods section includes new names.

### 7. Exercise behavior (Step-specific)

Use `logoscore call payment_streams_module <method> …` with arguments your Step
defines. Exact calls depend on implemented signatures; patterns:

Step 10 — chain reads

- Calls into PS helpers that wrap `get_account_public` / clock read
- Expected: JSON strings decodable by FFI tests; failures should be structured
  errors, not daemon crash
- Prerequisite: LEZ up, program deployed, accounts initialized or PDAs read as
  documented in step1 findings

Step 11 — writes and status

- Same LEZ stack as Step 10
- `lm methods` shows write + `getVaultStatus` / `getStreamStatus` when implemented
- Expected: lifecycle calls return wallet JSON; on-chain state visible via status
  helpers (integration plan Step 11 definition of done)

Step 12 — eligibility (user side)

- Methods such as `prepareEligibilityForStoreQuery`, `registerProviderMapping`,
  `listMyStreams`, `rediscoverStreams`
- Some DoD items are FFI verifier round-trips (may not need live chain for every
  test); chain sanity still uses Step 10 stack
- Restart logoscore and repeat calls to test `instancePersistencePath` survival

Step 13 — provider verify

- `verifyEligibilityForStoreQuery` via `logoscore call`
- Structural failure cases should not require chain; happy path needs seeded
  vault/stream state on localnet

Check after each call:

```bash
logoscore status
```

Expected: `0 crashed`; PS still `loaded`.

### 8. When Rust changed but C++ did not

If you only changed `lez-payment-streams-ffi` or core, you still run steps 2–5
(the module `.lgx` bundles the rebuilt `.so`). Skipping reinstall leaves an old
FFI inside the installed tree.

---

## Repeat loop — wallet module changes (Step 11+)

When pins move (491 / 19) or `sign_public_payload` lands on the patched wrapper:

```bash
cd "$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched"
nix bundle --bundler github:logos-co/nix-bundle-lgx .#lib -o ./wallet-lgx-out -L
```

```bash
export WALLET_LGX=$(readlink -f \
  "$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out"/*.lgx)

lgpm --modules-dir "$MODULES" install --file "$WALLET_LGX"
```

Check:

```bash
lm methods "$MODULES/lez_wallet_module/lez_wallet_module_plugin.so" | rg 'send_generic|sign_public'
```

Then rebuild/reinstall PS if CMake/codegen depended on wallet API headers, and
repeat [PS restart loop](#repeat-loop--after-ps-or-rust-ffi-edits-steps-10-13) from step 2.

---

## Steps 14–15 (delivery only)

| Step | Where to work | How to test |
|------|----------------|-------------|
| 14 | `logos-delivery` Store codec | Nim unit / round-trip tests in that repo; no logoscore |
| 15 | `liblogosdelivery` C ABI | C smoke test in delivery repo; no logoscore |

Revisit this document when Step 16 mounts `delivery_module` beside wallet + PS.

---

## Shutdown

```bash
logoscore stop
```

Optional when done for the day:

```bash
cd "$SCAFFOLD_WS"
lgs localnet stop
lgs localnet status
```

---

