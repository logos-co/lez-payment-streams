# Integration contracts

Cross-step APIs and wire shapes. Normative detail: [reference/decisions-and-notes.md](reference/decisions-and-notes.md)
(D1, D2, N3a–N3c, N8, N11, N12). Step 16 bridge summary:
[plan/completed/step-16.md](plan/completed/step-16.md#resolved-implementation-decisions-2025-06-18).
Operator commands stay in `step*.md` runbooks.

## Store wire (Step 14 — D1)

RFC 73 pattern on Store: proof on request, status on response
([RFC 73](https://rfc.vac.dev/spec/73/)).
Proof bytes are LIP-155 payment-stream `EligibilityProof` (not the legacy
`waku/incentivization` proof-of-payment POC).

- Request tag `30`: opaque `eligibility_proof` (protobuf `EligibilityProof`)
- Response tag `30`: nested `eligibility_status` (payment-stream verdict: `code` + `desc`)
- On eligibility failure: Store `BAD_REQUEST` (400), empty messages; verdict only in tag `30`
- Store-visible eligibility codes: `OK`, `PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`
- No new Store `StatusCode` enum values for eligibility

## Delivery hooks (Steps 15–16 — D2)

- MVP hooks are synchronous blocking C function pointers at the liblogosdelivery boundary ([N3](reference/decisions-and-notes.md#n3-provider-side-verification-latency-and-blocking-hooks))
- Opaque bytes on the hook are the full serialized `EligibilityProof` (not inner arms alone)
- Outbound: provider libp2p `PeerId` to `prepareEligibilityForStoreQuery`
- Inbound: requester `PeerId` to `verifyEligibilityForStoreQuery` (logged only in MVP)
- Inbound `out_desc`: Step 16 copies verify JSON `message` on failure; empty ⇒ default phrase (D2)
- Registration introspection: exact method names via `getPluginMethods` (D2)
- Step 16 bridge policy: [N3a](reference/decisions-and-notes.md#n3a-step-16-threading--approach-a-experiment-2025-06-18),
  [N3b](reference/decisions-and-notes.md#n3b-step-16-hook-registration-lifecycle-2025-06-18),
  [N3c](reference/decisions-and-notes.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18),
  [N12](reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)

## logosdelivery_store_query JSON (Step 15)

`queryJson` uses the same camelCase keys as
`logos-delivery/library/kernel_api/protocols/store_api.nim` `fromJsonNode`:
required `requestId`, `includeData`, `paginationForward`; optional `contentTopics`,
`pubsubTopic`, `messageHashes`, `timeStart`, `timeEnd`, `paginationCursor`, `paginationLimit`.
Omit `eligibilityProof`; the registered provider callback attaches proof bytes before send.

Step 16 `delivery_module.storeQuery(queryJson, providerAddr)` uses this shape.
Dispatch is asynchronous when a provider is registered; response JSON arrives on a typed
completion event ([N3a](reference/decisions-and-notes.md#n3a-step-16-threading--approach-a-experiment-2025-06-18)).

## payment_streams_module — LogosAPI methods (Step 16 must match)

| Method | Role |
| --- | --- |
| `prepareEligibilityForStoreQuery` | User / outbound: returns `bytes_hex` (`EligibilityProof`) |
| `verifyEligibilityForStoreQuery` | Provider / inbound: returns `eligibility` verdict |
| `registerProviderMapping` | User routing: `PeerId` → payee base58 (host before outbound queries; Step 17 demo) |
| `listMyStreams`, `rediscoverStreams` | Inventory / refresh |
| `chainAction` | On-chain writes (Step 11b router) |

Codegen: Universal module exports single-line `Q_INVOKABLE` declarations in
`payment_streams_module_impl.h` or `lm methods` omits them ([N11](reference/decisions-and-notes.md#n11-universal-module-public-api)).

## Argument encoding (Steps 12–13)

| Argument | Encoding |
| --- | --- |
| `proofBytes` | Lowercase hex of serialized `EligibilityProof` |
| `canonicalRequestBytes` | Lowercase hex of full N8 canonical payload (138-byte reference wire for the pinned demo query) |
| `requesterPeerId` / provider peer in prepare | Opaque UTF-8 libp2p peer id |

N8 tool:

```bash
cargo run -p lez-payment-streams-core --bin n8_canonical_wire_hex
```

## JSON — user prepare (Step 12)

Success shapes use `"status":"ok"` inside `result` with `"kind":"stream_proposal"` or
`"stream_proof"` and `"bytes_hex"`. User-side errors use `"code"` (`STREAM_DEPLETED`,
`NO_ELIGIBLE_VAULT`, etc.).

## JSON — provider verify (Step 13)

- OK: `{"status":"ok","eligibility":"OK"}`
- Verdict: `{"status":"error","eligibility":"<PARAMS_REJECTED|PROOF_INVALID|STREAM_NOT_ACTIVE>","message":"…"}`
- Caller fault: `{"status":"error","message":"…"}` without `eligibility`
- Missing proof on inbound Store: Step 16 passes empty `proofBytes`; paid demo expects a
  verdict failure, not OK ([N3c](reference/decisions-and-notes.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18))
- Step 17 demo: provider runs paid-only (verifier always on); users learn provider PeerId
  off-band ([step17-e2e-local.md](step17-e2e-local.md))

## Fixture and provider payee

- Default manifest: `fixtures/localnet.json` (`FIXTURE_MANIFEST`)
- Verify binds payee to manifest `provider_account_id` (same as Step 12 chain fixture)
- Demo `service_id`: UTF-8 `/vac/waku/store-query/3.0.0`
- Demo policy: `min_rate` 1, `min_allocation` 1, `max_create_stream_deadline_delay` 3600

## Persistence (N4)

File: `payment_streams_state.json` under logoscore `instancePersistencePath`.

- User: `negotiations`, session keys, `peer_mappings`, `inventory`
- Provider: `provider_acceptances` (`schema_version` 2)

## Wallet module

- Runtime id: `logos_execution_zone` ([D4](reference/decisions-and-notes.md#d4-wallet-module-runtime-name))
- Chain reads: `get_account_public`; writes: `send_generic_public_transaction_json` (N10)
- Signing: `sign_public_payload` (N1) for vault owner proofs

## Canonical payload (N8 summary)

Full specification: [N8](reference/decisions-and-notes.md#n8-canonical-store-request-bytes-format).
`canonical_payload = PREFIX || borsh(CanonicalStoreRequest)`; `StreamProof.signature` signs
`SHA-256(canonical_payload)`. Nim (Step 15) and Rust (Step 4) must byte-match; pinned tests in
core and the Step 15 Nim parity test on the delivery fork.
