# Integration contracts

Cross-step APIs and wire shapes. Normative detail: [reference/decisions-and-notes.md](reference/decisions-and-notes.md)
(D1, D2, N8, N11). Operator commands stay in `step*.md` runbooks.

## Store wire (Step 14 — D1)

- Request tag `30`: opaque `eligibility_proof` (protobuf `EligibilityProof`)
- Response tag `30`: opaque `eligibility_status` (eligibility verdict object)
- On eligibility failure: Store `BAD_REQUEST` (400), empty messages; verdict only in tag `30`
- Store-visible eligibility codes: `OK`, `PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`
- No new Store `StatusCode` enum values for eligibility

## Delivery hooks (Steps 15–16 — D2)

- Opaque bytes on the hook are the full serialized `EligibilityProof` (not inner arms alone)
- Outbound: `delivery_module` passes provider libp2p `PeerId` to the eligibility provider module
- Inbound: passes requester `PeerId` to the verifier module (logged / abuse only in MVP)
- Registration must expose exact LogosAPI method names (introspection via `getPluginMethods`)

## payment_streams_module — LogosAPI methods (Step 16 must match)

| Method | Role |
| --- | --- |
| `prepareEligibilityForStoreQuery` | User / outbound: returns `bytes_hex` (`EligibilityProof`) |
| `verifyEligibilityForStoreQuery` | Provider / inbound: returns `eligibility` verdict |
| `registerProviderMapping` | User routing: `PeerId` → payee base58 (not provider self-id for verify) |
| `listMyStreams`, `rediscoverStreams` | Inventory / refresh |
| `chainAction` | On-chain writes (Step 11b router) |

Codegen: Universal module exports single-line `Q_INVOKABLE` declarations in
`payment_streams_module_impl.h` or `lm methods` omits them ([N11](reference/decisions-and-notes.md#n11-universal-module-public-api)).

## Argument encoding (Steps 12–13)

| Argument | Encoding |
| --- | --- |
| `proofBytes` | Lowercase hex of serialized `EligibilityProof` |
| `canonicalRequestBytes` | Lowercase hex of full N8 canonical payload (177-byte reference wire) |
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

`canonical_payload = PREFIX || borsh(CanonicalStoreRequest)`; `StreamProof.signature` signs
`SHA-256(canonical_payload)`. Nim (Step 15) and Rust (Step 4) must byte-match; pinned tests in
core and Step 15 DoD.
