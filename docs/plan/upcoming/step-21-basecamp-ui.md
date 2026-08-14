# Step 21, payment streams Basecamp UI

Upcoming.
Index: [index.md](../index.md).
Protocol UI track:
[N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).

## Goal

A small Basecamp `ui_qml` plugin (`payment_streams_ui`) so an operator
can manage LIP-155 vaults and streams on one host.
The plugin calls `payment_streams_module`, which uses the patched
`logos_execution_zone` wallet.

## Principles

- Thin client of `chainAction` and status reads
  ([catalogue](../../payment-streams-module/README.md#chainaction-catalogue)).
  No second protocol, no Store, no eligibility.
- Chain state after sync is the source of truth.
  A `tx_hash` means submitted, not done.
- Status is pull-only.
  A Refresh button re-reads vault and stream status.
  No timers, no live accrual tick.
- Vault id and stream id stay visible and copyable.
  The counterparty is out of band (paste ids).
- Labels and JSON keys are `owner` and `provider`
  ([naming conventions](../../reference/naming-conventions.md)).
  `stream_state` is shown as Active, Paused, or Closed.
- Native token, public vaults.
  Omit `token_id`. Do not show ImageID or `program_id_hex`.
- Wallet create, fund, and `authenticated_transfer` stay operator setup.
  The UI uses an already-open wallet.
- Packaging follows tutorial
  [Part 2](https://github.com/logos-co/logos-tutorial)
  (QML-only `ui_qml`, `logos.callModule`).
  Layout is a management dashboard, not the calculator toy screen.
- Style with `import Logos.Theme`.
  Nix-built Basecamp provides it.

## What to build

In-repo tree `logos-payment-streams-ui/` next to
`logos-payment-streams-module/`
([D5](../../reference/integration-decisions.md#d5-new-module-naming)).

```text
Basecamp
  payment_streams_ui (Main.qml)
    logos.callModule("payment_streams_module", ...)
      payment_streams_module
        logos_execution_zone (patched wallet)
```

`metadata.json`:

- `name`: `payment_streams_ui`
- `type`: `ui_qml`
- `view`: `Main.qml`
- `dependencies`: `["payment_streams_module"]`
- `icon` for the sidebar launcher
- flake input attribute named `payment_streams_module`

Scaffold with `logos-module-builder` `#ui-qml` (`mkLogosQmlModule`).

One page, dashboard layout:

```text
session:  owner, vault id, last error / pending write
vault:    balance, allocated, initialize, deposit
streams:  table + Refresh
          new stream (provider, rate, allocation)
          row actions as they ship (pause, resume, top-up, close)
```

After each write, sync (`logos_execution_zone` `sync_to_block` or
equivalent), then the same status reads Refresh uses.
Disable the action while that is in flight.

Account ids accept base58 or 64-hex.

## Owner flows

Minimum (DoD):

- paste `owner` and `vault_id`
- `initializeVault`
- `deposit`
- `createStream` (paste `provider`)
- Refresh → `getVaultStatus` / `getStreamStatus`

Also in scope, not DoD gates:

- `pauseStream`, `resumeStream`, `topUpStream`
- `closeStream` as owner-close
- `withdraw`

## Runtime

Prerequisite: `MODE=module` on localnet or testnet, same wallet home and
guest binary as
[reproduce/payment-streams.md](../../reproduce/payment-streams.md)
and [scaffold layout](../../reference/naming-conventions.md#scaffold-layout).

Before first UI write:

- patched `logos_execution_zone` `.lgx` from this repo, installed and
  loaded ([feature-branch-pins.md](../../reference/feature-branch-pins.md))
- `payment_streams_module` `.lgx` installed and loaded
- wallet home exported (`NSSA_WALLET_HOME_DIR` and `LEE_WALLET_HOME_DIR`)
- `authenticated_transfer` registered for the accounts that will sign
- for localnet writes, `PAYMENT_STREAMS_GUEST_BIN` on the process that
  loads the wallet module

Build with `nix build .#lgx-portable`.
Pin `logos-module-builder` the same way `logos-payment-streams-module`
does.
Pin `payment_streams_module` as a path input.

Launch Nix-built Basecamp.
Install the local `.lgx` through the package manager UI
([Install and load a module](https://docs.logos.co/basecamp/install-and-load-a-module-in-logos-basecamp)).
Use `--user-dir` (or `LOGOS_USER_DIR`) while iterating.
After rebuilding an `.lgx`, kill Basecamp and orphaned `logos_host`
children, then reinstall.

## Out of scope

- Store, `storeQuery`, eligibility, `delivery_module`, dual-host layouts
- Live status or accrual polling
- Private-execution UX (`privacy_tier`, funder/shield)
- Non-native tokens
- Discovery or shared pools
- Replacing `MODE=module` CLI verification
- Reimplementing the wallet UI

## Deliver

- `logos-payment-streams-ui/` with `metadata.json`, `flake.nix`,
  `Main.qml`, sidebar icon
- Portable `.lgx` that loads in Nix-built Basecamp without QML errors
- Short operator notes in this repo (pillar README or
  [reproduce/payment-streams.md](../../reproduce/payment-streams.md)):
  build, install, load, wallet env, AT setup

Definition of done:

- Operator can initialize a vault, create a stream, and see that stream's
  status on local LEZ through the UI (Refresh)
- JSON and labels use `owner` / `provider`
- `Logos.Theme` is used
- No Store or eligibility surface

## Open questions

Discuss later, highest level first.

1. What should the UX provide?
   Owner vault and stream management only, or also provider claim
   (and provider-close) in the same plugin?
   Which of pause, resume, top-up, and withdraw belong on the first
   screen versus later?
2. How does the operator identify the session?
   Paste `owner` and `vault_id`, or pick an account from the open
   wallet (AMM-style Connect / account control)?
3. Where does the stream list come from?
   `listMyStreams` currently takes only `vaultId` and reads fixture
   `owner_account_id`.
   Extend the module to take `owner`, or have the UI remember ids it
   created and Refresh via `getStreamStatus`?
4. Where does post-write sync live?
   QML calls `logos_execution_zone` `sync_to_block` (list that module
   in UI `dependencies`), or `payment_streams_module` grows a helper
   so the UI never talks to the wallet?
5. When would a C++ backend (`#ui-qml-backend`) be worth it?
   Only if write → sync → Refresh cannot stay correct in QML, or if
   an account picker needs a typed replica.
6. Is standalone `nix run` in scope, or Basecamp load only?
