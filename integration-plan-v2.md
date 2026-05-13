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

## Component Overview

For background on hosts vs. modules,
the two distinct boundaries (FFI inside a module versus `LogosAPI` between modules),
the role of Qt and C++,
and the scope of the LEZ-only localnet,
see `logos-architecture-overview.md` in this directory.

`lez-payment-streams`
is the existing on-chain SPEL program plus the `lez_payment_streams_core` Rust crate.
This work adds a sibling `lez-payment-streams-ffi` crate
that exposes PDA derivation,
account decoders,
stream folding,
policy validators,
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
Writes go through `send_public_transaction`,
which is already exercised by `logos-rln-module`
(JSON request shape `program_id` / `accounts` / `instruction` / `signer_account`)
but is not yet present in the current `lez_wallet_module` interface.
This work adds it on our branch, matching the shape `logos-rln-module` already calls.

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

`logos-execution-zone`
is the LEZ source repo
(sequencer, indexer, NSSA account model, on-chain programs, `wallet_ffi`).
This work does not modify it;
it is a transitive dependency through
`lez_payment_streams_core`'s Cargo dependencies on `nssa_core` and `clock_core`
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
Store status codes (`status_code`, `status_desc`) remain reserved for query-execution outcomes.
Eligibility outcomes use a separate enumeration carried inside `eligibility_status`.
If eligibility fails,
the Store handler MUST return a `StoreQueryResponse`
with a 400-level `status_code` (e.g., 403 Forbidden)
and an empty `messages` list,
skipping the database query.
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
That method is not yet exposed by the current `lez_wallet_module`,
so we add it on our branch of `logos-execution-zone-module`
as part of Step 8, modeled on the consumer side already shipped in `logos-rln-module`.
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
placed as a sibling of `lez_payment_streams_core/` and `lez-payment-streams-ffi/`
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
For the MVP, we add a narrow public-account payload-signing method
to `lez_wallet_module` on our branch.

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
The LEZ demo binds that generic identity to two concrete values:
the provider's libp2p `PeerId` for Store routing
and the provider's on-chain LEZ account ID for stream creation and claims.
For the MVP, we assume off-band negotiation where the host application learns
this mapping and configures `payment_streams_module`
with it (e.g., via a `registerProviderMapping` method).
This keeps the delivery layer strictly agnostic to payment-stream identity
while avoiding hardcoded IDs.

### N6, Delivery module Store query exposure

The lower-level `logos-delivery` C ABI already exposes Store queries,
but the current `logos-delivery-module` Qt surface does not.
We will ask upstream whether `storeQuery(jsonQuery, peerAddr, timeoutMs)`
fits the intended module API and whether there are reasons Store querying
has not been exposed yet.
If that is not addressed in time for the demo,
we implement the method on our branch and may suggest it upstream later.

### N7, Session key concurrency

Session key signing is synchronous.
Concurrent Store queries to the same provider serialize at the session key.
The MVP assumes low concurrency; production would need key pooling or async queuing.

## Integration Steps

Each step is independently testable.
The definition of done is a statement that can be objectively verified
without reading the implementation.

### Step 1, Bootstrap the Rust FFI crate

Architectural context:
this and the next four steps build out Boundary A (C FFI) of `payment_streams_module`.
No Qt plugin shell, no Logos host, no chain are involved yet —
this is pure Rust crate work that will later be linked into the module.

Create `lez-payment-streams-ffi` as a sibling crate to `lez_payment_streams_core`,
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
each decoder round-trips against fixtures generated by `lez_payment_streams_core`,
and PDA derivation produces account IDs that match the values
already recorded in `docs/step1-findings-scaffold-rpc.md`
for a known program deployment.

### Step 3, Stream folding and policy in the FFI

Expose stream-state folding driven by `StreamConfig::at_time`,
returning effective state, accrued amount, and remaining allocation
for a given current sequencer time.
Expose policy validators for stream rate, allocation, max stream window,
response cap, and vault buffer percentage,
so proposals with parameters outside policy are rejected uniformly
on both sides of the wire.

Components required to run: none.

Definition of done:
the folded state and policy verdicts agree with `lez_payment_streams_core`
across a documented set of cross-language test vectors,
and the FFI returns a deterministic verdict for each input
independent of host endianness.

### Step 4, Off-chain proof types and canonicalization in the FFI

Define byte layouts for `StreamProposal`, `VaultProof`, and `StreamProof`.
Expose canonicalization for the bytes signed by `VaultProof.owner_signature`
and the bytes signed by `StreamProof.signature` over a Store request payload.
Expose sign and verify primitives keyed by 32-byte NSSA public-key bytes
using NSSA Schnorr signatures.
The FFI owns all domain-separated canonicalization and prehashing.
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
encoded payloads round-trip through `lez_payment_streams_core` Borsh decoders,
and account-list planners agree with the harness builders in
`lez_payment_streams_core/src/test_helpers.rs`.

### Step 6, Bootstrap the Logos Core module

Architectural context:
this step lays down the C++ Qt-plugin shell of `payment_streams_module`.
The shell is a Qt plugin (`type: core`)
that will host the Boundary A FFI (from Steps 1–5)
and expose Boundary B (`LogosAPI`) methods to other modules in later steps.

Scaffold the module from the `logos-module-builder`
`with-external-lib` template, modeled on `logos-rln-module`
and the Rust-FFI pattern documented in `logos-tutorial/logos-developer-guide.md`.
Per D5, the directory is `logos-payment-streams-module/`,
placed alongside `lez_payment_streams_core/` and `lez-payment-streams-ffi/`
in this repo.
It ships
`metadata.json` (with `name = "payment_streams_module"`,
`type = "core"`,
`dependencies = ["lez_wallet_module"]`,
and `include` covering the FFI shared library next to the plugin),
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
this step exercises Boundary B (`LogosAPI`) for the first time.
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
Sub-step 8b uses Boundary B from `payment_streams_module` to call it.
Instruction bytes and account lists are built through
`payment_streams_module`'s Boundary A (the Rust FFI from Steps 1–5).

Two sub-steps that ship together:

Sub-step 8a:
on our branch of `logos-execution-zone-module`,
add `send_public_transaction(QString jsonRequest) -> QString`
on `lez_wallet_module`,
matching the JSON shape already produced by `logos-rln-module`
(`program_id`, `accounts`, `instruction` hex, `signer_account`).
The method delegates to the underlying `wallet_ffi` signing-and-submit path.
Add a narrow `sign_public_payload(accountId, domain, prehashHex) -> QString`
method on the same branch,
returning a 64-byte NSSA Schnorr signature for a 32-byte prehash
with the public account's signing key.
The `domain` parameter is an explicit audit label;
the payment-stream FFI remains responsible for domain-separated prehashing.

Sub-step 8b:
add a private helper inside `payment_streams_module`
that takes an `Instruction` kind, its typed arguments,
and a signer account ID,
builds the Borsh instruction bytes and account list through the FFI,
serializes the JSON request,
and submits via `lez_wallet_module.send_public_transaction`.
Expose user-facing methods for the nine payment-stream operations
(initialize vault, deposit, withdraw, create stream, top up,
pause, resume, close, claim).

Components required to run:
sub-step 8a needs no runtime
(verified via `nix build` and `lm methods` showing the new surface).
Sub-step 8b needs the same set as Step 7
(`logoscore` daemon with both modules, LEZ sequencer, deployed program).

Definition of done:
through `logoscore` against scaffold localnet,
the module can drive a complete vault and stream lifecycle from initialization
through claim,
with on-chain state observable through the chain-read helpers from Step 7.

### Step 9, Session keys and user-side proof construction

Architectural context:
this is the user-side method that `delivery_module` will auto-invoke
once registered as the outbound eligibility provider in Step 13.
It does not, by itself, initiate any Store traffic;
it just produces opaque bytes when asked.

Add session-keypair management inside `payment_streams_module`,
backed by atomic JSON in `instancePersistencePath` (see N4).
Expose a single user-side `Q_INVOKABLE` method
`prepareEligibilityForStoreQuery(canonicalRequestBytes, providerPeerId)`
that returns either a `StreamProposal` or a `StreamProof` byte string,
depending on whether the stream for the `(vault, provider)` pair
has been established.
The module MUST actively poll `get_account_public`
to confirm the `StreamConfig` PDA exists on-chain
before switching from sending `StreamProposal` to `StreamProof`.
The module also exposes a `Q_INVOKABLE`
`registerProviderMapping(providerPeerId, providerId, providerAccountId)` method
to let the host configure the identity mapping (see N5).
Calling `prepareEligibilityForStoreQuery` for an unmapped `providerPeerId` returns an error.
For `StreamProposal` output,
the module asks `lez_wallet_module.sign_public_payload`
to produce `VaultProof.owner_signature` with the vault owner's LEZ key,
and signs later `StreamProof`s with its own persisted session key.
Eviction of stale proposals happens on a timer and on cold start.

The user is responsible for creating the stream on-chain after a `StreamProposal`
is accepted.
The user-side `payment_streams_module` allocates and persists the `stream_id`
for the vault-provider pair,
then uses that `stream_id` when constructing later `StreamProof`s.

Components required to run:
`logoscore` daemon hosting both modules.
The definition of done's verifier round-trip is in-process through the FFI;
a live sequencer is not strictly required for that verification itself,
but the same Step 7 stack remains useful for sanity-checking
that vault data the proof asserts matches chain state.

Definition of done:
the module produces a syntactically valid eligibility proof byte string
for fixed inputs;
restarts cleanly with state intact;
the FFI structural verifier accepts the proof format;
and (when chain state is available) the provider-side verifier accepts
the proof against actual on-chain stream state.

### Step 10, Provider-side proof verification

Architectural context:
this is the provider-side method that `delivery_module` will auto-invoke
once registered as the inbound eligibility verifier in Step 13.
Structural checks happen entirely through Boundary A (FFI);
chain checks happen through Boundary B to `lez_wallet_module`.

Expose a single provider-side `Q_INVOKABLE` method
`verifyEligibilityForStoreQuery(proofBytes, canonicalRequestBytes, requesterPeerId)`
that parses and dispatches the proof,
runs structural checks through the FFI,
queries chain state through the wallet module,
folds stream state at the current sequencer time,
and returns a structured verdict mapping to LIP-155 outcomes
(`OK`, `PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`).
This is the verifier callback; the user side uses `prepareEligibilityForStoreQuery` to generate proofs.
Pending-proposal tracking on the provider side is independent
of any user-side state and lives in `instancePersistencePath`.
The provider stores pending proposal state keyed by the vault,
the generic `providerId`,
the provider LEZ account ID,
and the committed session public key,
then matches later `StreamProof.stream_id` values against both that pending state
and the on-chain `StreamConfig`.
The inbound `requesterPeerId` is available for logs,
short-lived anti-abuse policy,
and proposal retry limits,
but Store eligibility is based on proof validity and chain state,
not on transport peer continuity.

Components required to run:
`logoscore` daemon hosting both modules.
The structural-failure portion of the definition of done needs nothing more.
The happy-path verdict portion needs the Step 7 stack
(LEZ sequencer plus deployed program plus seeded vault/stream state).

Definition of done:
for fixed inputs the verifier returns a serve verdict on the happy path
and a documented eligibility status code on each failure mode,
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
this step modifies Boundary A on the delivery side —
the C ABI between `liblogosdelivery` (Nim) and `delivery_module` (C++ Qt plugin).
No Qt, no `LogosAPI`, no Logos host yet;
the C ABI is consumed by a C smoke test.

In `liblogosdelivery`,
add a single C ABI registration entry point that lets a host attach
a verifier callback called for inbound Store requests carrying an `eligibility_proof`,
and a path for attaching opaque eligibility-proof bytes to outgoing Store queries.
Both surfaces are bytes-in / bytes-out and carry no payment-streams knowledge.
`canonicalRequestBytes` are produced by `liblogosdelivery`
from the Store query before eligibility bytes are attached.
They are a deterministic signable serialization of the Store request fields
that define the query,
with `eligibility_proof` omitted and response-only fields absent.
On the provider side,
`liblogosdelivery` recomputes the same bytes from the decoded inbound request
after extracting and clearing `eligibility_proof`.
These bytes are the payload signed by `StreamProof.signature`
and verified by `payment_streams_module`.
Existing behaviour is preserved when no callback is registered.
The verifier callback is synchronous (`Future`-returning) per N3.
Bump the `liblogosdelivery` ABI on our branch.

Components required to run:
none beyond a Nim test rig and a small C consumer
linking against the new `liblogosdelivery`.

Definition of done:
the new C ABI is documented and used by a Nim-side smoke test,
the inbound callback is invoked exactly once per Store request that carries a proof,
and the outbound path delivers attached bytes onto the wire unchanged.

### Step 13, Generic eligibility routing in `logos-delivery-module`

Architectural context:
this step modifies the C++ Qt-plugin shell of `delivery_module`.
It bridges the Step 12 C callbacks into Boundary B (`LogosAPI`) calls
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
a 403 Forbidden Store status code with an empty messages list.

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
