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
| [completed/](completed/) | Normative excerpts (12-16), completed Steps 17-19, 22, 24, 24b, 24c, 26-41 |
| [upcoming/](upcoming/) | Steps 20–21, 44–46 |
| [waiting/](waiting/) | (empty) |
| [wontfix/](wontfix/) | Not near-term; may return to upcoming — Steps 23, 25, 42, 43 |
| [../archive/completed-steps-index.md](../archive/completed-steps-index.md) | Full step map, completed summaries, verify scripts |
| [../reference/integration-contracts.md](../reference/integration-contracts.md) | Cross-step APIs |
| [../reference/integration-decisions.md](../reference/integration-decisions.md) | D1-D6, N1-N18 |

`waiting/` — blocked on external input before the step can progress.
`wontfix/` — decided against implementing near-term; may move to `upcoming/` if resurrected.

Developer Journey (Store integration): Step 20 upcoming (Store publish;
[logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)); Step 17 scripts remain.
Developer Journey generalization: Step 35 (complete).
User Journey (CLI / module): complete —
[logos-docs#370](https://github.com/logos-co/logos-docs/issues/370); Steps 22, 28, 34.
Forum progress report: Step 46 (upcoming).
User Journey UI: Step 21 (upcoming). Public hosted Store: Step 23 (wontfix).
Wrap-up polish: Steps 44–45 (payer and payee close; deps/patches).

Runbooks (`step10a` … `step13`, [store-integration/README.md](../store-integration/README.md), [archive/steps/local-store-dual-host-runbook.md](../archive/steps/local-store-dual-host-runbook.md)) hold operator commands; product gates in [verification-matrix.md](../reference/verification-matrix.md).
