# Developer Journey — payment-stream eligibility for a request-response protocol

## What the user achieves

A developer takes a request-response protocol of their choice and adds LIP-155
payment-stream eligibility to it, so that a provider verifies an active payment stream
before serving a request.
Store (a Logos Delivery protocol) is the worked example used throughout.
The runnable Store verification lives in [E2E.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/journeys/E2E.md).

## Why it matters

Logos networks should be self-sustaining:
users pay providers for services rather than relying on external subsidies.
Payment streams are a universal eligibility mechanism —
any request-response protocol can bind a proof to a request and a verdict to a response.
Store is one application; the same pattern applies to other request-response protocols.

## Key components

Reusable unchanged across protocols:

- `lez-payment-streams` on-chain program — LIP-155 vaults, streams, deposits, claims on LEZ.
- `payment_streams_module` — Universal Logos Core module exposing LIP-155 via
  `chainAction` and eligibility proof methods.
- `logos_execution_zone` (`wallet_module`) — chain interaction and signing for the
  payment streams module.
- `EligibilityProof` protobuf with `stream_proposal` / `stream_proof` arms — opaque to
  the transport.

Protocol-specific (the work this guide walks through):

- Canonical request bytes and their signing digest.
- Wire codec fields for `eligibility_proof` and `eligibility_status`.
- Per-protocol prepare and verify method names on `payment_streams_module`.
- Verifier and provider hook registration at the transport boundary.
- `service_id` and policy for the protocol.

## Repository

https://github.com/logos-co/lez-payment-streams

## Runtime target

This guide is a protocol-agnostic integration recipe.
The runnable reference is the Store eligibility end-to-end run:
two `logoscore` processes (user and provider) coordinated by `scripts/e2e.sh`,
on localnet and on public TestNet v0.2.
It lives in
[E2E.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/journeys/E2E.md).

## Prerequisites

For the Store reference run, host setup (Nix, scaffold, Store delivery checkout,
testnet bootstrap) is documented in the repository README prerequisites
(https://github.com/logos-co/lez-payment-streams#prerequisites).
For the reader's own protocol, the prerequisite is control over the protocol's codec and
transport boundary, plus a deployed LIP-155 program.

## Commands and expected outputs

The runnable Store commands live in
[E2E.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/journeys/E2E.md).
The developer's action sequence for a new protocol is the five steps below.
Each step lists its expected output.

1. Canonical request bytes — define a deterministic serialization (for example Borsh) of
   the request fields that bind the proof to this query, plus a domain prefix.
   Both sides must byte-match; prove it with a pinned test vector.
   The signed value is `SHA-256(prefix || canonical_request)`.
   Expected output: identical canonical bytes on requester and provider sides for a fixed
   request.

2. Wire codec — add an opaque `eligibility_proof` field on the request and an opaque
   `eligibility_status` field on the response, at unused tags.
   Expected output: a request that carries proof bytes and a response that carries a
   verdict, with protocol status codes unchanged.

3. Eligibility module surface — add `prepareEligibilityProofWithStreamProofFor<P>Query`
   and `verifyEligibilityFor<P>Query` methods to `payment_streams_module`, reusing the
   internal verify and prepare helpers.
   Expected output: the module produces proof bytes on prepare and a verdict on verify.

4. Verifier and provider hooks — register a verifier callback for inbound requests and a
   provider callback for outbound requests at the transport boundary;
   the transport forwards opaque proof and status bytes.
   Expected output: inbound requests are gated by the verifier;
   outbound requests carry an attached proof.

5. Policy and service identity — register the protocol's `service_id` and policy
   (minimum rate, minimum allocation, deadline delay).
   Expected output: the verifier accepts an active stream that meets policy and rejects
   otherwise.

The eligibility pattern itself is protocol-agnostic and follows RFC 73:
proof on request, status on response, opaque bytes, verifier callback.
The pattern fits request-response protocols (one request, one response).

## Expected result

Confirm each of the following manually:

- Canonical bytes round-trip identically on the requester and provider sides for a fixed
  request.
- The verifier returns `OK` for a request carrying a valid proof over an active stream.
- The verifier returns a failure verdict (`PROOF_INVALID`, `STREAM_NOT_ACTIVE`, or
  `PARAMS_REJECTED`) for a missing or invalid proof.
- The response carries `eligibility_status`, and the protocol handler short-circuits on
  failure (empty payload, existing bad-request status).

## Configuration details

### Naming format

We currently use the `*For<P>Query` naming format
(for example `verifyEligibilityForStoreQuery`,
`prepareEligibilityProofWithStreamProofForStoreQuery`).
Future versions may generalize this if needed.
Add new per-protocol methods next to the existing Store methods and reuse the internal
verify and prepare helpers.

### Codegen constraints

Universal module codegen requires one LogosAPI name per method
(use a distinct name for each method) and single-line declarations in
`*_impl.h`.
Proposal and proof are separate methods.

### Policy and service identity

Policy and `service_id` are documented in the payment-streams spec and the module docs.
The Store demo uses `service_id` `/vac/waku/store-query/3.0.0` with policy `min_rate` 1,
`min_allocation` 1, `max_create_stream_deadline_delay` 3600.

## Failure modes and limits

| Failure | Cause | Resolution |
|---------|-------|------------|
| `NO_ELIGIBLE_VAULT` | Vault missing or insufficient deposit | Run vault ensure / deposit; check vault scan |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | Create a new stream or top up |
| `PROOF_INVALID` | Eligibility proof verification failed | Confirm stream is active; check canonical bytes |
| `STREAM_NOT_ACTIVE` | Stream closed or not yet active | Create a new stream on the vault |
| `PARAMS_REJECTED` | Stream or request rejected by policy | Check `service_id`, rate, allocation, deadline |
| Verifier not invoked | Hook not registered at transport boundary | Register verifier and provider callbacks before serving |

## GitHub handle

@s-tikhomirov

## Discord handle

sergei.tikhomirov

## Existing docs or specs

- LIP-155 (Payment Streams): https://lip.logos.co/anoncomms/raw/payment-streams.html
- RFC 73 (Store Eligibility): https://rfc.vac.dev/spec/73/
- Integration contracts: https://github.com/logos-co/lez-payment-streams/blob/main/docs/reference/integration-contracts.md
- Integration decisions (D1, D2, N8): https://github.com/logos-co/lez-payment-streams/blob/main/docs/reference/integration-decisions.md
- Store integration: https://github.com/logos-co/lez-payment-streams/blob/main/docs/store-integration/README.md

## Additional context

### Sibling repositories

In the `lez-payment-streams` repository, Store integration uses patched forks
`logos-delivery` and `logos-delivery-module` on the branch recorded in
`docs/reference/feature-branch-pins.md`.
A different protocol requires the reader to modify its own codec and transport repository;
the hook contract stays opaque.

### Estimated time to complete

Integration time is dominated by the protocol-specific codec and transport work, which
varies by target protocol.
The eligibility-specific steps (canonical bytes, module methods, hook registration) are a
small fraction of the effort once the codec is in hand.

### Security notes

- Fixture manifests contain test keys; use on test networks only.
- Private keys stay in `wallet_module`; proofs are signed attestations.
- The reference Store journey uses transparent vault mode.
