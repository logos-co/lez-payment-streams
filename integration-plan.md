# Payment Streams Integration Plan

## Introduction

This document describes the integration of LEZ payment streams into the Store protocol in `logos-delivery`.

The goal is to enable Store requests backed by payment stream eligibility proofs.
A user establishes a payment stream on-chain via LEZ,
then uses stream-backed eligibility to query Store providers off-chain.
The provider verifies stream state on-chain before serving requests.

## Repositories

`lez-payment-streams`

The on-chain payment-stream program implemented in Rust using the SPEL framework.
It provides vault and stream semantics with lazy accrual.
The program is complete and tested.

`logos-scaffold`

CLI tool for managing LEZ infrastructure.
Provides localnet start, wallet management, and program deployment.
The scaffold handles sequencer lifecycle for local development.

`logos-delivery`

The implementation target.
Contains the Store protocol and the Waku node implementation.
The integration work happens here.

## Architecture Overview

The integration involves three local entities:

The Sequencer: A local LEZ instance managed by `logos-scaffold`.
Runs at `localhost:3040` and hosts the deployed `lez-payment-streams` program.

The Provider: A Waku node running in `logos-delivery` as Store server.
Queries the sequencer to validate eligibility proofs.
Claims accrued funds manually when desired.
Identified by a long-lived provider ID (advertised out-of-band).

The User: A Waku node running in `logos-delivery` as Store client.
Creates vaults and streams through the scaffold-managed command path.
Sends Store requests to the Provider with eligibility proofs.

## Communication Overview

The integration uses three distinct communication channels:

User to Provider: Store protocol over libp2p for service requests and responses.
The user prepares payment-stream proof bytes locally before sending Store requests.

User or Provider to Sequencer for writes:
scaffold-managed command path, expected to be the `lgs` CLI.
The command path wraps underlying `wallet` and `spel` binaries internally.
These binaries are not invoked directly by the demo flow.
Write operations include initializing vaults; depositing and withdrawing funds; creating, pausing, and closing streams; and claiming accrued funds.

User or Provider to Sequencer for reads: Direct JSON-RPC 2.0 over HTTP to `localhost:3040`.
Read operations include querying vault balance, stream state, and clock time.
This avoids subprocess overhead for hot-path validation.

## MVP Assumptions

The following constraints apply to this integration:

All parties (User, Provider, Sequencer) run on the same local network with well-synchronized clocks.
Provider ID is advertised out-of-band; no discovery protocol is used.
Policy constants are hard-coded; no per-session negotiation.
Pending proposals are stored in-memory only and lost on provider restart.
Provider claims accrued funds manually; no automatic claiming.
Paid Store mode is enabled explicitly on the provider.
The MVP does not support mixed paid and unpaid Store interoperability.
The MVP keeps `/vac/waku/store-query/3.0.0`;
no versioned Store request type or new protocol ID is introduced.
Payment-stream eligibility outcomes use existing Store `statusCode` and `statusDesc` fields.
No separate `eligibility_status` response field is added.
The demo covers public and private LEZ modes.
Privacy implications are documented from the payment-streams specification
and do not block the MVP implementation.

## LEZ Access Strategy

The intended write path is the scaffold-managed CLI from `logos-scaffold`.
It provides version pinning, a consistent interface,
and automatic binary location.
Step 1 confirms the exact commands before implementation depends on them.

Write operations through the confirmed command path:
- localnet start and stop
- program deployment
- user vault and stream operations
- provider claim operations

Read operations use direct RPC to `localhost:3040`.
The expected account-read method is `getAccount`,
but Step 1 confirms the exact method name, request shape, and response encoding.
The expected validation call chain is:
1. Derive the PDA (deterministic from seeds documented in `lez-payment-streams` test helpers).
2. Call `{"method": "getAccount", "params": [base58(account_id)], ...}` to `localhost:3040` via JSON-RPC.
3. Borsh-decode the returned `data` bytes into the target account type.

The MVP expects live sequencer state for validation.
Step 1 confirms whether direct sequencer reads are live or finalized.
An indexer service provides finalized state for production use.

## Scaffold State Management

Scaffold maintains state in `.scaffold/state/` (wallet, localnet PID, port) and `.scaffold/logs/`.
This state persists across runs and can cause issues if not handled properly.

Before starting a demo or test run:

Check current state with the localnet status command confirmed in Step 1.
The status reports `ownership: foreign` if another process uses port 3040.
The status reports `stale` if a previous run left stale PID state.

Handle foreign ownership by stopping the conflicting process or using a different port.
Handle stale state by stopping and restarting localnet.

For complete isolation in CI or repeated testing:
Use a fresh project directory for each run.
Or remove `.scaffold/state/` while preserving logs if needed.

The wallet state (`.scaffold/state/wallet.state`) persists between runs and contains the default funded account.
Preserve this file across demo runs to maintain the same wallet.

## Testing Approach

Tests follow three layers mirroring the `lez-payment-streams` pattern.

Layer 1: Helper Unit Tests

Test payment-stream helper logic with mocked LEZ responses.
Located in `tests/incentivization/payment_streams/`.
Test policy validation, proposal validation, proof verification, and pending tracking.
Run fast without external dependencies.

Layer 2: Store Protocol Integration Tests

Test Store request and response flow with mocked LEZ.
Located in `tests/waku_store/`.
Test codec round-trips and Store flow between two local nodes.
Use explicit peer addresses, no discovery needed.

Layer 3: End-to-End Integration Tests

Full stack with real LEZ sequencer.
Located in `tests/integration/`.
Require scaffold-managed sequencer running on `localhost:3040`.
Require deployed `lez-payment-streams` program and funded accounts.
Run manually or in CI with LEZ environment.

All definitions of done in this plan are testable statements.
A step is complete when its tests pass.

## Step 1 Discovery

Answer questions whose source of truth is external behavior
or another repository.

Discover the LEZ account-read contract:
Start the scaffold-managed sequencer.
Deploy `lez-payment-streams`.
Derive one known vault config PDA, vault holding PDA, and stream config PDA.
Call the sequencer account-read RPC directly.
Confirm the RPC method name, request shape, response shape, data encoding,
and whether the response represents live sequencer state or finalized indexer state.
Decode one `VaultConfig`, read the `VaultHolding` account balance,
decode one `StreamConfig`, and decode the selected clock account.

Discover cryptographic contracts:
Identify how LEZ derives `AccountId` from public keys.
Identify the transparent account signature scheme.
Identify available verification APIs in Nim, Rust, or existing dependencies.

Discover demo tooling contracts:
Record the exact localnet, deploy, account funding, and program interaction commands.
Confirm the repo pins a SPEL revision that includes the IDL fix from
`logos-co/spel` issue `#176` and PR `#180`.
Treat generated IDL as a reliable source for account and instruction schema
for this integration.

Definition of done: A short findings document records each discovered contract
with source references.
Primary record location:
`docs/step1-findings-scaffold-rpc.md`.
A spike command or script proves direct account-read RPC works against a
deployed local program.
Key derivation, signature verification options, command path, and dependency
alignment status are known.
Clean-rerun status is deferred.
Deferred gate: complete one clean rerun before opening the first
`logos-delivery` integration PR (and no later than Step 3 execution).
Rerun dependencies:
- fresh scaffold workspace state for the rerun
- scaffold-managed LEZ and SPEL repos reporting clean working trees in
  `lgs doctor`
Record the clean rerun output in the findings document when this gate is met.
IDL generation is verified with
`cargo run -p lez_payment_streams-examples --bin generate_idl`
and correctness checks on the output artifact:
top-level `accounts` is non-empty and includes
`VaultConfig`, `VaultHolding`, and `StreamConfig`;
instruction entries include expected account metadata
(`init`, `mut`, signer flags, and PDA seeds);
and one decode smoke test confirms generated account schema
matches Borsh layout used by on-chain account types.

## Step 2 Discovery Informed Decisions

Make MVP choices that depend on Step 1 findings.

Decide proof formats:
Choose the `VaultProof.owner_signature` scheme.
Define the exact public key format.
Define the exact signature encoding.
Define how proof public keys map to `VaultConfig.owner`.
Define the canonical byte layout for the signed `VaultProof` payload.
Decide whether `StreamProof.signature` uses the same scheme.
Define the canonical Store request payload signed by `StreamProof`.

Decide validation support strategy:
Choose whether stream folding is implemented directly in Nim
or delegated to Rust through FFI or a small sidecar.
For MVP, prefer Nim if the port of `StreamConfig.at_time` stays small
and can be covered by Rust test vectors.
Maintenance tradeoff to document explicitly:
Nim implementation keeps one language toolchain and simpler debugging,
but requires careful parity maintenance when LEZ stream logic evolves.
Rust sidecar or FFI keeps logic close to upstream Rust semantics and may reduce
drift risk, but adds cross-language build, release, and observability overhead.

Decide local demo configuration:
Choose how paid Store mode is enabled on the provider.
Choose how the user-side request-signing private key is supplied during the demo.

Definition of done: The plan records the chosen proof scheme, payload formats, stream-folding strategy, provider configuration surface, and user key handling. Later implementation steps no longer contain open design questions except locally chosen constants such as numeric status codes.

## Step 3 Inventory Integration Targets

Map the relevant code in `logos-delivery` before making changes.

Read Store protocol definitions in `waku/waku_store/`.
Identify `StoreQueryRequest` and `StoreQueryResponse` types in `common.nim`.
Locate codec implementation in `rpc_codec.nim`.
Find client implementation in `client.nim`.
Find server implementation in `protocol.nim`.

The `waku/incentivization/` directory houses the payment-stream helper.
Reuse the existing manager pattern where useful (lifecycle, async Result methods).
Extend `EligibilityProof` with `stream_proposal` and `stream_proof` fields per the spec.
Map eligibility outcomes onto existing Store response status fields.
Payment-streams uses LEZ state queries instead of Web3 and txhash validation.

Verify `lez-payment-streams` builds and deploys with `logos-scaffold`.
Run through the scaffold first-success path in a fresh directory.
Use the command path discovered in Step 1 for account funding and stream operations.
Use generated IDL from `examples/src/bin/generate_idl.rs` as the canonical
schema artifact for integration tooling.
Maintain generated IDL as the single schema source for integration tooling.

Definition of done: A document lists all files to be modified with one-line rationale per file. A script deploys `lez-payment-streams` to a fresh scaffold localnet, captures the program ID, and outputs the program ID for provider configuration.

## Step 4 Define Policy Constants

Create a focused module for MVP payment policy values.

Default values (following spec recommendations where available):
- Stream rate: `1` token/second
- Minimum allocation: `300` tokens (5 minutes of service at 1 token/second)
- Max stream window: `300` seconds (RECOMMENDED default per spec)
- VaultProof response cap: `65536` bytes (RECOMMENDED default per spec)
- Vault buffer: `5` percent (RECOMMENDED default per spec)
- Clock: `CLOCK_10` (10-second granularity)
- Clock account ID: Hardcoded constant from `clock_core` crate (documented in config)
- Service ID: `/vac/waku/store-query/3.0.0` (Store protocol identifier from RFC 13)

The clock account ID constant is defined in `waku/incentivization/payment_streams/config.nim` with a comment referencing the source in the `clock_core` crate.

These values live in a configuration module (`waku/incentivization/payment_streams/config.nim`).
They may be adjusted during implementation based on testing.

Place these in a new module under `waku/incentivization/payment_streams/`.
Name the module `config.nim`.
Document each constant with its semantics and rationale.

Definition of done: Values are chosen and documented. A test reads each policy constant and asserts it is greater than zero. A validation function rejects proposals with parameters outside policy bounds.

## Step 5 Extend Store Wire Types

Add eligibility fields to Store request types
and implement the MVP status-field assumption for payment-stream rejections.
If a request is not eligible,
the provider returns a non-2xx Store response
with a payment-stream-specific `statusCode` and `statusDesc`.

Extend `StoreQueryRequest` in `waku/waku_store/common.nim` with:
`EligibilityProof` message containing three mutually exclusive optional bytes fields:
- `proof_of_payment` for txhash-based proofs
- `stream_proposal` for first-time stream setup
- `stream_proof` for subsequent requests with established stream

Define new error codes for payment-stream-specific failures:
`PARAMS_REJECTED` for unacceptable stream parameters.
`PROOF_INVALID` for invalid vault or stream proof.
`STREAM_NOT_ACTIVE` for inactive or missing stream.
`PENDING_EXPIRED` for proposals past `open_stream_by`.
`PERMANENTLY_REJECTED` for unusable vault or stream state.

Assign concrete numeric values for these codes in the Store status-code space.
Use `statusDesc` for actionable rejection descriptions.

Update protobuf codec in `waku/waku_store/rpc_codec.nim` to encode and decode the new fields.
Use optional fields to preserve backward compatibility.

Paid Store mode is enabled by configuration on the provider.
When paid Store mode is enabled, requests without `stream_proposal` or `stream_proof`
are rejected with the configured payment-required status.

Add targeted codec tests for the new fields.
Ensure existing Store behavior still encodes and decodes correctly.

Definition of done: A codec test round-trips `EligibilityProof` message with its `stream_proposal` and `stream_proof` fields without data loss. Status-code tests cover each payment-stream-specific rejection. All existing Store codec tests pass unchanged.

## Step 6 Create Provider Payment-Stream Helper

Introduce a dedicated module for provider-side payment-stream logic.

Create `waku/incentivization/payment_streams/manager.nim`.
Define a `PaymentStreamManager` type.
The manager accepts the `lez-payment-streams` program ID via configuration (environment variable or config file) for PDA derivation.

Responsibilities of the helper:
Validate `stream_proposal` structure and provider binding.
Validate proposed parameters against policy constants.
Validate `stream_proof` signatures.
Query LEZ state for vault and stream verification.
Track pending proposals until stream establishment.
Expose enough stream state for the operator to make manual claim decisions.
Actual claim writes remain outside this helper.

Keep the API narrow. Store code calls into the helper for validation and state decisions. The helper owns stream-specific logic and LEZ-facing checks.

Definition of done: A unit test calls each public helper function with valid inputs and receives expected results. The Store protocol module imports only from the helper module for payment-stream logic, not from LEZ client directly.

## Step 7 Create User Payment-Stream Helper

Introduce a dedicated module for user-side payment-stream proof construction.

Create `waku/incentivization/payment_streams/client.nim`.
Define a narrow API for building proof bytes that can be attached to Store requests.

Responsibilities of the user helper:
Build `StreamProposal` from vault data, provider ID, policy parameters, and a request-signing public key.
Build `VaultProof` and sign its canonical payload
using the format chosen in Step 2.
Build `StreamProof` for subsequent Store requests.
Canonicalize Store request fields before signing.
Store or accept the request-signing private key used for the stream session.
Serialize proof messages into the byte fields carried by `EligibilityProof`.

Keep on-chain writes outside this helper for the MVP.
The user still creates vaults, deposits funds, and creates streams
through the command path confirmed in Step 1.
The helper only prepares off-chain proof material for Store requests.

Definition of done: A unit test constructs a valid `stream_proposal` and `stream_proof` from fixed inputs. A test vector fixes the canonical request-signing payload and resulting signature bytes. Store client tests can attach generated proof bytes without invoking the scaffold command path.

## Step 8 Implement Proposal Validation

Support the first Store request carrying `stream_proposal`.

Signature Scheme for `VaultProof.owner_signature`:
Use the proof scheme chosen in Step 2.
The `owner_signature` covers `vault_id`, `provider_id`, and `balance_commitment` fields.
Here `vault_id` means the user-chosen `u64` vault identifier stored in `VaultConfig`.
Use `vault_config_pda` or vault config PDA when referring to the derived vault config account address.
Use `vault_holding_pda` when referring to the derived vault holding account address.

Balance Verification Strategy:
Always re-derive unallocated balance from on-chain query: `vault_holding.balance - vault_config.total_allocated`.
`vault_holding.balance` is the native account balance returned by the sequencer.
It is not a field in the Borsh-encoded `VaultHolding` data.
Reject the proposal if `on_chain_unallocated < stream_params.stream_allocation * (1 + buffer_percentage/100)`.
The `balance_commitment` in the proof serves only as a hint; the on-chain state is the source of truth.

Time Source for `open_stream_by` Deadline:
Use local wall-clock time via `chronos.now()` for validating the `open_stream_by` freshness constraint.
No tolerance is applied (see MVP Assumptions).
When verifying stream establishment in Step 9, query the `StreamConfig` on-chain and compare its `accrued_as_of` field against the `open_stream_by` deadline.
This anchors the timeliness verification to the sequencer time at stream creation.

Response Cap Enforcement:
Limit Store response size to the configured `VaultProofResponseCap` (default 65536 bytes) when serving the first request backed by a `VaultProof`.
The provider tracks pending proposals with their associated caps.
If the response would exceed the cap, return `PARAMS_REJECTED`.
The cap is configurable in `waku/incentivization/payment_streams/config.nim`.

Implement parsing and validation in the payment-stream helper:
Parse `EligibilityProof.stream_proposal` bytes.
Validate `VaultProof` structure including provider binding.
Validate `VaultProof.owner_signature` using the Step 2 proof scheme.
Validate `StreamParams` against policy constants.

On the provider side in Store protocol:
Record pending proposal state keyed by vault-provider pair.
Reject duplicate proposals for the same vault-provider pair.
Set expiration timer based on `open_stream_by` deadline.
Serve the first Store request if validation succeeds.
Return appropriate rejection status on failure.

Clean up pending state on:
Successful rejection with explicit status.
Expiry of `open_stream_by` window.
Detection of established stream.

Definition of done: An integration test sends `StoreQueryRequest` with valid `stream_proposal` and receives response with `statusCode` 200. A test with invalid proposal receives `PARAMS_REJECTED` or `PROOF_INVALID` status. A test verifies pending state expires after `open_stream_by` window and subsequent requests receive `PENDING_EXPIRED` status.

## Step 9 Implement Stream Proof Validation

Support subsequent Store requests carrying `stream_proof`.

Signature Payload for `StreamProof.signature`:
Use the canonical Store request payload chosen in Step 2.
Verification uses the signature scheme chosen in Step 2.

Stream Matching Mechanism:
To correlate a `stream_proof` to a previously accepted proposal, extract `stream_id` from the `StreamProof`.
Look up pending proposal by vault-provider pair.
Derive the stream config PDA with seeds `["stream_config", vault_config_pda, stream_id]`.
Query `getAccount(base58(stream_config_pda))` and verify:
`stream_config.provider` matches provider ID,
`stream_config.rate` and `stream_config.allocation` match proposal,
the effective stream state is `ACTIVE` after folding to the current sequencer clock,
and `stream_config.accrued_as_of` is before `open_stream_by` deadline.

Stream State Folding:
Stored `StreamConfig` uses lazy accrual.
Each validation must evaluate the effective stream state at current sequencer time.
Use the stream-folding strategy chosen in Step 2.
If folding is implemented in Nim,
cover it with test vectors copied from `lez-payment-streams`.

Implement parsing and validation in the payment-stream helper:
Parse `EligibilityProof.stream_proof` bytes.
Extract and canonicalize the signable request payload.
Verify signature over request data using the `public_key` committed in the original `StreamProposal`.
Extract stream identifier from proof.
Derive the stream config PDA and query on-chain state.
Query the configured clock account and fold stream state before applying activity checks.

On the provider side in Store protocol:
Look up pending proposal by vault-provider key.
Verify the folded stream is `ACTIVE` with parameters matching proposal.
Verify stream was established before `open_stream_by` deadline.

Serve the Store request only if all validations pass.
Return `STREAM_NOT_ACTIVE` or `PROOF_INVALID` status on failure.

Definition of done: An integration test sends `StoreQueryRequest` with valid `stream_proof` and receives served response. A test with tampered signature receives `PROOF_INVALID` status. A test querying paused stream receives `STREAM_NOT_ACTIVE` status. A test with unknown stream ID receives error status.

## Step 10 Implement Termination Semantics

Handle service-stop conditions through Store responses.

Define eligibility status codes for termination-equivalent states:
`STREAM_NOT_ACTIVE` for paused or closed streams.
`PENDING_EXPIRED` for proposals past `open_stream_by`.
`PERMANENTLY_REJECTED` for unusable vault or stream state.

Make provider behavior consistent with these statuses.
Return appropriate status on the next request when service should stop.

Clean up session state when:
Stream transitions to closed.
Provider decides to stop serving.
User closes the stream.

Definition of done: An integration test simulates stream closure and subsequent Store request receives rejection status. A test with expired pending proposal returns `PENDING_EXPIRED` status. The test verifies no unsolicited messages are sent from provider to user.

## Step 11 Integrate LEZ Access

Wire the payment-stream helper to the local LEZ environment.

Implement LEZ state queries using `getAccount` via JSON-RPC 2.0 to `localhost:3040`.
Confirm the exact method name and response encoding during Step 1.
The account `data` field in each response is Borsh-encoded.
The account `balance` field is used for vault holding liquidity.
Implement Borsh decoding directly in Nim for:
`VaultConfig`, `VaultHolding`, `StreamConfig`, and `ClockAccountData`.
The field layout is defined in `lez_payment_streams_core/src/vault.rs`
and `lez_payment_streams_core/src/stream_config.rs`.
Direct Nim Borsh avoids FFI complexity for a read-only concern.

Queries needed:
- `getAccount(base58(vault_config_pda))` for vault metadata and allocated balance.
- `getAccount(base58(vault_holding_pda))` for native vault balance.
- `getAccount(base58(stream_config_pda))` for stream state, rate, allocation, and accrual fields.
- `getAccount(base58(clock_account_id))` for current sequencer time
  (use `CLOCK_10` for 10-second granularity; clock account IDs come from `clock_core`).

PDA derivation for each account type follows the seed scheme in
`lez_payment_streams_core/src/test_helpers.rs`.

Use the command path confirmed in Step 1 for:
Starting the scaffold-managed sequencer.
Deploying `lez-payment-streams`.
Funding wallets.
Creating vaults and streams.
Claiming accrued funds.

Definition of done: An integration test with real LEZ queries vault config, vault holding balance, stream state, and clock time via RPC. A test with mock LEZ client produces identical validation results as real LEZ query for same state. A script documented in README starts the sequencer, deploys the program, and runs a helper query through the confirmed command path.

## Step 12 Wire Paid Store Flow

Wire payment-stream validation into Store without changing archive semantics.

In `waku/node/kernel_api/store.nim`,
wrap the Store archive query handler with payment validation.
If paid Store mode is enabled,
validate `stream_proposal` or `stream_proof` before calling `toArchiveQuery`
and `node.wakuArchive.findMessages`.
If validation fails,
return a Store response with the payment-stream-specific status code and description.
If validation succeeds,
serve the request as an ordinary Store success and return `200 OK`.

Keep the Store archive path focused on message retrieval.
The payment-stream helper owns validation,
pending proposal tracking,
and LEZ-facing state checks.

On the client side,
allow Store callers to attach `EligibilityProof` bytes to a `StoreQueryRequest`.
For the MVP demo, proof construction can be driven by a test helper,
CLI wrapper, or explicit demo script that calls the user payment-stream helper.

Add a code comment in `waku/waku_store/protocol.nim`
near the successful response overwrite.
The comment should state that paid Store treats eligible requests as ordinary success
and that non-standard success statuses would require revisiting this overwrite.

Definition of done: With paid Store mode enabled, requests without proof are rejected before archive lookup. A valid proposal-backed request reaches archive lookup and returns `200 OK`. A valid stream-proof-backed request reaches archive lookup and returns `200 OK`. Invalid proofs return payment-stream-specific Store status codes.

## Step 13 Add Integration Tests

Cover the MVP flow with tests.

Add tests for codec changes:
Round-trip encoding of eligibility fields.
Backward compatibility with old messages.

Add tests for payment-stream helper:
Valid and invalid proposal validation.
Valid and invalid stream proof validation.
Pending proposal tracking and expiry.
Effective stream-state folding.

Add integration tests for Store flow:
First request with valid proposal.
First request with invalid proposal.
Subsequent requests with valid proof.
Subsequent requests with invalid proof.
Rejection of non-stream-backed requests.
Termination semantics.

Use mocking for LEZ state in unit tests.
Provide documented manual flow for true chain interaction.

Definition of done: Layer 1 tests pass with mocked LEZ and cover all helper validation paths. Layer 2 tests pass with two local nodes and mocked LEZ, covering all Store flow scenarios. CI runs Layer 1 and 2 tests automatically. Layer 3 test instructions are documented and executable manually against running LEZ.

## Step 14 Finalize Documentation

Create documentation that makes the MVP understandable and reproducible.

Document the integration architecture in `logos-delivery`.
Include component diagram showing User, Provider, Sequencer.
Include communication flow diagram.

Document local LEZ setup:
Prerequisites (`logos-scaffold` with its vendored `wallet` and `spel` binaries).
Step-by-step to running sequencer.
Deploying the `lez-payment-streams` program.
Creating accounts and funding.

Document MVP assumptions and limitations (see MVP Assumptions section).
Document public and private demo modes.
Summarize privacy implications by referencing the payment-streams specification.

Record future work:
Extracting payment-stream library for other protocols.
Multi-token vault support.
Auto-pause and other protocol extensions.

Definition of done: A new developer follows only the documented steps to run the full demo successfully. A code review confirms all public functions have doc comments. A test coverage report shows all payment-stream code paths are exercised.

## Step 15 Optional Basecamp UI Integration

Create a visual interface for the payment-streams demo in `logos-basecamp`.
This step builds on the completed protocol integration and provides a graphical view of vault and stream state.

Create a Qt/QML plugin in a new `ui/` directory:
Define QML views for vault creation and funding.
Define QML views for stream creation and monitoring.
Display stream state including accrued amount and activity status.
Show historical Store queries backed by each stream.

Implement optional Rust FFI layer only if required by Qt-runtime boundaries:
Create `ffi/` directory with Rust cdylib for Qt integration when needed.
Expose functions to query LEZ state for display.
Expose functions to initiate on-chain operations.

Build as portable `.lgx` module:
Create `flake.nix` with packages for default, ffi, and lgx outputs.
`nix build ./ui#lgx` produces `payment-streams-plugin.lgx`.

Test integration:
Load the `.lgx` into basecamp via plugin manager.
Verify vault/stream state displays correctly.
Verify Store queries work through the UI.

Document usage:
Provide environment variables needed (`NSSA_WALLET_HOME_DIR`, `NSSA_SEQUENCER_URL`, `PROGRAM_ID_HEX`).
Provide step-by-step for workshop scenarios.

Definition of done: `nix build ./ui#lgx` produces a `.lgx` file that loads into basecamp without errors. A UI test clicks through vault creation view and asserts state display updates. A user can complete the full demo scenario using only the UI, without CLI commands.
