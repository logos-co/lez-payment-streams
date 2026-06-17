# Step 15 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

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

