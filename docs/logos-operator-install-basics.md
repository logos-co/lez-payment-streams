# Logos operator guide — wallet and payment streams

End-to-end flow for this repo only: build two `.lgx` packages
(`lez_wallet_module`, `payment_streams_module`), install them with `lgpm`,
and load them in `logoscore`.

Delivery and other modules are out of scope here.

Related docs:

- Pin and wrapper flake rationale — [`feature-branch-pins.md`](feature-branch-pins.md)
- Step 6c module design — [`step6c-implementation-guidance.md`](step6c-implementation-guidance.md)
- Repeat build/install/load for Steps 7–12 — [`ps-module-integration-test-loop.md`](ps-module-integration-test-loop.md)
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

Dependency order: wallet first, then payment streams (`metadata.json` lists
`lez_wallet_module` as a dependency).

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

## Phase 5 — Verification (Step 6c expectations)

### Offline plugin inspection

```bash
lm methods "$MODULES/payment_streams_module/payment_streams_module_plugin.so"
```

Expected: minimal shell — notably `initLogos`, `name`, and any `PluginInterface` symbols;
no payment-streams business API yet.

```bash
lm methods "$MODULES/lez_wallet_module/lez_wallet_module_plugin.so" | rg list_accounts
```

Expected: a `list_accounts` (or similarly named) invokable method on the wallet plugin.

### Runtime module info

```bash
logoscore module-info payment_streams_module
```

Expected: `Status: loaded`, methods list centered on `initLogos()`.

```bash
logoscore call lez_wallet_module list_accounts
```

Expected without LEZ / wallet JSON-RPC: RPC or call failure is normal for Step 6c.
The host must stay running and `payment_streams_module` must have loaded without crashing
(wallet probe inside `initLogos` uses the same kind of call).

### Teardown

```bash
logoscore stop
```

Expected: daemon stops; a later `logoscore status` reports not running.

---

## Rebuild loop

After code or Nix changes:

1. Rebuild the affected `.lgx` (Phase 1a and/or 1b).
2. `export WALLET_LGX` / `export PS_LGX` again.
3. `lgpm --modules-dir "$MODULES" install --file …` for each changed package.
4. `logoscore stop`, then Phase 4 from a fresh daemon start.

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

Cross-reference: [`integration-plan-v2.md`](../integration-plan-v2.md) Steps 6b and 6c definition of done.
