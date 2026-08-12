# Step 46 — docs unify and forum post

Upcoming. Index: [index.md](../index.md).

Prerequisite [Step 45](../completed/step-45-dependencies-and-patches.md) is complete.
[Step 48](../waiting/step-48-program-graph-lez-unify.md) is waiting on an upstream
trigger and is not a prerequisite.
Reproduction docs and dogfood assume the post-45 tree.
Document AT hex / split-era config as it exists after 45; when Step 48 later
completes, re-check testnet sections and re-run dogfood legs (D46.13).

## Goal

Unify in-repo documentation around a clear audience split, then publish a forum
progress report that orients readers and links into that documentation.

Drop living “journey” branding (User Journey, Developer Journey, E2E.md as a
standalone brand). Keep the N18 conceptual split (protocol vs Store eligibility)
in prose and in two reproduce paths.

Formal logos-docs Developer Journey publish remains wontfix
([Step 20](../wontfix/step-20-developer-journey.md),
[logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)).
Step 46 does not revive it.

## Audiences and what belongs where

| Surface | Audience | Job | Does not contain |
| --- | --- | --- | --- |
| Root [README.md](../../../README.md) | First-time visitors, operators, contributors | What this repo is; install / prerequisites (section first; split out only if too long); Testing recipes (handles below); map to docs | Full walkthroughs; LIP rewrite; forum narrative; pasted ImageID / `program_id_hex` |
| `docs/reproduce/payment-streams.md` | Anyone reproducing LIP-155 via the module (no Store) | Manual teaching path (testnet primary); private notes + short callouts | Store wire; dual-host Store ops; pasted program identity hex |
| `docs/reproduce/store-eligibility.md` | Anyone reproducing paid Store + eligibility | Orchestrated happy path (`e2e.sh`, local primary); private notes + short callouts | Protocol-only novel; living manual dual-host twin; pasted program identity hex |
| `docs/integrate/eligibility.md` | Authors of request–response protocols (Store-like) | High-level eligibility integration approach; fixed pointer list | Full integration tutorial; full command dump; pasted program identity hex |
| `docs/README.md` | Doc navigators | Map of docs pillars and the surfaces above; Verify collapses to links only (README Testing + verification-matrix) | Command encyclopedia; duplicate Verify command block |
| [verification-matrix.md](../../reference/verification-matrix.md) | Maintainers | Flag detail, artifacts, make aliases, cold-start depth; on-chain confirmation principle; MODE labels without journey names | Human walkthrough prose |
| Forum post | Logos / AnonComms readers (technically able, not PS-specialists) | Progress report; plain-English protocol and eligibility tracks; links into repo | Command novel; spoiler SSOT; verification-matrix as a primary link |

N18 preserved as two reproduce docs plus distinct plain-English tracks in the forum.
Not preserved as journey packet names or logos-docs journey genre.

## Information architecture

```text
README.md                          product front door + install + Testing recipes
docs/
  README.md                        doc map (Verify = links only)
  reproduce/
    payment-streams.md             protocol-only SSOT (manual; depth ≈ old USER_JOURNEY)
    store-eligibility.md           Store eligibility SSOT (orchestrator-primary; absorbs E2E.md)
  integrate/
    eligibility.md                 high-level protocol-integrator pointers
  reference/                       contracts, decisions, verification-matrix (keep; relabel)
  plan/                            step packets (this file lives here)
```

### Migration of legacy living surfaces

After content moves, leave a five-line redirect stub at each retired path
(no new content under `docs/journeys/`). Stubs keep external GitHub / logos-docs
links alive.

| Legacy | Disposition |
| --- | --- |
| `docs/journeys/USER_JOURNEY.md` | Fold into `reproduce/payment-streams.md`; redirect stub |
| `docs/journeys/DEVELOPER_JOURNEY.md` | Fold high-level recipe into `integrate/eligibility.md`; Store run into `store-eligibility.md`; redirect stub |
| `docs/journeys/E2E.md` | Merge recipes into `store-eligibility.md` + scripts README; move “On-chain confirmation principle” into verification-matrix (normative home for both orchestrators); redirect stub |
| `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` | Fold deltas into each reproduce doc’s Private execution notes; redirect stub; no fourth living path |
| `docs/store-integration/README.md` | Redirect stub to `reproduce/store-eligibility.md` + `integrate/eligibility.md` |
| `docs/journeys/slides/` | Historical presentation assets; keep under archive or leave in place with a one-line README that they are point-in-time and may use retired journey names; not living operator docs |
| `docs/presentation.md` | Point-in-time historical; keep wording; not a living front door |
| `docs/handoff-deposit-zero-instruction.md` | Point-in-time historical; keep wording |
| `docs/payment-streams-module/README.md` | Living pillar — sweep journey links and “UJ” legend to reproduce / integrate names |
| `docs/reference/naming-conventions.md` | Living — retitle / reword “MODE values vs Journey names”; drop living journey links |

Archive and completed plan packets may keep historical “journey” wording.

### Reproduce docs (shared rules)

- Doc shapes differ by design:
  - `payment-streams.md` — manual teaching path (`logoscore call` / helpers), depth ≈ old
    USER_JOURNEY. “Aligned with automation” means the same phase names and order as
    `scripts/module-e2e.sh` so maintainers can diff walkthrough vs automated run.
  - `store-eligibility.md` — orchestrator-primary
    (`MODE=store ./scripts/e2e.sh local|testnet run`). No living manual dual-host twin;
    archive runbook link only for debugging.
- Network primary is per-doc (D46.8):
  - `payment-streams.md` — testnet primary (matches folded USER_JOURNEY).
  - `store-eligibility.md` — local primary; testnet as real-network follow-up
    (matches folded store-integration presentation; avoids first-run on shared
    testnet keys as the only path).
- One unified public happy-path scenario per doc.
- Program identity citation rule: never paste ImageID / `program_id_hex` into
  reproduce or integrate prose. Cite the root README “Public testnet guest program”
  section and `fixtures/*.json` (Step 45 sweep rationale).
- Cross-link (N18): each reproduce doc links the other for the other track;
  do not duplicate Store steps in the protocol doc or protocol lifecycle novels
  in the Store doc.
- Privacy structure (D46.6):
  - End-of-doc subsection “Private execution notes” holds the full delta
    (private accounts, pre-shield, PP deposit, `s:` prefix, `amount_le16_hex`,
    shared-seed session, shielded claim / `vault_holding` drop, privacy flags).
  - Per-step inline callouts are at most one or two lines pointing at that subsection.
  - No shared third privacy file (that would reborn a fourth journey).
- Step 47 terminology (`owner` / `provider` / `userPeerId`); run
  [check-terminology.sh](../../../scripts/check-terminology.sh) before publish
  (paths updated for `docs/reproduce/` and `docs/integrate/`).

### Integrate doc

High-level only. Write against post-45 names
(`storeQueryWithEligibility` / `storeQueryWithEligibilityCompleted` per D45.13;
do not propagate pre-rename event names from DEVELOPER_JOURNEY.md).

Fixed pointer list (do not expand ad hoc):

- Five-step altitude from Step 35 / DEVELOPER_JOURNEY: canonical bytes, wire codec,
  module prepare/verify surface, transport hooks, `service_id` / policy
- Canonical request bytes: `n8_canonical_wire_hex` generator (and related tests)
- Named anchors in [integration-contracts.md](../../reference/integration-contracts.md)
- RFC 73 tag 30; LIP-155
- Runnable instance: `reproduce/store-eligibility.md`

No full Store integration novel and no pasted program identity hex.

### Testing documentation (README)

Axes (no “tier” vocabulary in README or matrix support headings):

- Scope: `MODE=module` vs `MODE=store`
- Network: `./scripts/e2e.sh local|testnet …` (sets `CHAIN` internally)
- Privacy: `OWNER_PRIVACY` / `PROVIDER_PRIVACY`

User-facing commands never duplicate network selection
(never `CHAIN=testnet ./scripts/e2e.sh testnet run`).

Stable recipe handles (cite these from forum and reproduce docs):

| Handle | Purpose | Commands |
| --- | --- | --- |
| `fast` | No chain | `cargo clippy --workspace`; `RISC0_DEV_MODE=1 cargo test --workspace`; `make check-terminology` (Step 45 Phase 1). Optional later: `make check-fast` alias wrapping these. |
| `local-public` | Local E2E public | `MODE=module ./scripts/e2e.sh local run`; `MODE=store ./scripts/e2e.sh local run` |
| `local-private` | Local privacy path with stub receipts (`RISC0_DEV_MODE=1` default) | Same with `OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`. Exercises privacy account flow and module IPC; not real proving. |
| `testnet-public` | Public testnet (real network) | `MODE=module ./scripts/e2e.sh testnet run`; `MODE=store ./scripts/e2e.sh testnet run` |
| `testnet-private` | Testnet full privacy with real proving | `RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0` plus full privacy flags; one wrap-up dogfood leg (D46.11). Listed because Step 46 runs it once, not as everyday policy. |

`docs/README.md` Verify block becomes links to root README Testing and the matrix only.

Verification-matrix: relabel Terminology / MODE rows away from journey names in this
step; rename “Support tiers” heading away from banned “tier” wording; host the
on-chain confirmation principle moved from E2E.md.
`scripts/README.md` aligned.
`CHAIN=` sweep covers docs and executable surfaces: Makefile `verify-*` recipes,
`scripts/e2e.sh` `usage()`, `scripts/module-e2e.sh` header.

Maintainer policy (AGENTS / step Done-when): pick handles by risk; do not require
`testnet-private` after every engineering step. After Step 45-class changes:
`local-public` then `testnet-public` before calling the change done.

### Helper script branding

Keep `scripts/user-journey-shell.sh`, `scripts/user-journey-reset.sh`, and
`scripts/lib/user-journey-env.sh` filenames as a stable CLI surface
(D46.14). Living docs call them out as historical names that remain the supported
entry for the manual protocol path (including pinned logoscore/lgpm flake SHAs).
Renaming is out of scope for Step 46.

## Forum post

### Audience and tone

Logos / AnonComms forum readers.
Progress report, not a LIP rewrite.
Plain English; technically able audience unfamiliar with payment streams.

### Outline

1. Intro — payment streams on LEZ; module; Store eligibility as one application;
   wrap-up status; public and private LEZ execution supported (one sentence +
   link to reproduce Private execution notes / Testing recipes). Wording must
   match D46.11 evidence (includes one `testnet-private` wrap-up leg on final pins).
2. As a protocol user (plain English) — fund → vault → deposit → open stream → …
   → close / claim.
3. As a protocol developer (plain English) — eligibility pattern at high level
   (proof on request, verify before serve); not a Store command tutorial.
4. Links — root README (install + Testing); `reproduce/payment-streams.md`;
   `reproduce/store-eligibility.md`; `integrate/eligibility.md`; LIP-155.
   Do not link verification-matrix as a primary forum pointer (maintainer doc).
5. No command spoiler. At most two clone-and-run lines, copied verbatim from
   README Testing recipes (D46.5). Prefer links only when the venue allows.

Draft path (fixed): [step-46-forum-draft.md](step-46-forum-draft.md) beside this
packet until publish; then archive next to the gate log under `completed/`.
Venue TBD at publish time.

## Prerequisites

- Step 45 complete.
- Living module / script surfaces use Step 47 names and post-45 Store event names.
- Step 48 not required; document current AT-config / split-era state (D46.13).

## Sources to fold (substance, not living brands)

| Source | Fold into |
| --- | --- |
| [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md), [Step 34](../completed/step-34-user-journey-manual-walkthrough.md) | `reproduce/payment-streams.md` |
| [E2E.md](../../journeys/E2E.md), [store-integration/README.md](../../store-integration/README.md) | `store-eligibility.md` + scripts README; confirmation principle → matrix |
| [DEVELOPER_JOURNEY.md](../../journeys/DEVELOPER_JOURNEY.md), [Step 35](../completed/step-35-developer-journey-generalization.md) | `integrate/eligibility.md` (high-level + fixed pointers; post-45 names) |
| [PRIVACY_ENHANCED_JOURNEY.md](../../journeys/PRIVACY_ENHANCED_JOURNEY.md), Steps 36–39 | Per-doc Private execution notes |
| LIP-155, contracts, fixtures / README guest section | Citations only |

## Scope

In scope:

- README front door (overview, install section, Testing recipes with handles).
- `docs/README.md` map; Verify → links only.
- Two reproduce docs + one integrate doc as above.
- Redirect stubs for four journey files + store-integration README;
  no new content under `docs/journeys/`.
- Sweep living pillar / reference docs listed in the migration table.
- Relabel verification-matrix Terminology / support heading; move confirmation principle.
- `CHAIN=` dedupe in docs, Makefile `verify-*`, `e2e.sh` usage, `module-e2e.sh` header.
- Update `check-terminology.sh` path lists to `docs/reproduce/` and `docs/integrate/`;
  add living-doc journey-name scan where practical.
- Grep gate: no live script stdout, comment, or doc link to retired paths outside
  allowlisted historical trees (fix `e2e.sh` privacy hint, `run_local_e2e.py` E2E cite,
  `user-journey-reset.sh` banner text, etc.).
- Forum draft at fixed path + publish (or ready-to-post).
- Dogfood + [step-46-gate-log.md](../completed/step-46-gate-log.md) (create when closing).
- Rework plan index tracks table and AGENTS read-order / active-work rows onto new
  surfaces (not only a forum URL note).
- Index / AGENTS forum URL when published.

Out of scope:

- Step 20 logos-docs packet.
- Step 21 UI screenshots unless already shipped.
- Step 48 (waiting); do not block on it.
- New backend features.
- Living manual dual-host Store tutorial.
- Full protocol-integration tutorial in `integrate/eligibility.md`.
- Renaming `user-journey-*.sh` (D46.14).
- Requiring `testnet-private` after every prior engineering step.

## Relationship to Step 20

Step 20 was formal Store / integrator journey publish.
Step 46 replaces that public need with in-repo reproduce + integrate docs and a
forum orientation post with links.
Step 20 stays wontfix.

## Deliver

- Updated root README and `docs/README.md` map (Verify = links only).
- `docs/reproduce/payment-streams.md`
- `docs/reproduce/store-eligibility.md`
- `docs/integrate/eligibility.md`
- Redirect stubs for retired journey + store-integration fronts.
- Living sweeps: payment-streams-module README, naming-conventions MODE section,
  historical classification for presentation / handoff / slides.
- Testing recipes with stable handles; matrix + scripts README aligned;
  `CHAIN=` sweep on Makefile / usage / module-e2e header.
- `check-terminology.sh` path + journey-name living scan updates.
- Script/doc grep gate for retired paths.
- Forum draft at `docs/plan/upcoming/step-46-forum-draft.md`; published post.
- `docs/plan/completed/step-46-gate-log.md` with artifact paths and ImageID
  (cite README / fixtures; do not paste hex into reproduce prose).
- Plan index tracks section reworked; AGENTS read-order and active-work flipped
  to new surfaces; forum URL note when published.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D46.1 | Packet ownership | Docs unify + forum post; after Step 45 only (48 waiting) |
| D46.2 | Repo vs forum | Repo owns reproduction and integrate pointers; forum owns orientation and links |
| D46.3 | Two reproduce paths | Protocol (manual) vs Store (orchestrator); N18 split without journey names |
| D46.4 | Integrate doc | High-level + fixed pointer list; post-45 Store event names; not a full tutorial |
| D46.5 | Commands in forum | Links preferred; at most two lines verbatim from README Testing; no spoiler SSOT |
| D46.6 | Privacy in docs | Per-doc “Private execution notes” + short inline pointers; no shared fourth privacy doc |
| D46.7 | Store run shape | `e2e.sh` primary; no living manual dual-host twin |
| D46.8 | Network primary | Per-doc: testnet for `payment-streams.md`; local for `store-eligibility.md` (testnet follow-up) |
| D46.9 | Install | README section first; split only if too long |
| D46.10 | Testing docs | Handles `fast` / `local-public` / `local-private` / `testnet-public` / `testnet-private`; no “tier”; no user-facing `CHAIN=` duplication; matrix relabeled in this step |
| D46.11 | Dogfood | `local-private` (module + store; stub receipts, `RISC0_DEV_MODE=1`) + `testnet-public` (module + store) + one wrap-up `testnet-private` leg (`MODE=store`, `RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0`) on final post-45 pins; gate log records artifacts and ImageID |
| D46.12 | Journey names | Remove from living docs and nav; redirect stubs; archive / historical files may keep wording |
| D46.13 | Step 48 | Not a prerequisite; ship against post-45 AT-config / split-era; re-dogfood when 48 completes |
| D46.14 | `user-journey-*.sh` | Keep filenames as stable CLI; document historical names in reproduce docs |
| D46.15 | Program identity | No ImageID / `program_id_hex` in reproduce or integrate prose; cite README guest section + fixtures |
| D46.16 | Redirects | Five-line stubs for four journey files + store-integration README; no new journeys content |
| D46.17 | Forum draft path | `docs/plan/upcoming/step-46-forum-draft.md` until publish |

## Done when

- IA above is in place; retired fronts are redirect stubs only; no new
  `docs/journeys/` content.
- README has overview, install (or link), Testing recipes with handles, and links
  into reproduce / integrate; `docs/README.md` Verify is links-only.
- Verification-matrix Terminology / support heading relabeled; confirmation
  principle lives there; `CHAIN=` deduped on listed executable surfaces.
- `check-terminology.sh` covers new trees; living journey-name scan in place.
- Grep gate clean for retired path references in live scripts/docs.
- Index tracks table and AGENTS surfaces reworked off living journey nav.
- Dogfood recorded in `docs/plan/completed/step-46-gate-log.md`:
  - `local-private`: module + store
  - `testnet-public`: module + store
  - `testnet-private`: one Store wrap-up leg with real proving
- Forum draft at the fixed path; post published or ready-to-post (outline above).
- Step 20 remains wontfix.
- Terminology gate clean for touched living prose.
