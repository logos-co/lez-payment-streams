# Reproduce payment streams in Basecamp

Same LIP-155 lifecycle as [module.md](module.md), on one host, through
`payment_streams_ui`.
Initialize vault, deposit, create stream, wait for accrual, close, claim.

Commands are from the repository root.
Plan: [step-21-basecamp-ui.md](../plan/upcoming/step-21-basecamp-ui.md).
`chainAction` catalogue: [module README](../../module/README.md#chainaction-catalogue).

## Background

Nix-built portable Basecamp, user directory `.scaffold/basecamp-ui`, three
packages: `logos_execution_zone` (patched wallet), `payment_streams_module`,
`payment_streams_ui`.
The same `.lgx` files serve both networks.

Owner and provider are two public accounts in one wallet.
Session prefills them in that order.

`make basecamp-ui-prepare` does not create a wallet or accounts.
It registers authenticated transfer and pinata-funds the first two public
accounts already in `WALLET_HOME`.
`make basecamp-ui-run` exports that home, `FIXTURE_MANIFEST`, and
`PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` (live sequencer AT ImageID)
into child `logos_host` processes.
Deposit chains authenticated transfer using that id.
Claim and withdraw credit the destination in the payment-streams guest.
The FFI graph AT differs from the program that owns testnet accounts.

Basecamp needs exclusive access to `storage.json`
([N52](../reference/decisions.md#n52-exclusive-wallet-cli-stop-window-2026-08-14)).
`prepare` stops logoscore when it holds the wallet; leave it stopped.

Host: Git, Nix (flakes), sibling checkout `../logos-basecamp`.
Localnet also needs `lgs` on `PATH`.

## Build packages

```bash
make basecamp-ui-build
```

| Runtime id | `.lgx` after build |
| --- | --- |
| `logos_execution_zone` | `module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/*.lgx` |
| `payment_streams_module` | `result/logos-payment_streams_module-module-lib.lgx` |
| `payment_streams_ui` | `ui/result/logos-payment_streams_ui-module.lgx` |

## Localnet

1. Create the local wallet, two public accounts, and
   `verify/fixtures/localnet.json`, and start the sequencer:

```bash
make seed-fixture
```

2. Fund those accounts:

```bash
make basecamp-ui-prepare NETWORK=localnet
```

3. Launch Basecamp:

```bash
make basecamp-ui-run NETWORK=localnet
```

4. Continue at [In Basecamp](#in-basecamp).

## Testnet

Wallet home `.scaffold/e2e/testnet-wallet`; fixture
`verify/fixtures/testnet-module.json`.
Sequencer is the public endpoint in that wallet config
(`https://testnet.lez.logos.co/`).
Make defaults `NETWORK` to testnet.

1. Create the wallet and two public accounts:
   [module.md](module.md) Steps 4-7.

2. Fund those accounts:

```bash
make basecamp-ui-prepare
```

3. Launch Basecamp:

```bash
make basecamp-ui-run
```

4. Continue at [In Basecamp](#in-basecamp).

## In Basecamp

Install custom plugins through Package Manager or Modules → Install LGX Package
([Install and load a module](https://docs.logos.co/basecamp/install-and-load-a-module-in-logos-basecamp)).

Install the three `.lgx` files from the [Build packages](#build-packages) table,
wallet then `payment_streams_module` then `payment_streams_ui`.
Open `payment_streams_ui` from the sidebar.
Basecamp loads the two core modules from the UI package dependencies.
Turn Demo mode off.

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

## Notes

A later localnet session with wallet and fixture already present only needs
the sequencer up (`./verify/lifecycle.sh localnet start`), then prepare and
run.
If `seed-fixture` already created a vault, Session picks it up; start at
Deposit or Create stream.
Leave the sequencer running across UI rebuilds.
Stop it with `./verify/lifecycle.sh localnet stop` when finished.

Switching networks is a new Basecamp process with the other Make env.
Already-installed packages stay in `.scaffold/basecamp-ui`.

A header banner means `WALLET_HOME` is unset, wallet files are missing, or the
account list is empty.

## Rebuild after source changes

When editing `ui/`, `module/`, or the patched wallet tree:

```bash
pkill -9 -f 'logos_host|LogosBasecamp'
```

Rebuild only what changed:

```bash
nix build ./ui#lgx-portable -o ui/result
nix build ./module#lgx-portable
./verify/lib/build-wallet-lgx.sh
```

Relaunch with the same `NETWORK` (`make basecamp-ui-run` or
`NETWORK=localnet`).
Install LGX Package again for the rebuilt `.lgx`, then open `payment_streams_ui`.
