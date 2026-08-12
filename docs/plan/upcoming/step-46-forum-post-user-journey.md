# Step 46 — forum progress report

Upcoming. Index: [index.md](../index.md).

## Goal

Publish a forum post that reports progress on payment streams work.
Structure:

1. High-level introduction (what this is, why it matters, what is working now).
2. Plain-English involved steps for both tracks (N18):
   - As a user: fund account → create vault → deposit → open stream → … → close / claim.
   - As a developer (Store eligibility): add eligibility fields → set eligibility provider → …
3. Under a spoiler (collapsed detail): a full command list compiled from the current in-repo
   journeys (User Journey and Developer / Store journey), not a second conflicting SSOT.

This is the wrap-up public narrative for both tracks.
Formal logos-docs Developer Journey publish ([Step 20](../wontfix/step-20-developer-journey.md),
[logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)) is wontfix;
the forum post carries the high-level user and developer substance instead of a
standalone logos-docs journey packet.

## Audience and tone

- Primary: Logos / AnonComms forum readers.
- Progress report, not a LIP rewrite. Plain English in the body; commands only in the spoiler.
- Keep user vs developer tracks visibly separate in the plain-English section.

## Proposed outline

1. Intro — payment streams on LEZ; module; Store eligibility as one application; wrap-up status.
2. As a user (plain English numbered steps) — protocol-only path from USER_JOURNEY substance.
3. As a developer (plain English numbered steps) — Store + eligibility composition from
   Developer Journey / store-integration substance.
4. Spoiler — full command list compiled from current journeys (cite sources; keep in sync
   with SSOT when drafting).
5. Pointers — LIP-155, verification matrix / `MODE=module` and Store E2E, in-repo docs hub,
   optional privacy note.

## Sources (in-repo SSOT)

| Source | Use in the post |
| --- | --- |
| [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) | Plain-English user steps + spoiler commands (owner/provider) |
| [Step 22](../completed/step-22-ui-journey.md) / [logos-docs#370](https://github.com/logos-co/logos-docs/issues/370) | User Journey framing |
| [Step 34](../completed/step-34-user-journey-manual-walkthrough.md) | Testnet walkthrough detail for commands |
| [DEVELOPER_JOURNEY.md](../../journeys/DEVELOPER_JOURNEY.md) / [store-integration/README.md](../../store-integration/README.md) | Plain-English developer steps + spoiler Store commands |
| [E2E.md](../../journeys/E2E.md) | Phase / script command alignment for spoiler |
| [PRIVACY_ENHANCED_JOURNEY.md](../../journeys/PRIVACY_ENHANCED_JOURNEY.md) | Optional short privacy callout in intro or appendix |
| LIP-155, [docs/README.md](../../README.md) | Citations only; do not duplicate protocol semantics |
| [check-terminology.sh](../../scripts/check-terminology.sh) | Before publish: prefer Step 47 final names (`owner`/`provider`/`userPeerId`); rerun the gate |

## Scope

In scope:

- Progress-report intro and status as of wrap-up.
- Plain-English user and developer step lists (numbered, non-command).
- Spoiler with compiled full commands from current journeys.
- Venue TBD at execution time; draft may live under `docs/` until published.

Out of scope:

- Publishing a standalone logos-docs Developer Journey packet (Step 20 wontfix).
- Step 21 UI screenshots unless already shipped (additive later).
- New backend features; inventing commands not present in journey SSOT.
- Expanding the open body into a full dual-host Store tutorial
  (that detail stays in the spoiler and in-repo Store SSOT).

## Relationship to Step 20

Step 20 (wontfix) was the formal Store / integrator journey publish track
([logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)).
Step 46 is the public wrap-up: plain-English user and developer steps plus a
command spoiler compiled from in-repo journeys.
Step 46 does not revive Step 20 as a logos-docs packet.

## Deliver

- Draft post (in-repo until published).
- Published post (or ready-to-post markdown) matching the outline above.
- Short note in plan index / AGENTS with URL when published.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D46.1 | Packet ownership | New Step 46; wrap-up forum progress report. |
| D46.2 | Body vs spoiler | Plain English in the open post; full commands only under spoiler. |
| D46.3 | Both tracks | User and developer plain-English lists both in the body; N18 split preserved. |
| D46.4 | Command source | Compile from current journeys; do not fork a third walkthrough SSOT. |

## Done when

- Forum post published or checked in as ready-to-post.
- Intro + plain-English user and developer steps present.
- Spoiler carries the compiled command list from current journeys.
- Step 20 remains wontfix (no separate logos-docs Developer Journey publish).
