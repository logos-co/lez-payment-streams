# Step 40 — LIP-155 privacy-preserving workflow in the specification

Index: [index.md](../index.md). Status: **waiting** (PR review).

Blocker: [logos-lips#397](https://github.com/logos-co/logos-lips/pull/397)
on branch `docs/payment-streams-private-execution`.
LIP tip edits for user unlinkability and provider unlinkability are on that
branch; step close waits on review and publication under D40.5.

Goal: update LIP-155 so the privacy-preserving payer and provider workflows
verified by Steps 36–39 are reflected at the same abstraction level as the
rest of the specification — protocol intent, invariants, and LEZ binding —
without turning the LIP into an implementation or E2E transcript.

Product evidence (do not reopen):
[Step 36](../completed/step-36-payer-funder-unlinkability.md),
[Step 37](../completed/step-37-payee-receiver-privacy.md),
[Step 38](../completed/step-38-store-privacy-e2e.md),
[Step 39](../completed/step-39-testnet-privacy-e2e.md) (complete).

Prior LIP work: [Step 19](../completed/step-19-lip155-onchain-spec.md).
Canonical file: `docs/anoncomms/raw/payment-streams.md` in `logos-lips` /
local `rfc-index` (cite [feature-branch-pins.md](../../reference/feature-branch-pins.md)
once the reviewed tip is the citation rev).

## Problem

Steps 36–39 productized and verified:

1. Payer funder unlinkability (`PseudonymousFunding` vaults, pre-shield, shielded
   vault and stream operations).
2. Provider receiving privacy (shielded claims to private receiving accounts).

LIP-155 already states privacy goals and short funder / receiver sections
(Security and privacy; LEZ account privacy tiers). It does not yet narrate the
privacy-preserving lifecycle at the same depth as transparent vault/stream
lifecycle prose, nor does it make the independence of the two privacy choices
obvious to a reader who has not followed Steps 36–39.

## Scope

In scope:

- Edits to LIP-155 only (`payment-streams.md` in the spec repo).
- Normative and informative prose at the existing LIP abstraction level
  (see [Depth target](#depth-target)).
- Optional pin update in this integration repo after the LIP branch/rev lands
  ([feature-branch-pins.md](../../reference/feature-branch-pins.md), Step 19
  citation convention).

Out of scope for this step:

- Guest / FFI / module / delivery code changes.
- E2E scripts, fixtures, ImageIDs, env flags (`OWNER_PRIVACY`,
  `PROVIDER_PRIVACY`, `E2E_*`).
- logos-docs journey packets (Steps 20 / 22) unless a one-line cross-cite is
  needed after the LIP lands.
- Merging the LIP branch to `main` / live `lip.logos.co` (same optional rule as
  Step 19 unless this step explicitly decides otherwise).

## Depth target

Match Step 19 / current LIP style:

- Prefer roles, intents, invariants, MUST/SHOULD wallet policy, and what is
  visible on chain.
- Prefer one short workflow narrative per privacy goal over instruction grids.
- Keep LEZ binding in the existing LEZ integration / Security sections;
  do not add SPEL, wallet-FFI, or module JSON schemas to the LIP.
- Amounts, public PDAs, and the vault-to-stream graph stay public; identity
  shielding is the privacy surface.

## What should go into the specification

Candidate content (to refine in the decision log before editing):

### 1. Independent privacy choices

State that payer funder unlinkability and provider receiving privacy are
independent product choices:

A vault MAY be `Public` or `PseudonymousFunding`.
Independently, a provider MAY use a public or private account identifier as
`provider_id` (claim destination is that id; see D40.3).

A reader should not infer that shielded vaults imply shielded claims, or the
reverse.

### 2. Funder unlinkability workflow (payer)

At protocol level, narrate the intended path when funder unlinkability is
desired:

1. User obtains a shielded balance under the vault-owner identity
   (pre-shield; outside the payment-streams program).
2. User initializes a `PseudonymousFunding` vault whose owner is that
   nullifier-derived identity.
3. All subsequent vault and stream operations that touch that vault run as
   shielded transactions.
4. Deposit still changes public vault holding; amounts and the
   vault-to-stream graph remain observable.

Retain and, if needed, tighten existing wallet policy:

- Wallets MUST refuse transparent touches of `PseudonymousFunding` vaults.
- The guest records the tier and does not enforce execution mode.

### 3. Provider receiving privacy workflow (payee)

At protocol level, narrate selective privacy. Receiving privacy is optional;
the provider chooses it by which account id it publishes as `provider_id`.
Claim always credits `StreamConfig.provider`; there is no separate payout
address.

When receiving privacy is intended:

1. Provider publishes a private (nullifier-derived) account identifier as
   stream `provider_id` (public in stream state; globally linkable across
   streams that share it).
2. Provider MUST claim accrued funds through shielded transactions that
   credit that same account.
3. Claim amount remains observable via public vault holding / stream accrual
   state; shielding hides destination identity, not amount.

When receiving privacy is not intended, the provider uses a public account
identifier as `provider_id` and claims transparently.
A public claim that somehow targets an NPK-derived `provider_id` is not a
supported opt-out (destination is fixed to `provider_id`; use a public id at
create time instead).

### 4. What shielding does not provide

Keep or slightly expand the existing visibility section so readers do not
over-claim privacy:

- Vault and stream accounts stay public.
- Transparent creation or claim permanently links identities that later
  shielded ops cannot unlink.
- In-protocol owner / `provider_id` identities are linkable across vaults and
  streams that share them; unlinkability is to primary public keys via the
  shielding boundary.
- Timing and amount correlation across the shield boundary are side channels,
  not breaks of the nullifier scheme (informative, short).

### 5. Deposit / claim composition (high level only)

If the LEZ mapping section still reads as transparent-only deposit:

- Note that deposit composes payment-streams with the platform transfer path.
- Note that privacy-preserving deposit and claim MAY involve shielded
  multi-program execution — without SPEL or FFI detail.

### 6. Cross-links

Point Theory / vault initialize privacy fields and Security subsections at
each other so the privacy workflow is discoverable from both the lifecycle
and the privacy chapter.

## What should stay out of the specification

Do not put these in LIP-155 (they belong in this repo, journeys, or
implementation docs):

| Keep out | Belongs in |
| --- | --- |
| `OWNER_PRIVACY` / `PROVIDER_PRIVACY` / Make targets | E2E.md, verification-matrix |
| Module `chainAction` JSON, resolve paths, hex/base58 | module / integration-contracts |
| Wallet FFI names, logoscore IPC, prove timeouts | LEZ / operator docs |
| ImageIDs, guest ELF freeze, testnet gate logs | Steps 38–39 packets |
| Exact private-account identifier allocation strategy | module / User Journey hygiene notes |
| Store tag-30 wire or N8 byte layouts | existing Off-chain / LEZ bytes sections |

## Suggested LIP edit surfaces

Working tip: [logos-lips#397](https://github.com/logos-co/logos-lips/pull/397)
on `docs/payment-streams-private-execution` (ready for review).
Parallel with [Step 41](step-41-non-native-token-policy-spec.md)
`#379`; neither blocks the other.

| Section | Likely change |
| --- | --- |
| Security and privacy → Privacy goals | Keep; make independence of the two goals explicit. |
| Security and privacy → Funder unlinkability | Keep / tighten short workflow (pre-shield → init → shielded ops). |
| Security and privacy → Provider receiving privacy | Selective privacy; conditional MUST when private `provider_id` (D40.3). |
| Security and privacy → LEZ visibility / Residual linkage | Keep shape for now (D40.4). |
| LEZ integration → Private execution mapping | Keep subsection for now (D40.4); trim only if review finds excess impl detail. |
| On-chain vault initialize | Ensure privacy-tier attach language still points at Security. |

Do not open a parallel “Privacy Protocol” chapter (D40.4).

## Prerequisites

- Steps 36–39 complete (product + local Store privacy + testnet privacy E2E).
- Prefer freezing normative wording from “verified behavior” against the
  Step 39 tip (ImageID `072a26cc…`); publication pin SHOULD cite a tip that
  matches verified privacy v1.
- LIP work on `docs/payment-streams-private-execution` rebased onto current
  `logos-lips` / `rfc-index` `master` (D40.2).

## Decision log

| Id | Topic | Status |
| --- | --- | --- |
| D40.1 | Packet ownership | Closed — new Step 40; do not reopen Step 19. |
| D40.2 | Spec target branch | Closed — `docs/payment-streams-private-execution` / [logos-lips#397](https://github.com/logos-co/logos-lips/pull/397); parallel with `#379` (neither blocks; later rebase if the other merges first). |
| D40.3 | Receiver privacy strength | Closed — selective privacy. Optional. Private `provider_id` ⇒ claim MUST be shielded to that account (destination = `StreamConfig.provider`). Public `provider_id` ⇒ transparent claim. No separate public payout for an NPK-derived `provider_id`. |
| D40.4 | Workflow placement | Closed for now — keep Security expand + LEZ `Private execution mapping`; no new top-level chapter. Revisit depth only if review finds excess impl detail. |
| D40.5 | Publication / step close | Closed — Step 40 DoD is human review that the LIP change is finished (merged to `master`, or otherwise accepted as the cited tip). Agent does not self-close. |
| D40.6 | Integration-repo pin update | Closed — required when the LIP rev is the cited tip; cite in feature-branch-pins. |
| D40.7 | Journey doc follow-up | Closed — out of scope unless a single cite is needed; do not rewrite USER_JOURNEY in this step. |

## Definition of done

- LIP-155 text on the chosen branch reflects both privacy-preserving workflows
  at Security / LEZ depth (D40.3–D40.4), without FFI/module/E2E detail.
- Decision log closed for D40.1–D40.7.
- Human review (D40.5) confirms the tip is finished (on `master` or otherwise
  the accepted citation tip); then this packet moves to `completed/`.
- This integration repo pins the new rev when it is the citation tip (D40.6).
- No code or E2E changes required for close.

## Verification

Docs-only: link integrity in the LIP file; peer read against Steps 36–37
outcome tables (what is public vs shielded). No `make verify-*` gate.
Human close per D40.5.

## Related

- [step-19-lip155-onchain-spec.md](../completed/step-19-lip155-onchain-spec.md)
- [step-36-payer-funder-unlinkability.md](../completed/step-36-payer-funder-unlinkability.md)
- [step-37-payee-receiver-privacy.md](../completed/step-37-payee-receiver-privacy.md)
- [step-39-testnet-privacy-e2e.md](../completed/step-39-testnet-privacy-e2e.md)
- [step-41-non-native-token-policy-spec.md](step-41-non-native-token-policy-spec.md)
  (parallel LIP track; reconcile branches in D41.2)
- [logos-lips#397](https://github.com/logos-co/logos-lips/pull/397)
- [feature-branch-pins.md](../../reference/feature-branch-pins.md)
