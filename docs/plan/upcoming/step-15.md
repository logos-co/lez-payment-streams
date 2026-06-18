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
add two C ABI registration entry points
that let a host attach an eligibility verifier callback
(called for every inbound Store request; `proof_hex` is NULL when no proof field is present)
and an eligibility provider callback
(called by Nim before sending an outgoing Store query).
Both surfaces are bytes-in / bytes-out and carry no payment-streams knowledge.
Existing behaviour is preserved when no callback is registered.
Per N3, both callbacks are synchronous blocking C function pointers;
the Nim async handler awaits their result while the calling thread is held.
This is the MVP design; see Migration note below before productionising.
Extend `liblogosdelivery.h` with the new exports on our fork (additive ABI bump per [D2](../../reference/decisions-and-notes.md#d2-delivery-module-hook-design)).

##### C ABI types

Add to `liblogosdelivery.h`:

```c
/*
 * Inbound (provider-side) eligibility verifier.
 *
 * Called by liblogosdelivery for every inbound Store request.
 * Runs on the liblogosdelivery async handler thread;
 * implementation must not re-enter the library.
 *
 * proof_hex       – lowercase hex of the serialised EligibilityProof (D2),
 *                   or NULL when the request carries no proof field.
 *                   The callback controls whether to accept or reject
 *                   unauthenticated requests (those with proof_hex == NULL).
 * canonical_hex   – lowercase hex of the N8 canonical_payload produced
 *                   by liblogosdelivery after clearing eligibility_proof
 * requester_peer_id – UTF-8 libp2p PeerId of the requesting peer
 * out_desc          – caller-supplied buffer; callback writes a UTF-8 eligibility
 *                     description for the Store response (may be empty).
 *                     When empty after return, liblogosdelivery uses the default
 *                     phrase for the returned code (see D2).
 * out_desc_len      – size of out_desc in bytes; must be >= 512
 * user_data       – opaque pointer supplied at registration
 *
 * Returns an EligibilityStatusCode value (0–3).
 * Return -1 to signal an internal error; liblogosdelivery will respond
 * with BAD_REQUEST and eligibility code PROOF_INVALID.
 */
typedef int (*EligibilityVerifierCb)(
    const char *proof_hex,
    const char *canonical_hex,
    const char *requester_peer_id,
    char       *out_desc,
    size_t      out_desc_len,
    void       *user_data);

/*
 * Outbound (user-side) eligibility provider.
 *
 * Called by liblogosdelivery before sending an outgoing Store query
 * when a provider callback is registered.  Nim has already computed
 * canonical_hex from the request (N8 Borsh encoding) and resolved
 * the provider's libp2p PeerId from the connection.
 *
 * canonical_hex    – lowercase hex of the N8 canonical_payload
 * provider_peer_id – UTF-8 libp2p PeerId of the target Store provider
 * out_proof_hex    – caller-supplied buffer; callback writes lowercase
 *                   hex of the serialised EligibilityProof here
 * out_buf_len      – size of out_proof_hex in bytes; must be >= 4096
 * user_data        – opaque pointer supplied at registration
 *
 * Returns 0 on success (out_proof_hex is populated),
 * negative on error (query is aborted before sending).
 */
typedef int (*EligibilityProviderCb)(
    const char *canonical_hex,
    const char *provider_peer_id,
    char       *out_proof_hex,
    size_t      out_buf_len,
    void       *user_data);

/* Register or replace the inbound eligibility verifier.
 * Pass NULL to clear a previous registration. */
int logosdelivery_set_eligibility_verifier(
    void                *ctx,
    EligibilityVerifierCb cb,
    void                *user_data);

/* Register or replace the outbound eligibility provider.
 * Pass NULL to clear a previous registration. */
int logosdelivery_set_eligibility_provider(
    void                 *ctx,
    EligibilityProviderCb cb,
    void                 *user_data);

/*
 * Issue a Store query to the given provider.  Added on our fork of logos-delivery.
 *
 * queryJson    – JSON object with Store query fields using the same camelCase keys as
 *               `library/kernel_api/protocols/store_api.nim` `fromJsonNode`
 *               (required: `requestId`, `includeData`, `paginationForward`;
 *               optional: `contentTopics`, `pubsubTopic`, `messageHashes`,
 *               `timeStart`, `timeEnd`, `paginationCursor`, `paginationLimit`).
 *               Do not send `eligibilityProof`; the provider callback attaches it.
 * providerAddr – multiaddr string of the target Store provider peer
 *
 * The full StoreQueryResponse (messages list and, when present, eligibility_status)
 * is returned as a JSON string via the callback.
 * If a provider callback is registered, it is called automatically before the
 * request is sent to attach the eligibility_proof field.
 */
int logosdelivery_store_query(
    void        *ctx,
    FFICallBack  callback,
    void        *userData,
    const char  *queryJson,
    const char  *providerAddr);
```

`out_buf_len` of 4096 bytes is sufficient for any `EligibilityProof`
produced by the current `payment_streams_module`
(a `StreamProof` with signature is under 300 bytes → under 600 hex chars).

##### Migration note — async callbacks

The blocking callback design is the N3 MVP choice (acceptable latency,
low concurrency per N7).
Migrating to non-blocking callbacks requires a versioned ABI bump:
both typedefs gain `result_cb` and `result_user_data` trailing parameters,
the Nim decorator switches to a future-bridge (~15 lines),
and the C++ implementation calls `result_cb` asynchronously instead of returning.
Perform this migration when the system is productionised for concurrent traffic,
not during the demo.

#### Eligibility check injection pattern (inbound)

The verifier callback is injected as a decorator (wrapper)
around the existing `StoreQueryRequestHandler`.
`protocol.nim` is not modified.
At registration time,
`liblogosdelivery` replaces the active `requestHandler`
with a wrapper that:

1. Extracts `eligibility_proof` from the decoded request, if present.
2. Produces `canonicalRequestBytes` from the request
   (see Canonical Store request bytes below).
3. Calls the verifier callback with the hex-encoded `eligibility_proof` bytes
   (NULL when no proof is present), the canonical bytes hex, the requester `PeerId`,
   and the `out_desc` buffer.
4. On failure (`EligibilityStatusCode` != `OK` or callback returns -1),
   returns early with `BAD_REQUEST` status code (400),
   the `eligibility_status` object populated with the verdict code and `desc`
   (from `out_desc` when non-empty, otherwise the default phrase for that code per D2),
   and an empty `messages` list.
   The inner `requestHandler` is never called.
5. On success, delegates to the inner `requestHandler`.

This pattern keeps all eligibility logic
outside `protocol.nim` and `client.nim`.

#### Eligibility proof injection pattern (outbound)

The provider callback is symmetric with the verifier callback
on the outbound side.
When a provider callback is registered,
`liblogosdelivery`'s `logosdelivery_store_query` function
(added on our fork of `logos-delivery`; implement by adapting
`library/kernel_api/protocols/store_api.nim` `waku_store_query` — same JSON parsing,
`wakuStoreClient.query`, and hex JSON response — plus eligibility provider injection
before send and registration on the stable `liblogosdelivery.h` surface):

1. Builds the `StoreQueryRequest` from the caller-supplied parameters,
   without `eligibility_proof`.
2. Produces `canonicalRequestBytes` from that request (same N8 Borsh encoding used on the inbound side).
3. Resolves the target provider's libp2p `PeerId` from the connection.
4. Calls the provider callback with canonical bytes hex and provider `PeerId`.
5. On success, sets `eligibility_proof` on the request from the returned hex bytes,
   then sends the request.
6. On callback failure, aborts the query and returns an error to the caller.

This keeps the N8 Borsh serialization entirely within Nim (`liblogosdelivery`);
the C++ caller never computes or interprets canonical bytes.
The provider callback receives canonical bytes hex in the same format
as `prepareEligibilityForStoreQuery` expects for its `canonicalRequestBytes` argument (Step 12).

Both the inbound and outbound paths share the same N8 canonical bytes computation.
On the inbound side the canonical bytes are produced
after extracting and clearing `eligibility_proof` from the decoded request.
On the outbound side they are produced
from the clean request before `eligibility_proof` is attached.

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

#### Verification (logos-delivery repo)

- N8 cross-language vector: `tests/waku_store/test_store_eligibility_canonical.nim`
  (hex must match `n8_canonical_wire_hex` in `lez-payment-streams-core`).
- C ABI smoke: `library/tests/test_eligibility_hooks.c`, built like
  `library/examples/logosdelivery_example.c` (`make logosdelivery_example` precedent).

#### Definition of done

`logosdelivery_set_eligibility_verifier` and `logosdelivery_set_eligibility_provider`
are present in `liblogosdelivery.h` and exercised by a C smoke test.
When a verifier is registered:
the inbound verifier callback is invoked exactly once per inbound Store request,
with `proof_hex` NULL when no proof is present;
on a non-OK verdict the response carries `BAD_REQUEST` (400),
a populated `eligibility_status` (code + desc), and an empty messages list;
on an OK verdict the inner handler is called.
When no verifier is registered:
no eligibility check is performed and behaviour is identical to the pre-eligibility baseline.
When a provider callback is registered:
it is called before each outgoing query and its returned bytes appear unmodified
in the request's `eligibility_proof` field.
A cross-language test vector confirms
that the Nim canonical-bytes serializer
produces output identical to the Rust serializer
for a fixed `StoreQueryRequest` with known field values
(see [N8](../../reference/decisions-and-notes.md#n8-canonical-store-request-bytes-format)
for the test vector specification).

