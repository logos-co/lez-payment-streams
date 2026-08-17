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
LogosBasecamp  (export env on this process, then spawn)
  payment_streams_ui (Main.qml, in-process)
    logos.callModule(...)
  logos_host  logos_execution_zone     (wallet handle lives here)
  logos_host  payment_streams_module   (reads FIXTURE_MANIFEST here)
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

D21.5. Session model for v1: one owner, one vault, one provider,
and one active stream at any one time.
`owner`, `vault_id`, `provider`, and `stream_id` stay visible and
copyable.
On load, Session prefills from modules that are already loaded:
`logos_execution_zone` `list_accounts` (first two public accounts as
`owner` then `provider`, converted with `account_id_to_base58`) and
`payment_streams_module` `getVaultStatus` for the `vault_id` already
in the field, then ids 0, 1, and 2 if that vault is missing.
When a vault exists, `vault_id` is that id.
`stream_id` is the one active or paused stream in that vault, else the
closed stream that still has accrued (claim next), else
`next_stream_id` when ready to create the next stream.
When none of those vaults exist yet, `vault_id` and `stream_id` stay
0 (same integers as the module walkthrough).
The operator may edit the fields or paste other account ids from the
LEZ wallet UI.
Refresh keeps the owner, provider, and vault id as typed and re-reads
that vault; it retargets `stream_id` to the lifecycle stream above.
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
that is already open in the wallet-module `logos_host`.
v1 uses one Basecamp, one wallet home, two accounts, both
`authenticated_transfer` registrations done in operator setup.
Wallet create, fund, AT, and `open` are operator setup.
This screen never calls `open` or `create_new`.
Basecamp binds the home through `NSSA_WALLET_HOME_DIR` and
`LEE_WALLET_HOME_DIR` on the `LogosBasecamp` process so both
`logos_host` children inherit them at spawn
(see [Host process before live writes](#host-process-before-live-writes)).
A different wallet home is a new Basecamp process (and `--user-dir`).
Unload or `pkill` of `logos_host` drops the handle; `open` again
after the next Load.

D21.9. Status: chain state after sync is the source of truth.
A `tx_hash` means submitted.
Completion is the status read after sync.
Refresh is pull-only.
While a write is in flight, that action stays inactive.
After each live write: poll `logos_execution_zone` `sync_to_block`
then the same reads Refresh uses, until the expected vault or stream
state is visible or the inclusion timeout fires (D21.16).
`lastError` distinguishes submission failure (no `tx_hash`) from
inclusion timeout (hash present, state unchanged).

D21.10. Load-loop spike (done on `feat/step-21-basecamp-ui`):
`ui/` Hello World, then the v1 dashboard in `Main.qml`.
`ui/metadata.json` `dependencies` lists `payment_streams_module` and
`logos_execution_zone` so Basecamp loads those before mounting the UI.

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
Closed streams from this vault show in a right-hand Previous streams
column at close, with a Claimed tick after claim.

`getStreamStatus` returns `as_of`, `stream_state`, accrued, unaccrued,
`rate`, `allocation_lo` / `allocation_hi`, `accrued_as_of`, and
`accrued_as_of_seconds`.
v1 Refresh uses that payload only (no second `readStreamConfigDecoded`
call).

D21.12. Owner liquid balance lives in On-chain state, next to vault
holding.
Vault holding is funds already in the vault (`getVaultStatus`
`vault_holding_balance_hex`).
Deposit spends the owner’s public native account, so Refresh also
updates that wallet balance in the snapshot.
`getVaultStatus` returns `owner_wallet_balance_hex` (same
`get_account_public` path the module already uses for the holding).
If the vault is missing, the UI reads the owner public account through
`logos_execution_zone` so the snapshot still shows a wallet balance.

D21.13. Writes follow the cyclical v1 lifecycle:
initialize vault, deposit, create stream, close stream, claim,
and then repeat with the next stream id.
At each stage only the next stream write is enabled.
Deposit stays available once the vault exists, including after
create stream (holding top-up). Extra deposit does not change the
stream stage.
Owner close and provider close are the same stage (close stream).
Claim is enabled after the stream is Closed.
Closed streams move to Previous streams at close; a tick in Claimed
appears after claim.
Create stays off until that claim (UI restriction for this screen;
the protocol allows another stream sooner).
If a second stream is created out of band (CLI) while one is closed
and unclaimed, Refresh binds the active stream (`needClose`). The
unclaimed row sits in Previous without a Claim action on this screen;
claim that stream from the CLI.
After claim, the cycle advances to create stream with the next stream id
(or deposit if vault holding is exhausted).
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
The live path submits `chainAction` with catalogue keys, then polls
sync plus the status read (D21.16).
ETA copy uses one localnet block (`15s`) until cadence is read from
the node.
Turning the switch off loads chain ids and vault/stream status.
Remove the switch, `demoMode`, `applyDemoSnapshot`, and the demo
branch in `runAction` when the live path is the only path.

D21.16. Live inclusion poll (QML timers, same GUI thread as
`logos.callModule`).
Cadence `2s`.
Timeout `60s` (four localnet blocks).
Submit errors set `lastError` from the module `message` and clear
`pendingWrite`.
Inclusion timeout sets `lastError` to that timeout and the `tx_hash`.
`initializeVault` and `createStream` refuse immediately when that vault
or stream id already exists (avoids a false complete on a colliding id).
`tx_hash` is sequencer admission. Inclusion is inferred from this
operator’s vault/stream state; a second writer on the same vault is
outside v1.
`Qt.callLater` runs the blocking submit so `Confirming…` can paint
first.
The first live submit stays synchronous. The next change after a
localnet click-through is `logos.callModuleAsync` for submit, then for
the Refresh scan. A C++ `ui-host` backend stays reserved for private
prove (open question 4).
Testnet idle cadence remains an open question (longer timeout or a
node-derived interval).

D21.17. v1 builds `chainAction` JSON in QML `writePayload`.
Keys were checked against the module dispatch table on 2026-08-17
(`initializeVault`, `deposit`, `createStream`, `closeStream`, `claim`,
`getVaultStatus`, `getStreamStatus`; owner-close omits `provider`).
`deposit` sends `amount_lo` / `amount_hi`,
`createStream` sends `rate` / `allocation_lo` / `allocation_hi`,
`claim` sends `owner` / `provider` / `vault_id` / `stream_id`.
Amounts that fit the on-screen u64 fields use `*_hi` = 0.
`writePayload` sends `normalizeAccount` ids (lowercase 64-hex).
Keep payloads in QML for v1; revisit if the catalogue grows past the
current write ops.

D21.18. Live session probe at the top of `loadSessionDefaults`:
`get_current_block_height` > 0, `list_accounts` non-empty, and one
`getVaultStatus` round trip (module + wallet + fixture). Failure shows
a Session banner ("Wallet not open — restart Basecamp with the wallet
env set") and gates writes. That copy covers missing env, a closed
handle, and a rebuild-loop restart; `open` is still operator setup
(D21.8). Demo mode skips the probe. Genesis height 0 is accepted as a
false positive. `sync_to_block` and `list_accounts` stay on
`logos_execution_zone` for v1.

D21.19. This screen is public execution only.
If `getVaultStatus` `vault_config.privacy_tier` is non-zero, Refresh
still binds the snapshot, writes stay off, and Session shows
"public-only".

D21.20. Unify demo and live `deriveStage` (claim → `needStream` vs
`needDeposit` when holding is empty) after the first localnet
click-through, together with a `PaymentStreamsLogic.js` extract of the
pure helpers. Skip that extract if Demo-off lands first and D21.14
removes the demo branch.

## v1 screen

```text
title:          Demo mode switch (temporary)

session:        owner, vault_id, provider, stream_id
                (prefilled from wallet; probe typed vault_id then 0..2)
                Refresh keeps owner, provider, vault_id
                last error / pending write

on-chain state: Refresh
                owner wallet balance, vault holding, total allocated
                stream_state, rate, allocation, accrued, unaccrued
                accrual started (accrued_as_of)
                chain time (as_of)
                estimated depleted at (as_of + unaccrued / rate)
                previous closed streams (right column; claimed tick)

owner actions:  initialize, deposit (amount; stays on after vault exists)
                create stream (rate, allocation)
                owner-close
                (stream writes follow the next stage)

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
  on the `LogosBasecamp` process
- that instance’s `logos_execution_zone` already `open` on the same
  home (D21.8)
- `FIXTURE_MANIFEST` on the same process
- `authenticated_transfer` registered for owner and provider on that
  home’s two public accounts
- for localnet writes, `PAYMENT_STREAMS_GUEST_BIN` on the same process

Launch recipe: [basecamp-rebuild-loop.md](basecamp-rebuild-loop.md).

## Host process before live writes

Demo mode stays inside QML.
The first live failures are outside `payment_streams_ui`.
`logos.callModule` reaches core modules in isolated `logos_host`
children spawned by liblogos.
Those children inherit the environment of `LogosBasecamp` at spawn.
Exports in a logoscore shell, or in a terminal after Basecamp is
already running, never reach those children.
`pkill` of `logos_host` (the UI rebuild loop) drops both the env
binding of those children and the in-memory wallet handle.

Reuse the walkthrough’s wallet home, fixture, and AT, then hand the
same files to Basecamp.
Close the logoscore wallet and stop that daemon first so
`storage.json` is free for this instance’s `open`
([reproduce/module.md](../../reproduce/module.md) Step 9 and Step 17).

### Wallet home on logos_host

Pinned LEZ wallet code reads `LEE_WALLET_HOME_DIR`
([step-32-step0-validation.md](../completed/step-32-step0-validation.md)).
`NSSA_WALLET_HOME_DIR` alone loads `~/.lee/wallet/storage.json`.
D45.14 keeps both names exported to the same directory
(`ps_export_wallet_home` in `verify/lib/common.sh`).
Set both on the shell that execs `LogosBasecamp`, together with
`REPO` so `payment_streams_module` can resolve repo-relative paths.

Localnet v1 click-through uses
`$REPO/.scaffold/wallet` (scaffold default) or
`$REPO/.scaffold/e2e/user/wallet-local` (module E2E).
Testnet uses `$REPO/.scaffold/e2e/testnet-wallet` and Step 1 of
[reproduce/module.md](../../reproduce/module.md).

### Wallet already open

Loading `logos_execution_zone` only starts `logos_host`.
The wallet handle stays empty until that process receives `open`
(or `create_new`) with `wallet_config.json` and `storage.json`.
This screen never sends those RPCs (D21.8).

A closed wallet makes `get_current_block_height` return 0 and
`list_accounts` return `[]`.
`probeSession` then sets the Session banner
("Wallet not open — restart Basecamp with the wallet env set")
and gates writes (D21.18).
That banner also appears when the env is already correct and only
`open` is missing, and after every rebuild-loop restart.

Fix, in this Basecamp instance, after Load of `logos_execution_zone`
and before relying on Session:

```text
logos.callModule("logos_execution_zone", "open",
  [wallet_config.json, storage.json])
```

Same paths as logoscore Step 6.
The caller is any Logos client in this instance (LEZ wallet UI,
helper plugin, or a one-shot `logos.callModule`).
Keep that call off `payment_streams_ui`.
Then Load `payment_streams_ui` (or turn Demo mode off) so
`loadSessionDefaults` sees accounts and height.

### Fixture and authenticated transfer

`payment_streams_module` reads `FIXTURE_MANIFEST` in its own
`logos_host` (`fixtureManifestPath` in
`module/src/payment_streams_module_kit.cpp`).
Unset, it looks for `verify/fixtures/localnet.json` via cwd walk and
`REPO`.
`logos_host` cwd is the Basecamp user dir, so the walk misses the
repo unless `FIXTURE_MANIFEST` (or `REPO`) is inherited from
`LogosBasecamp`.
Symptom matches the logoscore table:
`cannot open fixture manifest: verify/fixtures/localnet.json`.
Export the same absolute path the walkthrough uses
(`verify/fixtures/localnet.json` on localnet,
`verify/fixtures/testnet-module.json` on testnet) before starting
Basecamp.
Changing the export after Load has no effect until that
`logos_host` is spawned again.

`getVaultStatus` is the probe’s fixture round-trip.
`probeSession` currently treats `status` `error` as success, so a
missing fixture can pass the banner when the wallet is already
open; the live write then fails with the manifest message.
Tighten that check when Demo-off is the working path.

Authenticated transfer is chain state on the two public account
ids in that wallet home.
Run [reproduce/module.md](../../reproduce/module.md) Step 8
(`verify/repro-auth-transfer.sh`) or the module E2E AT ensure
against this same `WALLET_HOME` while logoscore holds the wallet,
then close and stop logoscore before Basecamp `open`.
A different home, or a new pair of accounts created in Basecamp,
needs AT again before deposit or stream writes.
`PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` stays on the same
Basecamp process for the testnet program-graph split (D45.14).

Localnet writes also need `PAYMENT_STREAMS_GUEST_BIN` on that
process (module guest bytes, and the patched wallet submit path
when program bytes are empty).

## Later work

After v1, the same plugin can grow to:

- pause, resume, top-up, withdraw
- stream table, vault switcher, AMM-style account picker
- Store, `storeQuery`, eligibility, `delivery_module`, two-host layouts
- live accrual tick
- private-execution UX (`privacy_tier`, funder / shield)
- non-native tokens, Discovery, shared pools
- replacing CLI verification
- a second wallet UI (operator `open` for this instance)

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

Feedback welcome on these; v1 ships with a default on each.

1. Testnet inclusion timeout.
   Live poll is `2s` / `60s` (D21.16), sized for localnet ~15s blocks.
   Set the budget after a measured testnet cadence.

2. Sync helper ownership.
   Resolved for v1: keep `sync_to_block` and `list_accounts` on
   `logos_execution_zone` (D21.18).

3. Stream id on create.
   Refresh fills the lifecycle `stream_id`. The operator can still
   edit it before create.

4. GUI-thread submit and C++ backend.
   First live submit stays synchronous. After localnet click-through,
   switch submit (then Refresh scan) to `logos.callModuleAsync` once
   its threading is verified. C++ `ui-host` stays for private prove,
   which this public-only screen does not do.

5. Standalone `nix run` (logos-standalone-app) beside Basecamp load.

6. After v1, when the page grows: Owner / Provider tabs versus a
   stream table in each region.

7. Demo vs live stage unification and JS extract (D21.20).
   After click-through, or drop with D21.14 if Demo-off is default.
