# Step 21, payment streams Basecamp UI

Complete (2026-08-20).
Index: [index.md](../index.md).
Walkthrough: [docs/reproduce/basecamp-ui.md](../../reproduce/basecamp-ui.md)
(init → deposit → create → close → claim).
Withdraw UI is [Step 54](../upcoming/step-54-withdraw-owner-and-recipient.md).
Protocol UI track:
[N18](../../reference/decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).
Terminology: [names.md](../../reference/names.md).
`chainAction` catalogue: [module/README.md](../../../module/README.md#chainaction-catalogue).

## Context

LIP-155 vaults and streams run through Universal
`payment_streams_module` on a single host (`MODE=module`).
The operator uses logoscore and the reproduce walkthrough
([reproduce/module.md](../../reproduce/module.md)).

This step provides a Basecamp `ui_qml` plugin so the same lifecycle is
visible on one host: initialize vault, deposit, create stream, wait
for accrual, owner-close, provider claim.
The plugin calls `payment_streams_module`.
That module opens the patched `logos_execution_zone` wallet and submits
through it.

```text
LogosBasecamp  (spawn with runtime user directory)
  payment_streams_ui (Main.qml, in-process)
    logos.callModule(...)
  logos_host  logos_execution_zone     (wallet handle lives here)
  logos_host  payment_streams_module   (opens wallet; resolves program identity)
```

Service-layer `user` (who pays for a protocol) maps on this screen to
chain-layer `owner` (vault owner).
The recipient is `provider` on both layers.
On-screen labels and JSON keys are `owner` and `provider`.

Owner and provider are distinct account ids.
They reside in one wallet as two public accounts.
That follows the approach in [reproduce/module.md](../../reproduce/module.md)
for running close and claim on one host and one wallet home.

## Goal

A lean QML-only Basecamp plugin (`payment_streams_ui`) that demonstrates
the public stream lifecycle on localnet or testnet.

v1 is one session (one vault, one stream) on one page, with Owner and
Provider regions, so one pass can run create → close → claim.
The focus is simplicity for a clear and reliable demo.

## Current baseline

The visual layout is established.
`ui/Main.qml` contains the single-page layout, field validation,
mock-state transitions (`demoMode`, default on),
`logos.callModule` invocations,
session probing, vault and stream scanning, and inclusion polling.
`ui/metadata.json` declares the two core-module dependencies.

Live chain execution and the operator launch path shipped
(`make basecamp-ui-build` / `prepare` / `run`, Demo off walkthrough).

## Architecture

```text
payment_streams_ui
  account selection, network display, lifecycle UX
    |
    +-- payment_streams_module
    |     open wallet, program identity, instruction planning, status reads
    |
    +-- logos_execution_zone
          account queries, block height, sync, signature and submit
```

Operator launch is Make from this repository:

```bash
make basecamp-ui-build
make basecamp-ui-prepare              # testnet (default); AT + pinata
make basecamp-ui-run                  # testnet (default)
make basecamp-ui-prepare NETWORK=localnet
make basecamp-ui-run NETWORK=localnet
```

`NETWORK` is a Make variable.
The run target exports `WALLET_HOME`, `LEE_WALLET_HOME_DIR`,
`NSSA_WALLET_HOME_DIR`, `FIXTURE_MANIFEST`, `REPO`, and `USER_DIR`
as absolute paths, then starts Nix-built Basecamp with `--user-dir`.
Basecamp’s argument surface stays `--user-dir`.
`logos_host` children inherit that environment.

Chain choice at runtime:

- Sequencer URL comes from `WALLET_HOME/wallet_config.json` when the
  module opens the wallet.
- Program id and clock accounts come from `FIXTURE_MANIFEST`
  (`program_id_hex`).
- Authenticated-transfer ImageID comes from
  `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX`, exported by
  `make basecamp-ui-run` via `ps_export_authenticated_transfer_program_id_hex`
  (live LEZ-cache ELF). Deposit and claim serialize that id into the
  instruction. Unset, the module uses graph
  `programs::authenticated_transfer().id()`, which on testnet is a
  different ImageID than the owner's `program_owner`. The wallet still
  returns a `tx_hash`; the sequencer never includes those deposits.

The same three `.lgx` packages serve both networks.
Switching networks is a new Basecamp process with the other Make env.
Rebuild and reinstall only when source changes.
Walkthrough and rebuild loop:
[docs/reproduce/basecamp-ui.md](../../reproduce/basecamp-ui.md) (Slice 3).

Public v1 writes submit by program id.
`PAYMENT_STREAMS_GUEST_BIN` remains reserved for private proving and
local guest development.

Wallet creation and mnemonic handling stay in the existing LEZ wallet
tooling ([reproduce/module.md](../../reproduce/module.md) Steps 4-7).
Authenticated-transfer registration and pinata funding for the UI wallet
are `make basecamp-ui-prepare`, which reuses `ps_auth_transfer_ensure`
and `ps_fund_testnet_account` on both networks.

`logos.callModule` invokes core modules in `logos_host` subprocesses.
Wallet handle, fixture config, and sequencer connection live there.
Stop logoscore before Basecamp so `storage.json` is free
([N52](../../reference/decisions.md#n52-exclusive-wallet-cli-stop-window-2026-08-14)).
logoscore and E2E keep using their own fixture manifests and CLI path.

## Work plan

### Slice 0, pin the live baseline

Purpose:
verify package build and clean QML loading in Basecamp before enabling
live chain writes.

Files:

- `ui/Main.qml`
- `ui/metadata.json`
- `ui/flake.nix`

Work:

1. Build portable packages for `logos_execution_zone`,
   `payment_streams_module`, and `payment_streams_ui`.
2. Install them into an isolated Basecamp user directory
   (`.scaffold/basecamp-ui`).
3. Load the modules in order and verify that `Main.qml` renders without
   QML errors.
4. Verify the visual layout and mock transitions for initialize, deposit,
   create, close, and claim.

Gate:
`payment_streams_ui` loads inside Basecamp and completes the layout pass
cleanly.

### Slice 1, connect wallet and show network state

Purpose:
open the wallet from env, prefill accounts, and show live network state.

Files:

- `ui/Main.qml`
- `module/src/payment_streams_module_writes.cpp`
- `module/src/payment_streams_module_kit.cpp`

Work:

1. On first live probe, `payment_streams_module` reads `WALLET_HOME` /
   `LEE_WALLET_HOME_DIR`, ensures `statistics.json` beside storage, and
   calls `logos_execution_zone.open`.
2. UI queries `list_accounts` and `get_current_block_height` (and
   `get_sequencer_addr` when useful for the header).
3. When public accounts are present, the UI prefills owner and provider
   fields.
4. When the home is missing, files are missing, `open` fails, or the
   account list is empty, the UI shows a readiness banner.
5. Header shows mock versus live: simulated offline when Demo mode is
   on; live block height and sequencer target when connected.
6. Make exports an absolute `FIXTURE_MANIFEST`.
   Keep the existing fixture resolver.
   Surface a clear error when the manifest is missing or
   `program_id_hex` is invalid.

Gate:
Opening the plugin with a prepared wallet populates public accounts and
enables the initial lifecycle stage with network state visible.

### Slice 2, live lifecycle reads and writes

Purpose:
enable live on-chain payment stream operations through the UI.

Files:

- `ui/Main.qml`

Work:

1. Connect `submitLiveAction` to `payment_streams_module.chainAction` for
   `initializeVault`, `deposit`, `createStream`, `closeStream`, and
   `claim`.
2. Poll with `liveConfirmTimer` every 2s: `sync_to_block` then vault and
   stream status, until included or 120s elapse.
3. Keep live stage transitions synchronized:
   `needVault` → `needDeposit` → `needStream` → `needClose` →
   `needClaim` → `needStream`.
4. While the stream is Active (`needClose`), Refresh is the accrual wait.
   Close stays enabled at zero accrued.
   Show a one-line hint to Refresh until Accrued is greater than 0,
   then close.
5. Update the Recent transactions table on live submissions with short
   hashes and inclusion status.
6. Demo mode remains the visual mock switch (default on for Slice 0).
   The live walkthrough turns it off after the wallet is open.

Gate:
A user can complete the full lifecycle on localnet from the UI:
initialize vault, deposit, create stream, observe accrual, close, and
claim.

### Slice 3, reproducible runner and walkthrough

Purpose:
provide a simple launch flow and clear reproduction instructions.

Files:

- `Makefile`
- `docs/reproduce/basecamp-ui.md`

Work:

1. Add Makefile targets:
   `make basecamp-ui-build`
   `make basecamp-ui-prepare` (testnet default; AT + pinata)
   `make basecamp-ui-run` (testnet default)
   `make basecamp-ui-run NETWORK=localnet`
2. Document the operator walkthrough for localnet and testnet, including
   wallet creation from [reproduce/module.md](../../reproduce/module.md)
   Steps 4-7, `make basecamp-ui-prepare`, and stopping logoscore before
   Basecamp.
3. Verify that the UI walkthrough matches the documented CLI reproduce
   steps.

Gate:
An operator can follow `docs/reproduce/basecamp-ui.md` on a fresh
environment and complete the full payment stream lifecycle.

## User test journeys

Setup (once):

```bash
make basecamp-ui-build
```

Testnet (default):

```bash
make basecamp-ui-prepare
make basecamp-ui-run
```

Localnet (sequencer already running):

```bash
make basecamp-ui-prepare NETWORK=localnet
make basecamp-ui-run NETWORK=localnet
```

The operator opens Basecamp, loads the modules in order
(`logos_execution_zone`, `payment_streams_module`,
`payment_streams_ui`), turns Demo mode off, and follows the highlighted
lifecycle actions.

## Scope boundaries

v1 includes:
public owner and provider accounts in one wallet, native token streams,
one active lifecycle on screen, owner-close, provider-close, claim,
and a read-only network and block indicator.

v1 excludes:
wallet creation and mnemonic setup inside QML, faucet integration,
in-app authenticated transfer registration, private zero-knowledge
proving, Store eligibility hooks, non-native tokens, a Basecamp or QML
network-switching control, and multi-vault navigation tables.

Account creation stays in [reproduce/module.md](../../reproduce/module.md)
Steps 4-7.
Funding and authenticated transfer for the UI wallet are
`make basecamp-ui-prepare` (same helpers as
`./verify/testnet/fund-testnet-accounts.sh`).

## Decisions

D21.1. Tree `ui/` next to `module/`
(Step 53 layout; [D5](../../reference/decisions.md#d5-new-module-naming)
keeps metadata name `payment_streams_module` for the core plugin).
Runtime UI id `payment_streams_ui`.

D21.2. Shape: QML-only `ui_qml`, `view` `Main.qml`,
`mkLogosQmlModule`, tutorial
[Part 2](https://github.com/logos-co/logos-tutorial)
(`logos.callModule`).
Portable package: `nix build .#lgx-portable` from `ui/`.

D21.3. Host: Nix-built portable Basecamp
(`nix build '.#bin-bundle-dir'`).
Install the `.lgx` through Package Manager or Modules menu.
Operator launch is `make basecamp-ui-run`, which passes `--user-dir`.
Rebuild iteration:
[basecamp-ui.md](../../reproduce/basecamp-ui.md#rebuild-after-source-changes).

D21.4. Theme: `import Logos.Theme` and `import Logos.Controls`.
Page fill `Theme.palette.background`.
Visible copy and actions use `LogosText` and `LogosButton`.

D21.5. Session model for v1: one owner, one vault, one provider,
and one active stream at a time.
`owner`, `vault_id`, `provider`, and `stream_id` remain visible and
copyable.
On load, after a successful open, Session prefills from
`logos_execution_zone` `list_accounts`.

D21.6. Layout for v1: one page, two action regions (Owner, Provider),
plus Session and On-chain state cards.
Recent transactions table appears below state in live mode.

D21.7. v1 actions, in lifecycle order:

- Owner: `initializeVault`, `deposit`, `createStream`, owner-close
  (`closeStream` with `provider` omitted or equal to `owner`)
- On-chain state: Refresh → `getVaultStatus` / `getStreamStatus`
- Provider: `claim`, provider-close (`closeStream` with distinct
  `provider`)

D21.8. Signing and wallet connection:
The UI calls `chainAction` on `payment_streams_module`.
The module submits transactions through `logos_execution_zone`.
v1 assumes two public accounts in the wallet that the module opens.
`open` takes config, storage, and statistics paths and does not take a
password.
The UI has no mnemonic, create-wallet, or password field.

D21.9. Status: chain state after sync is the source of truth.
A `tx_hash` confirms submission.
Inclusion is confirmed by status reads after `sync_to_block`.

D21.10. Dependencies:
`ui/metadata.json` lists `payment_streams_module` and
`logos_execution_zone` in `dependencies`.

D21.11. Stream card displays:
`stream_state` (Active, Paused, Closed), rate, allocation, accrued,
unaccrued, accrual timestamp, and estimated depletion time.
While Active, a one-line hint tells the operator to Refresh until
Accrued is greater than 0, then close.

D21.12. Balances:
Owner wallet balance and vault holding display in On-chain state.

D21.13. Lifecycle stage progression:
`needVault` → `needDeposit` → `needStream` → `needClose` → `needClaim`
→ `needStream`.
Deposit remains available whenever the vault exists.
Close stays enabled at zero accrued.

D21.14. Field validation:
Account ids accept valid base58 or 64-hex strings.
Vault id, stream id, amounts, rate, and allocation validate as positive
integers before submission.

D21.15. Demo switch and network indicator:
A switch toggles live chain interaction and visual mock mode
(default on).
The header displays a read-only badge:
simulated offline when Demo mode is on,
or live sequencer target and block height when the wallet is open.

D21.16. Inclusion polling:
`liveConfirmTimer` ticks every 2s, runs `sync_to_block` and status
reads, and gives up after 120s.
The interval and timeout are the same on localnet and testnet.

D21.17. Payload construction:
`Main.qml` builds standard JSON payloads matching the `chainAction`
catalogue.

D21.18. Wallet open and readiness:
On first live probe, `payment_streams_module` reads `WALLET_HOME` /
`LEE_WALLET_HOME_DIR`, ensures `statistics.json` beside `storage.json`,
and calls `logos_execution_zone.open`.
The UI shows a readiness banner when the home is unset, files are
missing, `open` fails, or `list_accounts` is empty.
Environment reads stay in the C++ module.

D21.19. Public execution scope:
This screen focuses on public vaults (`privacy_tier = 0`).

D21.20. Consolidated helpers:
Pure parsing and formatting functions remain concise and centralized.

D21.21. Recent transactions:
A session-scoped table displays submitted transaction hashes, action
names, and confirmation status.

D21.22. Launch and network:
`make basecamp-ui-run` is the documented operator launch.
`make basecamp-ui-prepare` AT-inits and pinata-funds the first two public
accounts in that same `WALLET_HOME`.
`NETWORK` defaults to testnet.
`NETWORK=localnet` selects the localnet fixture and wallet home.
Make maps `NETWORK` onto `WALLET_HOME`, `FIXTURE_MANIFEST`, `CHAIN`,
and `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX`.
Basecamp’s extra argument is `--user-dir`.
The three `.lgx` packages are network-agnostic.
A network switch is a restart with the other env; load already-installed
modules.

D21.23. Stream id on Refresh:
Refresh writes `stream_id` to the active stream, else a closed
unclaimed stream, else `next_stream_id`.
Edits in the field last until the next Refresh.

D21.24. Program identity:
`payment_streams_module` keeps the existing fixture resolver.
Make exports `FIXTURE_MANIFEST` as an absolute path into the Basecamp
process.
Missing or invalid `program_id_hex` is an error on the readiness banner
or the failed `chainAction` message.

## v1 screen

```text
title:          Payment Streams Dashboard (live / mock switch)
                network indicator (Simulated offline / Live sequencer and height)

session:        owner, vault_id, provider, stream_id
                (prefilled from wallet; editable)
                session status banner

on-chain state: Refresh action
                owner balance, vault holding, total allocated
                stream state, rate, allocation, accrued, unaccrued
                accrual checkpoint, chain time, estimated depletion
                accrual hint while Active
                previous closed streams summary

transactions:   Recent transactions table
                timestamp, action, short hash, status

owner actions:  initialize vault, deposit, create stream, owner-close

provider:       claim, provider-close
```

## Runtime

Runtime uses the same wallet homes and deployed programs as
[reproduce/module.md](../../reproduce/module.md).

Before the first public UI write:

- the wallet home has two public accounts
  ([reproduce/module.md](../../reproduce/module.md) Steps 4-7);
- `make basecamp-ui-prepare` has registered authenticated transfer and
  pinata-funded those accounts;
- packages are loaded in order:
  `logos_execution_zone`, `payment_streams_module`, `payment_streams_ui`;
- logoscore is stopped.

## Later work

After v1, the UI can expand to:

- pause, resume, and top-up actions;
- withdraw to owner ([Step 54](../upcoming/step-54-withdraw-owner-and-recipient.md));
- multi-vault and multi-stream tables;
- interactive account switching;
- Store query and eligibility integration;
- private shielded streams and proving.

## Deliver

- `ui/` with `metadata.json`, `flake.nix`, and `Main.qml`
- Portable `.lgx` package buildable via Nix
- Live payment stream lifecycle execution in Basecamp
- `make basecamp-ui-build`, `make basecamp-ui-prepare`, and
  `make basecamp-ui-run` (`NETWORK` selects testnet or localnet)
- Reproduction documentation in `docs/reproduce/basecamp-ui.md`
