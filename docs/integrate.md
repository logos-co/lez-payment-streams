# Eligibility integration

Add LIP-155 payment-stream eligibility to a request-response protocol.
The provider verifies an active stream before serving.

Store is the worked example in this repository.
Runnable Store instance: [reproduce/store.md](reproduce/store.md).
Protocol-only module path: [reproduce/module.md](reproduce/module.md).

Spec: [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html).
Wire pattern: [RFC 73](https://rfc.vac.dev/spec/73/) (proof on request, status on response).
Named contracts: [integration-contracts.md](reference/wire.md).
Decisions D1, D2, N8: [integration-decisions.md](reference/decisions.md).

Canonical request bytes generator: `cargo run -p lez-payment-streams-core --bin n8_canonical_wire_hex`.

## Approach

Five steps.
Reuse on-chain program, `payment_streams_module`, and `EligibilityProof` protobuf (`stream_proposal` / `stream_proof`).
The transport treats proof and status bytes as opaque.

1. Canonical request bytes.
   Deterministic serialization of the fields that bind the proof to this query, plus a domain prefix.
   Signed value: `SHA-256(prefix || canonical_request)`.
   Both sides must byte-match.
   Prove it with a pinned test vector.

2. Wire codec.
   Opaque `eligibility_proof` on the request and opaque `eligibility_status` on the response, at unused tags.
   Store uses RFC 73 tag `30`.
   Protocol status codes stay as they are.
   Eligibility failure: Store `BAD_REQUEST` (400), empty messages, verdict in tag `30`.
   Visible codes: `OK`, `PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`.

3. Module surface.
   Add `prepareEligibilityProofWithStreamProofFor<P>Query` and `verifyEligibilityFor<P>Query` next to the Store methods.
   Reuse internal prepare and verify helpers.
   Universal codegen: one LogosAPI name per method, single-line declarations in `*_impl.h`.
   Proposal and proof are separate methods
   (`prepareEligibilityProofWithStreamProposalForStoreQuery` vs `prepareEligibilityProofWithStreamProofForStoreQuery`).

4. Transport hooks.
   Register a verifier callback for inbound requests and a provider callback for outbound requests.
   The transport forwards opaque proof and status bytes.
   Store paid path: `storeQueryWithEligibility` / `storeQueryWithEligibilityCompleted`.

5. Policy and `service_id`.
   Register the protocol’s `service_id` and policy (minimum rate, minimum allocation, deadline delay).
   Store demo: `service_id` `/vac/waku/store-query/3.0.0`.
   Policy minima live in one native `accepted_tokens` row.
   `VaultProof.token_id` is 32 zero octets on this path.

## Check

- Canonical bytes round-trip on both sides for a fixed request.
- Verifier returns `OK` for a valid proof over an active stream.
- Verifier returns `PROOF_INVALID`, `STREAM_NOT_ACTIVE`, or `PARAMS_REJECTED` for a missing or invalid proof.
- Response carries `eligibility_status`.
  The protocol handler short-circuits on failure (empty payload, existing bad-request status).

## Sibling repos

This repository’s Store path uses `logos-delivery` and `logos-delivery-module` on the branch in [feature-branch-pins.md](reference/pins.md).
Another protocol changes its own codec and transport repository.
The hook contract stays opaque.

Fixture manifests contain test keys.
Use them on test networks.
Private keys stay in the wallet module.
