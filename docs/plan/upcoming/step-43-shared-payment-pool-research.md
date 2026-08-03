# Step 43 — shared payment pool model for multiple service providers

Index: [index.md](../index.md). Status: **upcoming** (research / spec only).

Goal: close the Testnet v0.3 incentivisation deliverable
[Research a shared payment pool model for multiple service providers](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3)
by publishing a specification that investigates a shared payment pool with
double-spend prevention and proportional or receipt-based claiming
(FURPS F9–F11, U10).

Roadmap:

- F9. A payment pool can be shared across multiple service providers rather
  than being restricted to a single provider.
- F10. The payment protocol prevents double-spending/double-claiming of
  rewards from a shared payment pool.
- F11. Service providers can claim rewards from a shared payment pool in
  accordance with the service they provided, using a reputation- or
  receipt-based method.
- U10. A specification investigating a shared payment pool model supporting
  multiple service providers, double-spend prevention, and proportional
  reward claiming is published.

Prior LIP work: [Step 19](../completed/step-19-lip155-onchain-spec.md).
Baseline model today: one vault owner, streams each to a single
`provider_id`, claims authorized by that provider. Delivery Receipts exist
as an optional LIP extension for claim tied to user acknowledgment.

## Problem

MVP and current LIP-155 streams bind each stream’s claim rights to one
provider. Some deployments want one funded pool (or vault-like custody)
that several providers can draw from according to service delivered, with
each reward claimable only once. That shared-pool model still needs a
research packet before any guest or module commitment.

## Scope

In scope:

- Research and publish an investigation specification (LIP Protocol
  Extensions candidate, standalone research note, or rfc-index draft —
  choose in D43.2).
- Cover shared custody or pool identity (F9), double-claim / double-spend
  prevention (F10), and reputation- or receipt-based claim authorization
  aligned with service delivered (F11).
- Compare against the current single-provider stream model and existing
  Delivery Receipts extension; state what would stay off-chain and what
  would stay on-chain.
- Explicit open questions and follow-up boundaries for any later
  implementation step.
- Roadmap Specs checklist link once published.

Owned elsewhere (cite only):

- Guest / FFI / module / delivery implementation — later steps if research
  recommends build (D43.5).
- E2E or dogfood of multi-provider pools — after an implementation step.
- MVP Store eligibility (single provider) — unchanged by this research
  packet (D43.6).
- Production reputation system choice — sketch only the interfaces the
  payment protocol would require.

## Depth target

- Prefer roles, invariants, threat notes (double claim), and claim
  authorization sketches; leave bytecode and PDA grids to a later
  implementation packet if one opens.
- If recommending a LIP extension, match Step 19 extension style (short
  normative hooks + informative research); if the work stays investigative,
  label MUST/SHOULD as provisional.
- U10 is satisfied by a published investigation tip (D43.4).

## What the research should answer

Candidate questions (refine in the decision log):

1. Is the shared pool a new on-chain account type, a vault with multiple
   authorized claimers, or an off-chain coordination layer over many streams?
2. What is the unit of claim entitlement (time, receipts, attested work,
   reputation score), and who is the trust root?
3. How is double-claim prevented (nullifiers, spent-receipt sets, on-chain
   accounting, provider-local only)?
4. How do pause/resume and stream-backed eligibility compose with shared
   pools, if at all?
5. What moves to a later implementation step, and what stays outside payment
   streams?

## Prerequisites

- Step 19 LIP baseline available.
- Current MVP single-provider constraint understood (program index).

## Decision log (open — discuss before drafting)

| Id | Topic | Tentative |
| --- | --- | --- |
| D43.1 | Packet ownership | New Step 43; research only. |
| D43.2 | Artifact home | TBD — LIP Protocol Extensions draft or standalone investigation in logos-lips / rfc-index. |
| D43.3 | Relation to Delivery Receipts | Prefer compose with or explicitly supersede; account for the existing extension. |
| D43.4 | Publication bar for U10 | Merged or program-pinned published investigation tip (draft PR alone leaves U10 open). |
| D43.5 | Implementation follow-up | Later steps if research recommends build and product prioritizes it. |
| D43.6 | Store demo impact | Shared-pool research leaves Steps 17–39 evidence as-is. |

## Definition of done

- A published specification investigates shared pools with multi-provider
  claim, double-claim prevention, and reputation- or receipt-based reward
  claiming (F9–F11, U10).
- Open questions and follow-up implementation boundaries are explicit.
- Decision log closed for D43.1–D43.4 (D43.5–D43.6 optional).
- Roadmap Specs checklist can link the published artifact.
- Close is docs-only (investigation tip + checklist links).

## Verification

Docs-only: peer read against F9–F11/U10; threat pass on double-claim.
Docs review is sufficient (no `make verify-*` gate).

## Related

- [step-19-lip155-onchain-spec.md](../completed/step-19-lip155-onchain-spec.md)
- [incentivisation_v0.3](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3)
- [feature-branch-pins.md](../../reference/feature-branch-pins.md)
