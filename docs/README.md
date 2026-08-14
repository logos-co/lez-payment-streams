# Documentation

Entry for integrators and maintainers.
Protocol text lives in [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html).
Cite the LIP for semantics.

## Choose a path

| Path | When |
| --- | --- |
| [on-chain](on-chain/) | Guest program, Rust tests, review order |
| [payment-streams-module](payment-streams-module/) | Logos module, `chainAction` catalogue |
| [reproduce/payment-streams.md](reproduce/payment-streams.md) | Manual LIP-155 via the module (testnet primary) |
| [reproduce/store-eligibility.md](reproduce/store-eligibility.md) | Paid Store + eligibility (`e2e.sh`, local primary) |
| [integrate/eligibility.md](integrate/eligibility.md) | Eligibility for a request-response protocol |
| [external/](external/) | Outbound forum draft |
| [plan](plan/) | Program index, plan packets |

## Verify

Testing recipes: [root README Testing](../README.md#testing).
Maintainer flags, artifacts, cold start: [reference/verification-matrix.md](reference/verification-matrix.md).
Scripts: [verify/README.md](../verify/README.md).

## Reference

| Doc | Role |
| --- | --- |
| [integration-contracts.md](reference/integration-contracts.md) | Cross-repo APIs, tag 30 |
| [integration-decisions.md](reference/integration-decisions.md) | Integration decisions |
| [feature-branch-pins.md](reference/feature-branch-pins.md) | Fork branches and flakes |
| [naming-conventions.md](reference/naming-conventions.md) | `MODE` values, Makefile names, [scaffold layout](reference/naming-conventions.md#scaffold-layout) |
| [verification-matrix.md](reference/verification-matrix.md) | Mode × network matrix |

## Archive

Historical step runbooks and operator notes: [archive/](archive/).
Point-in-time notes: [presentation.md](presentation.md), [handoff-deposit-zero-instruction.md](handoff-deposit-zero-instruction.md).

## Maintainers

[AGENTS.md](../AGENTS.md), [plan/](plan/), lifecycle regression: `make verify-store-local-lifecycle`.
