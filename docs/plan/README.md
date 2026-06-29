# Integration plan archive layout

Former monolithic integration plan; split into program index, plan packets, and pillar docs to keep agent context small.

| Path | Contents |
| --- | --- |
| [../development-map/program-index.md](../development-map/program-index.md) | Program index — step status |
| [../../AGENTS.md](../../AGENTS.md) | Agent read order |
| [../reference/integration-contracts.md](../reference/integration-contracts.md) | Cross-step APIs |
| [completed/](completed/) | Normative excerpts (12–16), completed Steps 17–17b, 18b, 19, 24, 24b, 24c |
| [upcoming/](upcoming/) | Step 20; optional 21–22 (Track B), 23 |
| [cancelled/](cancelled/) | Step 25 won't fix |
| [../reference/decisions-and-notes.md](../reference/decisions-and-notes.md) | D1–D6, N1–N18 (N17 script orchestration; N18 Track A vs B) |
| [../archive/implementation-plan-on-chain.md](../archive/implementation-plan-on-chain.md) | On-chain SPEL guest milestones (archived) |

Track A (integrator Store integration): Step 20 + Step 17 scripts. Track B (optional payment
streams UI): Steps 21–22 — see [N18](../reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).

Runbooks (`step10a` … `step13`, [store-integration/runbook-localnet.md](../store-integration/runbook-localnet.md), [step17-e2e-local.md](../step17-e2e-local.md)) hold operator commands; product gates in [verification-matrix.md](../verification-matrix.md).
