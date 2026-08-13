# Step 41 — non-native token support in provider payment policies

Index: [index.md](../index.md). Status: **complete** (2026-08).

Deliverable: [logos-lips#379](https://github.com/logos-co/logos-lips/pull/379) merged to
`logos-lips` `master` as `f09f9e9e` (`docs(anoncomms): Non-native token support for
payment streams`). U9 closed at program-pin bar (D41.4); optional `lip.logos.co`
follow-up unchanged from Step 19.

Goal: close the Testnet v0.3 incentivisation deliverable
[Research non-native token support in provider payment policies](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3)
by publishing LIP-155 text that lets a provider’s payment policy name which
non-native tokens it accepts (FURPS F8, U9).

Roadmap:

- F8. A provider’s payment policy can specify which non-native tokens it
  accepts as payment.
- U9. A specification investigating support for non-native tokens in
  provider payment policies is published.

Prior LIP work: [Step 19](../completed/step-19-lip155-onchain-spec.md).
Canonical file: `docs/anoncomms/raw/payment-streams.md` in `logos-lips` /
local `rfc-index`.

## Problem

Vaults are single-token; the vault token MAY be native or non-native.
Providers need a concrete policy shape so advertisements can list accepted
vault tokens and verifiers reject proposals whose vault token falls outside
that list.

Working tip (local `rfc-index` / PR branch):

- Branch: `docs/payment-streams-multi-token`.
- Draft PR: [logos-lips#379](https://github.com/logos-co/logos-lips/pull/379).
- Related tracker: [anoncomms-pm#76](https://github.com/logos-co/anoncomms-pm/issues/76).

U9 closes when the tip leaves draft and is published as the program pin
(merge to `master` or an equivalent cited rev).

## Scope

In scope:

- Finalize LIP-155 vault-token / provider-policy prose on `#379`.
- Normative policy shape (`StreamProviderPolicy.accepted_tokens`,
  `TokenStreamPolicy`), vault `token_id` at init, proposal satisfaction
  rules, and LEZ binding depth consistent with Step 19 / current LIP style.
- Optional pin update in this integration repo when the rev lands
  ([feature-branch-pins.md](../../reference/feature-branch-pins.md)).
- Roadmap Specs checklist link once published.

Owned elsewhere (cite only):

- Multi-token guest / FFI / module implementation — [Step 49](step-49-native-token-spec-alignment.md)
  (complete; native-only demo; D41.6).
- E2E, ImageIDs, dogfood — verification steps after implementation exists.
- Wrapped-native unification —
  [raw TODO](../raw-todos/wrapped-native-token-unification.md); adopt into
  the LIP in this step only if D41.7 says so.
- Live `lip.logos.co` publication — same optional follow-up rule as Step 19
  (D41.4).

## Depth target

Match existing Off-chain / LEZ sections:

- Prefer policy messages, MUST/SHOULD verifier rules, and what is on chain.
- Keep LEZ `token_id` encoding and deposit/claim asset-path composition at
  the current Implementation Considerations / LEZ depth (protocol binding,
  SPEL and module JSON stay in their own docs).
- Stay at roles, messages, and invariants; leave guest instruction grids to
  implementation docs.

## Tip content (on `#379` working text)

The tip under review includes:

- Vault token identity recorded at initialization (native or non-native).
- `VaultProof.token_id` REQUIRED; MUST equal on-chain identity.
- `TokenStreamPolicy` + `StreamProviderPolicy.accepted_tokens` for every
  accepted vault token (including native).
- At least one `accepted_tokens` entry; no duplicate `token_id` values.
- Single satisfaction path by matching `token_id` to an entry’s minima.
- LEZ all-zeroes native encoding and Token definition `AccountId`, with
  native and Token custody paths for deposit/claim.

## Prerequisites

- Step 19 pin / branch convention understood.
- Step 40 may run in parallel (privacy workflow); rebase or merge order is a
  decision (D41.2). Multi-token close can finish with Step 40 still open.
- Access to `docs/payment-streams-multi-token` / `#379`.

## Decision log

| Id | Topic | Status |
| --- | --- | --- |
| D41.1 | Packet ownership | Closed — new Step 41; leave Step 19 closed. |
| D41.2 | Spec target branch | Closed — finish `docs/payment-streams-multi-token` / `#379`; reconcile with Step 40 privacy branch before or after merge. |
| D41.3 | Native vs `accepted_tokens` shape | Closed — unify; native and non-native minima live in `TokenStreamPolicy` / `accepted_tokens` only. |
| D41.4 | Publication bar for U9 | Closed — merged to `master` at `f09f9e9e` (#379); optional `lip.logos.co` follow-up like Step 19. |
| D41.5 | Integration-repo pin update | Closed — cite `f09f9e9e` on `master` for multi-token policy ([feature-branch-pins.md](../../reference/feature-branch-pins.md)). |
| D41.6 | Implementation follow-up | Closed — [Step 49](step-49-native-token-spec-alignment.md) (native-only types/wire; no Token custody path). |
| D41.7 | Wrapped-native | Optional — remain in raw TODO until this step or a later step adopts it. |

## Definition of done

- LIP-155 on the chosen branch/rev specifies that a provider policy can list
  accepted non-native tokens and how verifiers apply those thresholds (F8).
- That tip is published under the bar in D41.4 (U9).
- Decision log closed for D41.1–D41.6 (D41.7 optional).
- This integration repo pins the rev when it is the citation tip.
- Roadmap Specs checklist can link the PR and/or merged rev.
- Close is docs-only (LIP + optional pin).

## Verification

Docs-only: peer read of policy satisfaction rules against F8; link integrity
in the LIP file. Docs review is sufficient (no `make verify-*` gate).

## Related

- [step-19-lip155-onchain-spec.md](../completed/step-19-lip155-onchain-spec.md)
- [step-40-lip155-privacy-workflow-spec.md](step-40-lip155-privacy-workflow-spec.md)
  (parallel LIP branch; merged independently)
- [wrapped-native-token-unification.md](../raw-todos/wrapped-native-token-unification.md)
- Implementation follow-up (D41.6): [step-49-native-token-spec-alignment.md](step-49-native-token-spec-alignment.md)
- [logos-lips#379](https://github.com/logos-co/logos-lips/pull/379)
- [incentivisation_v0.3](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3)
- [feature-branch-pins.md](../../reference/feature-branch-pins.md)
