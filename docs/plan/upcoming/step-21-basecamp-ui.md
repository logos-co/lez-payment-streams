# Step 21, payment streams Basecamp UI

Upcoming.
Index: [index.md](../index.md).
Protocol UI track:
[N18](../../reference/decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).
Terminology: [names.md](../../reference/names.md).
`chainAction` catalogue: [module/README.md](../../../module/README.md#chainaction-catalogue).
Load-loop notes: [basecamp-rebuild-loop.md](basecamp-rebuild-loop.md).

## Context

LIP-155 vaults and streams already run through Universal
`payment_streams_module` on a single host (`MODE=module`).
The operator today uses logoscore and the reproduce walkthrough
([reproduce/module.md](../../reproduce/module.md)).

This step adds a Basecamp `ui_qml` plugin so the same lifecycle is
visible on one host: initialize vault, deposit, create stream, wait
for accrual, owner-close, provider claim.
The plugin calls `payment_streams_module`.
That module submits through the patched `logos_execution_zone` wallet.

```text
Basecamp
  payment_streams_ui (Main.qml)
    logos.callModule("payment_streams_module", ...)
      payment_streams_module
        logos_execution_zone (patched wallet)
```

Service-layer `user` (who pays for a protocol) maps on this screen to
chain-layer `owner` (vault owner).
The recipient is `provider` on both layers.
On-screen labels and JSON keys are `owner` and `provider`.

Owner and provider are distinct account ids.
They may live in one open wallet as two public accounts.
That is how [reproduce/module.md](../../reproduce/module.md) already
runs close and claim on one `logoscore` and one wallet home.

## Goal

A QML-only Basecamp plugin (`payment_streams_ui`) that demonstrates
the public stream lifecycle on localnet or testnet, then grows toward
the full `chainAction` catalogue.

v1 is one session (one vault, one stream) on one page, with Owner and
Provider regions, so one pass can run create → close → claim.

## Decisions

D21.1. Tree `ui/` next to `module/`
(Step 53 layout; [D5](../../reference/decisions.md#d5-new-module-naming)
keeps metadata name `payment_streams_module` for the core plugin).
Runtime UI id `payment_streams_ui`.

D21.2. Shape: QML-only `ui_qml`, `view` `Main.qml`,
`mkLogosQmlModule`, tutorial
[Part 2](https://github.com/logos-co/logos-tutorial)
(`logos.callModule`).
Portable package: `nix build .#lgx-portable` from the first `.lgx`.
Pin `logos-module-builder` the same way `module/flake.nix` does.
Lifecycle build pins `payment_streams_module` as a path input named
`payment_streams_module`.

D21.3. Host: Nix-built portable Basecamp
(`nix build '.#bin-bundle-dir'` or AppImage from this checkout).
Install the `.lgx` through Package Manager
([Install and load a module](https://docs.logos.co/basecamp/install-and-load-a-module-in-logos-basecamp)).
Iterate with `--user-dir` / `LOGOS_USER_DIR`.
After each `.lgx` rebuild, stop Basecamp and leftover `logos_host`
children, reinstall, Load.

D21.4. Theme: `import Logos.Theme` and `import Logos.Controls`.
Page fill `Theme.palette.background`.
Visible copy and actions use `LogosText` and `LogosButton`.
Popup chrome may use `QtQuick.Controls` `Popup`.

D21.5. Session model for v1: one owner, one vault, one provider, one
stream.
`owner`, `vault_id`, `provider`, and stream id stay visible and
copyable.
On load, Session prefills from modules that are already loaded:
`logos_execution_zone` `list_accounts` (first two public accounts as
`owner` then `provider`, converted with `account_id_to_base58`) and
`payment_streams_module` `getVaultStatus` for vault ids 0, 1, then 2.
When a vault exists, `vault_id` is that id and `stream_id` is the last
created stream (`next_stream_id - 1`).
When none of those vaults exist yet, `vault_id` and `stream_id` stay
0 (same integers as the module walkthrough).
The operator may edit the fields or paste other account ids from the
LEZ wallet UI.
Session stays as the operator set it until they edit it.
A later table is more rows of the same fields, once a list API can
feed them.

D21.6. Layout for v1: one page, two action regions of the same session
(Owner, Provider), plus a Session strip and an On-chain state strip.
Session holds the prefilled ids (editable).
On-chain state holds the read-only snapshot and Refresh.
Role is which action region the operator uses.
A first-run owner-or-provider choice and a two-list layout belong in
a later slice.
Tabs are the upgrade if the page grows.

D21.7. v1 actions, in lifecycle order:

- Owner: `initializeVault`, `deposit`, `createStream`, owner-close
  (`closeStream` with `provider` omitted or equal to `owner`)
- On-chain state: Refresh → `getVaultStatus` / `getStreamStatus`
  (and owner wallet balance)
- Provider: `claim`, provider-close (`closeStream` with a distinct
  `provider`)

`stream_state` display: Active, Paused, or Closed.
Account ids accept base58 or 64-hex.
Native token; leave `token_id` unset.
Visible chain fields: `owner`, `provider`, vault id, stream id,
balances, `stream_state`.

D21.8. Signing: the UI passes `owner` and `provider` in `chainAction`
JSON.
The module derives `submitter` (owner writes vs provider claim /
provider-close).
`logos_execution_zone` signs with that account id from the wallet
that is already open.
v1 uses one Basecamp, one wallet home, two accounts, both
`authenticated_transfer` registrations done in operator setup.
Wallet create, fund, and AT are operator setup.
Basecamp binds the home through `NSSA_WALLET_HOME_DIR` and
`LEE_WALLET_HOME_DIR` on the process that loaded the wallet module.
A different wallet home is a new Basecamp process (and `--user-dir`).

D21.9. Status: chain state after sync is the source of truth.
A `tx_hash` means submitted.
Completion is the status read after sync.
Refresh is pull-only.
While a write is in flight, that action stays inactive.
After each write: sync (`logos_execution_zone` `sync_to_block` or a
module helper; see open questions), then the same reads Refresh uses.

D21.10. Load-loop spike (done on `feat/step-21-basecamp-ui`):
`ui/` Hello World with empty `dependencies`, `test` button, themed
popup.
Next slice wires `payment_streams_module` and replaces Hello World
with the v1 dashboard.

D21.11. Status region: even with one vault and one stream, the page
shows their snapshot as of the last Refresh (and after each write’s
sync + status read).
Stream card fields:

- `stream_state` (Active, Paused, Closed)
- rate, allocation, accrued, unaccrued
- accrual checkpoint (`accrued_as_of` / seconds) — when the on-chain
  fold last committed
- chain clock at this snapshot (`getStreamStatus` `as_of`)
- estimated depletion time while Active with a positive rate:
  `as_of + unaccrued / rate`

Those times are labels on a frozen snapshot.
Refresh re-reads vault, stream, and owner wallet balance from chain.

`getStreamStatus` today returns `as_of`, `stream_state`, accrued, and
unaccrued.
v1 also needs rate, allocation, and `accrued_as_of` on that payload
(small catalogue extension, or the UI uses `readStreamConfigDecoded`
for the extra fields).

D21.12. Owner liquid balance lives in On-chain state, next to vault
holding.
Vault holding is funds already in the vault (`getVaultStatus`
`vault_holding_balance_hex`).
Deposit spends the owner’s public native account, so Refresh also
updates that wallet balance in the snapshot.
Prefer adding it to `getVaultStatus` (the module already calls
`logos_execution_zone` `get_account_public`) so the UI keeps one
Refresh path.

D21.13. Writes follow the linear v1 lifecycle: initialize vault, deposit,
create stream, close stream, claim.
At each stage only the next write is enabled.
Owner close and provider close are the same stage (close stream).
Claim is enabled after the stream is Closed.
After claim, every write stays inactive (Next: Complete).
Unavailable actions stay visible, dimmed, and unpressable.
The write in flight keeps its block at full opacity.
That button shows `Confirming… ~Ns` and ignores further clicks until
the status read (demo: 2s delay).
N is one localnet block (`15s`) on the live path, and `2s` in Demo mode.
Refresh stays available.
While `pendingWrite` is set, every write stays inactive.

D21.15. Session and write fields validate on the UI before a click
starts a write.
Account ids: trimmed base58 (32–44 chars, Bitcoin alphabet) or 64-hex
([names.md](../../reference/names.md)).
`vault_id` and `stream_id`: whole number in `0..2^64-1`.
Deposit `amount`, stream `rate`, and `allocation`: whole number in
`1..2^64-1` (LIP-155 positive rate; guest rejects zero amount, rate,
and allocation).
Create stream and claim: `provider` differs from `owner`
(LIP-155 stream create; guest `ProviderEqualsOwner`).
Provider close uses that same distinct-provider check.
Invalid fields show an error under the input and keep that write
unpressable.

D21.14. Temporary Demo mode switch on the title row.
Default on for this layout spike.
Writes go through one `runAction` entry.
Demo mode waits 2s (`Confirming… ~2s`) then advances `stage`.
The live path will wait for sync plus the status read; ETA copy uses
one localnet block (`15s`) until cadence is read from the node.
Turning the switch off loads chain ids and vault/stream status.
Remove the switch, `demoMode`, `applyDemoSnapshot`, and the demo
branch in `runAction` when chain writes land.

## v1 screen

```text
title:          Demo mode switch (temporary)

session:        owner, vault_id, provider, stream_id
                (prefilled from wallet + existing vault 0..2)
                last error / pending write

on-chain state: Refresh
                owner wallet balance, vault holding, total allocated
                stream_state, rate, allocation, accrued, unaccrued
                accrual started (accrued_as_of)
                chain time (as_of)
                estimated depleted at (as_of + unaccrued / rate)

owner actions:  initialize, deposit (amount)
                create stream (rate, allocation)
                owner-close
                (only the next write enabled)

provider actions: claim, provider-close
                  (close shares the close-stream stage)
```

## Runtime

Same wallet home and guest binary as
[reproduce/module.md](../../reproduce/module.md)
and [scaffold layout](../../reference/names.md#scaffold-layout).
`MODE=module` on localnet or testnet.

Before first UI write:

- patched `logos_execution_zone` `.lgx` from this repo, installed and
  loaded ([pins.md](../../reference/pins.md))
- `payment_streams_module` `.lgx` installed and loaded
- wallet home exported (`NSSA_WALLET_HOME_DIR`, `LEE_WALLET_HOME_DIR`)
- `authenticated_transfer` registered for owner and provider
- for localnet writes, `PAYMENT_STREAMS_GUEST_BIN` on the process that
  loads the wallet module

## Later work

After v1, the same plugin can grow to:

- pause, resume, top-up, withdraw
- stream table, vault switcher, AMM-style account picker
- Store, `storeQuery`, eligibility, `delivery_module`, two-host layouts
- live accrual tick
- private-execution UX (`privacy_tier`, funder / shield)
- non-native tokens, Discovery, shared pools
- replacing CLI verification
- a second wallet UI

## Deliver

- `ui/` with `metadata.json`, `flake.nix`, `Main.qml`, sidebar icon
- Portable `.lgx` that loads in Nix-built Basecamp
- Operator notes (pillar README or
  [reproduce/module.md](../../reproduce/module.md)):
  build, install, load, wallet env, AT setup for two accounts

v1 is complete when the operator can, through this plugin on local
LEZ: initialize a vault, deposit, create a stream, Refresh to see
status, owner-close, claim, and provider-close; copy uses `owner` /
`provider`; `Logos.Theme` and `Logos.Controls` are in use.

## Open questions

Feedback welcome on these; v1 can ship with a default on each.

1. Post-write sync.
   QML calls `logos_execution_zone` `sync_to_block` (list that module
   in UI `dependencies`), or `payment_streams_module` grows a helper
   so the UI only talks to `payment_streams_module`.

2. Stream id on create.
   Operator pastes `stream_id`, or the UI generates one and shows it.

3. Demo amounts.
   Fixed placeholders for deposit, rate, and allocation, or editable
   fields from the first screen.

4. C++ backend (`#ui-qml-backend`).
   Worth adding if write → sync → Refresh needs a typed replica, or if
   an account picker does.

5. Standalone `nix run` (logos-standalone-app) beside Basecamp load.

6. After v1, when the page grows: Owner / Provider tabs versus a
   stream table in each region.
