# Basecamp rebuild loop

Portable Basecamp load loop for this repository plugins.
Commands assume the current directory is the `lez-payment-streams` root.
Use Nix-built portable Basecamp (`Logos.Theme`) and an isolated user directory
(`.scaffold/basecamp-ui`).
The Nix-built binary loads local `.lgx` packages directly.

Install custom plugins through Package Manager or Modules → Install LGX Package
([Install and load a module](https://docs.logos.co/basecamp/install-and-load-a-module-in-logos-basecamp)).
The installed `.lgx` files remain in the user directory across restarts.
Relaunch and load without reinstalling unless the package was rebuilt.
Rebuild, reinstall that `.lgx`, restart, and load when modifying source files.

Mock mode runs without network or wallet environment variables.
Live chain writes read wallet and fixture paths from the environment inherited
by the child `logos_host` processes
([step-21 Host process](step-21-basecamp-ui.md#host-process-and-compatibility)).

```bash
USER_DIR="$PWD/.scaffold/basecamp-ui"
BASECAMP=../logos-basecamp/result/bin/LogosBasecamp
```

`nix build` writes `result/` in the current working directory.

Packages:

| Runtime id | Source | Build (from repo root) | `.lgx` |
| --- | --- | --- | --- |
| `logos_execution_zone` | `module/nix/flakes/logos-execution-zone-module-patched/` | `./verify/lib/build-wallet-lgx.sh` | `module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/logos-lez_core-module-lib-0.4.0-with-sdk-api-headers.lgx` |
| `payment_streams_module` | `module/` | `nix build ./module#lgx-portable` | `result/logos-payment_streams_module-module-lib.lgx` |
| `payment_streams_ui` | `ui/` | `nix build ./ui#lgx-portable -o ui/result` | `ui/result/logos-payment_streams_ui-module.lgx` |

`nix build ./module#lgx-portable` outputs the core `.lgx` to `./result/`.
`payment_streams_ui` declares dependencies on `payment_streams_module` and
`logos_execution_zone` in `ui/metadata.json`.
Basecamp requires both dependencies to be installed before mounting the UI.

## Initial setup

Perform these steps before the first load, or when the Basecamp binary,
user directory, or packages are missing.

1. Build the Basecamp binary in sibling repository:

```bash
( cd ../logos-basecamp && nix build '.#bin-bundle-dir' )
```

2. Create the isolated user directory:

```bash
mkdir -p "$USER_DIR"
```

3. Build all three `.lgx` packages:

```bash
./verify/lib/build-wallet-lgx.sh
nix build ./module#lgx-portable
nix build ./ui#lgx-portable -o ui/result
```

4. Start Basecamp:

```bash
"$BASECAMP" --user-dir "$USER_DIR"
```

5. In Modules, select Install LGX Package in this order:

   1. Patched wallet `.lgx` (`logos_execution_zone`)
   2. Payment streams core `.lgx` (`payment_streams_module`)
   3. UI `.lgx` (`payment_streams_ui`)

6. Load modules in that same order.

## Loop after a change

When editing `ui/`, `module/`, or the patched wallet tree:

1. Stop Basecamp and running hosts:

```bash
pkill -9 -f 'logos_host|LogosBasecamp'
```

2. Rebuild the changed package:

```bash
# ui/ (Main.qml, metadata, icons)
nix build ./ui#lgx-portable -o ui/result

# module/ (payment_streams_module C++ and FFI)
nix build ./module#lgx-portable

# patched logos_execution_zone
./verify/lib/build-wallet-lgx.sh
```

3. Start Basecamp with the chosen network environment
   (see Testnet launch or Localnet launch below):

```bash
"$BASECAMP" --user-dir "$USER_DIR"
```

4. Install LGX Package to overwrite the rebuilt `.lgx`.

5. Load `logos_execution_zone`, then `payment_streams_module`, then
   `payment_streams_ui`.

## Live writes

Skip this section when evaluating visual layout in Mock mode.

`logos_host` subprocesses inherit environment variables from the shell
that launches Basecamp.
The active network is determined by `FIXTURE_MANIFEST` and `WALLET_HOME`.

### Testnet launch

Public Testnet v0.2 is the primary live network (`https://testnet.lez.logos.co/`).
Use `verify/fixtures/testnet-module.json` and the testnet wallet directory
populated during the reproduce walkthrough
([reproduce/module.md](../../reproduce/module.md)).

```bash
export USER_DIR="$PWD/.scaffold/basecamp-ui"
export WALLET_HOME="$PWD/.scaffold/e2e/testnet-wallet"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
export NSSA_WALLET_HOME_DIR="$WALLET_HOME"
export FIXTURE_MANIFEST="$PWD/verify/fixtures/testnet-module.json"
export REPO="$PWD"
BASECAMP=../logos-basecamp/result/bin/LogosBasecamp

mkdir -p "$USER_DIR"
"$BASECAMP" --user-dir "$USER_DIR"
```

When using default CLI home `~/.lee/wallet` configured for testnet,
set `WALLET_HOME="$HOME/.lee/wallet"`.

### Localnet launch

Localnet runs against a local sequencer instance (`http://127.0.0.1:3040`).
Ensure the local sequencer is running before submitting live transactions.

```bash
export USER_DIR="$PWD/.scaffold/basecamp-ui"
export WALLET_HOME="$PWD/.scaffold/wallet"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
export NSSA_WALLET_HOME_DIR="$WALLET_HOME"
export FIXTURE_MANIFEST="$PWD/verify/fixtures/localnet.json"
export PAYMENT_STREAMS_GUEST_BIN="$PWD/program/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
export REPO="$PWD"
BASECAMP=../logos-basecamp/result/bin/LogosBasecamp

mkdir -p "$USER_DIR"
"$BASECAMP" --user-dir "$USER_DIR"
```

When using module E2E local state,
set `WALLET_HOME="$PWD/.scaffold/e2e/user/wallet-local"`.

### Wallet readiness

Before submitting live writes on either network:

1. Ensure the wallet contains two funded public accounts.
2. Ensure authenticated transfer is registered for both accounts.
   On testnet, run `./verify/repro-auth-transfer.sh` or complete Step 8 of
   [reproduce/module.md](../../reproduce/module.md).
   On localnet, run the local seeding helper.
3. Stop any background `logoscore` process so `storage.json` is accessible to
   Basecamp.
