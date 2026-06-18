# Step 14 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 14, Extend the Store wire format in `logos-delivery`

Architectural context:
this step modifies the Nim implementation that lives behind `delivery_module`'s FFI.
It is a wire-format change in `logos-delivery`,
not in any Logos module.
No Qt, no `LogosAPI`, no chain.

Pattern (RFC 73):
general incentivization attaches an eligibility proof to the request and an eligibility
status to the response.
This integration is a concrete instance of that pattern for the Store protocol,
with LIP-155 payment-stream bytes as the proof type (protobuf `EligibilityProof` with
`stream_proposal` / `stream_proof` arms).
RFC 73 also describes other proof flavors (for example proof-of-payment / TXID, optional
membership bytes); payment streams is another flavor, not named explicitly in RFC 73.

In `logos-delivery`, `waku/incentivization/` holds an older proof-of-payment POC on a stale
shape (`proofOfPayment` only).
It is not the Store tag `30` carrier.
Implement Store eligibility in `waku/waku_store/` only; do not reuse or merge
`waku/incentivization/rpc_codec.nim` for Step 14.
The shared names `EligibilityProof` / `EligibilityStatus` follow RFC 73 vocabulary on the
Store side; they are separate Nim modules from the POC types.

In `logos-delivery`,
add an optional opaque `eligibility_proof` field to `StoreQueryRequest` (tag `30`)
and an optional `eligibility_status` object to `StoreQueryResponse` (tag `30`),
together with an enumeration of eligibility status codes
distinct from Store status codes.
Update the codec in `waku/waku_store/rpc_codec.nim` and the typed surfaces in
`waku/waku_store/common.nim`.
Ship on our branch; no protocol-ID version bump.
Branch from upstream `logos-delivery` `master` ([integration branches](../../../integration-index.md#delivery-integration-branches)).

#### Wire types (normative)

Add to `waku/waku_store/common.nim`:

```nim
EligibilityStatusCode* {.pure, size: sizeof(uint32).} = enum
  OK              = 0
  PARAMS_REJECTED = 1
  PROOF_INVALID   = 2
  STREAM_NOT_ACTIVE = 3

EligibilityStatus* = object
  code*: EligibilityStatusCode  # field 1, uint32
  desc*: string                 # field 2, string; required per LIP-155
```

`desc` must always be present on the wire (LIP-155 requires both code and description).
For `OK`, `PROOF_INVALID`, and `STREAM_NOT_ACTIVE` a short fixed string suffices
(e.g. `"ok"`, `"proof invalid"`, `"stream not active"`).
For `PARAMS_REJECTED`, `desc` must indicate which parameter(s) failed
(e.g. `"rate below min_rate"`, `"allocation below min_allocation"`,
`"create_stream_deadline out of range"`, `"vault balance insufficient"`).
The provider fills this from the FFI's policy-check discriminant
before encoding the response.

Add to `StoreQueryRequest`:

```nim
eligibilityProof*: Option[seq[byte]]   # tag 30, opaque serialised EligibilityProof protobuf
```

Add to `StoreQueryResponse`:

```nim
eligibilityStatus*: Option[EligibilityStatus]  # tag 30, nested protobuf
```

In `rpc_codec.nim`:
encode `eligibility_proof` at request tag `30` as a length-delimited bytes field;
decode symmetrically.
Encode `eligibility_status` at response tag `30` as a nested protobuf message
with `code` at field `1` (uint32) and `desc` at field `2` (string);
decode symmetrically.
Tag `30` is confirmed unused on both request and response in the current codec.

Components required to run:
none beyond the Nim test infrastructure.

Definition of done:
extend `tests/waku_store/test_rpc_codec.nim` so encode → decode round-trips preserve the new
fields when present,
behaviour is unchanged when both fields are absent,
and existing Store codec coverage continues to pass (`make test` in `logos-delivery`).
Client/server Store tests are not required for Step 14.

