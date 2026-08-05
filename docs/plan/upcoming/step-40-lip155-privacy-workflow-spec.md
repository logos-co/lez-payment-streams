# Step 40 — LIP-155 privacy-preserving workflow in the specification

Index: [index.md](../index.md). Status: **upcoming** (planning;
spec text not started in this step yet).

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
once a Step 40 branch/rev is chosen).

## Problem

Steps 36–39 productized and verified:

1. Payer funder unlinkability (`PseudonymousFunder` vaults, pre-shield, shielded
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

- A vault MAY be `Public` or `PseudonymousFunder`.
- Independently, a provider MAY claim to a public or private receiving account.

A reader should not infer that shielded vaults imply shielded claims, or the
reverse.

### 2. Funder unlinkability workflow (payer)

At protocol level, narrate the intended path when funder unlinkability is
desired:

1. User obtains a shielded balance under the vault-owner identity
   (pre-shield; outside the payment-streams program).
2. User initializes a `PseudonymousFunder` vault whose owner is that
   nullifier-derived identity.
3. All subsequent vault and stream operations that touch that vault run as
   shielded transactions.
4. Deposit still changes public vault holding; amounts and the
   vault-to-stream graph remain observable.

Retain and, if needed, tighten existing wallet policy:

- Wallets MUST refuse transparent touches of `PseudonymousFunder` vaults.
- The guest records the tier and does not enforce execution mode.

### 3. Provider receiving privacy workflow (payee)

At protocol level, narrate the intended path when receiving-address
unlinkability is desired:

1. Stream `provider_id` remains the in-protocol claim authorizer (public in
   stream state; globally linkable across streams that share it).
2. Provider claims accrued funds through shielded transactions to receiving
   addresses not tied to its primary public key.
3. Claim amount remains observable via public vault holding / stream accrual
   state; shielding hides destination identity, not amount.

Clarify SHOULD vs MUST for receiver privacy to match product intent
(today the LIP uses SHOULD for shielded claim).

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

## Suggested LIP edit surfaces (no edits in this planning pass)

| Section | Likely change |
| --- | --- |
| Security and privacy → Privacy goals | Keep; make independence of the two goals explicit. |
| Security and privacy → Funder unlinkability | Expand to a short workflow (pre-shield → init → shielded ops). |
| Security and privacy → Receiver privacy | Expand to a short workflow; resolve SHOULD/MUST. |
| Security and privacy → LEZ visibility | Keep; add one sentence on independent claim destination privacy. |
| LEZ integration → Account types / Deposit path | High-level PP composition note if still transparent-only. |
| On-chain vault initialize | Ensure privacy-tier attach language still points at Security. |

Do not open a parallel “Privacy Protocol” chapter unless a decision says the
Security section cannot hold the workflow.

## Prerequisites

- Steps 36–39 complete (product + local Store privacy + testnet privacy E2E).
- Prefer freezing normative wording from “verified behavior” against the
  Step 39 tip (ImageID `072a26cc…`); publication pin SHOULD cite a tip that
  matches verified privacy v1.
- Step 19 pin / branch convention understood; Step 40 chooses whether to
  amend `feat/payment-streams-onchain-part`, open a new docs branch, or draft
  against current `main` LIP text.

## Decision log (open — discuss before editing)

| Id | Topic | Tentative |
| --- | --- | --- |
| D40.1 | Packet ownership | New Step 40; do not reopen Step 19. |
| D40.2 | Spec target branch | TBD (new docs branch vs amend existing LIP tip). |
| D40.3 | Receiver privacy strength | Keep SHOULD, or raise selected claim-path sentences to MUST when receiving privacy is intended. |
| D40.4 | Workflow placement | Prefer expand Security and privacy; avoid a new top-level chapter. |
| D40.5 | Merge to `main` / lip.logos.co | Optional follow-up, same as Step 19, unless decided otherwise. |
| D40.6 | Integration-repo pin update | Required when LIP rev lands; cite in feature-branch-pins. |
| D40.7 | Journey doc follow-up | Out of scope unless a single cite is needed; do not rewrite USER_JOURNEY in this step. |

## Definition of done

- LIP-155 text updated on the chosen branch/rev so a LEZ-familiar reader can
  follow both privacy-preserving workflows without reading Steps 36–39.
- Depth matches existing Security / LEZ sections (no FFI/module/E2E detail).
- Decision log closed for D40.1–D40.6 (D40.7 optional).
- This integration repo pins the new rev when the LIP change lands.
- No code or E2E changes required for close.

## Verification

Docs-only: link integrity in the LIP file; peer read against Steps 36–37
outcome tables (what is public vs shielded). No `make verify-*` gate.

## Related

- [step-19-lip155-onchain-spec.md](../completed/step-19-lip155-onchain-spec.md)
- [step-36-payer-funder-unlinkability.md](../completed/step-36-payer-funder-unlinkability.md)
- [step-37-payee-receiver-privacy.md](../completed/step-37-payee-receiver-privacy.md)
- [step-39-testnet-privacy-e2e.md](../completed/step-39-testnet-privacy-e2e.md)
- [step-41-non-native-token-policy-spec.md](../waiting/step-41-non-native-token-policy-spec.md)
  (parallel LIP track; reconcile branches in D41.2)
- [feature-branch-pins.md](../../reference/feature-branch-pins.md)
