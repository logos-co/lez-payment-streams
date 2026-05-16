# Step 3 — stream fold and StreamProviderPolicy

Normative protocol text:
`rfc-index/docs/ift-ts/raw/payment-streams.md` (LIP-155).

LEZ wire bindings:
LIP-155 [LEZ off-chain integration](rfc-index/docs/ift-ts/raw/payment-streams.md#lez-off-chain-integration).

Implementation:
`integration-plan-v2.md` Step 3a (core) and Step 3b (FFI).
Implementor checklist: `docs/step3a-implementor-notes.md`.

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
