# Step 13 — normative plan excerpt

Operator commands: [step13-provider-eligibility.md](../step13-provider-eligibility.md).
Agents on Step 14+ should read [integration-contracts.md](../integration-contracts.md) instead.

### Step 13, Provider-side proof verification

Architectural context:
this is the provider-side method that `delivery_module` will auto-invoke
once registered as the inbound eligibility verifier in Step 16.
Structural checks happen entirely through the Rust FFI;
chain checks happen via LogosAPI calls to `logos_execution_zone`.

Expose a single provider-side `Q_INVOKABLE` method
`verifyEligibilityForStoreQuery(proofBytes, canonicalRequestBytes, requesterPeerId)`
that parses and dispatches the proof,
runs structural checks through the FFI,
queries chain state through the wallet module,
folds stream state at the current sequencer time,
and returns a structured verdict mapping to LIP-155 outcomes.
`proofBytes` are the same opaque serialized `EligibilityProof` from the Store request (D2);
the module unwraps `stream_proposal` / `stream_proof` before FFI checks.

#### Provider-side verdicts

The verifier returns one of the following eligibility status codes.
These are carried inside the `eligibility_status` object (D1)
and never surface as Store status codes.

- `OK`:
  proof is valid, chain state confirms eligibility, request is served.
- `PARAMS_REJECTED`:
  stream parameters do not match `StreamProviderPolicy`
  (rate below `min_rate`, allocation below `min_allocation`,
  `create_stream_deadline` outside `max_create_stream_deadline_delay`),
  or vault unallocated balance is below the proposed `allocation`
  (`StreamParams`, same semantics as on-chain `StreamConfig::allocation` after `create_stream`),
  or the proposal's `create_stream_deadline` has already passed.
  The `VaultProof` is not marked as spent;
  the user may retry with adjusted parameters.
- `PROOF_INVALID`:
  proof format is malformed,
  `VaultProof.owner_signature` or `StreamProof.signature` verification failed,
  or the owner public key does not derive to `VaultConfig.owner`.
- `STREAM_NOT_ACTIVE`:
  the referenced stream exists on-chain
  but its folded state is not `ACTIVE`
  (paused, closed, or depleted).

#### Pending-proposal tracking

Pending-proposal tracking on the provider side is independent
of any user-side state and lives in `instancePersistencePath`.
The provider stores pending proposal state
keyed by `(vault_id, provider_id)`,
matching the LIP-155 constraint
that a user must not have more than one pending proposal
per vault-provider pair.
The stored record includes accepted or pending `StreamParams`,
the committed session public key, and `create_stream_deadline`.
After acceptance, add `stream_id` from the first valid `StreamProof`.
Evict when LIP-155 treats negotiation as failed (no acceptance or no
compliant stream by `create_stream_deadline`).

The inbound `requesterPeerId` is available for logs,
short-lived anti-abuse policy,
and proposal retry limits,
but Store eligibility is based on proof validity and chain state,
not on transport peer continuity.

#### Implementor approach (MVP demo)

Closed choices for Step 13 implementation. Align with Step 12
([`docs/step12-user-eligibility.md`](docs/step12-user-eligibility.md)) and keep protobuf
parsing, policy math, and signature checks in Rust FFI; the Qt module orchestrates wallet reads,
persistence, and JSON only. Store tag `30` / `eligibility_status` protobuf is Step 14; Step 16
forwards opaque bytes and peer ids without interpreting proofs (D2).

LogosAPI method (Step 16 must use this name exactly):

`verifyEligibilityForStoreQuery(proofBytes, canonicalRequestBytes, requesterPeerId)`

Argument encoding (same conventions as Step 12 `prepareEligibilityForStoreQuery`):

| Argument | Encoding |
| --- | --- |
| `proofBytes` | Lowercase hex of serialized protobuf `EligibilityProof` (Step 12 `bytes_hex`) |
| `canonicalRequestBytes` | Lowercase hex of full N8 canonical payload (Step 12 `canonical_request_hex`) |
| `requesterPeerId` | Opaque UTF-8 libp2p peer id; log only for MVP, not used in eligibility predicates |

Module JSON response (LogosAPI envelope, not the Step 14 Store object):

- Success: `{"status":"ok","eligibility":"OK"}`.
- Eligibility failure: `{"status":"error","eligibility":"<PARAMS_REJECTED|PROOF_INVALID|STREAM_NOT_ACTIVE>","message":"…"}`.
- Caller mistakes (malformed hex, wallet unavailable): `{"status":"error","message":"…"}` without
  `eligibility` (same split as Step 12 plain vs eligibility-shaped errors).

Step 12 prepare errors use `"code"` for user-side module errors (`NO_ELIGIBLE_VAULT`, etc.).
Step 13 verifier responses use `"eligibility"` for the four LIP-155 verdict strings only, not
`"code"`, so delivery and demos can distinguish transport/module faults from paid-Store outcomes
without overloading Step 12's code enum.

Do not expose `PolicyRejectReason` u32 on the module API for the demo; map core discriminants to
the four LIP-155 codes internally (D1 collapses policy detail into `PARAMS_REJECTED`).

Provider payee identity (decision A):

- Inbound verify binds `VaultProof.provider_id` and on-chain `StreamConfig.provider` to the demo
  stream payee account aligned with [`fixtures/localnet.json`](../fixtures/localnet.json)
  `provider_account_id` and the Step 10a seed.
- Implement as compile-time demo constants in the module (base58 → 32-byte id beside Step 12
  prepare logic), not a new public configure method and not runtime manifest loading.
- The module does not read `FIXTURE_MANIFEST`; operators and verify scripts use that env var
  (default `fixtures/localnet.json` relative to repo root) for chain fixture ids and logoscore
  smoke. A mismatch between re-seeded manifest and rebuilt module constants is an operator error.
- Do not use `registerProviderMapping` or `requesterPeerId` to infer provider self-id;
  `registerProviderMapping` remains user-outbound routing only (N5).
- Step 17 two-host may later add an explicit provider identity API if daemons need different
  payees without a shared fixture.

`StreamProviderPolicy` for the demo:

- Hardcode the Step 12 runbook table (`min_rate` 1, `min_allocation` 1,
  `max_create_stream_deadline_delay` 3600) next to existing demo rate/deadline constants in the
  module; no `configureProviderPolicy` for MVP.
- On proposal acceptance, snapshot that struct into persistence as `policy_at_acceptance` for
  `stream_satisfies_policy` / accepted terms (Step 3a). JSON shape (u64 fields as decimal strings
  in the file, same wide-integer style as other persist scalars):

```json
"policy_at_acceptance": {
  "min_rate": "1",
  "min_allocation": "1",
  "max_create_stream_deadline_delay": "3600"
}
```

Provider persistence (extends [N4](#n4-persistence-policy)):

- Same file `payment_streams_state.json`, separate concern from user `negotiations` / session keys.
- Bump `schema_version` to `2`. Informative v2 top-level shape (user keys unchanged from v1):

```json
{
  "schema_version": 2,
  "peer_mappings": { },
  "negotiations": [ ],
  "inventory": [ ],
  "provider_acceptances": [ ]
}
```

- v1 → v2 on load: keep `peer_mappings`, `negotiations`, and `inventory` as-is; set
  `schema_version` to `2`; add `provider_acceptances: []` if missing.
- Rows in `provider_acceptances` keyed by `(vault_id, provider_id_hex)` (payee octets, lowercase
  hex, same convention as negotiations).
- Row fields: accepted stream params (rate, allocation, `service_id`, `create_stream_deadline`,
  etc.), `policy_at_acceptance` object above, `session_public_key_hex`, optional `stream_id` (omit
  or null until bound by first valid `StreamProof`); no session private keys on the provider side.
- Upsert on proposal `OK`: key `(vault_id, provider_id_hex)`; overwrite params, policy snapshot,
  session pubkey, and deadline; preserve existing `stream_id` if already set (proposal retry with
  new terms before stream bind).
- Evict stale rows on cold start and on verify entry using clock-10 vs deadline (N4), no background
  timer.

FFI (Step 13 deliverable):

- Step 4/12 already expose inner sign/verify, policy predicates, and eligibility wrapper
  serialize (`payment_streams_ffi_serialize_eligibility_proof_*`). Core has
  `parse_eligibility_proof` in Rust tests only.
- Step 13 adds C ABI `payment_streams_ffi_parse_eligibility_proof_bytes` (outer protobuf → arm +
  inner byte slice) for the Qt bridge; inner `StreamProposal` / `StreamProof` decode stays on
  existing verify/decode FFI entry points.

Clock-10 for fold and deadlines (LEZ 510+):

- Wallet `get_account_public` on the clock account returns a timestamp in milliseconds (u64).
- Module and FFI fold path use seconds: `timestamp_secs = timestamp_ms / 1000` (integer division,
  truncate toward zero, do not round). Same rule as Step 12 list/fold reads
  (`chain_timestamp_to_fold_seconds` in `lez-payment-streams-ffi`).

Verification pipeline order (pragmatic chain reads):

1. Hex-decode `proofBytes` and `canonicalRequestBytes`.
2. Parse canonical Store request from N8 wire via existing FFI (digest/signing inputs for verify).
3. Parse outer `EligibilityProof` (FFI above); select `stream_proposal` vs `stream_proof` arm.
4. Decode inner `StreamProposal` or `StreamProof` (existing FFI).
5. Read `service_id` from proposal params (proof arm: from bound acceptance row or chain stream
   context after reads); compare to demo constant `/vac/waku/store-query/3.0.0` → `PARAMS_REJECTED`
   if mismatch.
6. Cryptographic verification (owner signature or session signature over canonical request) →
   `PROOF_INVALID`; no stream PDA reads yet.
7. Arm-specific chain reads and policy:
   - Proposal: vault config, vault holding, clock-10; `proposal_satisfies_policy` →
     `PARAMS_REJECTED` or persist upsert on `OK`.
   - Proof: stream PDA for `stream_id` from proof, clock-10, fold; if acceptance row has no
     `stream_id` yet, run `new_stream_satisfies_proposal` then set `stream_id` on success; else
     `stream_satisfies_policy` only.
8. Return `eligibility":"OK"` when the arm's checks pass.

“Purely structural” failures (malformed wrapper, bad signatures, wrong owner key) must not perform
stream PDA reads. Proposal-path vault holding reads are allowed and expected after crypto passes.

`requesterPeerId`: log on verify entry and log the final `eligibility` outcome; no rate limits or
peer-based predicates in MVP.

Side effects and out of scope for Step 13:

- `response_within_policy` applies to outbound Store response sizing, not inbound verify; defer to
  Step 17 serving path.
- Optional user-side `proposal_satisfies_policy` in `prepareEligibilityForStoreQuery` (Step 3a
  SHOULD) is alignment polish, not a Step 13 blocker.

Verification scripts (mirror Step 12):

- `VERIFY_LOGOSCORE=0`: FFI unit tests including new parse helper; existing verify/policy tests.
- `VERIFY_LOGOSCORE=1`: `./scripts/verify-step13-dod.sh` on one logoscore instance — mandatory
  happy path: `prepareEligibilityForStoreQuery` → `verifyEligibilityForStoreQuery` with the same
  `bytes_hex` and N8 canonical hex; assert `eligibility":"OK"` on seeded `stream_proof`. Mandatory
  negative: one tampered signature or canonical byte → `PROOF_INVALID`. Optional local cases (not
  CI-gated): fresh `stream_proposal` OK, expired deadline → `PARAMS_REJECTED`.
- Cross-test is single-host; two-host remains Step 17.

Runbook [`docs/step13-provider-eligibility.md`](docs/step13-provider-eligibility.md) (create with
implementation): API/encoding table (mirror Step 12), env vars for scripts (`FIXTURE_MANIFEST`,
wallet paths, `PAYMENT_STREAMS_GUEST_BIN`), prepare→verify demo sequence, troubleshooting (wallet
not open, provider id mismatch vs constants, depleted stream, clock/fold). Normative demo numbers
stay in Step 12 runbook.

#### Components required to run

`logoscore` daemon hosting both modules.
The structural-failure portion of the definition of done needs nothing more.
The happy-path verdict portion needs the Steps 10a–11b stack
(LEZ sequencer plus deployed program plus seeded vault/stream state).
Module retest loop:
[`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md) (Part 3).

#### Definition of done

For fixed inputs the verifier returns `OK` on the happy path
and the documented eligibility status code on each failure mode
(`PARAMS_REJECTED`, `PROOF_INVALID`, `STREAM_NOT_ACTIVE`).
Tampered or malformed proofs must fail with `PROOF_INVALID` without stream PDA reads; proposal-path
vault reads after successful crypto are in scope (see [Implementor approach](#implementor-approach-mvp-demo)).
`payment_streams_ffi_parse_eligibility_proof_bytes` ships in Step 13 FFI.
`./scripts/verify-step13-dod.sh` exits 0 with `VERIFY_LOGOSCORE=0`; with localnet up, logoscore
prepare → verify cross-test returns `eligibility":"OK"` on the seeded stream proof path.

Status: complete in tree — runbook [`docs/step13-provider-eligibility.md`](docs/step13-provider-eligibility.md),
`make verify-step13`. Logoscore cross-test may SKIP on depleted stream `0` until
`./scripts/demo-localnet-fresh.sh` (same recovery as Step 12 strict `stream_proof`).

