# Step 21, payment streams Basecamp UI

Upcoming.
Index: [index.md](../index.md).
Protocol UI track:
[N18](../../reference/decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).
Terminology: [names.md](../../reference/names.md).
`chainAction` catalogue: [module/README.md](../../../module/README.md#chainaction-catalogue).
Load-loop notes: [basecamp-rebuild-loop.md](basecamp-rebuild-loop.md).

## Context

LIP-155 vaults and streams run through Universal
`payment_streams_module` on a single host (`MODE=module`).
The operator uses logoscore and the reproduce walkthrough
([reproduce/module.md](../../reproduce/module.md)).

This step provides a Basecamp `ui_qml` plugin so the same lifecycle is
visible on one host: initialize vault, deposit, create stream, wait
for accrual, owner-close, provider claim.
The plugin calls `payment_streams_module`.
That module submits through the patched `logos_execution_zone` wallet.

```text
LogosBasecamp  (spawn with runtime user directory)
  payment_streams_ui (Main.qml, in-process)
    logos.callModule(...)
  logos_host  logos_execution_zone     (wallet handle lives here)
  logos_host  payment_streams_module   (resolves program identity here)
```

Service-layer `user` (who pays for a protocol) maps on this screen to
chain-layer `owner` (vault owner).
The recipient is `provider` on both layers.
On-screen labels and JSON keys are `owner` and `provider`.

Owner and provider are distinct account ids.
They reside in one open wallet as two public accounts.
That follows the approach in [reproduce/module.md](../../reproduce/module.md)
for running close and claim on one `logoscore` and one wallet home.

## Goal

A lean QML-only Basecamp plugin (`payment_streams_ui`) that demonstrates
the public stream lifecycle on localnet or testnet.

v1 is one session (one vault, one stream) on one page, with Owner and
Provider regions, so one pass can run create → close → claim.
The focus is simplicity for a clear and reliable demo.

## Current baseline

The visual layout is established.
`ui/Main.qml` contains the single-page layout, field validation,
mock-state transitions (the current `demoMode` branch),
`logos.callModule` invocations,
session probing, vault and stream scanning, and inclusion polling.
`ui/metadata.json` declares the two core-module dependencies.

The remaining work integrates live chain execution with the UI:

1. adopt the open wallet session in `logos_execution_zone`;
2. resolve program identity in `payment_streams_module` reliably;
3. display network mode and block status cleanly in the header;
4. verify live `chainAction` submissions and polling through localnet and testnet;
5. provide a streamlined build, launch, and walkthrough workflow.

## Implementation direction

The demo flow uses three layers.

```text
payment_streams_ui
  wallet adoption, account selection, network display, lifecycle UX
    |
    +-- logos_execution_zone
    |     account queries, block height, sync, signature and submit
    |
    +-- payment_streams_module
          program identity, instruction planning, status reads
```

The wallet configuration provides the sequencer connection.
The UI connects to `logos_execution_zone` upon loading.
When accounts are available from `list_accounts`, the UI populates owner and
provider fields and marks the session ready.

`payment_streams_module` resolves the payment-stream program id through
its standard fixture resolution and environment configuration.
Public v1 writes submit by program id and skip guest binary execution.
`PAYMENT_STREAMS_GUEST_BIN` remains reserved for private proving and local
guest development.

Wallet creation, mnemonic handling, funding, and authenticated-transfer
initialisation remain in the existing setup scripts and LEZ wallet tooling.
The UI focuses on presenting the payment stream lifecycle cleanly.

## Work plan

### Slice 0, pin the live baseline

Purpose:
verify package build and clean QML loading in Basecamp before enabling
live chain writes.

Files:

- `ui/Main.qml`
- `ui/metadata.json`
- `ui/flake.nix`
- `docs/plan/upcoming/basecamp-rebuild-loop.md`

Work:

1. Build portable packages for `logos_execution_zone`, `payment_streams_module`,
   and `payment_streams_ui`.
2. Install them into an isolated Basecamp user directory (`.scaffold/basecamp-ui`).
3. Load the modules in order and verify that `Main.qml` renders without QML errors.
4. Verify the visual layout and mock transitions for initialize, deposit, create,
   close, and claim.

Gate:
`payment_streams_ui` loads inside Basecamp and completes the layout pass
cleanly.

### Slice 1, connect wallet and resolve program identity

Purpose:
establish wallet readiness, network mode display, and program identity resolution.

Files:

- `ui/Main.qml`
- `module/src/payment_streams_module_writes.cpp`
- `module/src/payment_streams_module_kit.cpp`

Work:

1. UI queries `logos_execution_zone` via `list_accounts` and
   `get_current_block_height`.
2. When public accounts are present, the UI prefills owner and provider fields.
3. When the wallet is closed or empty, the UI presents a clear readiness banner.
4. UI displays passive network mode and chain height indicators in the header
   (Mock mode simulation badge versus Live mode block height and network target).
5. Ensure `payment_streams_module` resolves the deployed `program_id_hex`
   consistently for PDA derivation and `chainAction` planning.

Gate:
Opening the plugin with an active wallet populates public accounts and
enables the initial lifecycle stage with the active network state visible.

### Slice 2, live lifecycle reads and writes

Purpose:
enable live on-chain payment stream operations through the UI.

Files:

- `ui/Main.qml`

Work:

1. Connect `submitLiveAction` to `payment_streams_module.chainAction` for:
   `initializeVault`, `deposit`, `createStream`, `closeStream`, `claim`.
2. Use periodic QML timer ticks (`liveConfirmTimer`) to query `sync_to_block`
   and read vault and stream status until confirmed.
3. Keep live `deriveStage` transitions synchronized:
   `needVault` → `needDeposit` → `needStream` → `needClose` → `needClaim`.
4. Update the Recent transactions table on live submissions with short hashes
   and inclusion status.
5. Provide a toggle to switch between live mode and mock mode for demonstration
   flexibility.

Gate:
A user can complete the full lifecycle on localnet from the UI:
initialize vault, deposit, create stream, observe accrual, close, and claim.

### Slice 3, reproducible runner and walkthrough

Purpose:
provide a simple launch flow and clear reproduction instructions.

Files:

- `Makefile`
- `docs/reproduce/basecamp-ui.md`
- `docs/plan/upcoming/basecamp-rebuild-loop.md`

Work:

1. Add Makefile targets to build packages and launch Basecamp with the
   prepared user directory:
   `make basecamp-ui-build`
   `make basecamp-ui-run`
2. Document the step-by-step operator walkthrough for localnet and testnet.
3. Verify that the UI walkthrough matches the documented CLI reproduce steps.

Gate:
An operator can follow `docs/reproduce/basecamp-ui.md` on a fresh environment
and complete the full payment stream lifecycle.

## User test journeys

### Localnet

First run:

```bash
make basecamp-ui-build
make basecamp-ui-run
```

The operator opens Basecamp, loads the modules, and follows the highlighted
lifecycle actions in `payment_streams_ui`.

### Testnet

Run:

```bash
make basecamp-ui-build
make basecamp-ui-run
```

With a testnet wallet and fixture in the environment, the plugin displays
testnet accounts and allows running the live lifecycle against the public
sequencer.

## Scope boundaries

v1 includes:
public owner and provider accounts in one wallet, native token streams,
one active lifecycle on screen, owner-close, provider-close, claim,
and a read-only network and block indicator.

v1 excludes:
wallet creation and mnemonic setup inside QML, faucet integration,
in-app authenticated transfer registration, private zero-knowledge proving,
Store eligibility hooks, non-native tokens, dynamic network switching dropdowns,
and multi-vault navigation tables.

Setup tasks like account creation, funding, and authenticated transfer remain
handled by existing repository scripts and CLI helpers.

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
Iterate with `--user-dir`.

D21.4. Theme: `import Logos.Theme` and `import Logos.Controls`.
Page fill `Theme.palette.background`.
Visible copy and actions use `LogosText` and `LogosButton`.

D21.5. Session model for v1: one owner, one vault, one provider,
and one active stream at a time.
`owner`, `vault_id`, `provider`, and `stream_id` remain visible and
copyable.
On load, Session prefills from `logos_execution_zone` `list_accounts`.

D21.6. Layout for v1: one page, two action regions (Owner, Provider),
plus Session and On-chain state cards.
Recent transactions table appears below state in live mode.

D21.7. v1 actions, in lifecycle order:

- Owner: `initializeVault`, `deposit`, `createStream`, owner-close
  (`closeStream` with `provider` omitted or equal to `owner`)
- On-chain state: Refresh → `getVaultStatus` / `getStreamStatus`
- Provider: `claim`, provider-close (`closeStream` with distinct `provider`)

D21.8. Signing and wallet connection:
The UI calls `chainAction` on `payment_streams_module`.
The module submits transactions through `logos_execution_zone`.
v1 assumes two public accounts in the active wallet.

D21.9. Status: chain state after sync is the source of truth.
A `tx_hash` confirms submission.
Inclusion is confirmed by status reads after `sync_to_block`.

D21.10. Dependencies:
`ui/metadata.json` lists `payment_streams_module` and
`logos_execution_zone` in `dependencies`.

D21.11. Stream card displays:
`stream_state` (Active, Paused, Closed), rate, allocation, accrued,
unaccrued, accrual timestamp, and estimated depletion time.

D21.12. Balances:
Owner wallet balance and vault holding display in On-chain state.

D21.13. Lifecycle stage progression:
`needVault` → `needDeposit` → `needStream` → `needClose` → `needClaim` → `needStream`.
Deposit remains available whenever the vault exists.

D21.14. Field validation:
Account ids accept valid base58 or 64-hex strings.
Vault id, stream id, amounts, rate, and allocation validate as positive
integers before submission.

D21.15. Demo switch and network indicator:
A clean switch allows toggling between live chain interaction and
visual mock mode.
The header displays a read-only badge indicating active mode:
simulated offline mode when the demo switch is active,
or live on-chain status with current block height when connected to a sequencer.

D21.16. Inclusion polling:
`liveConfirmTimer` periodically triggers wallet sync and status checks
without blocking the UI thread.

D21.17. Payload construction:
`Main.qml` builds standard JSON payloads matching the `chainAction`
catalogue.

D21.18. Readiness detection:
The UI checks for an open wallet and available accounts on load,
displaying an informative banner if setup is required.

D21.19. Public execution scope:
This screen focuses on public vaults (`privacy_tier = 0`).

D21.20. Consolidated helpers:
Pure parsing and formatting functions remain concise and centralized.

D21.21. Recent transactions:
A session-scoped table displays submitted transaction hashes, action
names, and confirmation status.

## v1 screen

```text
title:          Payment Streams Dashboard (live / mock switch)
                network indicator (Simulated offline badge / Live block height)

session:        owner, vault_id, provider, stream_id
                (prefilled from wallet; editable)
                session status banner

on-chain state: Refresh action
                owner balance, vault holding, total allocated
                stream state, rate, allocation, accrued, unaccrued
                accrual checkpoint, chain time, estimated depletion
                previous closed streams summary

transactions:   Recent transactions table
                timestamp, action, short hash, status

owner actions:  initialize vault, deposit, create stream, owner-close

provider:       claim, provider-close
```

## Runtime

Runtime uses the same wallet and deployed programs as
[reproduce/module.md](../../reproduce/module.md).

Before the first public UI write:

- `logos_execution_zone` package is loaded;
- `payment_streams_module` and `payment_streams_ui` packages are loaded;
- the wallet contains two funded public accounts;
- authenticated transfer is registered for both accounts;
- the owner holds sufficient balance for deposit.

Launch recipe: [basecamp-rebuild-loop.md](basecamp-rebuild-loop.md).

## Host process and compatibility

`logos.callModule` invokes core modules running in `logos_host`
subprocesses.
Wallet handle and module configuration reside in those processes.

The logoscore and E2E scripts continue to function independently via
their established fixture manifests and CLI commands.

## Later work

After v1, the UI can expand to:

- pause, resume, top-up, and withdraw actions;
- multi-vault and multi-stream tables;
- interactive account switching;
- Store query and eligibility integration;
- private shielded streams and proving.

## Deliver

- `ui/` with `metadata.json`, `flake.nix`, and `Main.qml`
- Portable `.lgx` package buildable via Nix
- Live payment stream lifecycle execution in Basecamp
- Streamlined build and launch targets in `Makefile`
- Reproduction documentation in `docs/reproduce/basecamp-ui.md`

## Open questions

1. Testnet poll cadence:
   Default poll interval is `2s` with profile-adjusted timeout.

2. Stream id selection:
   Refresh selects the active or next available `stream_id` while allowing
   operator edit.
