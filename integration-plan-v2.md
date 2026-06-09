# Payment Streams Integration Plan v2

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
Steps 1–15: Rust and FFI through Step 5, logoscore path Steps 7–13, delivery-repo
Steps 14–15 only (no Store query on `delivery_module`).

## Onboarding

### Recommended reading order

1. This file (`integration-plan-v2.md`).
   Steps 1–5 (Rust FFI), Steps 6–18 (integration and demo).
   Step 3 splits into 3a (core) and 3b (FFI).
   Step 6 records the closed Store-query decision; Step 8 (probe) is done; Step 9
   bootstraps the Universal module.
   Definitions of done, decisions (D1–D6), and notes (N1–N8).
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
   Step 7 first install, Step 9 Universal module, Steps 10+ dev loop (`lgpm`, `logoscore`, LEZ).
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
It does not declare `lez_wallet_module` in `metadata.json`; wallet calls are
dynamic via `invokeRemoteMethod` (D6, Step 8).

`lez_wallet_module`
is the existing Logos Core module (repo `logos-execution-zone-module`)
that wraps `wallet_ffi`.
It is the single point of contact with the LEZ chain.
Reads of vault, stream, and clock accounts go through `get_account_public`.
Writes go through `lez_wallet_module` using
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
each loading `lez_wallet_module`, `payment_streams_module`,
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
and calls `lez_wallet_module` for chain reads and writes
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
and through `lez_wallet_module`'s wrap of `wallet_ffi`.

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

`payment_streams_module` chain writes go through `lez_wallet_module`, which delegates
to the generic public transaction APIs in `wallet_ffi`.

#### LEZ FFI — PR 491 (canonical)

Upstream work lives in
[logos-execution-zone PR 491](https://github.com/logos-blockchain/logos-execution-zone/pull/491)
(`feat(wallet_ffi): wallet ffi generic transactions`).
It supersedes the narrower
[PR 429](https://github.com/logos-blockchain/logos-execution-zone/pull/429)
(`wallet_ffi_send_public_transaction`); maintainers intend to close 429 after 491 merges.

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
Same author and timeline as PR 491; this is the intended 16 replacement for generic public (and eventually private) execution.

Primary path: pin and build the patched wallet wrapper against PR 19 head + LEZ PR 491 head (see [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md)).
Step 11 calls the upstream methods PR 19 adds once pinned; read that PR for the exact `Q_INVOKABLE` names and request shape when implementing.

Our wallet work (Step 11, reduced scope):

- `sign_public_payload` per [N1](#n1-off-chain-canonical-payload-signing) — not in 491 or 19; add on our patched wrapper (LEZ FFI + Qt) until upstream ships it.
- Packaging only: `lez_wallet_module` metadata rename, CMake `wallet_ffi.h` include, codegen headers for dependents — keep the local wrapper flake; do not reimplement generic public send if PR 19 already does.

Do not pin or build against
[PR 16](https://github.com/logos-blockchain/logos-execution-zone-module/pull/16) (429 JSON wrapper) or
[PR 429](https://github.com/logos-blockchain/logos-execution-zone/pull/429).

#### Pinning

Pin `logos-execution-zone` to `refs/pull/491/head` and the wallet module upstream input to `refs/pull/19/head`
until both merge; then pin `main` on both repos.
See [`docs/feature-branch-pins.md`](docs/feature-branch-pins.md).

#### Superseded — 429 / 16 JSON shape (reference only)

429 and PR 16 used a single JSON object with lowercase hex, no `0x` prefix:

```json
{
  "program_id": "hex",
  "accounts": ["hex", "hex", ...],
  "instruction": "hex",
  "signer_account": "hex"
}
```

`logos-rln-module` may still use this shape where deployed; payment streams uses the 491 generic path, not 429.

### D4, Wallet module runtime name

Call the loaded wallet module `lez_wallet_module` (patched wrapper `metadata.json`
and `name()` aligned with operator installs; see Step 7).
Universal `payment_streams_module` does not list the wallet in `metadata.json`
(D6); load wallet before payment streams at runtime.
`logos-rln-module` may still call a wallet plugin registered under the historical
id `liblogos_execution_zone_wallet_module`.
That is unrelated to the payment-streams demo, which installs only `lez_wallet_module`
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
A `lez_` prefix would only earn its keep on a concept that is generic across chains
(as is the case for `lez_wallet_module`).
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

Call `lez_wallet_module` at runtime via
`modules().api->getClient("lez_wallet_module")->invokeRemoteMethod(...)`.
Keep `"dependencies": []`.
Step 8 validated Universal caller to Legacy callee.

Justification.
Universal static dependencies exist so codegen can emit typed `modules().<name>` wrappers.
That assumes every dependency is Universal.
`lez_wallet_module` is still Legacy, so listing it in `metadata.json` would not produce safe typed calls to its `Q_INVOKABLE` API.
Dynamic access keeps payment streams on the Universal side (with `delivery_module`) while the wallet stays Legacy.
We rely on explicit load order and runtime errors if the wallet is absent.
Revisit a static dependency when the wallet module is Universal upstream and codegen supports it.

### N1, Off-chain canonical-payload signing

Neither `wallet_ffi` nor `lez_wallet_module` currently exposes
a primitive that signs an arbitrary canonical payload with a wallet account's key.
That primitive is required for `VaultProof.owner_signature`,
because the vault proof must prove control of the LEZ vault owner key.
For the MVP, we add `sign_public_payload` to `lez_wallet_module`
on our branch (see Step 11 wallet write helpers and N8).

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
as a flat JSON file in `instancePersistencePath`, atomically written.
If persistence fails (disk full, permissions), the module logs the error
and continues with in-memory state only.
Stale proposals are evicted on a timer and on cold start.
A hardened build would encrypt session keys through a wallet-rooted KDF
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
| 10–13 | Module reads, writes, eligibility | [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) Part 3 |
| 14–15 | Store wire + `liblogosdelivery` hooks | Nim/C repos; no logoscore loop |
| 16–18 | Routing, E2E demo, Basecamp UI | Blocked on upstream Store query (Step 6) |

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
how `.lgx` packages are produced for both `lez_wallet_module` and `payment_streams_module`,
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
   installing `lez_wallet_module` before `payment_streams_module`,
   running `lgpm list` (two modules),
   starting `logoscore -D -m` (daemon plus `capability_module`),
   then loading wallet and payment streams via `load-module` or `-l lez_wallet_module,payment_streams_module`,
   without ad hoc relative `PATH` hacks
   (prefer `nix shell` in each terminal tab; see operator guide).

### Step 8, Universal-to-Legacy wallet probe

Architectural context:
Before committing to Universal `payment_streams_module` (D6), we had to determine whether
`logoscore` can safely route a dynamic call from a Universal module to Legacy
`lez_wallet_module` without listing the wallet in `metadata.json`.
Historical rationale and probe design:
[`docs/step8-universal-legacy-probe-results.md`](docs/step8-universal-legacy-probe-results.md)
(appendix references the retired dilemma write-up).

The probe module uses the `logos-module-builder` Universal template.
It does not add `lez_wallet_module` to `metadata.json` `dependencies`.
It calls the wallet via
`modules().api->getClient("lez_wallet_module")->invokeRemoteMethod(...)`.
Compile the probe and run `logoscore`, loading Legacy wallet then Universal probe.

Status: done for Step 8 goal (2026-06-08).
Validated: Universal caller invokes Legacy `lez_wallet_module` without a static dependency;
daemon stable; `invokeRemoteMethod` dispatch works (empty account list OK for marshaling smoke).
Not validated in the probe: funded wallet / scaffold storage via module `open` (Pass B storage failure).
See [`docs/step8-universal-legacy-probe-results.md`](docs/step8-universal-legacy-probe-results.md) and
[`logos-universal-legacy-probe/docs/probe-runbook-and-results.md`](../logos-universal-legacy-probe/docs/probe-runbook-and-results.md).

Decision D6: build `payment_streams_module` as Universal; call the wallet dynamically with empty
`dependencies`.

Components required to run:
`logoscore`, patched `lez_wallet_module`, probe `.lgx`, optional LEZ for Pass B only.

Definition of done:
probe loads with wallet; `invokeRemoteMethod` reaches `lez_wallet_module` without host crash;
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
Call `lez_wallet_module` at runtime via `invokeRemoteMethod`; keep `"dependencies": []`.
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
Step 8 validated Universal-to-Legacy wallet `invokeRemoteMethod`; Step 10 adds the first
in-module wallet calls.

Implementor hints (FFI from Step 5, no extra Qt surface yet):

- Link `liblez_payment_streams_ffi` named in metadata `include`, and vendor
  [`lez_payment_streams_ffi.h`](lez-payment-streams-ffi/lez_payment_streams_ffi.h) like
  `logos-rln-module` (CMake + flake inputs).
  Step 9 is load/plumbing only; do not call instruction entrypoints from C++ until Step 11.
- On-chain instruction bytes and account-list planning live in
  [`lez-payment-streams-ffi/src/instruction_abi.rs`](lez-payment-streams-ffi/src/instruction_abi.rs).
  Skim the file-level doc for two-phase output sizing and 64-byte lowercase hex stride per account.
- Instruction payloads follow
  [`lez-payment-streams-core/src/instruction_wire.rs`](lez-payment-streams-core/src/instruction_wire.rs),
  not Step 4 protobuf/N8.
- Deposit helpers use `payment_streams_ffi_authenticated_transfer_program_id_bytes` where needed.

Pin `logos-module-builder` recent enough for Universal glue (same band as
`logos-universal-legacy-probe`). Wallet LEZ rev aligned with patched wallet flake
(module PR 19 + LEZ pin via `lez-python-overlay`; see feature-branch pins).

Components required to run:
`logoscore` host (first step needing a running Logos host).
No chain, messaging network, or UI host.

Status: done (Universal skeleton in `logos-payment-streams-module/`).

Definition of done:

1. `nix build ./logos-payment-streams-module#lgx` produces a valid `.lgx`
2. `lgpm install` places the module alongside patched `lez_wallet_module`
3. `logoscore` loads the module after explicit `load-module` (wallet first, then payment streams)
4. `payment_streams_module` reports loaded; daemon shows no crash
5. `logoscore call lez_wallet_module list_accounts` reaches the wallet RPC (LEZ optional for failure-only smoke)

Operator commands:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 1).
From Step 10 onward, repeat build/install/load and LEZ:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

### Step 10, Wire chain reads from the module

Architectural context:
this step adds stable read helpers and exercises wallet-backed chain reads end-to-end for the first time.
`payment_streams_module` calls into `lez_wallet_module`,
which in turn uses its own FFI (`wallet_ffi`) to reach the LEZ sequencer
over JSON-RPC.
The chain is now part of the picture.

Add helpers inside `payment_streams_module` that wrap
`lez_wallet_module.account_id_from_base58` and `lez_wallet_module.get_account_public`,
plus a higher-level helper that reads the configured clock account
and returns the current sequencer time.
These helpers are pure read paths and do not touch any payment-streams logic.
Use `LogosAPIClient::invokeRemoteMethod` directly,
following the safe pattern documented in
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 2).
Do not use the `LogosModules` typed wrapper as it crashes in core module sidecars.

Components required to run:
`logoscore` daemon hosting both `lez_wallet_module` and `payment_streams_module`,
LEZ sequencer on `127.0.0.1:3040`
(new prerequisite — first step that needs a chain;
brought up by `lgs init` + `lgs setup` + `lgs localnet start` from a scaffold workspace),
`lez_payment_streams` program deployed onto that sequencer
(new prerequisite — via `lgs deploy`).
No messaging network yet.

Scaffold RPC, account id formats, and deploy notes:
[`docs/step1-findings-scaffold-rpc.md`](docs/step1-findings-scaffold-rpc.md).
Repeatable logoscore + LEZ test loop after module edits:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

Definition of done:
against a scaffold-deployed `lez_payment_streams` program,
the module can read a known vault config, vault holding, stream config,
and clock account through `logoscore`,
and the JSON returned by `get_account_public` decodes through the FFI
into the expected typed values.

### Step 11, Add and wire chain writes through the wallet module

Architectural context:
Sub-step 11a pins PR 491 + PR 19, rebuilds wallet `.lgx`, and adds `sign_public_payload`
on the patched wrapper only (N1).
Generic public send comes from PR 19, not custom code in this repo.
Sub-step 11b uses LogosAPI from `payment_streams_module` to call PR 19’s methods.
Instruction bytes and account lists are built through
`payment_streams_module`'s Rust FFI layer (from Steps 1–5).

Two sub-steps that ship together:

Sub-step 11a (wallet — mostly upstream):

Ensure flakes pin LEZ [PR 491](https://github.com/logos-blockchain/logos-execution-zone/pull/491) and
wallet module [PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19).
Rebuild the patched wallet `.lgx`; `lm methods` should list PR 19’s generic transaction entry point(s).

Add `sign_public_payload(accountId, canonical_payload_digest_hex) -> QString` on the patched wrapper only
(LEZ FFI + Qt), per [N1](#n1-off-chain-canonical-payload-signing).
Do not duplicate 491’s generic public send in our patch if PR 19 already exposes it.

Sub-step 11b:
add a private helper inside `payment_streams_module`
that takes an `Instruction` kind, its typed arguments,
and a signer account ID,
builds Borsh instruction bytes and the ordered account list through the payment-streams FFI,
assembles the program-with-dependencies bundle for the guest program,
and submits via `lez_wallet_module` using PR 19’s API (match upstream request shape from that PR).

Expose user-facing `Q_INVOKABLE` write methods
for the nine payment-stream operations:
initialize vault, deposit, withdraw, create stream, top up,
pause, resume, close, claim.

Expose user-facing `Q_INVOKABLE` status helper methods:
`getVaultStatus(vaultConfigAccountId) -> QString`
reads the vault config and vault holding accounts via `get_account_public`,
decodes both through the FFI,
and returns a JSON object with owner, privacy tier, total allocated,
vault holding balance, and derived unallocated balance.
`getStreamStatus(streamConfigAccountId) -> QString`
reads the stream config account and the clock account,
decodes both through the FFI,
folds the stream to the current clock time via `at_time`,
and returns a JSON object with stream ID, provider, rate, allocation,
accrued, unaccrued, effective state, and `accrued_as_of`.

These read methods compose the Step 10 chain-read helpers
with the Step 2 decoders and Step 3a/3b folding logic.
They are used by the demo script in Step 17
to verify intermediate state
and by the optional Basecamp UI in Step 18.

Components required to run:
Sub-step 11a needs no runtime
(verified via `nix build` and `lm methods` showing the new surface).
Sub-step 11b needs the same runtime stack as Step 10
(`logoscore` with both modules loaded, LEZ sequencer, deployed program).
Retest loop: [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

Definition of done:
through `logoscore` against scaffold localnet,
the module can drive a complete vault and stream lifecycle from initialization
through claim,
with on-chain state observable through `getVaultStatus` and `getStreamStatus`.

### Step 12, Session keys and user-side proof construction

Architectural context:
this is the user-side method that `delivery_module` will auto-invoke
once registered as the outbound eligibility provider in Step 16.
It does not, by itself, initiate any Store traffic;
it just produces opaque bytes when asked.

#### Quick reference

| Method | Purpose | Called by |
|--------|---------|-----------|
| `prepareEligibilityForStoreQuery` | Returns `StreamProposal` or `StreamProof` | `delivery_module` (auto) |
| `registerProviderMapping` | Maps `PeerId` to `providerId` | Host application |
| `listMyStreams` | Lists streams for a vault | Host application |
| `rediscoverStreams` | Re-enumerates streams from chain | Host application (recovery) |

#### User-side flow

The intended sequence for a new provider relationship is:

1. Host application calls `registerProviderMapping`
   to bind the provider's libp2p `PeerId`
   to its generic `providerId` and LEZ account ID.
2. User issues a Store query.
   `delivery_module` invokes `prepareEligibilityForStoreQuery`.
   The module has no established stream for this `(vault, provider)` pair,
   so it generates a session keypair, persists it,
   and returns a `StreamProposal`.
3. Provider accepts the proposal and serves the first request.
4. User explicitly calls `create_stream`
   (the Step 11b write method) to open the stream on-chain.
   This is a manual action by the host application or demo script,
   never triggered automatically by any hook.
5. User issues the next Store query.
   `delivery_module` invokes `prepareEligibilityForStoreQuery` again.
   The module queries `get_account_public` for the `StreamConfig` PDA,
   confirms it exists and is `ACTIVE`,
   and returns a `StreamProof` signed by the session key.

#### Session and stream state management

Add session-keypair management inside `payment_streams_module`,
backed by atomic JSON in `instancePersistencePath` (see [N4](#n4-persistence-policy)).
The persisted state per `(vault_id, provider_id)` includes:
the `stream_id` (allocated locally, used as the PDA seed on-chain),
the session keypair,
the proposal status (pending, established, expired),
and the last known on-chain stream state.

The module maintains a local inventory of stream IDs per vault.
Every `create_stream` call records the new `stream_id` in the inventory.
This inventory is the backing store for `listMyStreams`.
Stale proposals are evicted on a timer and on cold start.

#### Exposed methods

`prepareEligibilityForStoreQuery(canonicalRequestBytes, providerPeerId) -> QString`
returns either a `StreamProposal` or a `StreamProof` byte string
depending on whether the stream for the `(vault, provider)` pair
has been established on-chain.
Before returning a `StreamProof`,
the module reads the `StreamConfig` PDA via `get_account_public`,
decodes it through the FFI,
folds it at the current clock time,
and checks that the effective state is `ACTIVE`.
For `StreamProposal` output,
the module asks `lez_wallet_module.sign_public_payload`
to produce `VaultProof.owner_signature` with the vault owner's LEZ key.
Later `StreamProof`s are signed with the persisted session key.

`registerProviderMapping(providerPeerId, providerId, providerAccountId) -> LogosResult`
lets the host configure the identity mapping (see [N5](#n5-provider-identity-mapping)).

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
  The module evicts the stale proposal.
  The next call generates a fresh `StreamProposal`.
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
- `WALLET_SIGNING_FAILED`:
  `sign_public_payload` call to wallet module failed.
  Error includes upstream details.
- `CHAIN_READ_FAILED`:
  `get_account_public` call failed.
  Error includes upstream details.

#### Components required to run

`logoscore` daemon hosting both modules.
The definition of done's verifier round-trip is in-process through the FFI;
a live sequencer is not strictly required for that verification itself,
but the same Step 10 stack remains useful for sanity-checking
that vault data the proof asserts matches chain state.
After code changes, rebuild and reload via
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

#### Definition of done

The module produces a syntactically valid eligibility proof byte string
for fixed inputs;
restarts cleanly with state intact;
the FFI structural verifier accepts the proof format;
`listMyStreams` returns correct folded status for locally known streams;
each user-side error condition returns the documented error code;
and (when chain state is available) the provider-side verifier accepts
the proof against actual on-chain stream state.

### Step 13, Provider-side proof verification

Architectural context:
this is the provider-side method that `delivery_module` will auto-invoke
once registered as the inbound eligibility verifier in Step 16.
Structural checks happen entirely through the Rust FFI;
chain checks happen via LogosAPI calls to `lez_wallet_module`.

Expose a single provider-side `Q_INVOKABLE` method
`verifyEligibilityForStoreQuery(proofBytes, canonicalRequestBytes, requesterPeerId)`
that parses and dispatches the proof,
runs structural checks through the FFI,
queries chain state through the wallet module,
folds stream state at the current sequencer time,
and returns a structured verdict mapping to LIP-155 outcomes.

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

#### Components required to run

`logoscore` daemon hosting both modules.
The structural-failure portion of the definition of done needs nothing more.
The happy-path verdict portion needs the Step 10 stack
(LEZ sequencer plus deployed program plus seeded vault/stream state).
Module retest loop:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

#### Definition of done

For fixed inputs the verifier returns `OK` on the happy path
and the documented eligibility status code on each failure mode
(`PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`),
without performing chain reads when the failure is purely structural.

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
   then calls the verifier callback with the proof bytes,
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
builds `.lgx` packages for `lez_wallet_module` (our branch),
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
`lez_payment_streams` program deployed onto it,
two `logoscore` daemons (one for user, one for provider),
each daemon hosting `lez_wallet_module`, `payment_streams_module`,
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
