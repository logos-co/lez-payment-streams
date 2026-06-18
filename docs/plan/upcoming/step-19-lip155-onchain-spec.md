# Step 19 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 19, LIP-155 on-chain spec alignment

Prerequisite: on-chain guest and `lez-payment-streams-core` accepted as complete
([`architecture.md`](../../../architecture.md),
[`docs/archive/implementation-plan-on-chain.md`](../../archive/implementation-plan-on-chain.md)).

Scheduling: may run in parallel with Steps 17–18 while integration work does not require guest or
core changes. If Step 17 or 18 exposes a guest/core defect, finish that fix before merging spec
text that describes the old behavior. Must reach `main` before Step 20 cites normative on-chain
text on `main` (linking to a spec branch is acceptable only as an interim doc-packet note).

Architectural context:
LIP-155 lives in `logos-lips` / `rfc-index` at
`docs/ift-ts/raw/payment-streams.md`. Upstream `master` lacks the full
`## On-Chain Protocol` section; draft prose exists on branch
`feat/payment-streams-onchain-part` (cherry-pick or focused PR — do not land unrelated spec-repo
churn from that branch unless explicitly scoped).

Implementation source of truth: guest handlers, `instruction.rs`, `program_tests`, and
[`architecture.md`](../../../architecture.md). Spec follows code; document known wallet-vs-guest
policy (for example shielded-only `PseudonymousFunder` not enforced in the guest).

Deliver:

- Audit matrix (markdown table in the spec PR or `docs/audit-matrix-step19.md` in `rfc-index`):
  columns Instruction | Spec subsection | Guest handler | Test (or gap note).
- Updated `payment-streams.md`: `## On-Chain Protocol` (accounts, PDAs, accounting, lazy accrual,
  authorization, privacy tiers) plus LEZ wire detail as needed (instruction catalog, clock accounts,
  PDA seeds) without duplicating off-chain Store material in [`integration-contracts.md`](../../integration-contracts.md).
- Merge to `logos-lips` / `rfc-index` `main`; spec CI / markdown lint pass.
- Update [`handoff.md`](../../../handoff.md) and [`architecture.md`](../../../architecture.md) links
  to published path on `main`.

Definition of done:

- External readers can rely on `main` for on-chain semantics matching the deployed demo program.
- LEZ canonical signing in the LIP remains consistent with N8 and Step 15 Nim parity.

Not in scope: Logos Store integration steps; rewriting the full off-chain protocol unless required
for cross-link consistency.
