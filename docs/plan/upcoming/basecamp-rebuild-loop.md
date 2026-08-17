Portable Basecamp load loop for this repo’s plugins.
Commands assume the current directory is the `lez-payment-streams` root.
Use Nix-built portable Basecamp (`Logos.Theme`) and an isolated user dir
(`.scaffold/basecamp-ui`).
Public Basecamp releases have historically ignored user-installed
modules; the Nix-built binary is the one that loads a local `.lgx`.

Installing a custom plugin is Package Manager (catalogue) or
Modules → Install LGX Package
([Install and load a module](https://docs.logos.co/basecamp/install-and-load-a-module-in-logos-basecamp)).
The `.lgx` stays in the user dir across restarts.
Relaunch and Load; do not reinstall unless the package was rebuilt.
Basecamp does not hot-reload, so a source change means rebuild,
reinstall that `.lgx`, restart, Load.

Exporting wallet and fixture paths is outside install.
Those vars exist because live `chainAction` still reads process env
the same way logoscore does
([step-21 Host process](step-21-basecamp-ui.md#host-process-before-live-writes)).
Demo-mode layout needs none of them.
A launch wrapper that execs Basecamp is the less-fiddly form of the
same live constraint until a wallet UI calls `open` and the module
carries its program id without `FIXTURE_MANIFEST`.

```bash
USER_DIR="$PWD/.scaffold/basecamp-ui"
BASECAMP=../logos-basecamp/result/bin/LogosBasecamp
```

`nix build` writes `result/` in the current working directory.

Packages:

| Runtime id | Source | Build (from repo root) | `.lgx` |
| --- | --- | --- | --- |
| `logos_execution_zone` | `module/nix/flakes/logos-execution-zone-module-patched/` | `./verify/lib/build-wallet-lgx.sh` | `module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/logos-lez_core-module-lib-0.4.0-with-sdk-api-headers.lgx` (filename still says `lez_core`) |
| `payment_streams_module` | `module/` | `nix build ./module#lgx-portable` | `result/logos-payment_streams_module-module-lib.lgx` |
| `payment_streams_ui` | `ui/` | `nix build ./ui#lgx-portable -o ui/result` | `ui/result/logos-payment_streams_ui-module.lgx` |

`nix build ./module#lgx-portable` puts the core `.lgx` in `./result/`.
`module/result` is unrelated unless you built inside `module/` or passed `-o`.
The catalog `logos_execution_zone` lacks the payment-streams wallet patches.
`payment_streams_ui` lists the other two in `ui/metadata.json` `dependencies`.
Basecamp refuses to mount the UI until both are installed.

## Once

Do this before the first Load, and again only if the Basecamp binary,
user dir, or first-install packages are missing.

1. Build the Basecamp binary (sibling checkout):

```bash
( cd ../logos-basecamp && nix build '.#bin-bundle-dir' )
```

2. Create the isolated user dir:

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

5. In Modules, Install LGX Package, in this order:

   1. Patched wallet `.lgx` (runtime id `logos_execution_zone`)
   2. Payment streams core `.lgx`
   3. UI `.lgx`

6. Load in that same order.
   For live writes later, finish the logoscore walkthrough through AT
   ([reproduce/module.md](../../reproduce/module.md) through Step 8,
   or module E2E), then `close` and `logoscore stop` so `storage.json`
   is free. See [Live writes](#live-writes).

## Loop after a change

Every time `ui/`, `module/` (C++ or FFI), or the patched wallet tree
changes:

1. Stop Basecamp and leftover hosts:

```bash
pkill -9 -f 'logos_host|LogosBasecamp'
```

2. Rebuild only the package whose source changed:

```bash
# ui/ (Main.qml, metadata, icons)
nix build ./ui#lgx-portable -o ui/result

# module/ (payment_streams_module C++ / FFI)
nix build ./module#lgx-portable

# patched logos_execution_zone
./verify/lib/build-wallet-lgx.sh
```

3. Start Basecamp with the same `--user-dir`
   (add [Live writes](#live-writes) env in this shell when you need
   chain I/O):

```bash
"$BASECAMP" --user-dir "$USER_DIR"
```

4. Install LGX Package again for only the rebuilt `.lgx` (overwrite).

5. Load `logos_execution_zone`, then `payment_streams_module`, then
   `payment_streams_ui`.
   A `pkill` drops every `logos_host`, so Load all three even after a
   UI-only rebuild.

## Live writes

Skip this section for Demo-mode layout work.

`logos_host` inherits the `LogosBasecamp` environment at spawn.
Basecamp has no settings panel for wallet home or fixture path.
Pointing this instance at the walkthrough wallet therefore means
those exports are present in the shell that starts Basecamp.

Default LEZ CLI home is `~/.lee/wallet` when `LEE_WALLET_HOME_DIR` is
unset. The block below pins the scaffold home instead.

From the repo root:

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

Module-E2E localnet home is `.scaffold/e2e/user/wallet-local`.
Testnet home and fixture are Step 1 of
[reproduce/module.md](../../reproduce/module.md)
(`.scaffold/e2e/testnet-wallet`, `verify/fixtures/testnet-module.json`).

After Load of `logos_execution_zone`, `open` with
`$WALLET_HOME/wallet_config.json` and `$WALLET_HOME/storage.json`
from a Logos client in this Basecamp instance.
`payment_streams_ui` never calls `open` (D21.8).
AT for both public accounts stays on chain from the once-only
walkthrough Step 8 (or module E2E) against that same `WALLET_HOME`.
