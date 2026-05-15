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

## Onboarding

### Recommended reading order

1. This file (`integration-plan-v2.md`).
   The 15-step plan (Step 3 is split into 3a core and 3b FFI),
   definitions of done,
   resolved decisions (D1–D5),
   and non-blocking notes (N1–N8).
   Day-to-day reference for the work.
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
   and `StreamProviderPolicy` (see `docs/step3-stream-provider-policy.md`).

### Prerequisites

- Nix with flakes enabled.
- Rust stable toolchain.
- `logos-scaffold` binary (`lgs` alias)
  for localnet, wallet, and program deploy.
- `logoscore`, `lgpm`, `lm` —
  available through `logos-module-builder` outputs
  or as separate flake-buildable binaries.
- Outbound internet access for the `logos.dev` messaging-network preset
  during Step 14 and Step 15.
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
(importable in Rust under `lez_payment_streams_core`).
This work extends `lez-payment-streams-core` with stream folding helpers,
`StreamProviderPolicy`, and pure policy predicates (Step 3a),
then adds a sibling `lez-payment-streams-ffi` crate
that exposes PDA derivation,
account decoders,
stream folding and policy across the C ABI (Step 3b),
off-chain proof construction and verification,
and Borsh instruction builders through `extern "C"`.
The shape mirrors `lez-rln-ffi` in `logos-lez-rln`.

`payment_streams_module`
is a new Logos Core module (Qt plugin in C++) that wraps the FFI.
It is built with `logos-module-builder`
using `mkLogosModule` against `metadata.json`,
mirroring the layout of `logos-rln-module`
(closest precedent for a Rust-FFI-backed core module)
and following the developer guide in `logos-tutorial`.
It owns
session keys for off-chain proofs,
pending-proposal state,
the user-facing flow for opening, topping up, claiming, and closing streams,
and the provider-facing flow for verifying eligibility proofs.
It declares a runtime dependency on `lez_wallet_module`.

`lez_wallet_module`
is the existing Logos Core module (repo `logos-execution-zone-module`)
that wraps `wallet_ffi`.
It is the single point of contact with the LEZ chain.
Reads of vault, stream, and clock accounts go through `get_account_public`.
Writes go through `send_public_transaction`
(JSON request shape `program_id` / `accounts` / `instruction` / `signer_account`),
which is already exercised by `logos-rln-module`
and is implemented in an open PR on the wallet module (see D3).

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
(repo `logos-delivery-module`, currently pinned at `v0.1.1`)
that wraps `liblogosdelivery` and exposes the typed `delivery_module` API
documented in the journey doc
[Use the Logos Delivery module API from an app](https://github.com/logos-co/logos-docs/blob/main/docs/messaging/journeys/use-the-logos-delivery-module-api-from-an-app.md).
This work adds two thin routing methods on its interface,
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
that bridge the new `liblogosdelivery` hooks to a named Logos module
via `LogosAPI` / `LogosAPIClient`.
It also exposes `storeQuery(jsonQuery, peerAddr, timeoutMs)`
so modules and apps can issue Store queries through `delivery_module`.
At registration time the bridge uses each module's auto-generated
`getPluginMethods` surface to confirm that the named module exposes
the expected verifier and provider methods,
so a misconfigured registration fails fast with a structured error.
`payment_streams_module` is one such named module;
future modules (different incentivization schemes) can register the same way.
We ship `logos-delivery-module` from our own branch
pinned in `payment_streams_module`'s `flake.nix`.

`logoscore`
is the headless runtime used to load and exercise modules during integration testing.
The end-to-end demo runs two `logoscore` instances on one host,
one as user and one as provider,
each loading `lez_wallet_module`, `payment_streams_module`,
and our branch of `delivery_module`.

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
the JSON request shape for `send_public_transaction`,
and its hex/byte conversion-helper patterns.
Start at `src/logos_rln_module.cpp`.
Note the JSON request shape it sends to `send_public_transaction`
(`program_id`, `accounts`, `instruction`, `signer_account`) —
we adopt the same shape per D3.

`logos-delivery-module` (currently pinned at `v0.1.1` upstream)
is the module we extend in Step 13.
Skim `README.md` and `src/delivery_module_plugin.h`
to see the surface we add `setEligibilityVerifier`
and `setEligibilityProvider` to.

`logos-delivery-demo`
is the reference `ui_qml` module showing how a Qt module consumes `delivery_module`.
It is the closest precedent for the optional Basecamp UI plugin in Step 15
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
`logos-delivery-module` (our branch) gains
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
`storeQuery(jsonQuery, peerAddr, timeoutMs)`,
plus a `paidStoreMode` configuration toggle.
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

`payment_streams_module` writes go through `lez_wallet_module.send_public_transaction`,
matching the JSON request shape `logos-rln-module` already uses
(`program_id`, `accounts`, `instruction`, `signer_account`).
That method is implemented in an open PR on `logos-execution-zone-module`.
If the PR is merged before we reach Step 8, we use mainline;
otherwise we use the feature branch where it is implemented.
No subprocess fallback is needed.

### D4, Wallet module dependency name

We adopt `lez_wallet_module` (the name reported in current `metadata.json`)
as the dependency name in `payment_streams_module`.
The fact that `logos-rln-module` still resolves the old name
`liblogos_execution_zone_wallet_module`
is out of scope for this work.

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

### N1, Off-chain canonical-payload signing

Neither `wallet_ffi` nor `lez_wallet_module` currently exposes
a primitive that signs an arbitrary canonical payload with a wallet account's key.
That primitive is required for `VaultProof.owner_signature`,
because the vault proof must prove control of the LEZ vault owner key.
For the MVP, we add `sign_public_payload` to `lez_wallet_module`
on our branch (see Step 8a for the method signature).

No domain parameter is included.
The NSSA wallet avoids exposing generic signing entirely
(`wallet_ffi` only exposes complete transaction workflows;
see `logos-execution-zone/wallet-ffi/src/transfer.rs`).
Our method introduces the first generic signing endpoint,
but any co-hosted module can already submit arbitrary transactions
via `send_public_transaction`, which is strictly more powerful.
Domain separation would not reduce the attack surface
in the current trust model.
The payment-streams FFI already applies its own domain-separated prehashing
(see N8).
If the ecosystem later introduces a module permission model,
domain separation on signing should be revisited.

Payment-stream proofs use NSSA's existing transparent signature contract:
32-byte x-only secp256k1 public keys,
64-byte Schnorr signatures,
and signatures over a 32-byte domain-separated SHA-256 prehash.
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

The lower-level `logos-delivery` C ABI already exposes Store queries,
but the current `logos-delivery-module` Qt surface does not.
We ask the Delivery team to expose the existing Store query functionality
through the module surface as
`storeQuery(jsonQuery, peerAddr, timeoutMs)`.
This is exposing already-implemented functionality, not a spec change;
no Store protocol semantics are modified.
If the upstream change does not land in time for the demo,
we implement the method on our branch.

### N7, Session key concurrency

Session key signing is synchronous.
Concurrent Store queries to the same provider serialize at the session key.
The MVP assumes low concurrency; production would need key pooling or async queuing.

### N8, Canonical Store request bytes format

The canonical Store request bytes are the payload
signed by `StreamProof.signature` and verified by `payment_streams_module`.
They are produced by `liblogosdelivery` (Nim, Step 12)
and consumed by `lez-payment-streams-ffi` (Rust, Step 4).
Both sides must produce identical bytes for the same input.

The format follows the NSSA precedent
from `nssa/src/public_transaction/message.rs`:
Borsh-serialize a struct,
prepend a fixed 32-byte domain prefix,
SHA-256 the concatenation to produce a 32-byte prehash.

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

#### Prehash computation

```
prehash = SHA-256(PREFIX || borsh(CanonicalStoreRequest))
```

This 32-byte prehash is what `StreamProof.signature` signs.

#### Cross-language test vector

The definition of done for Step 12 requires a pinned test vector:
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

### Step 1, Bootstrap the Rust FFI crate

Architectural context:
Steps 1 through 5 build the Rust artifacts for `payment_streams_module`:
`lez-payment-streams-ffi` (Steps 1–2 and 3b–5)
and `lez-payment-streams-core` extensions for folding and policy (Step 3a),
which Step 3b exposes across the C ABI.
No Qt plugin shell, no Logos host, no chain are involved yet —
this is pure Rust crate work that will later be linked into the module.

Create `lez-payment-streams-ffi` as a sibling crate to `lez-payment-streams-core`,
mirroring the `lez-rln-ffi` shape
(`crate-type = ["rlib", "cdylib", "staticlib"]`,
`cbindgen` build script,
generated `lez_payment_streams_ffi.h`).
Add the new crate to the `workspace.members` array in the root `Cargo.toml`.
Start with a stub function and an error enum
so the build pipeline is wired before functionality lands.

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

Architectural context:
pure `lez-payment-streams-core` work.
No `lez-payment-streams-ffi`, no C ABI, no Qt.
This is the single source of truth for fold math and policy comparisons
so Step 3b stays a thin wrapper.

Semantic reference:
LIP-155 at `rfc-index/docs/ift-ts/raw/payment-streams.md`
(StreamProviderPolicy, protocol phases, predicate names).
See also `docs/step3-stream-provider-policy.md` and
`docs/step3a-implementor-notes.md` in this repo.

#### Fold

Add or reuse stream-state folding driven by `StreamConfig::at_time`,
surfacing effective state, accrued amount, and remaining allocation
(unaccrued) for a given sequencer time.

#### StreamProviderPolicy

Rust struct mirroring LIP-155:

- `min_stream_rate`, `min_stream_allocation`
- `max_create_stream_deadline_delay`
- `vault_proof_max_response_bytes`

Proposal-phase solvency: read on-chain unallocated for `vault_id` and
require `unallocated ≥ stream_allocation`.
`VaultProof` wire fields per LIP-155 [LEZ off-chain integration](rfc-index/docs/ift-ts/raw/payment-streams.md#lez-off-chain-integration).

#### Policy predicates

Implement pure functions in `lez-payment-streams-core` using the same
names as LIP-155.
Each returns `Result<(), PolicyRejectReason>` (Rust enum; map to FFI in 3b).
Document test vectors for each:

| Function | Phase | Inputs (summary) |
| --- | --- | --- |
| `proposal_satisfies_policy` | Proposal | Policy mins; `create_stream_deadline` vs `now` and `max_create_stream_deadline_delay` (LEZ: clock account timestamp); `vault_holding_balance` and `vault_config.total_allocated` or `unallocated ≥ stream_allocation` |
| `new_stream_satisfies_proposal` | Service (first `StreamProof` per service session) | Folded `StreamConfig` at `now`; stored `allocation` and `rate` ≥ accepted `StreamParams`; `StreamConfig.provider` vs `VaultProof.provider_id` (LEZ: 32-byte equality). Later proofs need not run it (MAY re-run). |
| `stream_satisfies_policy` | Service (every `StreamProof`) | Folded stream `ACTIVE`; rate ≥ `min_stream_rate`; provider binding; policy vs session commitment; `now` |
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

| PolicyRejectReason (examples) | Eligibility status |
| --- | --- |
| Rate / allocation below policy or proposal | `PARAMS_REJECTED` |
| Deadline / unallocated | `PARAMS_REJECTED` |
| Signature / wire format | `PROOF_INVALID` (Step 4) |
| Stream not `ACTIVE` | `STREAM_NOT_ACTIVE` |

#### Out of scope for Step 3a

- Cryptographic binding (`VaultProof` / `StreamProof` signatures) — Step 4.
- Load cap (stateful metering) — LIP-155 extension; MVP deferred.
- Discovery wire encoding for policy — future; core defines struct + checks only.

#### Implementor clarifications

- Units: `stream_rate` and `stream_allocation` in proposals use the same
  integer scales as on-chain `TokensPerSecond` and `Balance` (native token).

- Response cap: demo default 65536; LIP-155 SHOULD, demo provider MUST
  (see `response_within_policy` above).

- Step 3a uses typed Rust policy inputs only (no protobuf in core).
  See `docs/step3a-implementor-notes.md` for suggested structs,
  `PolicyRejectReason` variants, predicate pitfalls, and vector checklist.

Components required to run: none.
`cargo test` on `lez-payment-streams-core` only.

Definition of done:
folding outputs match `StreamConfig::at_time` on a documented vector set;
each predicate (including `response_within_policy`) is deterministic with
documented pass/fail inputs;
vectors live in-repo (`docs/step3a-implementor-notes.md` and/or test modules)
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
  split wide amounts for C using the same low/high limb pattern as other
  `PaymentStreams` FFI types.
- `MAX_SERVICE_ID_LEN` documents the intended `StreamParams.service_id` cap;
  core does not reject overlong `Vec<u8>` — enforce length in the module
  before signing (Step 4 owns wire validation).
- `AcceptedStreamTerms.provider_id` is `AccountId` in Rust (32-byte id).

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
The FFI owns all domain-separated canonicalization and prehashing.

The canonicalization format follows the NSSA precedent
established in `nssa/src/public_transaction/message.rs`:
Borsh-serialize a struct, prepend a fixed 32-byte domain prefix,
SHA-256 the result to produce the 32-byte prehash that is signed.
See N8 for the full specification of this format
and the canonical Store request bytes structure
that is shared between the Rust FFI (this step) and Nim (Step 12).

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

Components required to run: none.

Definition of done:
encoded payloads round-trip through `lez-payment-streams-core` Borsh decoders,
and account-list planners agree with the harness builders in
`lez-payment-streams-core/src/test_helpers.rs`.

### Step 6, Bootstrap the Logos Core module

Architectural context:
this step lays down the C++ Qt-plugin shell of `payment_streams_module`.
The shell is a Qt plugin (`type: core`)
that will host the Rust FFI crate (from Steps 1–5)
and expose LogosAPI methods to other modules in later steps.

Pattern decision point:
the plan is written assuming the legacy `PluginInterface` pattern
(manual `Q_INVOKABLE`, `QString` types) to match `logos-rln-module`.
Before implementing Step 6, verify the current state of `delivery_module`
and `lez_wallet_module` — if they have adopted the universal pattern
(`"interface": "universal"`, pure C++ + code generation from `logos-dev-boost`),
the new approach is preferable and future-proof.
If they remain on legacy, stick with legacy to avoid runtime compatibility risk.
The ecosystem direction should be checked at this step; do not proceed
with either pattern until the partner module status is confirmed.

Before implementing Step 6, check the current state of
`lez_wallet_module` and `delivery_module`.
If they have adopted the universal pattern
(`"interface": "universal"` in `metadata.json`,
code generation via `logos-cpp-generator`),
use the new pattern for `payment_streams_module`.
If they remain on the legacy pattern
(manual `Q_INVOKABLE`, `PluginInterface`),
stay with legacy to avoid runtime compatibility risk.

Confirm the partner modules' status before proceeding,
and refer to the "Module Implementation Patterns" section in `logos-architecture-overview.md` for details.

Scaffold the module from the `logos-module-builder`
`with-external-lib` template, modeled on `logos-rln-module`
and the Rust-FFI pattern documented in `logos-tutorial/logos-developer-guide.md`.
Per D5, the directory is `logos-payment-streams-module/`,
placed alongside `lez-payment-streams-core/` and `lez-payment-streams-ffi/`
in this repo.
It ships
`metadata.json` (with `name = "payment_streams_module"`,
`type = "core"`,
`dependencies = ["lez_wallet_module"]`,
and `include` listing all platform variants of the FFI shared library:
`liblez_payment_streams_ffi.so`,
`liblez_payment_streams_ffi.dylib`,
`liblez_payment_streams_ffi.dll`),
`flake.nix` calling `mkLogosModule`,
`CMakeLists.txt`,
and `src/payment_streams_module_plugin.{h,cpp}` plus `src/i_payment_streams_module.h`.
The plugin implements `PluginInterface` and exposes only `initLogos` and `name` for now.

Components required to run:
`logoscore` daemon as the host
(new prerequisite — first step that needs a running Logos host).
No chain, no messaging network, no UI host.

Definition of done:
`nix build` produces an `.lgx`,
`lgpm install` lays it out alongside `lez_wallet_module`,
`logoscore` loads it without errors,
and `lm methods` reports the empty plugin surface as expected.

### Step 7, Wire chain reads from the module

Architectural context:
this step exercises LogosAPI (inter-module calls) for the first time.
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
mirroring `logos-rln-module/src/logos_rln_module.cpp`,
until the generated typed wrapper for `lez_wallet_module` lands here.

Components required to run:
`logoscore` daemon hosting both `lez_wallet_module` and `payment_streams_module`,
LEZ sequencer on `127.0.0.1:3040`
(new prerequisite — first step that needs a chain;
brought up by `lgs init` + `lgs setup` + `lgs localnet start` from a scaffold workspace),
`lez_payment_streams` program deployed onto that sequencer
(new prerequisite — via `lgs deploy`).
No messaging network yet.

Definition of done:
against a scaffold-deployed `lez_payment_streams` program,
the module can read a known vault config, vault holding, stream config,
and clock account through `logoscore`,
and the JSON returned by `get_account_public` decodes through the FFI
into the expected typed values.

### Step 8, Add and wire chain writes through the wallet module

Architectural context:
sub-step 8a adds new methods to `lez_wallet_module`'s Qt-plugin surface,
delegating to `wallet_ffi` underneath
(both boundaries of the wallet module are touched).
Sub-step 8b uses LogosAPI from `payment_streams_module` to call it.
Instruction bytes and account lists are built through
`payment_streams_module`'s Rust FFI layer (from Steps 1–5).

Two sub-steps that ship together:

Sub-step 8a:
on our branch of `logos-execution-zone-module`
(or mainline if the open PR has merged; see D3),
add `send_public_transaction(QString jsonRequest) -> QString`
on `lez_wallet_module`,
matching the JSON shape already produced by `logos-rln-module`
(`program_id`, `accounts`, `instruction` hex, `signer_account`).
The method delegates to the underlying `wallet_ffi` signing-and-submit path.
Add `sign_public_payload(accountId, prehashHex) -> QString`
on the same branch,
returning a 64-byte NSSA Schnorr signature for a 32-byte prehash
with the public account's signing key.
No domain parameter; see N1 for rationale.

Sub-step 8b:
add a private helper inside `payment_streams_module`
that takes an `Instruction` kind, its typed arguments,
and a signer account ID,
builds the Borsh instruction bytes and account list through the FFI,
serializes the JSON request,
and submits via `lez_wallet_module.send_public_transaction`.

Expose user-facing `Q_INVOKABLE` write methods
for the nine payment-stream operations:
initialize vault, deposit, withdraw, create stream, top up,
pause, resume, close, claim.

Expose user-facing `Q_INVOKABLE` read methods:
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

These read methods compose the Step 7 chain-read helpers
with the Step 2 decoders and Step 3a/3b folding logic.
They are used by the demo script in Step 14
to verify intermediate state
and by the optional Basecamp UI in Step 15.

Components required to run:
sub-step 8a needs no runtime
(verified via `nix build` and `lm methods` showing the new surface).
Sub-step 8b needs the same set as Step 7
(`logoscore` daemon with both modules, LEZ sequencer, deployed program).

Definition of done:
through `logoscore` against scaffold localnet,
the module can drive a complete vault and stream lifecycle from initialization
through claim,
with on-chain state observable through `getVaultStatus` and `getStreamStatus`.

### Step 9, Session keys and user-side proof construction

Architectural context:
this is the user-side method that `delivery_module` will auto-invoke
once registered as the outbound eligibility provider in Step 13.
It does not, by itself, initiate any Store traffic;
it just produces opaque bytes when asked.

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
   (the Step 8b write method) to open the stream on-chain.
   This is a manual action by the host application or demo script,
   never triggered automatically by any hook.
5. User issues the next Store query.
   `delivery_module` invokes `prepareEligibilityForStoreQuery` again.
   The module queries `get_account_public` for the `StreamConfig` PDA,
   confirms it exists and is `ACTIVE`,
   and returns a `StreamProof` signed by the session key.

#### Session and stream state management

Add session-keypair management inside `payment_streams_module`,
backed by atomic JSON in `instancePersistencePath` (see N4).
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
lets the host configure the identity mapping (see N5).

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
but the same Step 7 stack remains useful for sanity-checking
that vault data the proof asserts matches chain state.

#### Definition of done

The module produces a syntactically valid eligibility proof byte string
for fixed inputs;
restarts cleanly with state intact;
the FFI structural verifier accepts the proof format;
`listMyStreams` returns correct folded status for locally known streams;
each user-side error condition returns the documented error code;
and (when chain state is available) the provider-side verifier accepts
the proof against actual on-chain stream state.

### Step 10, Provider-side proof verification

Architectural context:
this is the provider-side method that `delivery_module` will auto-invoke
once registered as the inbound eligibility verifier in Step 13.
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
  (rate below `min_stream_rate`, allocation below `min_stream_allocation`,
  `create_stream_deadline` outside `max_create_stream_deadline_delay`),
  or vault unallocated balance is below proposed `stream_allocation`,
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
The happy-path verdict portion needs the Step 7 stack
(LEZ sequencer plus deployed program plus seeded vault/stream state).

#### Definition of done

For fixed inputs the verifier returns `OK` on the happy path
and the documented eligibility status code on each failure mode
(`PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`),
without performing chain reads when the failure is purely structural.

### Step 11, Extend the Store wire format in `logos-delivery`

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

### Step 12, Eligibility hooks in `liblogosdelivery`

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
These bytes are the payload signed by `StreamProof.signature`
and verified by `payment_streams_module`.

The struct layout, domain prefix, serialization rules,
and prehash computation are defined in N8.
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
(see N8 for the test vector specification).

### Step 13, Generic eligibility routing in `logos-delivery-module`

Architectural context:
this step modifies the C++ Qt-plugin shell of `delivery_module`.
It bridges the Step 12 C callbacks into LogosAPI calls
on a configurable named module
(`payment_streams_module` in our demo;
any module with the same method names in the future).
The registration uses the auto-generated `getPluginMethods`
introspection surface every Logos module already exposes.

On our branch of `logos-delivery-module`,
extend the `delivery_module` interface with
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`
plus `storeQuery(jsonQuery, peerAddr, timeoutMs)`,
and add a `paidStoreMode` configuration toggle to `createNode`.
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
The full Store query exchange is the Step 14 demo
and requires the full stack documented there.

Definition of done:
without any verifier registered,
`delivery_module` behaves exactly as it did at `v0.1.1`.
Registering a module that does not expose the expected methods
returns a structured error and leaves the previous registration in place.
Store queries can be issued through `delivery_module`
against an explicit provider peer address.
With `payment_streams_module` registered as both verifier and provider,
an end-to-end Store query produced by the user
returns a successful Store outcome
and a successful eligibility outcome on the provider side.
Requests failing eligibility checks immediately return
a `BAD_REQUEST` (400) Store status code,
a populated `eligibility_status` object with the specific verdict,
and an empty messages list.

### Step 14, End-to-end demo wiring

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
and `delivery_module` (our branch),
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

### Step 15, Optional Basecamp UI

Architectural context:
the UI plugin added here is itself a Logos module
(`type: ui_qml` with a C++ backend),
not a piece of Basecamp.
Basecamp is the host that loads it,
in the same sense that `logoscore` is the host for Steps 6–14.
The plugin calls the unchanged backend modules from earlier steps
through the same `LogosAPI`;
no backend work is repeated here.

Scaffold a `ui_qml` plugin under `logos-basecamp` (or a sibling repo)
from the `logos-module-builder` `ui-qml-backend` template,
modeled on `logos-delivery-demo`.
It depends on `payment_streams_module` and `delivery_module`,
constructs `LogosModules` in `initLogos`,
and calls both modules through the generated typed wrappers.
The plugin surfaces vault state, stream state,
the current pending-proposal slot,
and the result of the most recent Store query.
No custom backend is required for the MVP.

Components required to run:
everything from Step 14
plus `logos-basecamp` as the host
(new prerequisite — first step that uses a GUI host
instead of `logoscore`).
The new `ui_qml` module is installed via `lgpm`
into Basecamp's plugins directory.

Definition of done:
`nix build` produces a `.lgx` that loads in Basecamp without QML errors,
and a user can complete the full demo flow through the UI
without using the CLI.
