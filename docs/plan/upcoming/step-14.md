# Step 14 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

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

