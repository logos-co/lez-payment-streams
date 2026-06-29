# Step 10b — wallet runtime artifact (LEZ 510 + PR 19)

Installable `logos_execution_zone` `.lgx` for `logoscore`, aligned with the Step 10a
local chain fixture (sequencer `http://127.0.0.1:3040`, wallet under `.scaffold/wallet`).

LEZ pin bump (510 merge): [`step11d-wallet-510.md`](step11d-wallet-510.md).
Pins and flake layout: [`feature-branch-pins.md`](feature-branch-pins.md).
Build/install overview: [`logos-runtime-guide.md`](logos-runtime-guide.md) Phase 1b–5.
Step 10a prerequisite: [`step10a-local-chain-fixture.md`](step10a-local-chain-fixture.md).

`sign_public_payload` is Step 11c, not 10b.

## Prerequisites

- Step 10a green: `./scripts/archive/verify-step10a-dod.sh` exits 0 and `fixtures/localnet.json` exists.
- Nix (wallet bundle can take several minutes on first build).
- Shared module install dir (`MODULES`) — same path for `lgpm` and `logoscore -m` (see runtime guide).
- Tooling via `nix shell` (`lgpm`, `logoscore`, `lm`).

## Build patched wallet `.lgx`

From repo root:

```bash
chmod +x scripts/archive/build-wallet-lgx.sh
./scripts/archive/build-wallet-lgx.sh
```

Equivalent manual command:

```bash
cd logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched
nix bundle --bundler github:logos-co/nix-bundle-lgx .#lib -o ./wallet-lgx-out -L
```

Export for install:

```bash
export REPO="$PWD"
export WALLET_LGX=$(readlink -f \
  "$REPO/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out"/*.lgx)
```

## Install into MODULES

Load order is wallet first, then `payment_streams_module` (D6).

```bash
export MODULES="$HOME/Downloads/software/waku/lez-related/logos-cli/modules"
export PS_LGX=$(readlink -f "$REPO/result"/*.lgx)

nix shell \
  github:logos-co/logos-package-manager#cli \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-module#lm \
  --command bash

mkdir -p "$MODULES"
lgpm --modules-dir "$MODULES" install --file "$WALLET_LGX"
lgpm --modules-dir "$MODULES" install --file "$PS_LGX"
lgpm --modules-dir "$MODULES" list
```

Expected: `logos_execution_zone` and `payment_streams_module` under `$MODULES`, metadata name
`logos_execution_zone`, plugin `logos_execution_zone_plugin.so`.

If `$MODULES/lez_wallet_module` remains from an older install, remove it and reinstall the
patched wallet `.lgx` so the install dir matches upstream module id `logos_execution_zone`.

## PR 19 method surface (offline check)

```bash
lm methods "$MODULES/logos_execution_zone/logos_execution_zone_plugin.so" \
  | rg 'send_generic_public_transaction|get_account_public|^int open'
```

Expected invokables include:

- `send_generic_public_transaction` — generic public instruction submit (LEZ `wallet_ffi`)
- `send_generic_public_transaction_json` — JSON IPC entry used by `payment_streams_module` (11b; wrapper patch)
- `send_generic_private_transaction`, `send_program_deployment_transaction` (510 FFI via PR 19 Qt)
- `authenticated_transfer_elf` (and related ELF helpers)
- `get_account_public`, `account_id_from_base58`, `open`, `sync_to_block`

Patched 11b builds also load guest ELF from `PAYMENT_STREAMS_GUEST_BIN` inside
`send_generic_public_transaction` when program bytes are empty. Verify with
`rg -F PAYMENT_STREAMS_GUEST_BIN` on `logos_execution_zone_plugin.so`.

If `nix bundle … .#lib` fails on `wallet-ffi-deps` / `pol` download, see
[`step11b-chain-writes.md`](step11b-chain-writes.md) for the Qt-aligned manual build fallback.

## Wallet `open` for Step 10a

Use the same wallet home as seed/deploy (`scaffold.toml` `[wallet] home_dir`):

```bash
export WALLET_CONFIG="$REPO/.scaffold/wallet/wallet_config.json"
export WALLET_STORAGE="$REPO/.scaffold/wallet/storage.json"
```

`wallet_config.json` must point at the Step 10a sequencer:

- `sequencer_addr`: `http://127.0.0.1:3040`

That file is created by `lgs init` / `lgs setup` in this repo. Encrypted `storage.json` uses the
pinned LEZ wallet format; if seed fails to load storage, run [`scripts/archive/reinit-scaffold-wallet.sh`](../scripts/archive/reinit-scaffold-wallet.sh)
and re-seed (see Step 10a runbook).

Storage password for CLI `wallet` matches `SCAFFOLD_WALLET_SETUP_PASSWORD` (default
`scaffold-local-dev` in `reinit-scaffold-wallet.sh`). Module `open` uses the paths only; the
FFI loads storage with the same dev defaults as the pinned LEZ wallet CLI when applicable.

## logoscore load order and RPC smoke test

Tab A — daemon:

```bash
logoscore -D -m "$MODULES" -v
```

Tab B — client (same `nix shell` + exports):

```bash
logoscore load-module logos_execution_zone
logoscore load-module payment_streams_module
logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"
```

Expected: `"status":"ok"` on `open` (return value `0` when the wallet handle is ready).

Read a Step 10a on-chain PDA via the module (manifest field `vault_config_account_id` is base58;
`get_account_public` expects hex account id):

```bash
VC=$(python3 -c "import json; print(json.load(open('fixtures/localnet.json'))['vault_config_account_id'])")
HEX=$(logoscore call logos_execution_zone account_id_from_base58 "$VC" | python3 -c \
  "import sys,json; print(json.load(sys.stdin)['result'])")
logoscore call logos_execution_zone get_account_public "$HEX"
```

Expected: `"status":"ok"` and a `result` string containing JSON with non-empty `data` and
`program_owner` for the initialized vault config PDA.

Teardown:

```bash
logoscore stop
```

## Definition of done (automated)

```bash
./scripts/archive/verify-step10b-dod.sh
```

Checks:

1. Patched `.lgx` built (or `WALLET_LGX` set).
2. `logos_execution_zone` installed with correct metadata and plugin name.
3. `wallet_config` sequencer URL matches Step 10a.
4. `lm methods` includes `send_generic_public_transaction`.
5. With localnet up and fixture manifest: load order, `open`, `get_account_public` on fixture PDA.

Skip runtime RPC only:

```bash
VERIFY_LOGOSCORE=0 ./scripts/archive/verify-step10b-dod.sh
```

## Troubleshooting

| Symptom | Action |
| --- | --- |
| `lm` shows `send_public_transaction` only | Reinstall from `./scripts/archive/build-wallet-lgx.sh` output |
| `open` fails / return non-zero | Confirm `storage.json` is 491 encrypted layout; reinit wallet + re-seed 10a |
| `get_account_public` empty `data` | Re-run Step 10a seed; PDA not initialized on chain |
| Replica timeout on wallet call | Load `logos_execution_zone` before other modules; restart daemon |
| Wrong chain | Foreign process on `:3040` — see Step 10a troubleshooting |

## Next step

Step 11a — wire `payment_streams_module` read helpers to `get_account_public` and
`account_id_from_base58` (integration plan Step 11).
