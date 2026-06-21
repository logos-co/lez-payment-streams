# Step 19 — plan excerpt (complete)

Normative packet for agents. Index: [integration-index.md](../../../integration-index.md).

Status: complete. Canonical LIP-155 text lives on `logos-lips` / `rfc-index` branch
`feat/payment-streams-onchain-part` at rev `345c8eef` (local clone `lez-related/rfc-index`).
Integration program outcome does not require merge to `main`; merge to the spec default branch
is optional follow-up for published `lip.logos.co` URLs.

Convention: when citing “the LIP” in this integration repo, use the branch pin above unless
`main` has caught up.

Scope: Step 19 edits only LIP-155 in the spec repository —
`docs/anoncomms/raw/payment-streams.md` on `github.com/logos-co/logos-lips`
(local clone: `lez-related/rfc-index`; legacy vacp2p clone: `rfc-index-old`).
No changes to `lez-payment-streams`, delivery forks, or other repos in this step
(follow-up may update outbound links after merge).

### Step 19, LIP-155 on-chain spec alignment

Goal: land LIP-155 with a `## On-Chain Protocol` chapter that maps chain-agnostic
payment-stream notions (from Theory and Semantics) onto the LEZ reference architecture —
which programs participate, how they interact, and what MUST vs SHOULD hold on chain.
The LIP is not a transcript of the guest. **Done** on branch `feat/payment-streams-onchain-part`
(`345c8eef`).

Normative boundaries:

- On-chain section: behavior, roles, invariants, and LEZ program composition at
  the level of “what the chain must provide” (for example a time signal used for
  folding). Exclude guest error enums, exact PDA seed string literals, per-instruction
  account metas, and LEZ-specific account or program ids (those belong in
  Implementation Considerations). Layout detail stays in `lez-payment-streams`.
- Canonical signing split:
  - Requirements live in `## Off-Chain Protocol`: protobuf for interchange; cryptographic
    commitments use a chain-specific canonical form; implementations MUST NOT sign
    raw protobuf unless a chain integration explicitly says so; chain integrations MUST
    define deterministic signed material; `VaultProof` / `StreamProof` MUST cover the
    fields the protocol specifies (see existing VaultProof and StreamProof subsections).
    Strengthen or cross-link these MUSTs if the feature-branch draft is thin; do not
    move requirement prose into On-Chain or Implementation Considerations.
  - LEZ bytes live in `## Implementation Considerations`: Borsh struct layouts, domain
    prefix values, field order, and prehash steps for vault-proof and Store eligibility
    preimages (LEZ demo binding). `### LEZ off-chain integration` keeps identifier
    encodings and points readers to Implementation Considerations for preimage bytes;
    it MUST NOT duplicate full byte layouts. Demo MUST still match
    [N8](../../reference/decisions-and-notes.md#n8-canonical-store-request-bytes-format)
    and Step 15 Nim parity; N8 remains the engineering check against Implementation
    Considerations text.
- Alternative on-chain layouts: matching Theory prose and operation correspondence
  is sufficient; no separate “compliance surface” section required.
- Do not normatively publish a reference payment-streams program id in the LIP.

Editor decisions (Step 19):

- Clock: On-Chain describes time-signal semantics (caller-supplied clock, allowlist
  validation, folding, deadline domain). Concrete LEZ clock account ids and
  frequencies live in Implementation Considerations (testnet may change).
- Wallet: dual — short summary under On-Chain; full detail under Security and Privacy.
- Operation correspondence: one summary table (Theory op, reference instruction name,
  authorizer, effect on holding / allocation / `total_allocated`); not a Writable/Signer grid.
- Deposit / PP: high-level program composition only (payment-streams + platform transfer;
  PP MAY involve multi-program proofs); no SPEL-level detail in the LIP.
- Canonical signing: requirements in Off-Chain Protocol; LEZ preimage bytes in
  Implementation Considerations (see normative boundaries above).
- Implementation Considerations: recreate for the current LEZ demo (do not copy obsolete
  `master` prose). Place the section after `## Protocol Extensions` and before
  `## References`. Content is LEZ-specific ids and signing bytes only; On-Chain and
  Security carry binding and privacy narrative.

Prerequisite: on-chain guest and `lez-payment-streams-core` accepted as complete
([`architecture.md`](../../../architecture.md),
[`docs/archive/implementation-plan-on-chain.md](../../archive/implementation-plan-on-chain.md)).

Scheduling: ran in parallel with Steps 17–18. Step 20 may cite the branch pin above;
optional later PR merges the same file to `main` for public lip URLs only.

Architectural context:
LIP-155 lives at `docs/anoncomms/raw/payment-streams.md` in `github.com/logos-co/logos-lips`
(local clone `lez-related/rfc-index`). **Integration canonical branch:**
`feat/payment-streams-onchain-part` (`345c8eef`) includes `## On-Chain Protocol` and
`## Implementation Considerations` in the prescribed placement. Upstream `main` may lag until
an optional merge PR lands.

Reference implementation (informative for PR review, not LIP body):
guest handlers, `instruction.rs`, `program_tests`, and
[`architecture.md`](../../../architecture.md).
On-chain MUST/SHALL in the LIP MUST align with this reference LEZ program; where the LIP is
silent on layout, the demo program is the conformance artifact.
Document wallet-vs-guest policy explicitly (for example shielded-only
`PseudonymousFunder` is wallet responsibility, not guest enforcement).

Sources for Implementation Considerations prose (outside the LIP file):
[N8](../../reference/decisions-and-notes.md#n8-canonical-store-request-bytes-format),
`lez-payment-streams-core` canonical types, LEZ clock constants / demo fixtures for account ids.

Depth target (vs other ift-ts specs):
LIP-155 On-Chain is a LEZ binding chapter, not an eth-mls-style smart-contract ABI spec.
If the draft branch already lists semantic subsections (accounts, PDAs, accounting, lazy accrual,
authorization, privacy tiers), extend those rather than replacing them with a guest transcript.

Deliver (satisfied on `feat/payment-streams-onchain-part`):

- `docs/anoncomms/raw/payment-streams.md` on branch `feat/payment-streams-onchain-part`
  (`345c8eef`); optional follow-up PR to `main` for published lip URLs only.
- Section updates within that file:
  - `## On-Chain Protocol` — scope; chain-agnostic invariants remain in Theory; LEZ binding covers
    time signal for folding, shielded vs transparent execution model, authorization model, and
    program composition at high level.
  - Conceptual account roles; balance accounting and lazy accrual; authorization; privacy tiers
    stored on chain.
  - Programs and interactions (payment-streams program, platform transfer for deposit, system
    clock for folding, wallet for shielded policy; PP deposit multi-program at high level only).
  - One operation correspondence summary table covering all reference guest operations (Theory op,
    instruction name, authorizer, effects on holding / allocation / `total_allocated`).
  - Close vs claim accounting in On-Chain text: close releases unaccrued to vault; accrued
    claimable later; claim pays provider and reduces allocation and `total_allocated`.
  - Clock semantics in On-Chain (no genesis account id literals here).
  - Wallet summary on On-Chain (MUST refuse transparent touch of `PseudonymousFunder` vaults;
    guest cannot detect mode); expand Security and Privacy with detail (pre-shield, linkability).
  - Cross-links to Theory and LEZ off-chain integration (vault identity, deadlines).
  - `## Off-Chain Protocol` — ensure wire encoding / canonical signing and VaultProof /
    StreamProof coverage MUSTs are clear and chain-agnostic; remove any “integration
    plan Step 4” deferrals; for LEZ, state that preimage bytes are specified under
    Implementation Considerations.
  - `### LEZ off-chain integration` — identifier encoding table; protocol MUSTs that
    reference signing point to Implementation Considerations for bytes (no full Borsh
    struct reproduction here).
  - `## Implementation Considerations` (new, after Protocol Extensions, before References) —
    LEZ clock account ids for the demo testnet; LEZ canonical signing byte layouts
    (vault-proof and Store eligibility preimages) aligned with N8; note that LEZ ids and
    layouts may change across testnet revisions. Do not duplicate On-Chain or Security prose.

Definition of done:

Integration closure (met):

- Canonical spec text on `feat/payment-streams-onchain-part` at `345c8eef` with
  `## On-Chain Protocol` and `## Implementation Considerations` as specified below.
- Content and link-integrity criteria in this packet satisfied on that branch.
- Reference LEZ demo (`program_tests`, N8 / Step 15 parity) gates contradictions with the LIP.

Not required for integration “done”:

- Merge to `logos-lips` / `rfc-index` `main` or live `lip.logos.co` publication.

Audience (what “done” means for readers):

- A developer familiar with LEZ program and account models but new to payment-streams can
  map the protocol onto LEZ architecture (programs, time signal, authorization, operation
  correspondence) using the LIP only — not guest source. Preimage bytes and clock account
  ids come from Implementation Considerations; on-chain layout detail remains in the
  reference repo.
- A security or protocol reviewer can follow solvency and lazy-accrual invariants from
  Theory and On-Chain without reading Rust.

Content and conformance:

- Operation correspondence summary table is present with correct close vs claim accounting.
- `## Implementation Considerations` exists in the prescribed place with recreated LEZ content
  (not obsolete `master` text).
- The reference LEZ program used in the demo implements the on-chain binding described in the LIP;
  PR review plus existing `program_tests` gate contradictions (no standing audit matrix required).
- LEZ signing bytes in Implementation Considerations match N8 and Step 15 Nim parity tests;
  Off-Chain Protocol states signing requirements without embedding those bytes.

Link integrity (within LIP-155 only):

- All internal cross-links inside `payment-streams.md` resolve (Theory and Semantics,
  Off-Chain Protocol, On-Chain Protocol, Implementation Considerations, Security and Privacy
  Considerations, LEZ off-chain integration anchors).

Not in scope:

- Any file outside `docs/anoncomms/raw/payment-streams.md` (including `handoff.md` and
  `architecture.md` in `lez-payment-streams`).
- Logos Store integration ([`integration-contracts.md`](../../integration-contracts.md)).
- Rewriting the full off-chain protocol except signing-requirement clarity, LEZ integration
  cross-links, and Implementation Considerations LEZ bytes.
- Full Borsh preimage layouts under Off-Chain or On-Chain (Implementation Considerations only).
- Normative on-chain account Borsh layouts, per-instruction account metas, or guest error enums
  in On-Chain (see `lez-payment-streams` repo).
- Normative reference program id in the LIP.
- A mandatory audit matrix artifact on `main`.
- Mandatory merge to spec `main` before Step 20 or integration sign-off.
- eth-mls/LIP-101-depth instruction tables (Writable/Signer grids) as DoD.

Optional (PR author, not definition of done): spot-check LIP MUSTs against the reference guest.
