# Integration plan archive layout

Split from the monolithic `integration-plan.md` to keep agent context small.

| Path | Contents |
| --- | --- |
| [../integration-index.md](../../integration-index.md) | Short index — start here for step status |
| [../AGENT-BRIEF.md](../AGENT-BRIEF.md) | Agent read order |
| [../integration-contracts.md](../integration-contracts.md) | Cross-step APIs |
| [completed/](completed/) | Normative excerpts (12–16), completed Steps 17–17b, 19, 24, 24b |
| [upcoming/](upcoming/) | Step 24c packet (local gate complete); Step 20; Step 18 Part B; optional 21–22 (Track B), 23 |
| [cancelled/](cancelled/) | Step 25 won't fix |
| [../reference/decisions-and-notes.md](../reference/decisions-and-notes.md) | D1–D6, N1–N18 (N17 script orchestration; N18 Track A vs B) |
| [../archive/implementation-plan-on-chain.md](../archive/implementation-plan-on-chain.md) | On-chain SPEL guest milestones (archived) |

Track A (integrator Store integration): Step 20 + Step 17 scripts. Track B (optional payment
streams UI): Steps 21–22 — see [N18](../reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).

Runbooks (`step10a` … `step13`, [step17-e2e-local.md](../step17-e2e-local.md)) hold operator commands; do not duplicate procedures in plan excerpts.
