> Historical snapshot. Canonical index: [integration-index.md](../../integration-index.md). Agents: [AGENT-BRIEF.md](../AGENT-BRIEF.md).

# Payment Streams integration plan

## Task Summary

The goal is to let a Logos Delivery Store request carry
a payment-stream-backed eligibility proof,
have a Store provider verify the proof against on-chain state,
and have the provider serve the request only when the proof is valid.

The integration is structured around the Logos Core module model.
Cryptographic and on-chain semantics live in Rust through a thin FFI.
A dedicated Logos Core module hosts payment-stream policy and proof handling.
Logos Delivery is extended only with a generic eligibility-verification hook
so that future eligibility schemes can plug into the same path
without changes to the delivery layer.

The deliverable is a demo.
Changes to external components are kept minimal but are not avoided
when they unblock the demo flow.
Spec changes are not negotiated with external teams during this work;
everything ships in our own branches and is revisited later.

In scope for the MVP demo are
LIP-155 transparent (public) vaults,
a single user and a single provider on the same local sequencer,
and an explicitly enabled paid Store mode on the provider.

Out of scope for the MVP are
LIP-155 `PseudonymousFunder` (private) vaults,
mixed paid and unpaid Store interoperability,
production hardening of off-chain key custody,
and any new wire format for protocols other than Store.

### Store query dependency

Steps that issue Store queries through `delivery_module` (Step 16 eligibility
routing, Step 17 demo, Step 18 UI) depend on upstream Store query exposure on
`logos-delivery-module` `master`. The Delivery roadmap is implementing that
capability with a different approach than our earlier
`logosdelivery_query_store` / `queryStore` PRs.

We do not integrate against those PRs, feature branches, or local forks of them,
and we will not implement or upstream our own Store query exposure (Step 6, closed).
Until the Delivery team ships Store query support on `master`, active work stays on
Steps 1–15: Rust and FFI through Step 5, logoscore path Steps 7–13 (Steps 10–11
cover chain fixture, wallet runtime, and module chain I/O), delivery-repo Steps 14–15
only (no Store query on `delivery_module`).

## Onboarding

### Recommended reading order

1. [`integration-index.md`](../../integration-index.md) (short index). Full text: this archive file.
   Steps 1–5 (Rust FFI), Steps 6–18 (integration and demo).
   Step 3 splits into 3a (core) and 3b (FFI).
   Step 6 records the closed Store-query decision; Step 8 (probe) is done; Step 9
   bootstraps the Universal module;    Steps 10–11 (fixture, wallet runtime, module chain I/O,
   including Step 11d LEZ 510 wallet upgrade — landed)
   precede eligibility in Steps 12–13.
   Definitions of done, decisions (D1–D6), and notes (N1–N11).
   Supporting doc index: [`docs/README.md`](docs/README.md).
2. `logos-architecture-overview.md` in this directory.
   Architectural facts about hosts versus modules,
   the Rust FFI layer inside a module versus `LogosAPI` between modules,
   Qt's three roles,
   C++ as the module-shell language,
   and the LEZ-only chain side.
   Read once end-to-end; refer back when terms feel ambiguous.
   That document uses "Boundary A" and "Boundary B" terminology
   for the two layers;
   this plan uses descriptive terms (Rust FFI and LogosAPI) instead.
3. `docs/step1-findings-scaffold-rpc.md` in this repo.
   Concrete, validated commands for launching localnet,
   deploying `lez_payment_streams`,
   and reading accounts through `getAccount`.
4. LIP-155 spec at `rfc-index/docs/ift-ts/raw/payment-streams.md`.
   Protocol source of truth for vault, stream, proof types, lifecycle,
   and `StreamProviderPolicy` (see `docs/step3-policy-and-implementor-notes.md`).
5. [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md).
   Why the payment-streams wallet dependency pins Git refs on feature branches
   or PR heads, what files encode those pins, and how to reproduce builds.
   Store querying is explicitly out of scope for that doc; see N6.
6. [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md).
   Step 7 first install, Step 9 Universal module, Steps 10–11 (chain fixture + module I/O),
   Steps 12+ dev loop (`lgpm`, `logoscore`, LEZ).
7. [`docs/step8-universal-legacy-probe-results.md`](docs/step8-universal-legacy-probe-results.md).
   Step 8 probe evidence and historical Universal vs Legacy appendix (D6).
8. [`docs/step3-policy-and-implementor-notes.md`](docs/step3-policy-and-implementor-notes.md).
   Step 3a policy summary and implementor notes (companion to plan Step 3a/3b).

### Prerequisites

- Nix with flakes enabled.
- Rust stable toolchain.
- `logos-scaffold` binary (`lgs` alias)
  for localnet, wallet, and program deploy.
- `logoscore`, `lgpm`, `lm` —
  use the three-package `nix shell` documented in
  [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md)
  (or equivalent flake-built binaries from the developer guide).
  Step 7 covers install (runtime guide Part 1); Step 9 module shape is Part 2 of the same guide.
- Outbound internet access for the `logos.dev` messaging-network preset
  during Step 17 and Step 18.
- A working `git` and the workspace already checked out
  with all repos pulled to their latest commits.

## Component Overview

For background on hosts vs. modules,
the two distinct boundaries (FFI inside a module versus `LogosAPI` between modules),
the role of Qt and C++,
and the scope of the LEZ-only localnet,
see `logos-architecture-overview.md` in this directory.

`lez-payment-streams`
is the existing on-chain SPEL program plus the `lez-payment-streams-core` package
(importable in Rust under `lez_payment_streams_core`),
with a sibling `lez-payment-streams-ffi` crate listed in the workspace `Cargo.toml`.
Core already ships Step 3a pieces (`fold_stream`, policy predicates, `StreamProviderPolicy`,
`StreamParams` with `rate` / `allocation` matching on-chain `StreamConfig`, and `PolicyRejectReason`).
The remaining Rust work extends `lez-payment-streams-ffi`:
PDA derivation and account decoders (Step 2),
exposing folding and policy across the C ABI (Step 3b),
then off-chain proof construction and verification,
and Borsh instruction builders through `extern "C"` (Steps 4–5).
The shape mirrors `lez-rln-ffi` in `logos-lez-rln`.

`payment_streams_module`
is a Universal Logos Core module (`"interface": "universal"`) that wraps the FFI.
It is built with `logos-module-builder` / `mkLogosModule` and
`src/payment_streams_module_impl.{h,cpp}` (see Step 9).
It owns session keys, pending-proposal state, user and provider eligibility flows.
It does not declare `logos_execution_zone` in `metadata.json`; wallet calls are
dynamic via `invokeRemoteMethod` (D6, Step 8).

`logos_execution_zone`
is the existing Logos Core module (repo `logos-execution-zone-module`)
that wraps `wallet_ffi`.
It is the single point of contact with the LEZ chain.
Reads of vault, stream, and clock accounts go through `get_account_public`.
Writes go through `logos_execution_zone` using
[PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19)
on LEZ
[PR 491](https://github.com/logos-blockchain/logos-execution-zone/pull/491)
(see [D3](#d3-wallet-write-path)).

`logos-delivery` and `liblogosdelivery`
host the Store protocol implementation in Nim and its C FFI surface.
This work extends the Store v3 wire format with an opaque eligibility-proof field
on requests and an opaque eligibility-status field on responses,
and adds a callback hook in the C FFI
that lets a host application register an eligibility verifier
and attach eligibility-proof bytes on outbound queries.
Neither layer learns anything about payment streams.

`logos-delivery-module`
is the existing Logos Core module
(repo `logos-delivery-module`; upstream release line is `v0.1.2`, not pinned in this repo until Step 16)
that wraps `liblogosdelivery` and exposes the typed `delivery_module` API
documented in the journey doc
[Use the Logos Delivery module API from an app](https://github.com/logos-co/logos-docs/blob/main/docs/messaging/journeys/use-the-logos-delivery-module-api-from-an-app.md).
This work adds two thin routing methods on its interface,
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
that bridge the new `liblogosdelivery` hooks to a named Logos module
via `LogosAPI` / `LogosAPIClient`.
When upstream lands Store query support, the module will expose the upstream
method (planned name in this doc: `queryStore(jsonQuery, peerAddr, timeoutMs)`)
so modules and apps can issue Store queries through `delivery_module`.
We do not ship or pin our own `queryStore` PR implementation; see N6.
At registration time the bridge uses each module's auto-generated
`getPluginMethods` surface to confirm that the named module exposes
the expected verifier and provider methods,
so a misconfigured registration fails fast with a structured error.
`payment_streams_module` is one such named module;
future modules (different incentivization schemes) can register the same way.
Eligibility hook changes in `logos-delivery` / `logos-delivery-module` (Steps 14–16)
ship on our branches until upstreamed; Store query consumption uses upstream
`master` once available, not our retired query-store PR branch.

`logoscore`
is the headless runtime used to load and exercise modules during integration testing.
The end-to-end demo runs two `logoscore` instances on one host,
one as user and one as provider,
each loading `logos_execution_zone`, `payment_streams_module`,
and `delivery_module` built from upstream `master` plus our eligibility-hook branch
when Step 16 is in progress (see Step 17).

`logos-basecamp`
is optional for the MVP.
A small `ui_qml` plugin can surface vault and stream state visually
once the headless integration is stable.

### Build and tooling repositories

`logos-module-builder`
is the Nix flake library that provides `mkLogosModule` and `mkLogosQmlModule`,
plus the `with-external-lib`, `ui-qml`, and `ui-qml-backend` templates.
It transitively pulls in `liblogos_core`, the C++ SDK,
and `nix-bundle-lgx` for `.lgx` packaging,
so a module flake rarely needs to declare those explicitly.

`scaffold` (binary `lgs`)
is the LEZ localnet CLI.
It builds and runs the standalone LEZ sequencer on `127.0.0.1:3040`,
manages the demo wallet, and deploys the `lez_payment_streams` program.
The exact command flow used by this work is recorded in
`docs/step1-findings-scaffold-rpc.md`.

`logoscore` (binary from `logos-logoscore-cli`),
`lgpm` (from `logos-package-manager`),
and `lm` (from `logos-module`)
are the headless tooling used throughout the steps:
host the modules, install `.lgx` packages, and inspect plugin surfaces respectively.
They are transitively available through `logos-module-builder`.

### Reference and precedent repositories

`logos-rln-module` inside `logos-lez-rln`
is the closest precedent for `payment_streams_module`.
It is a `core` module that wraps a Rust FFI crate (`lez-rln-ffi`)
and calls `logos_execution_zone` for chain reads and writes
via `LogosAPIClient::invokeRemoteMethod`.
We reuse its file layout,
cross-module `invokeRemoteMethod` patterns and hex/byte helpers.
Start at `src/logos_rln_module.cpp`.
RLN may still call a legacy JSON `send_public_transaction` where deployed;
payment streams chain writes follow [D3](#d3-wallet-write-path) (491 generic transactions).

`logos-delivery-module` (upstream `v0.1.2` on `master` as reference; extend on our branch in Step 16)
is the module we extend in Step 16.
Skim `README.md` and `src/delivery_module_plugin.h`
to see the surface we add `setEligibilityVerifier`
and `setEligibilityProvider` to.

`logos-delivery-demo`
is the reference `ui_qml` module showing how a Qt module consumes `delivery_module`.
It is the closest precedent for the optional Basecamp UI plugin in Step 18
and uses the typed `LogosModules` wrapper pattern from the journey doc.

`logos-tutorial`
carries the canonical developer guide
(`logos-developer-guide.md`)
and worked examples
(`logos-calc-module` for an external-library wrap,
`logos-calc-ui*` for UI variants).
First reading material for the module shape.
Read it once early;
it covers `metadata.json`, `mkLogosModule`, `lgpm`, `logoscore`, `lm`,
and the C++ SDK code generator.

`logos-execution-zone`
is the LEZ source repo
(sequencer, indexer, NSSA account model, on-chain programs, `wallet_ffi`).
This work does not modify it;
it is a transitive dependency through
`lez-payment-streams-core`'s Cargo dependencies on `nssa_core` and `clock_core`
and through `logos_execution_zone`'s wrap of `wallet_ffi`.

LIP-155
is the protocol specification for payment streams.
The local copy used during this work lives at
`rfc-index/docs/ift-ts/raw/payment-streams.md`.
The journey doc for `delivery_module` lives at
`logos-docs/docs/messaging/journeys/use-the-logos-delivery-module-api-from-an-app.md`.

## Decisions and Notes

Items resolved before implementation starts,
plus non-blocking notes carried forward into the demo.

### D1, Store wire format

Add an optional opaque `eligibility_proof` field on `StoreQueryRequest`
and an optional opaque `eligibility_status` object on `StoreQueryResponse`.
Tags currently used on the request are `1`, `2`, `10`, `11`, `12`, `13`, `20`, `51`, `52`, `53`.
The new fields take tags from a fresh block starting at `30`,
i.e. `eligibility_proof` at tag `30` on the request
and `eligibility_status` at tag `30` on the response.

Eligibility semantics are fully hidden from Store.
Store status codes (`status_code`, `status_desc`) remain reserved for query-execution outcomes.
Eligibility outcomes use a separate enumeration carried exclusively inside `eligibility_status`.
If eligibility fails,
the Store handler returns a `StoreQueryResponse`
with the existing `BAD_REQUEST` status code (`400`)
and an empty `messages` list,
skipping the database query.
The detailed eligibility verdict
(`OK`, `PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`)
lives only inside the `eligibility_status` object at tag `30`.
Those four codes are the Store-visible vocabulary; multiple distinct
`PolicyRejectReason` / FFI discriminant failures may map into `PARAMS_REJECTED`
on the wire (see Step 3a mapping table).
Store never interprets these values;
the payment-streams module on each side
reads and acts on the structured `eligibility_status` payload.
The `eligibility_proof` field carries a protobuf `EligibilityProof`.
`stream_proposal` and `stream_proof` contain serialized
`StreamProposal` or `StreamProof` messages (see LIP-155 LEZ integration).
No new status codes are added to the Store `StatusCode` enum.

No protocol-ID version bump and no codec migration is required for the demo.
Confirm tag `30` is unused in `waku/waku_store/rpc_codec.nim` before implementation.

### D2, Delivery module hook design

`liblogosdelivery` gains a generic registration entry point
that takes a verifier callback (called for inbound Store requests carrying an `eligibility_proof`)
and a path for attaching opaque eligibility-proof bytes to outbound Store queries.
`logos-delivery-module` (our branch for eligibility hooks) gains
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`
plus a `paidStoreMode` configuration toggle.
Store query on the module surface comes from upstream `master` (N6),
not from our retired `queryStore` PR branch.
The bridge validates the named module's surface at registration time
via the auto-generated `getPluginMethods` introspection.
Both layers stay payment-streams-agnostic.
On the eligibility hooks, opaque bytes mean the complete serialized protobuf `EligibilityProof`
(D1) as carried on `StoreQueryRequest` tag `30`: the registered module produces and parses that
blob; Delivery forwards it without interpreting `stream_proposal`, `stream_proof`, or other
extension fields. Future eligibility mechanisms can register a different module or populate
other `EligibilityProof` arms without changing that hook contract.
For outbound Store queries,
`delivery_module` passes the target provider's libp2p `PeerId`
as `providerPeerId` to the eligibility provider hook.
For inbound Store requests,
it passes the caller's libp2p `PeerId`
as `requesterPeerId` to the eligibility verifier hook.
It does not interpret either value or know about LEZ account IDs.
We ship this on our branches and do not negotiate spec changes.

### D3, Wallet write path

Reproducible flake refs and pin maintenance:
[`docs/feature-branch-pins.md`](docs/feature-branch-pins.md).
This section records integration intent; pin tables live in that doc only.

`payment_streams_module` chain writes go through `logos_execution_zone`, which delegates
to the generic public transaction APIs in `wallet_ffi`.

#### LEZ FFI — PR 491 (canonical)

Generic public transactions on LEZ `main` ([491 merged](https://github.com/logos-blockchain/logos-execution-zone/pull/491)).
Deprecated 429/16 wallet JSON path: [`docs/archive/superseded-wallet-pr-429-16.md`](docs/archive/superseded-wallet-pr-429-16.md).

491 exposes (among others):

- `wallet_ffi_resolve_public_account` — map each 32-byte account id to `FfiAccountIdentity`
  (signer vs read-only via `needs_sign`).
- `wallet_ffi_serialization_helper` — Borsh instruction bytes to RISC0 `u32` instruction words
  (same wire step as `lez-payment-streams-core` / `instruction_wire.rs`).
- `wallet_ffi_send_generic_public_transaction` — submit using ordered account identities,
  instruction words, and `FfiProgramWithDependencies` (guest program ELF plus dependency ELFs).

Private/generic PP paths exist in 491 but are out of scope for the MVP transparent vault demo.

#### Wallet module Qt surface — PR 19 (primary)

Upstream exposes 491 to Logos modules via
[logos-execution-zone-module PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19)
(`feat: general transactions flow`, branch `Pravdyvy/generic-transactions-extension`).
Same author and timeline as PR 491.

Primary path: pin and build the patched wallet wrapper against PR 19 head + LEZ `main`
(510 merge; includes 491 generic public tx — see [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md)).
Step 11b submits through PR 19 `send_generic_public_transaction` in the wallet; the Universal
module uses a repo-specific `send_generic_public_transaction_json` IPC helper (N10). Read PR 19
for the underlying QList request shape.

Our wallet work (Steps 10b and 11c, reduced scope):

- Step 10b: packaging — patched wrapper flake pins PR 491 + PR 19; CMake `wallet_ffi.h` include,
  codegen headers for dependents, `.lgx` bundle. Module id stays upstream `logos_execution_zone`.
  Do not reimplement generic public send if PR 19 already does. Step 11b adds guest-ELF-from-env and
  JSON submit patches on the same wrapper (see N10).
- Step 11c: `sign_public_payload` per [N1](#n1-off-chain-canonical-payload-signing) — not in
  491 or 19; add on our patched wrapper (LEZ FFI + Qt) until upstream ships it.

Do not pin [PR 429 / PR 16](docs/archive/superseded-wallet-pr-429-16.md).

#### Pinning

Pin `logos-execution-zone` to `main` at the current LEZ revision in
[`docs/feature-branch-pins.md`](docs/feature-branch-pins.md) (510 merge after Step 11d) and the wallet module upstream
input to `refs/pull/19/head` until PR 19 merges; then pin `main` on the wallet module repo.
LEZ is no longer pinned to `refs/pull/491/head` in this integration.

### D4, Wallet module runtime name

Use the upstream PR 19 Logos module id `logos_execution_zone`
(`metadata.json` `name`, `LogosExecutionZoneWalletModule::name()`,
plugin `logos_execution_zone_plugin`). The payment-streams wrapper flake adds
behavioral patches only (guest ELF from env, JSON submit helper, future
`sign_public_payload`); it does not rename the module (see Step 7).
Universal `payment_streams_module` does not list the wallet in `metadata.json`
(D6); load wallet before payment streams at runtime.
`logos-rln-module` may still call a wallet plugin registered under the historical
id `liblogos_execution_zone_wallet_module`.
That is unrelated to the payment-streams demo, which installs `logos_execution_zone`
from the patched wrapper (D4).

### D5, New module naming

Metadata `name` is `payment_streams_module`.
Plugin binary stem is `payment_streams_module_plugin`.
Repository directory is `logos-payment-streams-module`,
placed as a sibling of `lez-payment-streams-core/` and `lez-payment-streams-ffi/`
inside the existing `lez-payment-streams` repo
(mirroring `logos-lez-rln`,
which co-locates `lez-rln/` and `logos-rln-module/`).

Justification.
The metadata name follows the protocol-named convention of `delivery_module`:
the module speaks the LIP-155 payment-streams protocol,
with the LEZ-specific bits behind its FFI.
The wallet module keeps upstream PR 19 naming (`logos_execution_zone`), not a separate
`lez_`-prefixed id in this integration.
Snake_case in the metadata name plus a `-module` kebab-case suffix on the directory
matches every existing module in the ecosystem.
Co-locating the module with the SPEL program and the FFI crate
keeps the demo versioned as one unit and matches the `logos-lez-rln` precedent.
Extracting into a separate repository remains an option after the demo stabilises.

### D6, Universal module interface

Build `payment_streams_module` with `"interface": "universal"` and
`LogosModuleContext` (`payment_streams_module_impl.{h,cpp}`).
Do not restore the Legacy `PluginInterface` shell.
Archived bootstrap notes live under `docs/archive/`.

Call `logos_execution_zone` at runtime via
`modules().api->getClient("logos_execution_zone")->invokeRemoteMethod(...)`.
Keep `"dependencies": []`.
Step 8 validated Universal caller to Legacy callee.

Justification.
Universal static dependencies exist so codegen can emit typed `modules().<name>` wrappers.
That assumes every dependency is Universal.
`logos_execution_zone` is still Legacy, so listing it in `metadata.json` would not produce safe typed calls to its `Q_INVOKABLE` API.
Dynamic access keeps payment streams on the Universal side (with `delivery_module`) while the wallet stays Legacy.
We rely on explicit load order and runtime errors if the wallet is absent.
Revisit a static dependency when the wallet module is Universal upstream and codegen supports it.

### N1, Off-chain canonical-payload signing

Neither `wallet_ffi` nor `logos_execution_zone` currently exposes
a primitive that signs an arbitrary canonical payload with a wallet account's key.
That primitive is required for `VaultProof.owner_signature`,
because the vault proof must prove control of the LEZ vault owner key.
For the MVP, we add `sign_public_payload` to `logos_execution_zone`
on our branch (see Step 11c; patch delivery uses `lez-wallet-ffi-patched`).

Decided call convention (Step 11c):
`sign_public_payload(account_id_hex: QString, digest_hex: QString) -> QString`
where `account_id_hex` is the 64-char hex account ID (same format as
`get_account_public` and `get_public_account_key`; convert base58 with
`account_id_from_base58` first if needed),
`digest_hex` is the 64-char hex SHA-256 digest (32 bytes),
and the return is a JSON envelope `{"status":"ok","result":"<128-char hex>"}`.
Verification uses `smoke_verify` from `lez-payment-streams-ffi`
(same `verify_canonical_payload_digest` path as the Step 13 provider FFI).

No domain parameter is included.
The NSSA wallet avoids exposing generic signing entirely
(`wallet_ffi` only exposes complete transaction workflows;
see `logos-execution-zone/wallet-ffi/src/transfer.rs`).
Our method introduces the first generic signing endpoint,
but any co-hosted module can already submit arbitrary transactions
via `send_public_transaction`, which is strictly more powerful.
Domain separation would not reduce the attack surface
in the current trust model.
The payment-streams FFI already builds a domain-prefixed `canonical_payload`
and hashes it to a 32-byte `canonical_payload_digest` for signing
(see [N8](#n8-canonical-store-request-bytes-format)).
If the ecosystem later introduces a module permission model,
domain separation on signing should be revisited.

Payment-stream proofs use NSSA's existing transparent signature contract:
32-byte x-only secp256k1 public keys,
64-byte Schnorr signatures,
and signatures over a 32-byte `canonical_payload_digest`
(SHA-256 of that `canonical_payload`).
This matches the `nssa::PublicKey` and `nssa::Signature` types
already used for public and privacy-preserving LEZ transactions.

`VaultProof.owner_signature` is signed by the LEZ vault owner key.
The verifier checks that the provided owner public key derives to
the owner account stored in `VaultConfig`,
then verifies the signature over the canonical vault-proof payload.
That payload covers the vault proof fields,
the proposed stream parameters,
and the `StreamProposal.public_key`,
so the owner authorizes this exact proposal and session key.

`StreamProposal.public_key` is a module-generated session public key.
`payment_streams_module` owns the matching session private key
and persists it in `instancePersistencePath`.
`StreamProof.signature` is signed by that session key over
the canonical Store request payload.
Session-key persistence policy is covered in N4.

### N2, Read freshness from the wallet module

`get_account_public` returns whatever state the underlying wallet client returns,
without distinguishing live sequencer state from indexer-finalized state.
The MVP treats it as authoritative for hot-path eligibility verification,
which is acceptable on a local sequencer where finality lag is small.

### N3, Provider-side verification latency

Routing eligibility verification from Nim through `liblogosdelivery` to `delivery_module`
to `payment_streams_module` adds two IPC hops per Store request,
plus wallet-module chain reads inside the verifier.
The MVP accepts this; the hook is implemented as a synchronous `Future`-returning callback
that fits the existing Store handler shape.

### N4, Persistence policy

`payment_streams_module` persists pending-proposal state and per-stream session keys
as a flat JSON file `payment_streams_state.json` in `instancePersistencePath`, atomically written.
If persistence fails (disk full, permissions), the module logs the error
and continues with in-memory state only.
Stale proposals are evicted on cold start and when eligibility/inventory APIs run, by comparing
stored `create_stream_deadline` to clock-10 (no background timer required for the MVP demo).
Session private keys are stored as lowercase hex in that JSON for the demo; treat the persistence
directory as sensitive. A hardened build would encrypt session keys through a wallet-rooted KDF
or keep them ephemeral.

### N5, Provider identity mapping

LIP-155's `provider_id` remains the generic provider identity
used by the payment-stream protocol for replay protection
and provider-specific policy.
The LEZ demo puts the 32-byte stream payee `AccountId` in
`VaultProof.provider_id`.
Predicates compare it to `StreamConfig.provider` with octet equality.
The provider's libp2p `PeerId` is used only for Store routing.
For the MVP, the host configures `payment_streams_module` with a mapping
from `PeerId` to that `AccountId` (e.g. `registerProviderMapping`).
This keeps the delivery layer agnostic to payment-stream identity
while fixing the bytes used in proofs and on-chain streams.

### N6, Delivery module Store query exposure

Store retrieval through `delivery_module` is an upstream deliverable on the
Delivery roadmap, not something this integration implements locally (Step 6,
abandoned; done, won't fix).

We opened exploratory PRs (`logosdelivery_query_store` /
`queryStore`) that exposed existing `liblogosdelivery` Store query hooks.
Those PRs are not the integration path: upstream is implementing Store access
with a different design. We wait for that work on `logos-delivery` and
`logos-delivery-module` `master` and do not pin, fork, or maintain our PR
branch (`feat/liblogosdelivery-query-store`) in payment-streams flakes.

Steps 16–17 assume an upstream module method with the same call shape planned
for the demo (`queryStore(jsonQuery, peerAddr, timeoutMs)` or whatever name
ships on `master`). Until then, all other integration steps proceed in parallel.

`logos-chat` is not a reusable Store path.
We checked it at `origin/main` (`3a5f508`) and at the `logos-chat-module`
flake pin (`53302e4`): the embedded delivery node mounts only metadata,
filter, and relay (`src/chat/delivery/waku_client.nim`), issues no Store query,
and exposes no Store method.
Chat fetches messages live over relay/filter, so it neither uses nor
re-exports the Store protocol and does not shortcut the upstream dependency above.

### N7, Session key concurrency

Session key signing is synchronous.
Concurrent Store queries to the same provider serialize at the session key.
The MVP assumes low concurrency; production would need key pooling or async queuing.

### N8, Canonical Store request bytes format

Normative spec for Store eligibility canonical bytes in this integration.
Step 4, Step 15, and Step 3b reference this section; do not copy the struct
layout elsewhere.

The Store eligibility `canonical_payload` is the concatenation
of the domain `PREFIX` and Borsh(`CanonicalStoreRequest`) (see below).
`StreamProof.signature` signs `canonical_payload_digest = SHA-256(canonical_payload)`;
`payment_streams_module` recomputes that digest when verifying.
They are produced by `liblogosdelivery` (Nim, Step 15)
and consumed by `lez-payment-streams-ffi` (Rust, Step 4).
Both sides must produce identical `canonical_payload` for the same input.

#### Domain prefix

```
b"/LEZ/v0.1/StoreEligibility/\x00\x00\x00\x00\x00"
```

Padded to exactly 32 bytes with null bytes,
matching the `PREFIX` pattern in `nssa/src/public_transaction/message.rs`.

#### `CanonicalStoreRequest` struct

The struct is Borsh-serialized in the following field order.
This matches `StoreQueryRequest` from `waku/waku_store/common.nim`
with `eligibility_proof` excluded
and response-only fields absent.

| Field | Borsh type | Source |
| --- | --- | --- |
| `request_id` | `string` (4-byte LE length + UTF-8 bytes) | `StoreQueryRequest.requestId` |
| `include_data` | `u8` (0 or 1) | `StoreQueryRequest.includeData` |
| `has_pubsub_topic` | `u8` (0 or 1) | presence flag |
| `pubsub_topic` | `string` (only if present) | `StoreQueryRequest.pubsubTopic` |
| `content_topics_count` | `u32` LE | length of `contentTopics` |
| `content_topics[i]` | `string` each | `StoreQueryRequest.contentTopics` |
| `has_start_time` | `u8` (0 or 1) | presence flag |
| `start_time` | `i64` LE (only if present) | `StoreQueryRequest.startTime` |
| `has_end_time` | `u8` (0 or 1) | presence flag |
| `end_time` | `i64` LE (only if present) | `StoreQueryRequest.endTime` |
| `message_hashes_count` | `u32` LE | length of `messageHashes` |
| `message_hashes[i]` | 32 bytes each | `StoreQueryRequest.messageHashes` |
| `has_pagination_cursor` | `u8` (0 or 1) | presence flag |
| `pagination_cursor` | 32 bytes (only if present) | `StoreQueryRequest.paginationCursor` |
| `pagination_forward` | `u8` (0 or 1) | `StoreQueryRequest.paginationForward` |
| `has_pagination_limit` | `u8` (0 or 1) | presence flag |
| `pagination_limit` | `u64` LE (only if present) | `StoreQueryRequest.paginationLimit` |

Borsh `string` encoding is a 4-byte little-endian length prefix
followed by the raw UTF-8 bytes (no null terminator).
Optional fields use a presence byte:
`0x00` means absent (field bytes omitted),
`0x01` means present (field bytes follow immediately).

#### Canonical payload digest

```
canonical_payload = PREFIX || borsh(CanonicalStoreRequest)
canonical_payload_digest = SHA-256(canonical_payload)
```

This 32-byte `canonical_payload_digest` is what `StreamProof.signature` signs.

#### Cross-language test vector

The definition of done for Step 15 requires a pinned test vector:
construct a `StoreQueryRequest` with fixed known field values,
produce canonical bytes from the Nim serializer and the Rust serializer independently,
and assert byte-level equality.
This mirrors the `hash_public_pinned` test
in `nssa/src/public_transaction/message.rs`
that spells out the expected Borsh encoding byte by byte.

Step 12 demos and `./scripts/verify-step12-dod.sh` use the same field values via Rust:
`cargo run -p lez-payment-streams-core --bin n8_canonical_wire_hex` prints lowercase hex of the
full `canonical_payload` (32-byte domain prefix + Borsh body, 177 bytes for the reference
fixture). LogosAPI `canonicalRequestBytes` must be that full wire, not Borsh-only bytes.
Digest checks use `store_eligibility_digest_matches_n8_reference_fixture` in
`lez-payment-streams-core/src/off_chain/canonical.rs`.

### N9, Step 10a local chain fixture (decisions)

Scaffold config and runtime layout

- Commit `scaffold.toml` in this repo (LEZ/SPEL pins move with the integration).
- Do not commit sequencer or wallet state. Use `SCAFFOLD_WS` outside the git tree
  or a gitignored `.scaffold/` under the repo; keys and `.scaffold/state/` stay local.

LEZ pin

- `[repos.lez]` in `scaffold.toml` uses the same revision as LEZ PR 491 and
  [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md) / `nix/payment-streams-ffi.nix`
  (no separate “old LEZ” localnet for 10a and 491 for 10b). Re-run `lgs setup` after pin bumps.

Deploy (canonical for 10a script and runbook)

- After `lgs init`, `lgs setup`, `lgs localnet start` (from the chosen workspace):
  from repo root, `make build`, `make idl`, `make deploy` (`wallet deploy-program` on the guest
  binary), then `make program-id` into the fixture manifest.
- Operator detail and RPC formats: [`docs/step1-findings-scaffold-rpc.md`](docs/step1-findings-scaffold-rpc.md)
  (scaffold discovery doc — not integration plan Step 1, which is Rust FFI only).

Fixture scope

- Step 10a seeds the full demo chain state in one pass: fund owner, deploy program,
  `initialize_vault`, deposit, and `create_stream` for a designated demo vault/stream
  (CLI / IDL + `wallet` until Step 11b), so Step 11a can decode all account types.
- PDAs in the manifest come from the same derivation as `lez-payment-streams-ffi` tests,
  not fragile SPEL CLI account-seed helpers.
- Step 11b still proves module-driven lifecycle; use a fresh `vault_id` or reset chain
  when testing init from `payment_streams_module`, not an empty chain at 11a.

Artifacts

- Idempotent seed script, gitignored `fixtures/localnet.json`, committed
  `fixtures/localnet.json.example`, brief runbook under `docs/`.
- Runbook troubleshooting: owner or `SIGNER_ID` vs wallet home, and foreign localnet on
  3040 — [`docs/step10a-local-chain-fixture.md`](docs/step10a-local-chain-fixture.md)
  (Troubleshooting).
- Step 10a operator troubleshooting and verify failures —
  [`docs/step10a-handoff-and-follow-up.md`](docs/step10a-handoff-and-follow-up.md).
- Why `seed_localnet_fixture` inflates workspace `Cargo.lock` (fat LEZ `wallet`/`lee` deps,
  dual LEZ pins) and optional slimming paths —
  [`docs/step10a-local-chain-fixture.md`](docs/step10a-local-chain-fixture.md)
  (Seed binary and workspace Cargo.lock).

SPEL-on-LEE cleanup (public PDA prefix)

Published SPEL (`v0.5.0`) derives in-guest public PDAs with the NSSA prefix
(`/NSSA/v0.2/...`). LEZ PR 491 and `lez-payment-streams-core` use LEE
(`/LEE/v0.2/...`). Until upstream SPEL matches LEZ, this repo vendors
`vendor/spel-framework-core` and patches `compute_pda` to call
`lee_core::AccountId::for_public_pda` (root `Cargo.toml` and
`methods/guest/Cargo.toml` `[patch]` on `spel-framework-core`).

When SPEL officially targets LEE, that vendor fork should be removable.
Do not drop it on a version bump alone; confirm upstream `compute_pda`
matches host PDA derivation (FFI tests) and 491 localnet
(`initialize_vault` without `MismatchedPdaClaim`).

Then simplify: remove `vendor/spel-framework-core`, remove both
`[patch."https://github.com/logos-co/spel.git"]` entries, bump the SPEL
pin if needed, `make build`, full 10a chain reset, and
`./scripts/verify-step10a-dod.sh` exit 0.

The guest deposit `authenticated_transfer` enum encoding is implemented in tree; SPEL-on-LEE may
allow removing that shim later — verify deposit on 491 before deleting it.

### N10, Step 11b module writes (decisions)

Wallet submit and module shape

- Submit via PR 19 `send_generic_public_transaction` inside the wallet. The Universal module
  calls `send_generic_public_transaction_json` (one JSON string) over LogosAPI because
  QList-shaped cross-module IPC to the Legacy wallet is unreliable.
- Instruction bytes passed to the wallet are Borsh-serialized guest instruction bytes as
  `QList<uint8_t>`; the wallet runs `wallet_ffi_serialization_helper` to LE u32 words (not
  caller-supplied decimal string lists).
- Guest ELF: Step 10a `lez_payment_streams.bin` with `PAYMENT_STREAMS_GUEST_BIN` on the daemon;
  the PS module omits the ELF blob from IPC when that env var is set. Deposit uses wallet
  `authenticated_transfer_elf()` as a dependency when deps are empty. Bundling guest ELF inside
  the PS `.lgx` remains a follow-on. **Step 11d** ([LEZ PR 510](https://github.com/logos-blockchain/logos-execution-zone/pull/510))
  should replace or narrow this env-var path once deploy and program ELF are registered through
  `wallet_ffi` and exposed on `logos_execution_zone`.
- Nine write operations plus two status queries are implemented on the impl class; a single
  public `chainAction(operation, paramsJson)` router exposes them on the LogosAPI surface
  (see [N11](#n11-universal-module-public-api)). `signing_requirements` are derived
  from the signer vs the FFI-planned account list.
- Submit-level JSON only (`success`, `tx_hash`, `error`) from writes. Callers and
  `./scripts/verify-step11b-dod.sh` use wallet `sync_to_block` when sequencer height is
  available, retries on status `chainAction`, and may SKIP status when derived PDAs are not yet
  readable after successful submits.

E2E signer and wallet (G)

- Writes take signer/provider base58 in `chainAction` JSON. DoD uses manifest
  `owner_account_id` and `provider_account_id` with wallet storage copied from
  `.scaffold/wallet/storage.json` into the e2e dir (seeded owner keys).
- Module-driven lifecycle uses `vault_id` 1 by default or a reset chain (`reserved_for_step_11b`
  in the manifest). Demo vault `0` stays for Step 11a decode only.

Fixture and config (H)

- Chain fixture: gitignored `fixtures/localnet.json` (template `fixtures/localnet.json.example`).
  Override with env `FIXTURE_MANIFEST`.
- The module loads `program_id_hex` and related manifest fields once (init or first chain use).
  Write and status helpers do not take `program_id` on every call. Default clock account is
  `CLOCK_10` from the manifest when needed.
- Wallet sequencer RPC comes from `wallet_config.json` (`sequencer_addr`). Manifest
  `sequencer_url` documents the expected endpoint for operators and verify scripts.

### N11, Universal module public API

Universal modules (`"interface": "universal"`) export every `public:` method on
`PaymentStreamsModuleImpl` through `logos-cpp-generator --from-header` (plugin
`callMethod` / `getMethods`). There is no fixed cap on method count. Reserved
`LogosModuleContext` hooks (`onContextReady`, `modules`, `modulePath`, `instanceId`,
`instancePersistencePath`) are not exported.

Step 11b uses a `chainAction` router to keep write/status operations behind one
Logos entry point; that is an API ergonomics choice, not a codegen limit. Step 12
adds named eligibility methods; Step 16 registration must match those names exactly
(for example `prepareEligibilityForStoreQuery`).

## Integration Steps

Each step is independently testable.
The definition of done is a statement that can be objectively verified
without reading the implementation.
Step 3 is intentionally split into Step 3a (core) and Step 3b (FFI)
so policy and fold logic ship with tests before the C ABI wraps it.

Doc index: [`docs/README.md`](docs/README.md).

### Step map (integration tail)

| Step | Focus | Notes |
| --- | --- | --- |
| 6 | Store query via `delivery_module` | Closed decision; wait on upstream (N6) |
| 7 | Operator install | [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) Part 1 |
| 8 | Universal → Legacy wallet probe | Done; [`docs/step8-universal-legacy-probe-results.md`](docs/step8-universal-legacy-probe-results.md) |
| 9 | Universal module bootstrap | Done; [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) Part 2 |
| 10 | LEZ fixture + wallet runtime | 10a chain fixture ([N9](#n9-step-10a-local-chain-fixture-decisions)); 10b wallet (510 + PR 19) |
| 11 | Module chain access | 11a reads; 11b writes + status ([N10](#n10-step-11b-module-writes-decisions)); 11c `sign_public_payload`; 11d wallet deploy/ELF ([LEZ 510](https://github.com/logos-blockchain/logos-execution-zone/pull/510), landed) |
| 12 | User eligibility | Complete — [`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md), `./scripts/verify-step12-dod.sh` |
| 13 | Provider eligibility | Complete — [`docs/step13-provider-eligibility.md`](docs/step13-provider-eligibility.md), `./scripts/verify-step13-dod.sh` |
| 14–15 | Store wire + `liblogosdelivery` hooks | Nim/C repos; no logoscore loop |
| 16–18 | Routing, E2E demo, Basecamp UI | Blocked on upstream Store query (Step 6) |

Step 10 and Step 11 use lettered sub-steps (same convention as Step 3a/3b).
Document order: 10a → 10b → 11a → 11b → 11c → 12 → 11d → 13.
Execution order (current): Steps 12, 11d, and 13 are complete in tree. Next focus is Steps
14–16 (Store wire in `logos-delivery`, `liblogosdelivery` hooks, `delivery_module` routing)
when upstream Store query (Step 6) is available on the branch you integrate.
Step 17 E2E still needs delivery wiring plus Steps 12–13 on a fresh fixture; use
[`demo-localnet-recovery.md`](docs/demo-localnet-recovery.md) for local demos.

### Step 1, Bootstrap the Rust FFI crate

Architectural context:
Steps 1 through 5 build the Rust artifacts for `payment_streams_module`:
`lez-payment-streams-ffi` (Steps 1–2 and 3b–5)
and `lez-payment-streams-core` (Step 3a is implemented in core before Step 3b wraps it).
No Qt plugin shell, no Logos host, no chain are involved yet —
this is pure Rust crate work that will later be linked into the module.

The repo already has `lez-payment-streams-ffi` as a sibling crate to `lez-payment-streams-core`
and both are workspace members in the root `Cargo.toml`.
Bootstrap or extend that crate to mirror the `lez-rln-ffi` shape
(`crate-type = ["rlib", "cdylib", "staticlib"]`,
`cbindgen` build script,
generated `lez_payment_streams_ffi.h`).
Keep a stub function and an error enum in place if the pipeline is still being wired;
replace stubs as later steps land.

Components required to run: none.
Cargo plus a working Rust toolchain.

Definition of done:
`cargo build` produces `cdylib` and `staticlib` artifacts on a clean checkout,
the generated header exists and compiles when included from a C source,
and a placeholder unit test runs in CI.

### Step 2, Account decoding and PDA derivation in the FFI

Expose Borsh decoders for `VaultConfig`, `VaultHolding`, `StreamConfig`,
and the clock account data.
Expose PDA derivation that takes the deployed program ID and the relevant seeds
and returns 32-byte account IDs for the vault config, vault holding, stream config,
and clock accounts.
Decoders are read-only and infallible apart from version and length checks.

Components required to run: none.

Definition of done:
each decoder round-trips against fixtures generated by `lez-payment-streams-core`,
and PDA derivation produces account IDs that match the values
already recorded in `docs/step1-findings-scaffold-rpc.md`
for a known program deployment.

### Step 3a, Core stream folding and provider policy predicates

Policy and implementor detail: [`docs/step3-policy-and-implementor-notes.md`](docs/step3-policy-and-implementor-notes.md).

Architectural context:
pure `lez-payment-streams-core` work.
No `lez-payment-streams-ffi`, no C ABI, no Qt.
This is the single source of truth for fold math and policy comparisons
so Step 3b stays a thin wrapper.

Semantic reference:
LIP-155 at `rfc-index/docs/ift-ts/raw/payment-streams.md`
(StreamProviderPolicy, protocol phases, predicate names).
See also [`docs/step3-policy-and-implementor-notes.md`](docs/step3-policy-and-implementor-notes.md).

#### Fold

`fold_stream` applies `StreamConfig::at_time` for a caller-supplied `as_of` timestamp
and returns `StreamFoldedAtTime`: folded config, `accrued`, `unaccrued`, and `as_of`.
`unaccrued` is remaining principal not yet accrued at that time (not vault unallocated balance).

#### StreamProviderPolicy

Rust struct mirroring LIP-155 (`lez_payment_streams_core::StreamProviderPolicy`):

- `min_rate`, `min_allocation`
- `max_create_stream_deadline_delay`
- `vault_proof_max_response_bytes`

Proposal-phase solvency in core does not read accounts by itself: callers build
`ProposalCheckInputs` (see `lez-payment-streams-core/src/stream_provider_policy.rs`) with
`vault_holding_balance` and `vault_total_allocated` from decoded accounts
(`VaultHolding` balance and `VaultConfig::total_allocated`, or equivalent RPC snapshots).
Core computes `unallocated_balance(holding, total_allocated)` (saturating subtract) and requires
`params.allocation <= unallocated`, equivalent to the LIP-155 unallocated check.
`StreamParams::allocation` uses the same name and scale as on-chain `StreamConfig::allocation`
after `create_stream`.
`VaultProof` wire fields per LIP-155 [LEZ off-chain integration](rfc-index/docs/ift-ts/raw/payment-streams.md#lez-off-chain-integration).

#### Policy predicates

These pure functions live in `lez_payment_streams_core::policy` (same public names as LIP-155).
Each returns `Result<(), PolicyRejectReason>` (Rust enum; map to FFI in 3b).
Document test vectors for each:

| Function | Phase | Inputs (summary) |
| --- | --- | --- |
| `proposal_satisfies_policy` | Proposal | `ProposalCheckInputs` (`stream_provider_policy.rs`): minima on `params.rate` / `params.allocation`; deadline band via `create_stream_deadline_satisfies_policy_as_of` using `now` (LEZ: clock-account timestamp); solvency `params.allocation <= unallocated_balance(vault_holding_balance, vault_total_allocated)` |
| `new_stream_satisfies_proposal` | Service (first `StreamProof` per service session) | Folded `StreamConfig` at verification time; on-chain `allocation` and `rate` ≥ accepted `StreamParams`; `StreamConfig.provider` vs `VaultProof.provider_id` (LEZ: 32-byte equality). Later proofs need not run it (MAY re-run). |
| `stream_satisfies_policy` | Service (every `StreamProof`) | Folded `StreamConfig` already evaluated at the verifier's clock; state `ACTIVE`; provider binding; `folded_stream.rate` ≥ `accepted_terms.policy_at_acceptance.min_rate` and ≥ `accepted_terms.params.rate`. Does not re-check `allocation` on every proof — use `new_stream_satisfies_proposal` on the first proof for the allocation cap. |
| `response_within_policy` | Provider outbound | `response_len ≤ vault_proof_max_response_bytes` (MVP: enforce on first vault-proof `OK`; reject or trim) |

The provider learns `stream_id` from the first `StreamProof`.
Chain monitoring before that is optional (LIP-155).

`proposal_satisfies_policy` and `stream_satisfies_policy` SHOULD run on
user preflight and on provider verification so rejections stay aligned.

`service_id` is not part of `stream_satisfies_policy`.
The module MUST compare accepted `StreamParams.service_id` to the
configured service identifier before serving (demo: UTF-8
`/vac/waku/store-query/3.0.0`).
On-chain predicates read `StreamConfig` payment fields only.

`response_within_policy(response_len, policy)` is a core helper for
outbound response sizing, not inbound `StreamProof` verification.

#### PolicyRejectReason and eligibility mapping

Core uses `PolicyRejectReason` (`#[repr(u32)]` in Rust with fixed discriminants
`0`–`8`, plus `#[non_exhaustive]`; see `lez-payment-streams-core`).
FFI Step 3b should expose stable integer verdict codes mapped from those
discriminants, not the Rust enum’s memory layout treated as `repr(C)`.
`payment_streams_module` maps variants to LIP-155 eligibility codes, for example:

| PolicyRejectReason (core) | Eligibility status |
| --- | --- |
| `RateBelowPolicyMin`, `AllocationBelowPolicyMin`, `CreateStreamDeadlineInvalid`, `UnallocatedInsufficient`, `RateBelowAcceptedParams`, `AllocationBelowAcceptedParams`, `ProviderMismatch`, `ResponseTooLarge` | `PARAMS_REJECTED` |
| `StreamNotActive` | `STREAM_NOT_ACTIVE` |
| Signature / wire format / malformed proof | `PROOF_INVALID` (Step 4; not `PolicyRejectReason`) |

Finer splits inside `PARAMS_REJECTED` are optional on the wire (D1).
Variant list and predicates: `docs/step3-policy-and-implementor-notes.md`.

#### Out of scope for Step 3a

- Cryptographic binding (`VaultProof` / `StreamProof` signatures) — Step 4.
- Load cap (stateful metering) — LIP-155 extension; MVP deferred.
- Discovery wire encoding for policy — future; core defines struct + checks only.

#### Implementor clarifications

- Units: `rate` and `allocation` in `StreamParams` use the same
  integer scales as on-chain `TokensPerSecond` and `Balance` (native token).

- Response cap: demo default 65536; LIP-155 SHOULD, demo provider MUST
  (see `response_within_policy` above).

- Step 3a uses typed Rust policy inputs only (no protobuf in core).
  See `docs/step3-policy-and-implementor-notes.md` for suggested structs,
  `PolicyRejectReason` variants, predicate pitfalls, and vector checklist.

Components required to run: none.
`cargo test` on `lez-payment-streams-core` only.

Definition of done:
`cargo test -p lez-payment-streams-core` passes;
folding outputs match `StreamConfig::at_time` on a documented vector set;
each predicate (including `response_within_policy`) is deterministic with
documented pass/fail inputs;
vectors live in-repo (`docs/step3-policy-and-implementor-notes.md` and/or test modules)
and are reused verbatim in Step 3b.

### Step 3b, FFI exposure for folding and policy

Architectural context:
`lez-payment-streams-ffi` only.
Call Step 3a implementations only;
do not duplicate folding or policy arithmetic here.

Implementor notes (Step 3a as-shipped in `lez-payment-streams-core`):

- Map `PolicyRejectReason` by `u32` discriminant (`0`–`8`), not by
  assuming a C layout identical to Rust’s `enum`.
- `response_within_policy(response_payload_byte_len, policy)` — argument
  order matches core (subject size, then policy pointer).
- `create_stream_deadline_satisfies_policy_as_of` is public in core for
  deadline-only checks without assembling full `ProposalCheckInputs`.
- `StreamFoldedAtTime` carries `Balance` (`u128`) in `accrued` / `unaccrued`;
  split wide amounts for C using the same low/high pattern as other
  `PaymentStreams` FFI types.
- `MAX_SERVICE_ID_LEN` documents the intended `StreamParams.service_id` cap;
  core does not reject overlong `Vec<u8>` — enforce length in the module
  before signing (Step 4 owns wire validation).
- `AcceptedStreamTerms.provider_id` is `AccountId` in Rust (32-byte id).

- Policy verdicts vs FFI status: keep decode/null-pointer/version problems in
  `PaymentStreamsFfiStatus`. Surface `PolicyRejectReason` as its own stable `u32`
  discriminant on the failure path (or an out-parameter / small `repr(C)` result
  struct). Do not encode `RateBelowPolicyMin`, `UnallocatedInsufficient`, etc. as
  extra `PaymentStreamsFfiStatus` variants, or call sites lose the distinction
  between malformed inputs and an honest policy rejection.

- `repr(C)` mirrors of `StreamProviderPolicy` and `StreamParams` should match
  core field names and scales (`min_rate`, `min_allocation`, `rate`, `allocation`,
  `create_stream_deadline`, …) so generated headers stay searchable next to Rust.

- `ProposalCheckInputs` carries `vault_holding_balance` and `vault_total_allocated`
  as LEZ `Balance` (`u128`). Split them with the same `_lo` / `_hi` convention as
  `total_allocated` in `PaymentStreamsFfiDecodedVaultConfig`, not a different
  wide-integer representation.

Expose `extern "C"` entry points for stream folding and policy verdicts,
using stable `repr(C)` structs and enums alongside the existing
`PaymentStreamsFfiStatus` pattern.
Expose `PolicyRejectReason` equivalents (map `u32` discriminants from core) through the C ABI.
Map core errors to stable FFI status codes.
Behavior must stay deterministic independent of host endianness
(existing decode paths already use explicit low/high limbs for wide
integers).

Components required to run: none.

Definition of done:
the Step 3a vectors exercise the new symbols through the C ABI;
`cbindgen` output stays in sync;
no policy or fold math duplicated outside `lez-payment-streams-core`.

### Step 4, Off-chain proof types and canonicalization in the FFI

Align wire layouts and signed-field names with LIP-155
(`rfc-index/docs/ift-ts/raw/payment-streams.md`), including
[LEZ off-chain integration](rfc-index/docs/ift-ts/raw/payment-streams.md#lez-off-chain-integration).
Protobuf is for interchange; Borsh + domain prefix (N8) is for signatures.
`StreamParams` includes `create_stream_deadline`.
`StreamProviderPolicy` includes `max_create_stream_deadline_delay`.
Liquidity at proposal verification uses on-chain unallocated only.

Define protobuf parse/serialize for `StreamProposal`, `VaultProof`, and
`StreamProof` with LEZ length checks on `bytes` fields.
Define Borsh canonical structs and field order for signed payloads
(test vectors in-repo).
Expose canonicalization for the bytes signed by `VaultProof.owner_signature`
and the bytes signed by `StreamProof.signature` over a Store request payload.
Expose sign and verify primitives keyed by 32-byte NSSA public-key bytes
using NSSA Schnorr signatures.
Expose domain-separated `canonical_payload` and `canonical_payload_digest` helpers through `extern "C"` in `lez-payment-streams-ffi`,
but implement layouts and hashing in `lez-payment-streams-core` (unit-tested Rust, same layering as Step 3a/3b).
Keep the workspace `borsh` crate line aligned with core so `CanonicalStoreRequest` matches N8 exactly across crates.

The canonicalization format follows the NSSA precedent
established in `nssa/src/public_transaction/message.rs`:
Borsh-serialize a struct, prepend a fixed 32-byte domain prefix,
SHA-256 the result to produce the 32-byte `canonical_payload_digest` that is signed.
See N8 for the full specification of this format
and the canonical Store request bytes structure
that is shared between the Rust FFI (this step) and Nim (Step 15).

The `VaultProof.owner_signature` payload covers the vault proof fields,
the proposed stream parameters,
and the session public key committed in `StreamProposal.public_key`.
The `StreamProof.signature` payload covers the canonical Store request bytes.

Components required to run: none.

Definition of done:
canonicalization is deterministic for a fixed input,
sign-then-verify round-trips,
the LEZ owner public key derives to the expected `VaultConfig.owner`,
the `VaultProof` verifier rejects a proposal whose session public key
or stream parameters are changed after signing,
and tampering with any field of the canonicalized payload
flips the verifier verdict.

### Step 5, Instruction builders in the FFI

Expose Borsh encoders for every payment-stream `Instruction` variant,
together with an account-list planner that returns the ordered list
of PDA and signer hex strings each instruction needs.
The encoders take typed arguments and return raw bytes;
the planner returns hex strings ready for the
`send_public_transaction` JSON shape used by `logos-rln-module`.

Implementor notes (Steps 3b–4 carryover):

- Step 5 serializes on-chain `Instruction` Borsh from `lez-payment-streams-core`
  (guest program types and PDAs). Do not route this through Step 4’s protobuf stack
  or N8 Store canonicalization; those layers stay separate.
- Keep encoders, decoders for tests, and account-list derivation in core; add only
  thin `extern "C"` shims in `lez-payment-streams-ffi` (same layering as `policy_abi`
  / `proof_abi`) and regenerate `lez_payment_streams_ffi.h` with `cbindgen`.
- Wide values use the existing `_lo` / `_hi` pattern and helpers (`balance_pair`, …);
  do not introduce another wide-integer ABI convention for this step.
- Treat encoding and planning mistakes (unsupported variant, bad sizes, impossible
  account lists) like Step 3b input faults: map to `PaymentStreamsFfiStatus`
  `Malformed` / null-pointer family — not `PolicyRejected` or `ProofInvalid`.
- The harness builders in `lez-payment-streams-core/src/test_helpers.rs` are the
  normative account ordering oracle; match them exactly rather than re-deriving
  ordering from narrative spec text alone.

Components required to run: none.

Definition of done:
encoded payloads round-trip through `lez-payment-streams-core` Borsh decoders,
and account-list planners agree with the harness builders in
`lez-payment-streams-core/src/test_helpers.rs`.

### Step 6, Store query via `delivery_module` (closed decision)

Closed decision (2026-05-19): this integration will not pursue an independent
implementation, feature branch, or upstream PR to expose Store queries through
`logos-delivery-module`. Exploratory PRs for `logosdelivery_query_store` /
`queryStore` are retired. Store access is an upstream deliverable on the Delivery
roadmap (different design than those PRs).

Steps 16–18 remain blocked on upstream Store query support landing on
`logos-delivery` and `logos-delivery-module` `master`. All other steps proceed
without calling Store query APIs on `delivery_module`. Normative detail: N6.

Active work proceeds on Steps 1–15 without calling Store query APIs on
`delivery_module`.

Components required to run: none.

Definition of done: decision recorded; no payment-streams work item remains for
local Store query exposure.

Status: done (wait for upstream only).

### Step 7, Operator install basics (Nix, LGX, lgpm, logoscore)

Goal.

Understand how payment-streams artifacts are built with Nix,
how `.lgx` packages are produced for both `logos_execution_zone` and `payment_streams_module`,
how `lgpm` installs them into one `modules/` directory,
and how `logoscore` loads that directory,
before treating Step 9 definition-of-done items 2–5 as the operating checklist
(install, explicit `load-module`, `lm`, plumbing — see operator guide).

This step is documentation and environment setup only.
No change to module source code is required.

Components required.

Read access to this repo,
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 1),
and [`logos-tutorial/logos-developer-guide.md`](../logos-tutorial/logos-developer-guide.md)
(package manager and logoscore sections).

Definition of done.

1. You can explain why `nix build .#lgx` at the `lez-payment-streams` repository root does not work,
   and which flake attribute builds `payment_streams_module` instead.
2. You can produce a wallet `.lgx` via
   `logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`
   and `nix bundle --bundler github:logos-co/nix-bundle-lgx .#lib`,
   and produce `payment_streams_module` via `nix build ./logos-payment-streams-module#lgx`.
3. You use one absolute `modules/` path for both `lgpm --modules-dir` and `logoscore -m`,
   installing `logos_execution_zone` before `payment_streams_module`,
   running `lgpm list` (two modules),
   starting `logoscore -D -m` (daemon plus `capability_module`),
   then loading wallet and payment streams via `load-module` or `-l logos_execution_zone,payment_streams_module`,
   without ad hoc relative `PATH` hacks
   (prefer `nix shell` in each terminal tab; see operator guide).

### Step 8, Universal-to-Legacy wallet probe

Architectural context:
Before committing to Universal `payment_streams_module` (D6), we had to determine whether
`logoscore` can safely route a dynamic call from a Universal module to Legacy
`logos_execution_zone` without listing the wallet in `metadata.json`.
Historical rationale and probe design:
[`docs/step8-universal-legacy-probe-results.md`](docs/step8-universal-legacy-probe-results.md)
(appendix references the retired dilemma write-up).

The probe module uses the `logos-module-builder` Universal template.
It does not add `logos_execution_zone` to `metadata.json` `dependencies`.
It calls the wallet via
`modules().api->getClient("logos_execution_zone")->invokeRemoteMethod(...)`.
Compile the probe and run `logoscore`, loading Legacy wallet then Universal probe.

Status: done for Step 8 goal (2026-06-08).
Validated: Universal caller invokes Legacy `logos_execution_zone` without a static dependency;
daemon stable; `invokeRemoteMethod` dispatch works (empty account list OK for marshaling smoke).
Not validated in the probe: funded wallet / scaffold storage via module `open` (Pass B storage failure).
See [`docs/step8-universal-legacy-probe-results.md`](docs/step8-universal-legacy-probe-results.md) and
[`logos-universal-legacy-probe/docs/probe-runbook-and-results.md`](../logos-universal-legacy-probe/docs/probe-runbook-and-results.md).

Decision D6: build `payment_streams_module` as Universal; call the wallet dynamically with empty
`dependencies`.

Components required to run:
`logoscore`, patched `logos_execution_zone`, probe `.lgx`, optional LEZ for Pass B only.

Definition of done:
probe loads with wallet; `invokeRemoteMethod` reaches `logos_execution_zone` without host crash;
results recorded in the Step 8 doc; D6 recorded in this plan.

### Step 9, Bootstrap Universal `payment_streams_module`

Architectural context:
this step lays down the Universal C++ module shell of `payment_streams_module`.
The module is a `type: core` plugin with `"interface": "universal"`
that hosts the Rust FFI crate (from Steps 1–5)
and will expose LogosAPI methods to other modules in later steps.

Prerequisite: Step 7 ([`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 1)).

Pattern decision (D6, Step 8 probe):
build `payment_streams_module` as Universal (`LogosModuleContext` in
`payment_streams_module_impl.{h,cpp}`).
Do not restore the Legacy `PluginInterface` shell.
Call `logos_execution_zone` at runtime via `invokeRemoteMethod`; keep `"dependencies": []`.
Archived Legacy bootstrap notes:
[`docs/archive/legacy-module-bootstrap.md`](docs/archive/legacy-module-bootstrap.md).

`logos-delivery-module` [Issue #31](https://github.com/logos-co/logos-delivery-module/issues/31)
documents that the `LogosModules` typed wrapper crashes in core module sidecars.
Outbound wallet calls use `LogosAPIClient::invokeRemoteMethod` directly regardless of interface.

Implementation checklist and operator loop:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) Part 2.

For flake pins (LEZ PR 491, wallet module PR 19), see [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md).
Store query pins are intentionally absent; see N6.

Scaffold from `logos-module-builder` with external lib input, modeled on `logos-rln-module`
and the Rust-FFI pattern in `logos-tutorial/logos-developer-guide.md`.
Per D5, directory `logos-payment-streams-module/` sits alongside `lez-payment-streams-core/`
and `lez-payment-streams-ffi/` in this repo.
It ships
`metadata.json` (`name = "payment_streams_module"`, `type = "core"`,
`"interface": "universal"`, `dependencies = []`,
`include` listing platform variants of `liblez_payment_streams_ffi`),
`flake.nix` (`mkLogosModule` + `externalLibInputs.lez_payment_streams_ffi`),
`CMakeLists.txt`,
and `src/payment_streams_module_impl.{h,cpp}` extending `LogosModuleContext`.
Do not declare `onInit` on the impl class (codegen conflict; see Step 8 probe).
Step 8 validated Universal-to-Legacy wallet `invokeRemoteMethod`; Step 11a adds the first
in-module wallet calls.

Implementor hints (FFI from Step 5, no extra Qt surface yet):

- Link `liblez_payment_streams_ffi` named in metadata `include`, and vendor
  [`lez_payment_streams_ffi.h`](lez-payment-streams-ffi/lez_payment_streams_ffi.h) like
  `logos-rln-module` (CMake + flake inputs).
  Step 9 is load/plumbing only; do not call instruction entrypoints from C++ until Step 11b.
- On-chain instruction bytes and account-list planning live in
  [`lez-payment-streams-ffi/src/instruction_abi.rs`](lez-payment-streams-ffi/src/instruction_abi.rs).
  Skim the file-level doc for two-phase output sizing and 64-byte lowercase hex stride per account.
- Instruction payloads follow
  [`lez-payment-streams-core/src/instruction_wire.rs`](lez-payment-streams-core/src/instruction_wire.rs),
  not Step 4 protobuf/N8.
- Deposit helpers use `payment_streams_ffi_authenticated_transfer_program_id_bytes` where needed.

Pin `logos-module-builder` recent enough for Universal glue (same band as
`logos-universal-legacy-probe`). Wallet LEZ rev aligned with patched wallet flake
(module PR 19 + LEZ pin via `lez-wallet-ffi-patched`; see feature-branch pins).

Components required to run:
`logoscore` host (first step needing a running Logos host).
No chain, messaging network, or UI host.

Status: done (Universal skeleton in `logos-payment-streams-module/`).

Definition of done:

1. `nix build ./logos-payment-streams-module#lgx` produces a valid `.lgx`
2. `lgpm install` places the module alongside patched `logos_execution_zone`
3. `logoscore` loads the module after explicit `load-module` (wallet first, then payment streams)
4. `payment_streams_module` reports loaded; daemon shows no crash
5. `logoscore call logos_execution_zone list_accounts` reaches the wallet RPC (LEZ optional for failure-only smoke)

Operator commands:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 1).
From Step 11a onward, repeat build/install/load and LEZ after `payment_streams_module` edits:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).
Step 10a–10b use scaffold and wallet `.lgx` setup (Part 1 and fixture notes in step1 findings).

### Step 10, LEZ fixture and wallet runtime

Architectural context:
Step 10 prepares local chain state and the patched `logos_execution_zone` artifact.
No `payment_streams_module` chain reads or writes yet (those are Step 11).
Scaffold CLI (`lgs`) owns localnet lifecycle and program deploy; the wallet `.lgx`
from the patched wrapper (PR 491 + PR 19) is what logoscore loads as `logos_execution_zone`.

Sub-step order: 10a → 10b before any Step 11 work.

#### Step 10a, Local chain fixture

Goal:
reproducible localnet, deployed `lez_payment_streams`, funded public owner,
initialized demo vault and stream, recorded fixture ids, and persistent sequencer
state between dev sessions until an explicit reset.
See [N9](#n9-step-10a-local-chain-fixture-decisions).

Work (operator / script — idempotent seed script, gitignored manifest, runbook in `docs/`):

- `scaffold.toml` in repo; runtime state per N9 (`SCAFFOLD_WS` or gitignored `.scaffold/`).
- `lgs init`, `lgs setup`, `lgs localnet start`; deploy via `make build`, `make idl`, `make deploy`
  (CLI `wallet deploy-program`, N9) until **Step 11d** adds an optional logoscore/wallet-module
  deploy path; record program id (`make program-id`).
- Fund public owner: `lgs wallet topup --address Public/<base58-id>` (pinata path in
  [`docs/step1-findings-scaffold-rpc.md`](docs/step1-findings-scaffold-rpc.md)).
- Full seed: `initialize_vault`, deposit, `create_stream` for demo `vault_id` / `stream_id`
  (manifest records which ids are pre-seeded vs reserved for Step 11b lifecycle tests).
- Manifest: program id, owner, PDAs (FFI-consistent derivation), `CLOCK_10`, demo vault/stream ids.
- Reuse: keep `.scaffold/state/` when stopping localnet; reset = stop, remove `.scaffold/state/`,
  re-run 10a.

Components required to run:
`lgs`, `wallet` on PATH after `lgs setup`, guest build toolchain, local sequencer only.

Definition of done:

1. Localnet reachable at `http://127.0.0.1:3040`; `lgs wallet -- check-health` passes.
2. `lez_payment_streams` deployed; program id in fixture manifest.
3. Funded public owner; topup scripted or documented.
4. Demo vault and stream initialized on chain; manifest supports Step 11a decode of vault,
   holding, stream, and clock accounts.
5. Reset procedure documented (N9).

Follow-up (LEZ PR 491 localnet — operator)

Checklist and troubleshooting:
[`docs/step10a-handoff-and-follow-up.md`](docs/step10a-handoff-and-follow-up.md),
[`docs/step10a-local-chain-fixture.md`](docs/step10a-local-chain-fixture.md).

Guest alignment already in tree (rebuild + redeploy after changes):

1. LEE public PDAs via vendored `spel-framework-core` — remove when SPEL-on-LEE matches 491
   ([N9](#n9-step-10a-local-chain-fixture-decisions) SPEL-on-LEE cleanup).
2. Deposit `ChainedCall` uses LEZ `authenticated_transfer` enum `Transfer { amount }`; NSSA
   in-process harness tests are `#[ignore]` until SPEL-on-LEE or a LEE executor.
3. After any guest rebuild, refresh gitignored `fixtures/localnet.json` and redeploy; PDAs and
   `program_id_hex` follow ImageID.
4. If seed fails on deposit, use sequencer logs (execution vs poller); see handoff — do not assume
   a `nssa_core` tag bump removes the vendor or enum encoding.
5. Run `./scripts/verify-step10a-dod.sh` (vault config, vault holding, stream config on chain)
   before Step 10b.

#### Step 10b, Wallet runtime artifact (PR 491 + PR 19)

Goal:
installable `logos_execution_zone` `.lgx` built from
`logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`
(upstream [PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19)
on LEZ `main` / [491 merged](https://github.com/logos-blockchain/logos-execution-zone/pull/491)),
with wrapper behavioral patches only (guest ELF, JSON submit, future signing) — generic public
send comes from PR 19, not a reimplementation in this repo.

Work:

- Pin and build per [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md).
- `nix bundle` wallet `.lgx`; `lgpm install` into shared `MODULES`.
- Document `open` (config + storage + sequencer RPC) so `logoscore call logos_execution_zone …`
  reaches the same localnet as Step 10a (`get_account_public`, `list_accounts`).
- Step 11b wallet extras (same flake / manual Qt build): `PAYMENT_STREAMS_GUEST_BIN`,
  `send_generic_public_transaction_json` — see [`docs/step11b-chain-writes.md`](docs/step11b-chain-writes.md).
  **Step 11d** may reduce reliance on `PAYMENT_STREAMS_GUEST_BIN` after LEZ 510 + Qt wrappers land.
- Verify: `lm methods` lists PR 19 generic public transaction entry point(s).

`sign_public_payload` is Step 11c, not 10b.

Components required to run:
Step 10a localnet (for RPC validation); logoscore + `lgpm` per runtime guide Part 1.

Definition of done:

1. Patched wallet `.lgx` installs as `logos_execution_zone`.
2. `logoscore load-module logos_execution_zone` succeeds after Step 9 payment streams load order rules.
3. Documented `open` path; `get_account_public` returns JSON for a fixture account id from 10a.
4. `lm methods` confirms PR 19 send surface (names per that PR).

Deliverables in tree:
[`docs/step10b-wallet-runtime.md`](docs/step10b-wallet-runtime.md),
`scripts/build-wallet-lgx.sh`, `scripts/verify-step10b-dod.sh`, `make wallet-lgx` / `make verify-step10b`.

### Step 11, Module chain access

Architectural context:
Step 11 wires `payment_streams_module` to `logos_execution_zone` for reads, writes, and
(off-chain) digest signing support. Requires 10a → 10b complete.
Sub-step order: 11a → 11b → 11c (11c must complete before Step 12 eligibility).
11d (LEZ 510 wallet pin) is documented after 11c and is landed in tree; see
[Step 11d](#step-11d-program-deploy-and-program-elf-lez-pr-510).

Universal module surface: all public methods on `PaymentStreamsModuleImpl` are exported
([N11](#n11-universal-module-public-api)). Step 11a/11b use five read helpers plus one
`chainAction` router for writes and status (see N10 and
[`docs/step11b-chain-writes.md`](docs/step11b-chain-writes.md)).

#### Step 11a, Wire chain reads from the module

Runbook: [`docs/step11a-chain-reads.md`](docs/step11a-chain-reads.md).

This step adds stable read helpers and exercises wallet-backed chain reads end-to-end
for the first time in `payment_streams_module`.
`payment_streams_module` calls into `logos_execution_zone`,
which uses `wallet_ffi` to reach the LEZ sequencer over JSON-RPC.

Add helpers inside `payment_streams_module` that wrap
`logos_execution_zone.account_id_from_base58` and `logos_execution_zone.get_account_public`,
plus a higher-level helper that reads the configured clock account
(default `CLOCK_10`, see Step 10a fixture)
and returns the current sequencer time.
These helpers are pure read paths: decode via `lez-payment-streams-ffi`, no fold,
no payment-stream transactions.
Use `LogosAPIClient::invokeRemoteMethod` directly
([`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) Part 2).
Do not use the `LogosModules` typed wrapper (Issue #31).

Components required to run:
`logoscore` with both modules loaded, Step 10a fixture (deployed program, funded owner),
Step 10b wallet `open` + RPC. No messaging network.

Scaffold RPC and formats: [`docs/step1-findings-scaffold-rpc.md`](docs/step1-findings-scaffold-rpc.md).
Module edit loop: [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) Part 3.

Definition of done:
against the Step 10a fixture,
the module reads known vault config, vault holding, stream config,
and clock accounts through `logoscore`,
and wallet JSON decodes through the FFI into expected typed values.
If 10a left vault/stream PDAs empty, minimum DoD is clock plus any accounts 10a pre-initialized;
full four-account decode requires initialized vault/stream bytes on chain.

#### Step 11b, Chain writes and status helpers

See [N10](#n10-step-11b-module-writes-decisions) for wallet submit shape, E2E signer,
fixture config, and tx completion boundaries. Operator runbook:
[`docs/step11b-chain-writes.md`](docs/step11b-chain-writes.md).

Add a private submit helper inside `payment_streams_module` that builds Borsh instruction
bytes and ordered accounts via the payment-streams FFI, assembles program-with-dependencies
(guest ELF from env when set), and submits through the patched wallet (JSON IPC wrapper).

Expose chain I/O through public `chainAction(operation, paramsJson)` covering initialize
vault, deposit, withdraw, create stream, top up, pause, resume, close, claim, plus
`getVaultStatus` and `getStreamStatus` operations in JSON (implementation derives PDAs from
owner base58 + vault/stream ids and fixture `program_id_hex`).

Status helpers compose wallet reads with Step 2 decoders and Step 3a/3b fold where applicable.

Components required to run:
same stack as 11a; PR 19 send surface and Step 11b wallet patches from Step 10b.

Definition of done:
`./scripts/verify-step11b-dod.sh` exits 0: full submit lifecycle via `chainAction` on the
Step 10a fixture with seeded wallet storage; status checks pass or SKIP with documented
account-read gap for fresh vault PDAs.

#### Step 11c, `sign_public_payload` on the patched wallet

Goal:
expose digest signing for `VaultProof.owner_signature` (N1) on the patched wrapper only
(LEZ FFI + Qt); required before Step 12.

Work:

- Add `wallet_ffi_sign_public_payload(handle, account_id, digest_32, out_sig_64)`
  to `lez/wallet-ffi/src/keys.rs` in the LEZ source,
  following the pattern of `wallet_ffi_get_public_account_key`:
  retrieve the `PrivateKey` via `wallet.get_account_public_signing_key(account_id)`,
  call `Signature::new(private_key, digest)` (BIP-340 Schnorr, 64 bytes out).
  Add the corresponding declaration to `lez/wallet-ffi/wallet_ffi.h`.
  Deliver as `lez-rust-sign-public-payload.patch` in
  `logos-execution-zone-module-patched/` applied via `postPatch` in
  `lez-wallet-ffi-patched/flake.nix`.
- Add `Q_INVOKABLE QString sign_public_payload(const QString& account_id_hex,
  const QString& digest_hex)` to `logos_execution_zone_wallet_module.{h,cpp}`.
  Deliver as `wallet-qt-sign-public-payload.patch` in
  `logos-execution-zone-module-patched/` applied alongside
  `wallet-qt-guest-elf-from-env.patch` in `patchWalletInclude`.
- Signature for callers:
  - `account_id_hex`: 64-char hex account ID (same format as `get_account_public`
    and `get_public_account_key`; call `account_id_from_base58` first if the
    caller holds a base58 value).
  - `digest_hex`: 64-char hex SHA-256 digest (32 bytes).
  - Return: `{"status":"ok","result":"<128-char hex signature>"}` on success;
    `{"status":"error","error":"..."}` on failure.
    Consistent with all other wallet Qt methods.
- Rebuild and reinstall wallet `.lgx` (same flake as 10b).
- Smoke via `logoscore call logos_execution_zone …` (no full Store flow required).

Components required to run:
Step 10b pipeline; logoscore; `smoke_verify` binary from
`lez-payment-streams-ffi` (built as part of Step 4; Step 11c DoD uses it).

Definition of done:

1. `lm methods` lists `sign_public_payload`.
2. Sign-then-verify smoke with a hex account ID from Step 10a fixture
   (`owner_account_id` converted via `account_id_from_base58`):
   call `sign_public_payload` with a known 32-byte test digest,
   extract `result` from the JSON response,
   retrieve the matching public key via `get_public_account_key`,
   run `./target/debug/smoke_verify <pubkey_hex> <digest_hex> <sig_hex>` and assert exit 0.
3. Step 12 may depend on this method without further wallet feature work.

#### Step 11d, Program deploy and program ELF (LEZ PR 510)

Status: landed in tree — [`docs/step11d-wallet-510.md`](docs/step11d-wallet-510.md),
`./scripts/verify-step11d-dod.sh`. Remaining product gap: reliable 11b logoscore E2E on a fresh
fixture (DoD items 2–3 below) without depleted-stream bypass.

Architectural context:
[LEZ PR 510](https://github.com/logos-blockchain/logos-execution-zone/pull/510) merged program
deployment and test-program ELF exposure into `wallet_ffi`, with zones API updates on LEZ `main`.
This step upgraded the wallet runtime stack from the earlier 491-era pin — not new
`payment_streams_module` eligibility logic. It addresses operator pain where CLI
`wallet deploy-program` (Step 10a) and logoscore `open` + `send_generic_public_transaction`
(Step 11b) share wallet home but diverge in practice, and where `PAYMENT_STREAMS_GUEST_BIN` patches
guest ELF into the wallet process ([N10](#n10-step-11b-module-writes-decisions)).

Schedule (historical): Step 12 landed before 11d pin work; Step 12 strict verify against the 510
stack is documented under Step 12
([Verification (Step 11d follow-up — landed)](#verification-step-11d-follow-up--landed)).
Step 13 provider verify is complete in tree (`./scripts/verify-step13-dod.sh`); do not treat
Step 17 E2E as complete until Steps 14–16 delivery integration and 11b writes are green on a
fresh fixture or Step 17 documents CLI-only deploy fallback.

Goal:

- Bump LEZ and wallet-wrapper pins to a revision ≥ 510 merge; rebuild patched
  `logos_execution_zone` `.lgx`; re-run `lgs setup` and Step 10a–11b verifies.
- Expose deploy (and any program-ELF registration helpers) on `logos_execution_zone` once
  upstream Qt / [PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19)
  successor wraps the new FFI (510 notes dependent C++ PRs).
- Optional: extend `./scripts/demo-localnet-fresh.sh` / seed path with logoscore deploy so
  fixture refresh does not depend on a separate CLI `wallet` invocation.
- Narrow or remove `PAYMENT_STREAMS_GUEST_BIN` wrapper behavior when submit can use wallet-held
  program ELF after deploy.

Work:

- Update [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md),
  `nix/payment-streams-ffi.nix`, `scaffold.toml`, and
  `logos-execution-zone-module-patched` flakes; full regression
  `./scripts/verify-step10a-dod.sh`, `./scripts/verify-step10b-dod.sh`,
  `./scripts/verify-step11b-dod.sh`.
- Inventory `wallet_ffi.h` for deploy and ELF symbols added in 510; add Qt / LogosAPI shims in
  the patched wrapper (or track upstream wallet-module PR).
- Document operator path: deploy + `open` + `chainAction` submit from one logoscore session.

Components required to run:
Same as Steps 10b–11b; network for pin bump and localnet re-seed.

Definition of done:

1. Patched wallet `.lgx` builds against LEZ ≥ 510; `lm methods` lists deploy (or documented
   interim name) when Qt wrapper lands.
2. At least one successful `chainAction` write (e.g. `topUpStream` or `deposit`) on a fresh
   fixture after `./scripts/demo-localnet-fresh.sh` without relying on
   `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF`.
3. `./scripts/verify-step11b-dod.sh` exits 0 on the upgraded stack (or documented SKIP removed).
4. Step 12 strict logoscore verify documented and runnable (`REQUIRE_STREAM_PROOF=1`; see Step 12).

Deliverables:
[`docs/step11d-wallet-510.md`](docs/step11d-wallet-510.md), [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md),
`./scripts/verify-step11d-dod.sh`, `./scripts/deploy-program-logoscore.sh`; guest-env patch retained for 11b.

### Step 12, Session keys and user-side proof construction

Architectural context:
this is the user-side method that `delivery_module` will auto-invoke
once registered as the outbound eligibility provider in Step 16.
It does not, by itself, initiate any Store traffic;
it just produces opaque bytes when asked.
Requires Step 11c (`sign_public_payload`) and the Step 11a read path;
user flows that open streams on-chain use Step 11b (`chainAction` / `createStream`).

Runbook (demo vault, API encoding):
[`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md).
Local fixture age and reset policy:
[`docs/demo-localnet-recovery.md`](docs/demo-localnet-recovery.md).

#### Status (implementation)

Step 12 is complete for this integration plan:

- Feature: session keygen FFI, `EligibilityProof` wrapper serialize, module methods
  (`registerProviderMapping`, `prepareEligibilityForStoreQuery`, `listMyStreams`,
  `rediscoverStreams`), N4 persistence, N8 tool (`n8_canonical_wire_hex`).
- Verify: `./scripts/verify-step12-dod.sh` (offline + logoscore); strict `stream_proof` via
  `REQUIRE_STREAM_PROOF=1` and `./scripts/step12-topup-and-prepare.sh` after Step 11d wallet stack.
- Runbooks: [`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md),
  [`docs/demo-localnet-recovery.md`](docs/demo-localnet-recovery.md).

Not in Step 12 scope: Step 16 `delivery_module` auto-invoke; Step 13 provider verifier cross-test
(lives in `./scripts/verify-step13-dod.sh`, not Step 12 DoD); full Step 17 demo without top-up
helper on aged stream `0`.

#### Quick reference

| Method | Purpose | Called by |
|--------|---------|-----------|
| `prepareEligibilityForStoreQuery` | Returns serialized `EligibilityProof` (stream proposal or proof arm) | `delivery_module` (auto) |
| `registerProviderMapping` | Maps `PeerId` to LEZ payee account (base58) | Host application |
| `listMyStreams` | Lists streams for a vault | Host application |
| `rediscoverStreams` | Re-enumerates streams from chain | Host application (recovery) |

#### User-side flow

The intended sequence for a new provider relationship is:

1. Host application calls `registerProviderMapping`
   to bind the provider's libp2p `PeerId`
   to its LEZ stream payee account (base58; module derives LIP-155 `provider_id` bytes per N5).
2. User issues a Store query.
   `delivery_module` invokes `prepareEligibilityForStoreQuery`.
   The module has no established stream for this `(vault, provider)` pair,
   so it generates a session keypair, persists it,
   and returns an `EligibilityProof` byte string (opaque to Delivery; `stream_proposal` arm).
3. Provider accepts the proposal and serves the first request.
4. User explicitly calls `chainAction` with operation `createStream`
   (Step 11b) to open the stream on-chain.
   This is a manual action by the host application or demo script,
   never triggered automatically by any hook.
5. User issues the next Store query.
   `delivery_module` invokes `prepareEligibilityForStoreQuery` again.
   The module queries `get_account_public` for the `StreamConfig` PDA,
   confirms it exists and is `ACTIVE`,
   and returns an `EligibilityProof` byte string (`stream_proof` arm).

#### Session and stream state management

Add session-keypair management inside `payment_streams_module`,
backed by `payment_streams_state.json` in `instancePersistencePath` (see [N4](#n4-persistence-policy)).
Generate session keypairs via `lez-payment-streams-ffi` (see [FFI session keypair](#ffi-session-keypair-step-12-deliverable));
sign proofs with existing Step 4 FFI helpers. Persist keys as plaintext hex for the demo.
The persisted state per `(vault_id, provider_id)` includes:
the `stream_id` (allocated locally, used as the PDA seed on-chain),
the session keypair,
the proposal status (pending, established, expired),
and the last known on-chain stream state.

The module maintains a local inventory of stream IDs per vault.
Every `create_stream` call records the new `stream_id` in the inventory.
This inventory is the backing store for `listMyStreams`.
Stale proposals are evicted on cold start and on eligibility/inventory API calls (deadline vs clock-10).

#### FFI session keypair (Step 12 deliverable)

Step 4 exports sign/verify with caller-supplied 32-byte NSSA secrets only; it does not generate
session keypairs. Implemented in `payment_streams_ffi_*` naming family as
`payment_streams_ffi_sign_canonical_payload_digest` (`lez-payment-streams-ffi/src/proof_abi.rs`;
core logic in `lez-payment-streams-core`).

Add in `lez-payment-streams-core` (unit-tested) and expose:

```c
PaymentStreamsFfiStatus payment_streams_ffi_generate_session_keypair(
    uint8_t *out_secret_key_32,
    uint8_t *out_public_key_32);
```

Both outputs are 32 bytes (NSSA `PrivateKey` / public key bytes used elsewhere in proof FFI).
Use a CSPRNG; return `PaymentStreamsFfiStatus` on null pointers or generation failure. Regenerate
`cbindgen` output (`lez_payment_streams_ffi.h`); wire through `payment_streams_ffi_bridge` if the
Qt module calls via the existing C bridge pattern.

Step 12 definition of done includes a Rust unit test in `proof_abi.rs`: generate, sign a digest
with `payment_streams_ffi_sign_canonical_payload_digest`, verify with
`payment_streams_ffi_verify_canonical_payload_digest`.

#### Exposed methods

`prepareEligibilityForStoreQuery(canonicalRequestBytes, providerPeerId) -> QString`
LogosAPI passes `canonicalRequestBytes` as lowercase hex of the N8 `canonical_payload` (see
runbook). Returns compact JSON whose `bytes_hex` is the serialized protobuf `EligibilityProof` for
Store tag `30` (D1, D2). Set `stream_proposal` or `stream_proof` (mutually exclusive) with
nested serialized `StreamProposal` or `StreamProof` per LIP-155, depending on whether the
stream for the `(vault, provider)` pair has been established on-chain.
Before returning a `StreamProof`,
the module reads the `StreamConfig` PDA via `get_account_public`,
decodes it through the FFI,
folds it at the current clock time,
and checks that the effective state is `ACTIVE`.
For `StreamProposal` output,
the module calls `logos_execution_zone.account_id_from_base58` to convert
the configured vault owner base58 account ID to hex,
then asks `logos_execution_zone.sign_public_payload(account_id_hex, digest_hex)`
to produce `VaultProof.owner_signature` with the vault owner's LEZ key,
and reads the 64-byte signature from the `result` field of the JSON response.
Later `StreamProof`s are signed with the persisted session key.

`registerProviderMapping(providerPeerId, providerAccountId) -> QString`
lets the host configure the identity mapping (see [N5](#n5-provider-identity-mapping)).
Returns compact JSON (`status` ok/error). `providerAccountId` is base58; the module derives
32-byte `provider_id` for proofs and persistence.

`listMyStreams(vaultId) -> QString`
returns a JSON array of stream statuses
for all locally known streams belonging to the given vault.
For each stream in the local inventory,
the module derives the `StreamConfig` PDA,
reads it via `get_account_public`,
decodes and folds to the current clock time,
and returns the typed status.

`rediscoverStreams(vaultId) -> QString`
re-enumerates streams from the chain
by deriving PDA addresses for `stream_id = 0, 1, 2, ...` sequentially,
reading each via `get_account_public`,
and stopping when an uninitialized account is encountered.
Discovered streams are added to the local inventory.
This is a recovery path for cold-start or persistence-loss scenarios.
For the MVP demo, `listMyStreams` is the primary query path.

#### User-side error conditions

`prepareEligibilityForStoreQuery` returns a structured error
in each of the following cases.
The error string includes a machine-readable code
and a human-readable description.

- `UNKNOWN_PROVIDER`:
  `providerPeerId` not registered via `registerProviderMapping`.
- `NO_ELIGIBLE_VAULT`:
  no vault configured or no vault with sufficient unallocated balance.
- `PROPOSAL_PENDING`:
  a `StreamProposal` for this `(vault_id, provider_id)` pair
  was already issued and has not expired or been resolved.
  User must wait for expiry or call `create_stream`.
- `PROPOSAL_EXPIRED`:
  the pending proposal's `create_stream_deadline` has passed
  without stream creation.
  The module evicts the stale proposal and returns this error on that call;
  a subsequent call may issue a fresh `StreamProposal`.
- `STREAM_NOT_CONFIRMED`:
  user called `create_stream` but the `StreamConfig` PDA
  does not yet exist on-chain.
  User should retry after a short delay.
- `STREAM_DEPLETED`:
  folded stream state shows allocation fully accrued (unaccrued is zero).
  User must top up or close.
- `STREAM_PAUSED`:
  stream is paused (user-initiated).
  User must resume before querying.
- `STREAM_CLOSED`:
  stream has been closed (by user or provider).
  User must open a new stream to this provider.
- When chain state is `ACTIVE` for the `(vault_id, provider_id)` pair,
  `prepareEligibilityForStoreQuery` returns a `stream_proof` (not an error).
  Duplicate on-chain `createStream` for an occupied `stream_id` is rejected by the chain, not a
  separate module error code.
- `WALLET_SIGNING_FAILED`:
  `sign_public_payload` returned `{"status":"error",...}` or IPC failed.
  Error includes upstream details from the `error` field.
- `CHAIN_READ_FAILED`:
  `get_account_public` call failed.
  Error includes upstream details.

#### Components required to run

`logoscore` daemon hosting both modules.
The definition of done's verifier round-trip is in-process through the FFI;
a live sequencer is not strictly required for that verification itself,
but the same Steps 10a–11b stack remains useful for sanity-checking
that vault data the proof asserts matches chain state.
After code changes, rebuild and reload via
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

#### Definition of done

Landed (current tree):

1. `./scripts/verify-step12-dod.sh` with `VERIFY_LOGOSCORE=0` exits 0: N8 digest test, FFI
   session keygen + eligibility wrapper tests, N8 wire tool, installed module plugin, four
   Step 12 methods in `lm methods`.
2. With `VERIFY_LOGOSCORE=1` and `REQUIRE_STREAM_PROOF=1`: logoscore path via
   `step12-topup-and-prepare.sh` (register, `topUpStream`, `stream_proof` prepare) and
   persistence under `--persistence-path`. Default `REQUIRE_STREAM_PROOF=0` may SKIP prepare when
   stream `0` is depleted on an aged localnet (see recovery doc).
3. Runbook [`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md) matches API and
   error codes.

Product criteria (unchanged):

The module produces a syntactically valid eligibility proof byte string
for fixed inputs;
`payment_streams_ffi_generate_session_keypair` is implemented and covered by FFI tests;
restarts cleanly with state intact;
the FFI structural verifier accepts the proof format;
`listMyStreams` returns correct folded status for locally known streams;
each user-side error condition returns the documented error code;
and (when chain state is available) the provider-side verifier accepts
the proof against actual on-chain stream state (Step 13; recommended cross-test).

#### Verification (Step 11d follow-up — landed)

After the Step 11d wallet pin bump:

1. `./scripts/verify-step12-dod.sh` supports `REQUIRE_STREAM_PROOF=1` (top-up + prepare via
   `step12-topup-and-prepare.sh`). Default logoscore smoke allows SKIP on depleted stream when
   `REQUIRE_STREAM_PROOF=0`.
2. Demo scripts document `PAYMENT_STREAMS_GUEST_BIN`, `ensure-scaffold-lez-layout.sh`, and
   `REINIT_WALLET=1` recovery; `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF` stays emergency-only.
3. FFI fold normalizes LEZ 510+ millisecond clock timestamps to seconds for accrual checks.

CI may keep `VERIFY_LOGOSCORE=0`; local strict checks use
[`docs/demo-localnet-recovery.md`](docs/demo-localnet-recovery.md) and `REQUIRE_STREAM_PROOF=1`.

### Step 13, Provider-side proof verification

Architectural context:
this is the provider-side method that `delivery_module` will auto-invoke
once registered as the inbound eligibility verifier in Step 16.
Structural checks happen entirely through the Rust FFI;
chain checks happen via LogosAPI calls to `logos_execution_zone`.

Expose a single provider-side `Q_INVOKABLE` method
`verifyEligibilityForStoreQuery(proofBytes, canonicalRequestBytes, requesterPeerId)`
that parses and dispatches the proof,
runs structural checks through the FFI,
queries chain state through the wallet module,
folds stream state at the current sequencer time,
and returns a structured verdict mapping to LIP-155 outcomes.
`proofBytes` are the same opaque serialized `EligibilityProof` from the Store request (D2);
the module unwraps `stream_proposal` / `stream_proof` before FFI checks.

#### Provider-side verdicts

The verifier returns one of the following eligibility status codes.
These are carried inside the `eligibility_status` object (D1)
and never surface as Store status codes.

- `OK`:
  proof is valid, chain state confirms eligibility, request is served.
- `PARAMS_REJECTED`:
  stream parameters do not match `StreamProviderPolicy`
  (rate below `min_rate`, allocation below `min_allocation`,
  `create_stream_deadline` outside `max_create_stream_deadline_delay`),
  or vault unallocated balance is below the proposed `allocation`
  (`StreamParams`, same semantics as on-chain `StreamConfig::allocation` after `create_stream`),
  or the proposal's `create_stream_deadline` has already passed.
  The `VaultProof` is not marked as spent;
  the user may retry with adjusted parameters.
- `PROOF_INVALID`:
  proof format is malformed,
  `VaultProof.owner_signature` or `StreamProof.signature` verification failed,
  or the owner public key does not derive to `VaultConfig.owner`.
- `STREAM_NOT_ACTIVE`:
  the referenced stream exists on-chain
  but its folded state is not `ACTIVE`
  (paused, closed, or depleted).

#### Pending-proposal tracking

Pending-proposal tracking on the provider side is independent
of any user-side state and lives in `instancePersistencePath`.
The provider stores pending proposal state
keyed by `(vault_id, provider_id)`,
matching the LIP-155 constraint
that a user must not have more than one pending proposal
per vault-provider pair.
The stored record includes accepted or pending `StreamParams`,
the committed session public key, and `create_stream_deadline`.
After acceptance, add `stream_id` from the first valid `StreamProof`.
Evict when LIP-155 treats negotiation as failed (no acceptance or no
compliant stream by `create_stream_deadline`).

The inbound `requesterPeerId` is available for logs,
short-lived anti-abuse policy,
and proposal retry limits,
but Store eligibility is based on proof validity and chain state,
not on transport peer continuity.

#### Implementor approach (MVP demo)

Closed choices for Step 13 implementation. Align with Step 12
([`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md)) and keep protobuf
parsing, policy math, and signature checks in Rust FFI; the Qt module orchestrates wallet reads,
persistence, and JSON only. Store tag `30` / `eligibility_status` protobuf is Step 14; Step 16
forwards opaque bytes and peer ids without interpreting proofs (D2).

LogosAPI method (Step 16 must use this name exactly):

`verifyEligibilityForStoreQuery(proofBytes, canonicalRequestBytes, requesterPeerId)`

Argument encoding (same conventions as Step 12 `prepareEligibilityForStoreQuery`):

| Argument | Encoding |
| --- | --- |
| `proofBytes` | Lowercase hex of serialized protobuf `EligibilityProof` (Step 12 `bytes_hex`) |
| `canonicalRequestBytes` | Lowercase hex of full N8 canonical payload (Step 12 `canonical_request_hex`) |
| `requesterPeerId` | Opaque UTF-8 libp2p peer id; log only for MVP, not used in eligibility predicates |

Module JSON response (LogosAPI envelope, not the Step 14 Store object):

- Success: `{"status":"ok","eligibility":"OK"}`.
- Eligibility failure: `{"status":"error","eligibility":"<PARAMS_REJECTED|PROOF_INVALID|STREAM_NOT_ACTIVE>","message":"…"}`.
- Caller mistakes (malformed hex, wallet unavailable): `{"status":"error","message":"…"}` without
  `eligibility` (same split as Step 12 plain vs eligibility-shaped errors).

Step 12 prepare errors use `"code"` for user-side module errors (`NO_ELIGIBLE_VAULT`, etc.).
Step 13 verifier responses use `"eligibility"` for the four LIP-155 verdict strings only, not
`"code"`, so delivery and demos can distinguish transport/module faults from paid-Store outcomes
without overloading Step 12's code enum.

Do not expose `PolicyRejectReason` u32 on the module API for the demo; map core discriminants to
the four LIP-155 codes internally (D1 collapses policy detail into `PARAMS_REJECTED`).

Provider payee identity (decision A):

- Inbound verify binds `VaultProof.provider_id` and on-chain `StreamConfig.provider` to the demo
  stream payee account aligned with [`fixtures/localnet.json`](../fixtures/localnet.json)
  `provider_account_id` and the Step 10a seed.
- Implement as compile-time demo constants in the module (base58 → 32-byte id beside Step 12
  prepare logic), not a new public configure method and not runtime manifest loading.
- The module does not read `FIXTURE_MANIFEST`; operators and verify scripts use that env var
  (default `fixtures/localnet.json` relative to repo root) for chain fixture ids and logoscore
  smoke. A mismatch between re-seeded manifest and rebuilt module constants is an operator error.
- Do not use `registerProviderMapping` or `requesterPeerId` to infer provider self-id;
  `registerProviderMapping` remains user-outbound routing only (N5).
- Step 17 two-host may later add an explicit provider identity API if daemons need different
  payees without a shared fixture.

`StreamProviderPolicy` for the demo:

- Hardcode the Step 12 runbook table (`min_rate` 1, `min_allocation` 1,
  `max_create_stream_deadline_delay` 3600) next to existing demo rate/deadline constants in the
  module; no `configureProviderPolicy` for MVP.
- On proposal acceptance, snapshot that struct into persistence as `policy_at_acceptance` for
  `stream_satisfies_policy` / accepted terms (Step 3a). JSON shape (u64 fields as decimal strings
  in the file, same wide-integer style as other persist scalars):

```json
"policy_at_acceptance": {
  "min_rate": "1",
  "min_allocation": "1",
  "max_create_stream_deadline_delay": "3600"
}
```

Provider persistence (extends [N4](#n4-persistence-policy)):

- Same file `payment_streams_state.json`, separate concern from user `negotiations` / session keys.
- Bump `schema_version` to `2`. Informative v2 top-level shape (user keys unchanged from v1):

```json
{
  "schema_version": 2,
  "peer_mappings": { },
  "negotiations": [ ],
  "inventory": [ ],
  "provider_acceptances": [ ]
}
```

- v1 → v2 on load: keep `peer_mappings`, `negotiations`, and `inventory` as-is; set
  `schema_version` to `2`; add `provider_acceptances: []` if missing.
- Rows in `provider_acceptances` keyed by `(vault_id, provider_id_hex)` (payee octets, lowercase
  hex, same convention as negotiations).
- Row fields: accepted stream params (rate, allocation, `service_id`, `create_stream_deadline`,
  etc.), `policy_at_acceptance` object above, `session_public_key_hex`, optional `stream_id` (omit
  or null until bound by first valid `StreamProof`); no session private keys on the provider side.
- Upsert on proposal `OK`: key `(vault_id, provider_id_hex)`; overwrite params, policy snapshot,
  session pubkey, and deadline; preserve existing `stream_id` if already set (proposal retry with
  new terms before stream bind).
- Evict stale rows on cold start and on verify entry using clock-10 vs deadline (N4), no background
  timer.

FFI (Step 13 deliverable):

- Step 4/12 already expose inner sign/verify, policy predicates, and eligibility wrapper
  serialize (`payment_streams_ffi_serialize_eligibility_proof_*`). Core has
  `parse_eligibility_proof` in Rust tests only.
- Step 13 adds C ABI `payment_streams_ffi_parse_eligibility_proof_bytes` (outer protobuf → arm +
  inner byte slice) for the Qt bridge; inner `StreamProposal` / `StreamProof` decode stays on
  existing verify/decode FFI entry points.

Clock-10 for fold and deadlines (LEZ 510+):

- Wallet `get_account_public` on the clock account returns a timestamp in milliseconds (u64).
- Module and FFI fold path use seconds: `timestamp_secs = timestamp_ms / 1000` (integer division,
  truncate toward zero, do not round). Same rule as Step 12 list/fold reads
  (`chain_timestamp_to_fold_seconds` in `lez-payment-streams-ffi`).

Verification pipeline order (pragmatic chain reads):

1. Hex-decode `proofBytes` and `canonicalRequestBytes`.
2. Parse canonical Store request from N8 wire via existing FFI (digest/signing inputs for verify).
3. Parse outer `EligibilityProof` (FFI above); select `stream_proposal` vs `stream_proof` arm.
4. Decode inner `StreamProposal` or `StreamProof` (existing FFI).
5. Read `service_id` from proposal params (proof arm: from bound acceptance row or chain stream
   context after reads); compare to demo constant `/vac/waku/store-query/3.0.0` → `PARAMS_REJECTED`
   if mismatch.
6. Cryptographic verification (owner signature or session signature over canonical request) →
   `PROOF_INVALID`; no stream PDA reads yet.
7. Arm-specific chain reads and policy:
   - Proposal: vault config, vault holding, clock-10; `proposal_satisfies_policy` →
     `PARAMS_REJECTED` or persist upsert on `OK`.
   - Proof: stream PDA for `stream_id` from proof, clock-10, fold; if acceptance row has no
     `stream_id` yet, run `new_stream_satisfies_proposal` then set `stream_id` on success; else
     `stream_satisfies_policy` only.
8. Return `eligibility":"OK"` when the arm's checks pass.

“Purely structural” failures (malformed wrapper, bad signatures, wrong owner key) must not perform
stream PDA reads. Proposal-path vault holding reads are allowed and expected after crypto passes.

`requesterPeerId`: log on verify entry and log the final `eligibility` outcome; no rate limits or
peer-based predicates in MVP.

Side effects and out of scope for Step 13:

- `response_within_policy` applies to outbound Store response sizing, not inbound verify; defer to
  Step 17 serving path.
- Optional user-side `proposal_satisfies_policy` in `prepareEligibilityForStoreQuery` (Step 3a
  SHOULD) is alignment polish, not a Step 13 blocker.

Verification scripts (mirror Step 12):

- `VERIFY_LOGOSCORE=0`: FFI unit tests including new parse helper; existing verify/policy tests.
- `VERIFY_LOGOSCORE=1`: `./scripts/verify-step13-dod.sh` on one logoscore instance — mandatory
  happy path: `prepareEligibilityForStoreQuery` → `verifyEligibilityForStoreQuery` with the same
  `bytes_hex` and N8 canonical hex; assert `eligibility":"OK"` on seeded `stream_proof`. Mandatory
  negative: one tampered signature or canonical byte → `PROOF_INVALID`. Optional local cases (not
  CI-gated): fresh `stream_proposal` OK, expired deadline → `PARAMS_REJECTED`.
- Cross-test is single-host; two-host remains Step 17.

Runbook [`docs/step13-provider-eligibility.md`](docs/step13-provider-eligibility.md) (create with
implementation): API/encoding table (mirror Step 12), env vars for scripts (`FIXTURE_MANIFEST`,
wallet paths, `PAYMENT_STREAMS_GUEST_BIN`), prepare→verify demo sequence, troubleshooting (wallet
not open, provider id mismatch vs constants, depleted stream, clock/fold). Normative demo numbers
stay in Step 12 runbook.

#### Components required to run

`logoscore` daemon hosting both modules.
The structural-failure portion of the definition of done needs nothing more.
The happy-path verdict portion needs the Steps 10a–11b stack
(LEZ sequencer plus deployed program plus seeded vault/stream state).
Module retest loop:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

#### Definition of done

For fixed inputs the verifier returns `OK` on the happy path
and the documented eligibility status code on each failure mode
(`PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`).
Tampered or malformed proofs must fail with `PROOF_INVALID` without stream PDA reads; proposal-path
vault reads after successful crypto are in scope (see [Implementor approach](#implementor-approach-mvp-demo)).
`payment_streams_ffi_parse_eligibility_proof_bytes` ships in Step 13 FFI.
`./scripts/verify-step13-dod.sh` exits 0 with `VERIFY_LOGOSCORE=0`; with localnet up, logoscore
prepare → verify cross-test returns `eligibility":"OK"` on the seeded stream proof path.

Status: complete in tree — runbook [`docs/step13-provider-eligibility.md`](docs/step13-provider-eligibility.md),
`make verify-step13`. Logoscore cross-test may SKIP on depleted stream `0` until
`./scripts/demo-localnet-fresh.sh` (same recovery as Step 12 strict `stream_proof`).

### Step 14, Extend the Store wire format in `logos-delivery`

Architectural context:
this step modifies the Nim implementation that lives behind `delivery_module`'s FFI.
It is a wire-format change in `logos-delivery`,
not in any Logos module.
No Qt, no `LogosAPI`, no chain.

In `logos-delivery`,
add an optional opaque `eligibility_proof` field to `StoreQueryRequest` (tag `30`)
and an optional `eligibility_status` object to `StoreQueryResponse` (tag `30`),
together with an enumeration of eligibility status codes
distinct from Store status codes.
Update the codec in `waku/waku_store/rpc_codec.nim` and the typed surfaces in
`waku/waku_store/common.nim`.
Ship on our branch; no protocol-ID version bump.

Components required to run:
none beyond the Nim test infrastructure.
Round-trip tests run as two in-process Nim nodes.

Definition of done:
two Nim nodes agree on a round-trip of the new fields when present,
behaviour is unchanged when both fields are absent,
and existing Store codec coverage continues to pass.

### Step 15, Eligibility hooks in `liblogosdelivery`

Architectural context:
this step modifies the internal FFI on the delivery side —
the C ABI between `liblogosdelivery` (Nim) and `delivery_module` (C++ Qt plugin).
No Qt, no `LogosAPI`, no Logos host yet;
the C ABI is consumed by a C smoke test.

#### Registration and callbacks

In `liblogosdelivery`,
add a single C ABI registration entry point that lets a host attach
a verifier callback called for inbound Store requests carrying an `eligibility_proof`,
and a path for attaching opaque eligibility-proof bytes to outgoing Store queries.
Both surfaces are bytes-in / bytes-out and carry no payment-streams knowledge.
Existing behaviour is preserved when no callback is registered.
The verifier callback is synchronous (`Future`-returning) per N3.
Bump the `liblogosdelivery` ABI on our branch.

#### Eligibility check injection pattern

The verifier callback is injected as a decorator (wrapper)
around the existing `StoreQueryRequestHandler`.
`protocol.nim` is not modified.
At registration time,
`liblogosdelivery` replaces the active `requestHandler`
with a wrapper that:

1. Extracts `eligibility_proof` from the decoded request.
2. If present and `paidStoreMode` is enabled,
   produces `canonicalRequestBytes` from the request (see below),
   then calls the verifier callback with the serialized `eligibility_proof` bytes (opaque `EligibilityProof` protobuf, D2),
   the canonical bytes, and the requester `PeerId`.
3. On failure, returns early with `BAD_REQUEST` status code (400),
   the `eligibility_status` object populated with the verdict,
   and an empty `messages` list.
   The inner `requestHandler` is never called.
4. On success (or if no proof is present and `paidStoreMode` is off),
   delegates to the inner `requestHandler`.

This pattern keeps all eligibility logic
outside `protocol.nim` and `client.nim`.

#### Canonical Store request bytes

`canonicalRequestBytes` are produced by `liblogosdelivery`
from the Store query before eligibility bytes are attached.
On the provider side,
`liblogosdelivery` recomputes the same bytes
from the decoded inbound request
after extracting and clearing `eligibility_proof`.
These bytes are the Store eligibility `canonical_payload`;
`StreamProof.signature` signs `canonical_payload_digest`,
which `payment_streams_module` verifies.

The struct layout, domain prefix, serialization rules,
and `canonical_payload` / `canonical_payload_digest` computation are defined in N8.
The Nim serializer in this step must produce bytes
identical to the Rust serializer in Step 4.

#### Components required to run

None beyond a Nim test rig and a small C consumer
linking against the new `liblogosdelivery`.

#### Definition of done

The new C ABI is documented and used by a Nim-side smoke test.
The inbound callback is invoked exactly once
per Store request that carries a proof.
The outbound path delivers attached bytes onto the wire unchanged.
A cross-language test vector confirms
that the Nim canonical-bytes serializer
produces output identical to the Rust serializer
for a fixed `StoreQueryRequest` with known field values
(see [N8](#n8-canonical-store-request-bytes-format) for the test vector specification).

### Step 16, Generic eligibility routing in `logos-delivery-module`

Blocked on upstream Store query API (N6, Step 6).
Implement when `master` exposes query entrypoints;
method name may differ from early PR sketches.

Architectural context:
this step modifies the C++ Qt-plugin shell of `delivery_module`.
It bridges the Step 15 C callbacks into LogosAPI calls
on a configurable named module
(`payment_streams_module` in our demo;
any module with the same method names in the future).
The registration uses the auto-generated `getPluginMethods`
introspection surface every Logos module already exposes.

On our branch of `logos-delivery-module` (eligibility hooks only; build
`liblogosdelivery` / module against upstream `master` for Store query),
extend the `delivery_module` interface with
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
wire through upstream `queryStore` when present on `master`,
and add a `paidStoreMode` configuration toggle to `createNode`.
Do not add a parallel `queryStore` implementation in our fork.
Implement the bridge that translates the new `liblogosdelivery` callbacks
into `LogosAPIClient` calls on the named module
(`verifyEligibilityForStoreQuery`, `prepareEligibilityForStoreQuery`).
Note that the host application is responsible for calling
`registerProviderMapping` on the streams module before initiating queries.
On registration, the bridge calls the named module's auto-generated
`getPluginMethods` and rejects the registration with a structured error
if the expected method names are not present,
so misconfiguration surfaces at setup time rather than on the first Store request.

Components required to run:
the unit-level checks (no verifier registered, structured error on misregistration)
needs only a `logoscore` daemon with `delivery_module` loaded.
The full Store query exchange is the Step 17 demo
and requires the full stack documented there.

Definition of done:
Prerequisite: upstream Store query API on `logos-delivery-module` `master`.
Without any verifier registered,
`delivery_module` behaves exactly as it did at the pre-eligibility baseline aside from upstream
Store query APIs.
Registering a module that does not expose the expected methods
returns a structured error and leaves the previous registration in place.
Store queries can be issued through `delivery_module`
against an explicit provider peer address using the upstream Store query API.
With `payment_streams_module` registered as both verifier and provider,
an end-to-end Store query produced by the user
returns a successful Store outcome
and a successful eligibility outcome on the provider side.
Requests failing eligibility checks immediately return
a `BAD_REQUEST` (400) Store status code,
a populated `eligibility_status` object with the specific verdict,
and an empty messages list.

### Step 17, End-to-end demo wiring

Blocked on Step 16 / upstream Store query (N6).

Architectural context:
this is the only step that exercises every layer at once:
two Logos hosts (`logoscore` daemons),
all three backend modules in each host,
the LEZ sequencer for chain reads and writes,
and direct Store traffic from the user host to the provider host.

Create a single shell script that
starts a fresh scaffold workspace,
deploys `lez_payment_streams`,
builds `.lgx` packages for `logos_execution_zone` (our branch),
`payment_streams_module`,
and `delivery_module` (upstream `master` with eligibility hooks merged or
branched as in Step 16; Store query API from upstream only),
installs them with `lgpm` into two module directories,
launches two `logoscore` instances loaded with all three modules
on disjoint `portsShift` values
(per the workaround documented in
[`logos-delivery-module#18`](https://github.com/logos-co/logos-delivery-module/issues/18)
and used by `logos-delivery-demo`;
example: user `portsShift: 0`, provider `portsShift: 100`),
starts the provider `delivery_module` with relay and Store service enabled,
backed by a SQLite archive and a demo retention policy,
starts the user `delivery_module` with Store client support
and the provider's explicit peer address configured as the Store target,
drives the user flow from vault initialization through Store query,
and drives a manual claim on the provider side.
The script captures structured logs at each phase.

The first smoke path uses two nodes:
the provider archives messages and the user queries the provider directly.
For the fastest integration smoke test,
the user may publish a message that the provider archives
and then issue a paid Store query for it.
If time allows,
the demo should add a third publisher node
that publishes messages for the provider to archive,
so the user retrieves historical messages it did not originate.

Components required to run:
LEZ sequencer on `127.0.0.1:3040`,
`lez_payment_streams` program deployed onto it
(**Step 11d** complete, or documented CLI deploy from Step 10a on a clean workspace),
two `logoscore` daemons (one for user, one for provider),
each daemon hosting `logos_execution_zone`, `payment_streams_module`,
and `delivery_module`,
provider `delivery_module` configuration with relay and Store service enabled,
a SQLite Store archive path,
a retention policy such as `capacity:10000`,
user `delivery_module` configuration with the provider's explicit peer address
as the Store target,
and direct network reachability between the two local hosts.

Definition of done:
the script runs to completion against a clean workspace
and produces a log artifact that documents
every chain transaction, every Store request,
and the eligibility outcomes observed on both ends.

### Step 18, Optional Basecamp UI

Blocked on Step 17 demo wiring.

Architectural context:
the UI plugin added here is itself a Logos module
(`type: ui_qml` with a C++ backend),
not a piece of Basecamp.
Basecamp is the host that loads it,
in the same sense that `logoscore` is the host for Steps 7–17.
The plugin calls the unchanged backend modules from earlier steps
through the same `LogosAPI`;
no backend work is repeated here.

Scaffold a `ui_qml` plugin under `logos-basecamp` (or a sibling repo)
from the `logos-module-builder` `ui-qml-backend` template,
modeled on `logos-delivery-demo`.
It depends on `payment_streams_module` and `delivery_module`,
constructs `LogosModules` in `initLogos`,
and calls both modules through the generated typed wrappers.

Note: `LogosModules` is used here because UI modules run in-process with the host,
not in a `logos_host` sidecar. The crash documented in Issue #31 affects core modules only.

The plugin surfaces vault state, stream state,
the current pending-proposal slot,
and the result of the most recent Store query.
No custom backend is required for the MVP.

Components required to run:
everything from Step 17
plus `logos-basecamp` as the host
(new prerequisite — first step that uses a GUI host
instead of `logoscore`).
The new `ui_qml` module is installed via `lgpm`
into Basecamp's plugins directory.

Definition of done:
`nix build` produces a `.lgx` that loads in Basecamp without QML errors,
and a user can complete the full demo flow through the UI
without using the CLI.
