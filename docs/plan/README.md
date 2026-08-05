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
| [completed/](completed/) | Normative excerpts (12-16), completed Steps 17-19, 22, 24, 24b, 24c, 26-39 |
| [upcoming/](upcoming/) | Steps 40, 42–43 |
| [waiting/](waiting/) | Waiting on external input — Step 20 ([logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)); Step 41 ([logos-lips#379](https://github.com/logos-co/logos-lips/pull/379)) |
| [wontfix/](wontfix/) | Not near-term; may return to upcoming — Steps 21, 23, 25 |
| [../archive/completed-steps-index.md](../archive/completed-steps-index.md) | Full step map, completed summaries, verify scripts |
| [../reference/integration-contracts.md](../reference/integration-contracts.md) | Cross-step APIs |
| [../reference/integration-decisions.md](../reference/integration-decisions.md) | D1-D6, N1-N18 |

`waiting/` — blocked on external input before the step can progress.
`wontfix/` — decided against implementing near-term; may move to `upcoming/` if resurrected.

Developer Journey (Store integration): Step 20 waiting on
[logos-docs#369](https://github.com/logos-co/logos-docs/issues/369); Step 17 scripts remain.
Developer Journey generalization: Step 35 (complete).
User Journey (CLI / module): complete —
[logos-docs#370](https://github.com/logos-co/logos-docs/issues/370); Steps 22, 28, 34.
User Journey UI: Step 21 (wontfix). Public hosted Store: Step 23 (wontfix).

Runbooks (`step10a` … `step13`, [store-integration/README.md](../store-integration/README.md), [archive/steps/local-store-dual-host-runbook.md](../archive/steps/local-store-dual-host-runbook.md)) hold operator commands; product gates in [verification-matrix.md](../reference/verification-matrix.md).
