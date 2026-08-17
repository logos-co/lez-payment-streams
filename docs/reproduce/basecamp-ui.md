# Reproduce payment streams in Basecamp

Protocol UI for LIP-155 through `payment_streams_ui` on one host.
The CLI teaching path remains [module.md](module.md).
This walkthrough uses the same wallets, fixtures, and lifecycle:
initialize vault, deposit, create stream, wait for accrual, close, claim.

`chainAction` catalogue: [payment-streams-module README](../../module/README.md#chainaction-catalogue).

## What you run

Nix-built Basecamp with three packages loaded in order:
`logos_execution_zone`, `payment_streams_module`, `payment_streams_ui`.
Owner and provider are two public accounts in one wallet.

## Prerequisites

Complete wallet setup from [module.md](module.md) (testnet primary):
accounts funded, authenticated transfer registered, logoscore stopped
so `storage.json` is free
([N52](../reference/decisions.md#n52-exclusive-wallet-cli-stop-window-2026-08-14)).

Sibling checkout `../logos-basecamp` for the Nix-built binary
(`nix build '.#bin-bundle-dir'`).
Host: Git, Nix (flakes).

## Launch

From the repository root:

```bash
make basecamp-ui-build
make basecamp-ui-run
```

Testnet is the default (`verify/fixtures/testnet-module.json`,
`.scaffold/e2e/testnet-wallet`).

Localnet (sequencer already running):

```bash
make basecamp-ui-run NETWORK=localnet
```

`NETWORK` is a Make variable.
Make exports `WALLET_HOME`, `LEE_WALLET_HOME_DIR`, `NSSA_WALLET_HOME_DIR`,
`FIXTURE_MANIFEST`, `REPO`, and `USER_DIR`, then starts Basecamp with
`--user-dir .scaffold/basecamp-ui`.
The three `.lgx` packages are the same on both networks.
Switching networks is a new Basecamp process with the other env.

Override `WALLET_HOME` or `FIXTURE_MANIFEST` when the wallet lives
somewhere else (for example `WALLET_HOME=$HOME/.lee/wallet` on testnet,
or `WALLET_HOME=$PWD/.scaffold/e2e/user/wallet-local` for module E2E
local state).

## First load

1. In Modules, Install LGX Package in this order:
   patched wallet (`logos_execution_zone`),
   `payment_streams_module`,
   `payment_streams_ui`.
2. Load the same order.
3. Open `payment_streams_ui`.
4. Turn Demo mode off.

The header shows the sequencer host and block height when the wallet
opens.
Session prefills `owner` and `provider` from public accounts.
A banner explains missing `WALLET_HOME`, missing wallet files, or an
empty account list.

Package paths after `make basecamp-ui-build`:

- Wallet: `module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/*.lgx`
- Module: `result/logos-payment_streams_module-module-lib.lgx`
- UI: `ui/result/logos-payment_streams_ui-module.lgx`

Installed packages stay in `.scaffold/basecamp-ui` across restarts.
Rebuild and reinstall only when source changes.

## Lifecycle

Defaults match [module.md](module.md): deposit 500, allocation 80, rate 1,
`vault_id` 0, `stream_id` 0.

1. Initialize vault.
2. Deposit.
3. Create stream.
4. Refresh until Accrued is greater than 0 (stream stays Active;
   close stays enabled at zero accrued).
5. Owner close (or provider close).
6. Claim.

Each write shows Confirming until `sync_to_block` plus status agree,
or 120 seconds elapse.
Recent transactions lists short hashes for this session.

## After a QML or module change

Stop Basecamp and `logos_host`, rebuild the changed `.lgx`
(`nix build ./ui#lgx-portable -o ui/result`,
`nix build ./module#lgx-portable`, or `./verify/lib/build-wallet-lgx.sh`),
relaunch with `make basecamp-ui-run`, reinstall that package, load again.
