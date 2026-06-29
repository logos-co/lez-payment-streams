# Step 3 — policy and implementor notes

Normative protocol: LIP-155 (`rfc-index/docs/ift-ts/raw/payment-streams.md`).
Integration plan: Step 3a (core) and Step 3b (FFI).

## Policy summary

Phases, policy struct fields, and MVP scope for Step 3a core predicates.

## Phases and checks

| Phase | Who acts | Step 3 predicates (core) | Other |
| --- | --- | --- | --- |
| Proposal | User sends `StreamProposal`; provider verifies | `proposal_satisfies_policy` | Signatures: Step 4 |
| Stream creation | User `create_stream` on-chain | — | No provider request |
| Service | User sends `StreamProof` | First: `new_stream_satisfies_proposal`, then: `stream_satisfies_policy` | Signatures: Step 4; `service_id` in module |

Service session and pending proposals: LIP-155 Assumptions and Protocol Flow.

## StreamProviderPolicy (struct fields)

| Field | Used by |
| --- | --- |
| `min_rate`, `min_allocation` | `proposal_satisfies_policy`, `stream_satisfies_policy` |
| `max_create_stream_deadline_delay` | `proposal_satisfies_policy` (with `create_stream_deadline`) |
| `vault_proof_max_response_bytes` | `response_within_policy` (provider outbound only) |

Proposal solvency: `vault_holding_balance - total_allocated ≥` proposed `allocation` (`StreamParams`, same semantics as on-chain stream `allocation` after `create_stream`)
(read vault after resolving `vault_id` + owner).

## Core functions (Step 3a)

| Function | Returns | Notes |
| --- | --- | --- |
| `fold_stream` | folded state | `StreamConfig::at_time` at clock `now` |
| `proposal_satisfies_policy` | `Result<(), PolicyRejectReason>` | LEZ `now` = clock account timestamp |
| `new_stream_satisfies_proposal` | `Result<(), PolicyRejectReason>` | First `StreamProof` only; compare stored `allocation`, not unaccrued |
| `stream_satisfies_policy` | `Result<(), PolicyRejectReason>` | Every `StreamProof`; stream must be `ACTIVE` |
| `response_within_policy` | `Result<(), PolicyRejectReason>` | Outbound `response_data` size; not inbound proof verify |

`service_id`: module compares accepted `StreamParams.service_id` to configured
Store protocol id (`/vac/waku/store-query/3.0.0`); not inside `stream_satisfies_policy`.

LEZ: `VaultProof.provider_id` (32 bytes) equals `StreamConfig.provider`.

## MVP scope (demo)

- All predicates + `PolicyRejectReason` in core; `repr(C)` verdict in FFI (3b).
- Map `PolicyRejectReason` → `PARAMS_REJECTED` / `PROOF_INVALID` / `STREAM_NOT_ACTIVE` in module.
- Demo provider MUST enforce `response_within_policy` on first vault-proof `OK`.
- Deferred: load cap extension, discovery wire encoding for policy blobs.

## Privacy (summary)

`service_id` stays in signed off-chain params and session state.
On-chain `StreamConfig` exposes payment terms only (LIP-155 Security).


## Implementor notes

### Scope boundary

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
| `StreamParams` | Proposed or accepted stream terms (`rate`, `allocation`, `create_stream_deadline`, `service_id` bytes); field names match on-chain [`StreamConfig`] |
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

Module maps to eligibility status (full mapping table in integration plan Step 3a).
Signature and malformed wire → `PROOF_INVALID` in Step 4, not these variants.

## Predicate semantics (pitfalls)

### `proposal_satisfies_policy`

- `now` and `create_stream_deadline` use LEZ clock-account timestamps
  (same domain as `StreamConfig::at_time`).
- Require `t < create_stream_deadline` and
  `create_stream_deadline <= t + max_create_stream_deadline_delay`.
- Solvency: `vault_holding_balance.saturating_sub(vault_total_allocated)
  >= allocation` for the proposed `StreamParams` (or equivalent explicit `unallocated` input).

### `new_stream_satisfies_proposal`

- Run on folded `StreamConfig` at verification `now`.
- Compare stored `allocation` field and `rate` to accepted `StreamParams`
  (>= semantics).
- Do not compare unaccrued to `StreamParams::allocation`.
  Accrual lowers unaccrued without reducing `allocation` until a claim.
- LEZ: `StreamConfig.provider` octets equal `provider_id` from acceptance.

### `stream_satisfies_policy`

- Input folded stream must be `ACTIVE`.
- On-chain `rate` >= `policy_at_acceptance.min_rate` (and >= accepted
  params if you also enforce proposal floors on every request).
- Provider binding: same 32-byte payee check as establishment.
- Use policy pinned at acceptance, not the provider's latest
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
