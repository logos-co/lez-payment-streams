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
| [completed/](completed/) | Normative excerpts (12-16), completed Step 17-17b, 18b, 19, 24, 24b, 24c |
| [upcoming/](upcoming/) | Steps 20, 22, 26-29 (active); optional 21 (UI), 23 |
| [cancelled/](cancelled/) | Step 25 won't fix |
| [../archive/completed-steps-index.md](../archive/completed-steps-index.md) | Full step map, completed summaries, verify scripts |
| [../reference/integration-contracts.md](../reference/integration-contracts.md) | Cross-step APIs |
| [../reference/integration-decisions.md](../reference/integration-decisions.md) | D1-D6, N1-N18 |

Developer Journey (Store integration): Step 20 + Step 17 scripts.
User Journey: Step 22 (CLI doc packet, active).
User Journey UI (optional): Step 21 (Basecamp plugin) — see [N18](../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).

Runbooks (`step10a` … `step13`, [store-integration/README.md](../store-integration/README.md), [archive/steps/local-store-dual-host-runbook.md](../archive/steps/local-store-dual-host-runbook.md)) hold operator commands; product gates in [verification-matrix.md](../reference/verification-matrix.md).
