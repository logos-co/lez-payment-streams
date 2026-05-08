# Payment Streams Integration Plan v2

## Task Summary

The goal is to let a Logos Delivery Store request carry a payment-stream-backed eligibility proof,
have a Store provider verify the proof against on-chain state,
and have the provider serve the request only when the proof is valid.

The integration is structured around the Logos Core module model.
Cryptographic and on-chain semantics live in Rust through a thin FFI.
A dedicated Logos Core module hosts payment-stream policy and proof handling.
Logos Delivery is extended only with a generic eligibility-verification hook
so that future eligibility schemes can plug into the same path
without changes to the delivery layer.

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

`lez-payment-streams`
is the existing on-chain SPEL program plus the `lez_payment_streams_core` Rust crate.
This work adds a sibling `lez-payment-streams-ffi` crate that exposes
PDA derivation,
account decoders,
stream folding,
policy validators,
off-chain proof construction and verification,
and Borsh instruction builders
through `extern "C"`.
The shape mirrors `lez-rln-ffi` in `logos-lez-rln`.

`payment_streams_module`
is a new Logos Core module (Qt plugin in C++) that wraps the FFI.
It owns
session keys for off-chain proofs,
pending-proposal state,
the user-facing flow for opening, topping up, claiming, and closing streams,
and the provider-facing flow for verifying eligibility proofs.
It declares a runtime dependency on `lez_wallet_module`.

`lez_wallet_module`
is the existing Logos Core module that wraps `wallet-ffi`.
It is the single point of contact with the LEZ chain.
Reads of vault, stream, and clock accounts go through `get_account_public`.
Writes go through `send_public_transaction`,
which is currently being added by
`logos-blockchain/logos-execution-zone#429` and
`logos-blockchain/logos-execution-zone-module#16`.

`logos-delivery` and `liblogosdelivery`
host the Store protocol implementation in Nim and its C FFI surface.
This work extends the Store v3 wire format with an opaque eligibility-proof field
on requests and an opaque eligibility-status field on responses,
and adds a callback hook that lets a host application register an
eligibility verifier and provider for paid Store mode.
Neither layer learns anything about payment streams.

`delivery_module`
is the existing Logos Core module that wraps `liblogosdelivery`.
This work adds two thin methods on its interface,
`setEligibilityVerifier(moduleName)` and `setEligibilityProvider(moduleName)`,
that bridge the new `liblogosdelivery` hooks to a named module via the SDK.
`payment_streams_module` is one such named module;
future modules (different incentivization schemes) can register the same way.

`logoscore`
is the headless runtime used to load and exercise modules during integration testing.
The end-to-end demo runs two `logoscore` instances on one host,
one as user and one as provider.

`logos-basecamp`
is optional for the MVP.
A small `ui_qml` plugin can surface vault and stream state visually
once the headless integration is stable.

## Open Questions

These questions block specific steps below.
Each item is self-contained so a colleague can pick it up
without reading the rest of the document.

### Q1, Store wire format ownership

The Store v3 messages `StoreQueryRequest` and `StoreQueryResponse`
do not currently have eligibility fields.
We need to add an optional `eligibility_proof` field on the request
and an optional `eligibility_status` field on the response.
The plan keeps Store status codes for query-execution outcomes
and uses a separate enumeration for eligibility outcomes,
so paid Store can return Store `200` together with an eligibility rejection.
Open points are
who owns the spec change in the relevant RFC,
which protobuf tag numbers are safe to take
(the request must avoid tag `10` already used for `pubsub_topic`),
and whether any version bump or codec migration is expected.
Until this clears, Step 11 cannot be merged into a release.

### Q2, Delivery module hook acceptance

`delivery_module` and `liblogosdelivery` are owned outside this work.
The new design adds a generic eligibility-verification callback to `liblogosdelivery`
and two routing methods on `delivery_module`.
The change is small in surface
but it touches the public C API of `liblogosdelivery`
and the `DeliveryModuleInterface` Qt interface.
We need agreement from the maintainers
that the generic-hook approach is acceptable,
that the proposed method names are fine,
and that the `liblogosdelivery` ABI bump is acceptable.
Until this clears, Steps 12 and 13 ship as a fork.

### Q3, Wallet write path PR landing

`payment_streams_module` requires `send_public_transaction` on `lez_wallet_module`.
That method is being added by
`logos-blockchain/logos-execution-zone#429` (Rust FFI side)
and `logos-blockchain/logos-execution-zone-module#16` (Qt module side),
both currently in draft.
If both PRs merge in time,
Step 8 lands cleanly and writes flow through the wallet module.
If they slip,
the fallback for the demo is to invoke `lgs wallet` subprocess calls
from `payment_streams_module` for writes only,
keeping reads through the wallet module unchanged.
We should commit to one of the two paths before Step 8 starts.

### Q4, Off-chain canonical-payload signing

Neither `wallet_ffi` nor `lez_wallet_module` currently exposes
a primitive that signs an arbitrary canonical payload with a wallet account's key.
LIP-155 commits a separate `public_key` in `StreamProposal`,
which lets `payment_streams_module` generate and own a session keypair per stream
for both `VaultProof.owner_signature` and `StreamProof.signature`.
That avoids the missing primitive at the cost of `payment_streams_module`
holding a session-private-key in `instancePersistencePath`.
The MVP accepts this posture.
A follow-up to `logos-execution-zone#422` should add a generic
"sign canonical bytes with this account's key" primitive
so the vault-owner key never has to leave the wallet for off-chain use.
This question is informational, not blocking.

### Q5, Wallet module canonical name

The local copy of the LEZ wallet module reports `name = "lez_wallet_module"` in metadata.
The RLN module declares a dependency on `liblogos_execution_zone_wallet_module`
and looks the wallet module up by that older name at runtime,
which means the RLN module's write path has not been exercised end-to-end
against the current code.
For `payment_streams_module` we adopt `lez_wallet_module` as the dependency name,
matching the live code.
We should confirm with the LEZ team that this is the canonical name going forward
and that the RLN module is the one out of date.

### Q6, Read freshness from the wallet module

`get_account_public` returns whatever account state the underlying wallet client returns,
without specifying live sequencer state versus indexer-finalized state.
The MVP treats whatever it returns as authoritative for hot-path eligibility verification,
which is acceptable on a local sequencer where finality lag is small.
For production we will eventually need to know the freshness model
and may need a live-read path.
This question is informational, not blocking.

### Q7, Provider-side verification handoff latency

Inbound Store requests are handled in Nim today.
Routing eligibility verification through `liblogosdelivery` to `delivery_module`
to `payment_streams_module` introduces two extra IPC hops per Store request,
plus the wallet-module chain reads inside the verifier.
The MVP accepts this latency for a demo.
We should confirm with the Store team that the `liblogosdelivery` hook
can be implemented as a synchronous (Future-returning) callback
without breaking the existing Store handler shape.
This question constrains the design of Step 12.

### Q8, Pending-proposal persistence policy

Logos Core gives every module an `instancePersistencePath`.
The MVP persists pending-proposal state and per-stream session keys there
as a flat JSON file, atomically written.
Open questions for the team are
whether session keys belong on disk at all in this posture
(they do for an MVP demo, but a hardened build would encrypt them
through a wallet-rooted KDF or keep them ephemeral),
and how aggressively to expire stale proposals on cold start.
This question is informational, not blocking.

## Integration Steps

Each step is independently testable.
The definition of done is a statement that can be objectively verified
without reading the implementation.

### Step 1, Bootstrap the Rust FFI crate

Create `lez-payment-streams-ffi` as a sibling crate to `lez_payment_streams_core`,
mirroring the `lez-rln-ffi` shape
(`crate-type = ["rlib", "cdylib", "staticlib"]`,
`cbindgen` build script,
generated `lez_payment_streams_ffi.h`).
The crate starts with a stub function and an error enum
so that the build pipeline is wired before functionality lands.

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
so that proposals with parameters outside policy are rejected uniformly
on both sides of the wire.

Definition of done:
the folded state and policy verdicts agree with `lez_payment_streams_core`
across a documented set of cross-language test vectors,
and the FFI returns a deterministic verdict for each input independent of host endianness.

### Step 4, Off-chain proof types and canonicalization in the FFI

Define byte layouts for `StreamProposal`, `VaultProof`, and `StreamProof`.
Expose canonicalization for the bytes signed by `VaultProof.owner_signature`
and the bytes signed by `StreamProof.signature` over a Store request payload.
Expose sign and verify primitives keyed by 32-byte public-key bytes
using the LIP-155 signature scheme.

Definition of done:
canonicalization is deterministic for a fixed input,
sign-then-verify round-trips,
and tampering with any field of the canonicalized payload
flips the verifier verdict.

### Step 5, Instruction builders in the FFI

Expose Borsh encoders for every payment-stream `Instruction` variant,
together with an account-list planner that returns the ordered list
of PDA and signer hex strings each instruction needs.
The encoders take typed arguments and return raw bytes;
the planner returns hex strings ready for `send_public_transaction`.

Definition of done:
encoded payloads round-trip through `lez_payment_streams_core` Borsh decoders,
and account-list planners agree with the harness builders in
`lez_payment_streams_core/src/test_helpers.rs`.

### Step 6, Bootstrap the Logos Core module

Create the `payment_streams_module` repository (or sibling directory) following the
`logos-core-module-builder` skeleton (`metadata.json`, `flake.nix` using `mkLogosModule`,
`CMakeLists.txt`, `src/payment_streams_module_plugin.{h,cpp}`,
`src/i_payment_streams_module.h`).
The metadata declares
`name = "payment_streams_module"`,
`dependencies = ["lez_wallet_module"]`,
and `include` covering the FFI shared library next to the plugin.
The plugin implements `PluginInterface` and exposes only `initLogos` and `name` for now.

Definition of done:
`nix build` produces an `.lgx` that `logoscore` can load alongside `lez_wallet_module`
without errors,
and `lm methods` reports the empty plugin surface as expected.

### Step 7, Wire chain reads from the module

Add helpers inside `payment_streams_module` that wrap
`lez_wallet_module.account_id_from_base58` and `lez_wallet_module.get_account_public`,
plus a higher-level helper that reads the configured clock account
and returns the current sequencer time.
These helpers are pure read paths and do not touch any payment-streams logic.

Definition of done:
against a scaffold-deployed `lez_payment_streams` program,
the module can read a known vault config, vault holding, stream config,
and clock account through `logoscore`,
and the JSON returned by `get_account_public` decodes through the FFI
into the expected typed values.

### Step 8, Wire chain writes from the module

Add a private helper that takes an `Instruction` kind, its typed arguments,
and a signer account ID, builds the borsh instruction bytes and account list
through the FFI, and submits via
`lez_wallet_module.send_public_transaction`.
Expose user-facing methods for the eight payment-stream operations
(initialize vault, deposit, withdraw, create stream, top up,
pause, resume, close, claim).
This step depends on Q3 resolving in favour of Option A.
If Q3 forces the fallback, the helper invokes `lgs wallet` as a subprocess instead.

Definition of done:
through `logoscore` against scaffold localnet,
the module can drive a complete vault and stream lifecycle from initialization
through claim,
with on-chain state observable through the chain-read helpers from Step 7.

### Step 9, Session keys and user-side proof construction

Add session-keypair management inside `payment_streams_module`,
backed by atomic JSON in `instancePersistencePath`.
Expose a single user-side method
`prepareEligibilityForStoreQuery(canonicalRequestBytes, providerId)`
that returns either a `StreamProposal` or a `StreamProof` byte string,
depending on whether the stream for the (vault, provider) pair has been established.
Eviction of stale proposals happens on a timer and on cold start.

Definition of done:
the module produces a syntactically valid eligibility proof byte string
for fixed inputs,
restarts cleanly with state intact,
and the FFI verifier accepts the produced bytes when matched against
the same canonical request and the corresponding chain state.

### Step 10, Provider-side proof verification

Expose a single provider-side method
`verifyEligibilityForStoreQuery(proofBytes, canonicalRequestBytes, providerId)`
that parses and dispatches the proof,
runs structural checks through the FFI,
queries chain state through the wallet module,
folds stream state at the current sequencer time,
and returns a structured verdict mapping to LIP-155 outcomes.
Pending-proposal tracking on the provider side is independent
of any user-side state and lives in `instancePersistencePath`.

Definition of done:
for fixed inputs the verifier returns a serve verdict on the happy path,
and a documented eligibility status code on each failure mode,
without performing chain reads when the failure is purely structural.

### Step 11, Extend the Store wire format

In `logos-delivery`,
add an optional opaque `eligibility_proof` field to `StoreQueryRequest`
and an optional `eligibility_status` object to `StoreQueryResponse`,
together with an enumeration of eligibility status codes
distinct from Store status codes.
Update the codec in `rpc_codec.nim` and the typed surfaces in
`waku/waku_store/common.nim`.
Q1 must clear before this lands.

Definition of done:
two Nim nodes agree on a round-trip of the new fields when present,
behaviour is unchanged when both fields are absent,
and existing Store codec coverage continues to pass.

### Step 12, Eligibility hooks in liblogosdelivery

In `liblogosdelivery`,
add a callback registration entry point that lets a host attach
a verifier callback called for inbound Store requests carrying an `eligibility_proof`,
and a path for attaching opaque eligibility-proof bytes to outgoing Store queries.
Both surfaces are bytes-in / bytes-out and carry no payment-streams knowledge.
Existing behaviour is preserved when no callback is registered.
Q2 and Q7 must clear before this lands.

Definition of done:
the new C ABI is documented and used by a Nim-side smoke test,
the inbound callback is invoked exactly once per Store request that carries a proof,
and the outbound path delivers attached bytes onto the wire unchanged.

### Step 13, Generic eligibility routing in delivery_module

In `delivery_module`,
extend `DeliveryModuleInterface` with `setEligibilityVerifier(moduleName)`
and `setEligibilityProvider(moduleName)`.
Implement the bridge that translates the new `liblogosdelivery` callbacks
into SDK calls on the named module
(`verifyEligibilityForStoreQuery`, `prepareEligibilityForStoreQuery`).
Add a configuration toggle for paid Store mode.
While Q2 is open, ship this as a fork.

Definition of done:
without any verifier registered,
`delivery_module` behaves exactly as it does today,
and with `payment_streams_module` registered as both verifier and provider,
an end-to-end Store query produced by the user
returns a successful Store outcome and a successful eligibility outcome
on the provider side.

### Step 14, End-to-end demo wiring

Create a single shell script that
starts a fresh scaffold workspace,
deploys `lez_payment_streams`,
launches two `logoscore` instances loaded with
`lez_wallet_module`,
`payment_streams_module`,
and `delivery_module`,
drives the user flow from vault initialization through Store query,
and drives a manual claim on the provider side.
The script captures structured logs at each phase.

Definition of done:
the script runs to completion against a clean workspace
and produces a log artifact that documents
every chain transaction, every Store request,
and the eligibility outcomes observed on both ends.

### Step 15, Optional Basecamp UI

Add a `ui_qml` plugin under `logos-basecamp` (or a sibling repo)
that calls `payment_streams_module` and `delivery_module` through `logos.callModule()`.
The plugin surfaces vault state, stream state,
the current pending-proposal slot,
and the result of the most recent Store query.
No custom backend is required for the MVP.

Definition of done:
`nix build` produces a `.lgx` that loads in Basecamp without QML errors,
and a user can complete the full demo flow through the UI
without using the CLI.
