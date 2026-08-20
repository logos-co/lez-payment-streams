# Step 51 — forum post

Index: [index.md](../index.md). Status: complete (2026-08-20).
Gate log: [step-51-gate-log.md](step-51-gate-log.md).

Published:
[Payment streams on LEZ](https://forum.research.logos.co/t/payment-streams-on-lez/725)
(2026-08-20).

Spun out of [Step 46](step-46-docs-unify-and-forum-post.md)
so living-docs IA can close without waiting on outbound publish.
Prerequisites: Step 46 (reproduce / integrate / README Testing) and
[Step 52](step-52-wrap-up-verification.md)
(wrap-up matrix on the wrap-up ImageID).
[Step 50](step-50-consistency-and-clarity.md) is already in
the tree that Step 52 ran against.

Draft: [forum-post.md](../../external/forum-post.md).
Stays under `docs/external/` after publish (D46.17).

## Goal

Publish a forum progress report that orients readers and links into the
in-repo documentation. Fill wrap-up claims only from the Step 52 gate log.

## Audience and tone

Logos / AnonComms forum readers.
Progress report.
Plain English; technically able audience unfamiliar with payment streams.

## Outline (from D46)

1. Intro — payment streams on LEZ; module; Store eligibility as one application;
   wrap-up status; public and private LEZ execution supported (one sentence +
   link to reproduce Private execution notes / Testing recipes).
   Fill wrap-up claims from
   [step-52-gate-log.md](step-52-gate-log.md).
2. As a protocol user (plain English) — fund → vault → deposit → open stream → …
   → close / claim.
3. As a protocol developer (plain English) — eligibility pattern at high level
   (proof on request, verify before serve).
4. Links — root README (install + Testing); `reproduce/payment-streams.md`;
   `reproduce/store-eligibility.md`; `integrate/eligibility.md`; LIP-155.
5. At most two clone-and-run lines, copied verbatim from
   README Testing recipes (D46.5). Prefer links only when the venue allows.

Venue:
[Payment streams on LEZ](https://forum.research.logos.co/t/payment-streams-on-lez/725).

## Scope

- Finalize `docs/external/forum-post.md` from Step 52 results.
- Publish at
  [Payment streams on LEZ](https://forum.research.logos.co/t/payment-streams-on-lez/725).
- Index / AGENTS carry that URL.
- Gate log with the forum URL and a pointer to the Step 52 artifacts.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D51.1 | Dogfood | Moved to Step 52 as D52.1. |
| D51.2 | Timing | Wrap-up claims wait for the Step 52 gate log. |
| D51.3 | Commands | Inherit D46.5. |
| D51.4 | Path | Inherit D46.17. |
| D51.5 | Venue | [Payment streams on LEZ](https://forum.research.logos.co/t/payment-streams-on-lez/725) (2026-08-20). Recorded in the gate log, index, and AGENTS. |

## Done when

- Forum draft TODOs replaced with claims that match the Step 52 gate log.
- Post published at
  [Payment streams on LEZ](https://forum.research.logos.co/t/payment-streams-on-lez/725).
- Index and AGENTS note that URL.

## Related

- [step-46-docs-unify-and-forum-post.md](step-46-docs-unify-and-forum-post.md)
- [step-52-wrap-up-verification.md](step-52-wrap-up-verification.md)
- [forum-post.md](../../external/forum-post.md)
- [README Testing](../../../README.md#testing)
