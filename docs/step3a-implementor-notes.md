# Step 3a — implementor notes

Companion to `integration-plan-v2.md` (Step 3a) and
`docs/step3-stream-provider-policy.md`.
Normative protocol: LIP-155 (`rfc-index/docs/ift-ts/raw/payment-streams.md`).

## Scope boundary

Step 3a is pure `lez-payment-streams-core`:

- Typed Rust inputs and outputs.
- No protobuf parse, no FFI, no Qt module.
- Wire decoding and signatures belong to Step 4 and the module.

Predicates MUST NOT depend on decoded `VaultProof` / `StreamProof` blobs.
Callers (FFI, module) supply numbers and byte arrays already validated or
parsed upstream.

## Suggested module layout

- `stream_provider_policy.rs` — `StreamProviderPolicy`, `StreamParams`,
  `PolicyRejectReason`.
- `policy.rs` (or `policy/`) — predicate functions.
- Reuse `StreamConfig::at_time` for folding; optional thin `fold_stream`
  wrapper returning `{ config, accrued, unaccrued }` at `t`.

Export public types from `lib.rs` for Step 3b.

## Input structs (recommended)

Avoid coupling predicates to Step 4 wire types.

| Struct | Role |
| --- | --- |
| `StreamProviderPolicy` | Advertised / pinned policy fields |
| `StreamParams` | Proposed or accepted stream terms (`stream_rate`, `stream_allocation`, `create_stream_deadline`, `service_id` bytes) |
| `ProposalCheckInputs` | `params`, `policy`, `vault_holding_balance`, `vault_total_allocated`, `now` |
| `AcceptedStreamTerms` | `params`, `provider_id` (`AccountId`), `policy_at_acceptance` |
| (API shape) | First proof vs stream: `new_stream_satisfies_proposal(folded_stream, proposal_params, proposal_provider_id)` |

`service_id` is compared in the module against a configured constant,
not inside `stream_satisfies_policy`.

## PolicyRejectReason (suggested variants)

Define a concrete enum in core.
Extend as needed; keep FFI mapping stable in Step 3b.

- `RateBelowPolicyMin`
- `AllocationBelowPolicyMin`
- `CreateStreamDeadlineInvalid`
- `UnallocatedInsufficient`
- `RateBelowAcceptedParams`
- `AllocationBelowAcceptedParams`
- `ProviderMismatch`
- `StreamNotActive`
- `ResponseTooLarge`

Module maps to eligibility status (see integration plan table).
Signature and malformed wire → `PROOF_INVALID` in Step 4, not these variants.

## Predicate semantics (pitfalls)

### `proposal_satisfies_policy`

- `now` and `create_stream_deadline` use LEZ clock-account timestamps
  (same domain as `StreamConfig::at_time`).
- Require `t < create_stream_deadline` and
  `create_stream_deadline <= t + max_create_stream_deadline_delay`.
- Solvency: `vault_holding_balance.saturating_sub(vault_total_allocated)
  >= stream_allocation` (or equivalent explicit `unallocated` input).

### `new_stream_satisfies_proposal`

- Run on folded `StreamConfig` at verification `now`.
- Compare **stored `allocation` field** and `rate` to accepted `StreamParams`
  (>= semantics).
- Do **not** compare unaccrued to `stream_allocation`.
  Accrual lowers unaccrued without reducing `allocation` until a claim.
- LEZ: `StreamConfig.provider` octets equal `provider_id` from acceptance.

### `stream_satisfies_policy`

- Input folded stream must be `ACTIVE`.
- On-chain `rate` >= `policy_at_acceptance.min_stream_rate` (and >= accepted
  params if you also enforce proposal floors on every request).
- Provider binding: same 32-byte payee check as establishment.
- Use **policy pinned at acceptance**, not the provider's latest
  advertisement (LIP-155).

### `response_within_policy`

- Pure `response_len <= policy.vault_proof_max_response_bytes`.
- Demo module chooses reject vs trim before send; core only reports over-cap.

## Test vectors

Definition of done requires documented pass/fail cases.

Prefer either:

- `lez-payment-streams-core/tests/policy_vectors.rs`, or
- `docs/step3a-policy-vectors.md` referenced from tests.

Minimum coverage:

- Fold: active accrual, depletion to paused, paused unchanged.
- Proposal: deadline edge, unallocated edge, below min rate/allocation.
- New stream: allocation equal, greater, less; wrong provider bytes.
- Stream policy: active vs paused; rate below min.
- Response cap: at limit, over limit.

Reuse existing `stream_config` unit tests for fold where possible.

## Step 3b handoff

- Same `PolicyRejectReason` variants through `repr(u32)` (stable discriminants; FFI wraps in 3b).
- No duplicate arithmetic in FFI.
- Document endianness for any `u64` / wide integers crossing the C ABI
  (follow existing FFI patterns).

## Out of scope (do not pull into 3a)

- Pending-proposal persistence, session JSON, eviction timers (module).
- `service_id` string config and Store handler binding (module).
- Protobuf eligibility envelope (Step 4 + Delivery).
- Load cap extension (deferred).
