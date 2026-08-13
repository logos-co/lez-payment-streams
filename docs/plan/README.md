# Integration plan archive layout

Former monolithic integration plan; split into program index, plan packets, and pillar docs to keep agent context small.

The condensed index is at [`index.md`](index.md).
Archived step map and completed summaries moved to
[`../archive/completed-steps-index.md`](../archive/completed-steps-index.md).
This file is retained as a short redirect.

| Path | Contents |
| --- | --- |
| [index.md](index.md) | Program scope, upcoming steps, delivery forks, components, onboarding |
| [../../AGENTS.md](../../AGENTS.md) | Agent read order |
| [completed/](completed/) | Normative excerpts (12-16), completed Steps 17-19, 22, 24, 24b, 24c, 26-41, 44–47, 49 |
| [upcoming/](upcoming/) | Steps 21, 50, 51 |
| [waiting/](waiting/) | Empty |
| [wontfix/](wontfix/) | Not near-term; may return to upcoming — Steps 20, 23, 25, 42, 43, 48 |
| [../archive/completed-steps-index.md](../archive/completed-steps-index.md) | Full step map, completed summaries, verify scripts |
| [../reference/integration-contracts.md](../reference/integration-contracts.md) | Cross-step APIs |
| [../reference/integration-decisions.md](../reference/integration-decisions.md) | D1-D6, N1-N18 |

`waiting/` — blocked on external input before the step can progress.
`wontfix/` — decided against implementing near-term; may move to `upcoming/` if resurrected.

Developer Journey (Store integration): formal logos-docs publish is Step 20 wontfix
([logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)).
Developer Journey generalization: Step 35 (complete).
User Journey (CLI / module): complete as historical track —
[logos-docs#370](https://github.com/logos-co/logos-docs/issues/370); Steps 22, 28, 34.
Docs unify: Step 46 (complete) — README, reproduce paths,
integrate pointers, Testing recipes; drops living “journey” branding.
Forum post: Step 51 (upcoming) — orientation with links into those docs.
Multi-token type alignment (native-only demo): Step 49 (complete).
Consistency and clarity polish: Step 50 (upcoming; after 49).
Protocol UI (Basecamp): Step 21 (upcoming). Public hosted Store: Step 23 (wontfix).
Wrap-up polish: Step 45 complete (deps/patches); Step 48 wontfix (program-graph LEZ
unify + drop AT hex config); Step 44 complete (payer and payee close).

Runbooks ([store-eligibility.md](../reproduce/store-eligibility.md), [archive/steps/local-store-dual-host-runbook.md](../archive/steps/local-store-dual-host-runbook.md)) hold operator commands. Product gates in [verification-matrix.md](../reference/verification-matrix.md).
