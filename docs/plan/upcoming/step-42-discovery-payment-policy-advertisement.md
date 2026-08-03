# Step 42 — service discovery integration for provider payment policy advertisement

Index: [index.md](../index.md). Status: **upcoming** (research / spec;
Discovery-track support required).

Goal: close the Testnet v0.3 incentivisation deliverable
[Research service discovery integration for provider payment policy advertisement](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3)
by publishing a specification for discovering service providers and the
payment policies they advertise via existing service discovery
(FURPS F6, F7, U8).

Roadmap:

- F6. A client can discover service providers that offer a specific service.
- F7. A client can discover the payment policy advertised by a discovered
  service provider.
- U8. A specification for discovering service providers and their payment
  policies via existing service discovery is published.

Owners (roadmap): AnonComms Incentivisation (primary), AnonComms Discovery
(support).

Prior LIP work: [Step 19](../completed/step-19-lip155-onchain-spec.md).
Canonical payment-streams file: `docs/anoncomms/raw/payment-streams.md` in
`logos-lips` / local `rfc-index`.

## Problem

LIP-155 already requires that stream-backed eligibility advertisements carry
`StreamProviderPolicy`, and leaves discovery mechanics to a separate
surface. Clients still need a published contract for:

1. Finding providers that offer a given service (for example Store).
2. Retrieving the payment policy those providers advertise.

That contract gives policy bytes (including any published
`accepted_tokens` fields) a standard discovery path. Historical notes
deferred “discovery wire encoding for policy blobs”
([policy-implementor-notes.md](../../archive/steps/policy-implementor-notes.md)).

## Scope

In scope:

- Research and publish a specification (or LIP / discovery-doc chapter) that
  binds payment-streams policy advertisement to the existing AnonComms
  service discovery surface.
- Define the payment-streams side of the contract: which policy object is
  advertised (`StreamProviderPolicy` and related fields), freshness /
  applicability to new proposals only (align with LIP), and how a client
  obtains service identity plus policy together.
- Explicit handoff of discovery transport, record formats, and peer
  resolution to the Discovery track.
- Optional small LIP-155 cross-links that point at the discovery doc while
  discovery mechanics stay in the Discovery artifact.
- Roadmap Specs checklist link once published.

Owned elsewhere (cite only):

- Discovery client/server implementation in this repo or delivery forks —
  later product step if prioritized (D42.6).
- Guest / FFI / module work beyond citing the published policy shape.
- E2E dogfood of discovery — after an implementation step exists.
- Full discovery protocol text — Discovery track; LIP-155 keeps its
  mechanics handoff.
- Basecamp UI / public hosted Store — Steps 21 / 23 (wontfix).

## Depth target

- Prefer an investigation or discovery-integration spec at the same
  abstraction level as LIP off-chain policy prose: roles, message or record
  fields, MUST/SHOULD for advertisers and clients.
- Reuse the existing AnonComms service discovery surface from this program.
- Cite LIP-155 as the SSOT for `StreamProviderPolicy`.

## What the research should answer

Candidate questions (refine in the decision log):

1. Which existing discovery mechanism is in scope for Testnet v0.3
   (ENR capabilities, DNS, rendezvous, other — Discovery track decides)?
2. How is “offers service X” encoded so F6 is met for Store and, informatively,
   for other request-response services?
3. How is `StreamProviderPolicy` (and extension fields such as
   `accepted_tokens`) carried or referenced so F7 is met?
4. What is the trust and freshness model (unsigned ads, signed policy, pin at
   proposal acceptance already in LIP)?
5. Where does the published text live (Discovery LIP/RFC vs payment-streams
   Protocol Extensions vs joint note)?

## Prerequisites

- Discovery-track contact or draft surface available enough to name the
  target mechanism (if blocked hard, move this packet to `waiting/`).
- Prefer citing the latest published LIP-155 `StreamProviderPolicy` (rebase
  citations if the LIP tip moves while drafting).
- Step 19 / LIP discovery handoff wording understood.

## Decision log (open — discuss before drafting)

| Id | Topic | Tentative |
| --- | --- | --- |
| D42.1 | Packet ownership | New Step 42; Incentivisation drafts payment-streams side; Discovery owns transport. |
| D42.2 | Spec home | TBD — Discovery doc with payment-streams cite, short joint note, or LIP pointer only. |
| D42.3 | Service scope | Store-first for this program; keep service_id generic per LIP. |
| D42.4 | Policy blob SSOT | LIP-155 `StreamProviderPolicy`; discovery doc cites that message. |
| D42.5 | Blocker handling | Stay in `upcoming/` while drafting against a named Discovery draft; move to `waiting/` if Discovery input is unavailable. |
| D42.6 | Implementation follow-up | Later step if product prioritizes a concrete discovery client path. |

## Definition of done

- A published specification satisfies U8: clients can be described as
  discovering providers for a service (F6) and reading advertised payment
  policy (F7) via existing service discovery.
- Payment-streams policy SSOT remains LIP-155; discovery text cites it.
- Decision log closed for D42.1–D42.5 (D42.6 optional).
- Roadmap Specs checklist can link the published artifact.
- Close is docs-only (spec + checklist links).

## Verification

Docs-only: peer read with Discovery support; checklist against F6/F7/U8.
Docs review is sufficient (no `make verify-*` gate).

## Related

- [step-19-lip155-onchain-spec.md](../completed/step-19-lip155-onchain-spec.md)
- [policy-implementor-notes.md](../../archive/steps/policy-implementor-notes.md)
- [incentivisation_v0.3](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3)
- [feature-branch-pins.md](../../reference/feature-branch-pins.md)
