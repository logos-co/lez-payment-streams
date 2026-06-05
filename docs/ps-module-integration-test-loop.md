# Payment streams module — dev test loop (Steps 7–12)

Repeatable sequence after editing `payment_streams_module` and/or
`lez-payment-streams-ffi` / `lez-payment-streams-core`, through Step 12.

Assumes the directory layout and tooling from
[`logos-operator-install-basics.md`](logos-operator-install-basics.md)
(wallet + payment streams only, no `delivery_module` yet).

Steps 11–12 change `logos-delivery` / `liblogosdelivery` (Nim, C ABI smoke tests).
They do not use this Logos host loop; see [Steps 11–12](#steps-11-12-delivery-only) at the end.

---

## What repeats vs one-time

| Frequency | Work |
|-----------|------|
| One-time per machine | Pick `SCAFFOLD_WS`, run `lgs init` / `lgs setup`, deploy `lez_payment_streams`, note program id and test account ids ([`step1-findings-scaffold-rpc.md`](step1-findings-scaffold-rpc.md)) |
| One-time per machine | First `lgpm` install of wallet + PS (operator guide) |
| Each new terminal | `export` paths + `nix shell` (tools) |
| Each dev iteration | PS `nix build` → `lgpm install` PS → restart logoscore → `load-module` |
| Each test session | Start LEZ localnet if stopped; point wallet at `http://127.0.0.1:3040` |
| When wallet Qt/FFI changes (Step 8a+) | Re-bundle wallet `.lgx`, `lgpm install` wallet, then PS reinstall if needed |

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

Run from `SCAFFOLD_WS` with `lgs` on `PATH` (scaffold dev shell or install per
[`step1-findings-scaffold-rpc.md`](step1-findings-scaffold-rpc.md)).

```bash
cd "$SCAFFOLD_WS"
lgs init
lgs setup
lgs localnet start
lgs localnet status
lgs wallet -- check-health
```

Check:

- `localnet status` reports running
- Sequencer RPC at `http://127.0.0.1:3040` (per step1 findings)

Deploy the payment streams program from the scaffold project wired to this repo
(exact subcommand depends on your scaffold pin; integration plan Step 7 expects
`lgs deploy` after localnet is up). Record `program_id` and any seed PDAs you use in tests.

Optional account setup for public reads (step1 findings):

```bash
lgs wallet topup --address Public/<base58-id>
```

Keep wallet CLI account refs as `Public/...` or `Private/...`;
JSON-RPC `getAccount` uses raw base58 only.

---

## Each test session — LEZ before logoscore (Steps 7–10)

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

## Repeat loop — after PS or Rust FFI edits (Steps 7–10)

### 1. Edit code

Typical touch points:

- `logos-payment-streams-module/src/*.cpp` — new `Q_INVOKABLE` methods, `invokeRemoteMethod`
- `lez-payment-streams-ffi/` — decoders, instruction builders, proof bytes
- `lez-payment-streams-core/` — domain logic consumed by FFI

Step 8a wallet changes happen in `logos-execution-zone-module` (patched flake), not only here.

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

Compare to Step 6c baseline (`initLogos` only) — Step 7+ should list your new
`Q_INVOKABLE` entries.

```bash
logoscore module-info payment_streams_module
```

Expected: `Status: loaded`, methods section includes new names.

### 7. Exercise behavior (Step-specific)

Use `logoscore call payment_streams_module <method> …` with arguments your Step
defines. Exact calls depend on implemented signatures; patterns:

**Step 7 — chain reads**

- Calls into PS helpers that wrap `get_account_public` / clock read
- Expected: JSON strings decodable by FFI tests; failures should be structured
  errors, not daemon crash
- Prerequisite: LEZ up, program deployed, accounts initialized or PDAs read as
  documented in step1 findings

**Step 8b — writes and status**

- Same LEZ stack as Step 7
- `lm methods` shows write + `getVaultStatus` / `getStreamStatus` when implemented
- Expected: lifecycle calls return wallet JSON; on-chain state visible via status
  helpers (integration plan Step 8 definition of done)

**Step 9 — eligibility (user side)**

- Methods such as `prepareEligibilityForStoreQuery`, `registerProviderMapping`,
  `listMyStreams`, `rediscoverStreams`
- Some DoD items are FFI verifier round-trips (may not need live chain for every
  test); chain sanity still uses Step 7 stack
- Restart logoscore and repeat calls to test `instancePersistencePath` survival

**Step 10 — provider verify**

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

## Repeat loop — wallet module changes (Step 8a+)

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
repeat [PS restart loop](#repeat-loop--after-ps-or-rust-ffi-edits-steps-710) from step 2.

---

## Steps 11–12 (delivery only)

| Step | Where to work | How to test |
|------|----------------|-------------|
| 11 | `logos-delivery` Store codec | Nim unit / round-trip tests in that repo; no logoscore |
| 12 | `liblogosdelivery` C ABI | C smoke test in delivery repo; no logoscore |

Revisit this document when Step 13 mounts `delivery_module` beside wallet + PS.

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

## Cross-references

- Install and first load — [`logos-operator-install-basics.md`](logos-operator-install-basics.md)
- Step definitions — [`integration-plan-v2.md`](../integration-plan-v2.md) Steps 7–12
- Scaffold RPC and account formats — [`step1-findings-scaffold-rpc.md`](step1-findings-scaffold-rpc.md)
- Cross-module call pattern — [`step6c-implementation-guidance.md`](step6c-implementation-guidance.md)
