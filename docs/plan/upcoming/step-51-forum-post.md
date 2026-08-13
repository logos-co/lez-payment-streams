# Step 51 — forum post

Upcoming. Index: [index.md](../index.md).

Spun out of [Step 46](../completed/step-46-docs-unify-and-forum-post.md)
so living-docs IA can close without waiting on outbound publish.
Prerequisite: Step 46 complete (reproduce / integrate / README Testing).
[Step 50](../completed/step-50-consistency-and-clarity.md) is complete, so
Makefile help and persist merge are in the tree the wrap-up runs against.
[Step 48](../wontfix/step-48-program-graph-lez-unify.md) is not a prerequisite.
[Step 21](step-21-basecamp-ui.md) is not a prerequisite.

Draft: [forum-post.md](../../external/forum-post.md).
Stays under `docs/external/` after publish (D46.17).
Gate log: [step-51-gate-log.md](../completed/step-51-gate-log.md)
(create when closing).

## Goal

Publish a forum progress report that orients readers and links into the
in-repo documentation. Fill wrap-up claims only from a recorded matrix on
the current ImageID. Do not invent evidence.

## Audience and tone

Logos / AnonComms forum readers.
Progress report, not a LIP rewrite.
Plain English; technically able audience unfamiliar with payment streams.

## Outline (from D46)

1. Intro — payment streams on LEZ; module; Store eligibility as one application;
   wrap-up status; public and private LEZ execution supported (one sentence +
   link to reproduce Private execution notes / Testing recipes).
   Until D51.1 legs have been run, keep wrap-up claims as
   `TODO: wait for results of local-private / testnet-public / testnet-private`.
2. As a protocol user (plain English) — fund → vault → deposit → open stream → …
   → close / claim.
3. As a protocol developer (plain English) — eligibility pattern at high level
   (proof on request, verify before serve); not a Store command tutorial.
4. Links — root README (install + Testing); `reproduce/payment-streams.md`;
   `reproduce/store-eligibility.md`; `integrate/eligibility.md`; LIP-155.
   Do not link verification-matrix as a primary forum pointer (maintainer doc).
5. No command spoiler. At most two clone-and-run lines, copied verbatim from
   README Testing recipes (D46.5). Prefer links only when the venue allows.

Venue TBD at publish time.

## Wrap-up matrix (D51.1)

This step owns the post-50 protocol matrix that Step 46 called D46.11.
Step 50’s own gate is `fast` only. Record artifacts here (or cite a short
follow-up note) and then drop the forum TODOs.

Handles:

| Handle | Command |
| --- | --- |
| `local-private` module | `MODE=module OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` (`RISC0_DEV_MODE=1`) |
| `local-private` store | `MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` (`RISC0_DEV_MODE=1`) |
| `testnet-public` module | `MODE=module ./scripts/e2e.sh testnet run` |
| `testnet-public` store | `MODE=store ./scripts/e2e.sh testnet run` |
| `testnet-private` store | `RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh testnet run` |

Also run `fast` and `local-public` (module + store) if they were not recorded
after Step 50. Optional extras: module provider-close and close-negatives.

Cite README guest section and fixtures for ImageID.
Do not paste hex into reproduce or forum prose.

If the matrix was already run on the post-50 tree, record those artifacts
instead of re-running.

## Scope

In scope:

- Finalize `docs/external/forum-post.md` from D51.1 results.
- Publish, or leave ready-to-post if venue is still TBD.
- Index / AGENTS forum URL when published.
- Gate log with artifact paths and ImageID citations.

Out of scope:

- Rewriting reproduce / integrate IA (Step 46).
- Step 50 glue (persist, kit, Makefile help).
- Step 21 UI screenshots unless already shipped.
- LIP rewrite. Command encyclopedia. Verification-matrix as a forum link.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D51.1 | Dogfood | Absorbs D46.11. Legs above on the current ImageID after Step 50. Forum draft keeps explicit TODOs until those rows exist. |
| D51.2 | Timing | Draft may be edited in parallel with Step 50. Publish and wrap-up claims wait for Step 50 close plus D51.1. |
| D51.3 | Commands | Inherit D46.5. |
| D51.4 | Path | Inherit D46.17. |
| D51.5 | Venue | TBD at publish time. Record the URL in the gate log, index, and AGENTS. |

## Done when

- D51.1 rows filled in the gate log (or cited from an equivalent post-50 run).
- Forum draft TODOs replaced with claims that match those artifacts, or the
  post is ready-to-post with TODOs removed only where evidence exists.
- Post published or ready-to-post.
- Index and AGENTS note the forum URL when published.
- Step 20 remains wontfix.

## Related

- [step-46-docs-unify-and-forum-post.md](../completed/step-46-docs-unify-and-forum-post.md)
- [step-50-consistency-and-clarity.md](../completed/step-50-consistency-and-clarity.md)
- [forum-post.md](../../external/forum-post.md)
- [README Testing](../../../README.md#testing)
