# Development map

Step program, historical runbooks, and agent packets. External product documentation starts at
the [repository README](../../README.md) and the three pillars; this tree is for maintainers and
agents who need step status or legacy operator detail.

## Start here

| Document | Role |
| --- | --- |
| [program-index.md](program-index.md) | Step map, delivery forks, verify table |
| [../plan/README.md](../plan/README.md) | Completed / upcoming / cancelled packets |
| [../../AGENTS.md](../../AGENTS.md) | Active step and read order |

Product verification gates: [verification-matrix.md](../reference/verification-matrix.md) (not step DoD scripts).

## Historical runbooks

Step-scoped operator files remain under `docs/` for reference:

| Area | Examples |
| --- | --- |
| Fixture and wallet | [archive/steps/local-chain-fixture.md](../archive/steps/local-chain-fixture.md), [archive/steps/wallet-runtime-runbook.md](../archive/steps/wallet-runtime-runbook.md) |
| Module chain I/O | [archive/steps/module-chain-reads-runbook.md](../archive/steps/module-chain-reads-runbook.md) through [archive/steps/wallet-510-runbook.md](../archive/steps/wallet-510-runbook.md) |
| Eligibility (not external product path) | [archive/steps/user-eligibility-runbook.md](../archive/steps/user-eligibility-runbook.md), [archive/steps/provider-eligibility-runbook.md](../archive/steps/provider-eligibility-runbook.md) |
| Store E2E detail | [archive/steps/local-store-dual-host-runbook.md](../archive/steps/local-store-dual-host-runbook.md), [archive/steps/public-sequencer-store-runbook.md](../archive/steps/public-sequencer-store-runbook.md) |
| Runtime install spine | [logos-runtime-guide.md](../archive/steps/logos-runtime-guide.md) |
| Discovery / policy | [archive/steps/scaffold-rpc-findings.md](../archive/steps/scaffold-rpc-findings.md), [archive/steps/policy-implementor-notes.md](../archive/steps/policy-implementor-notes.md), [archive/steps/universal-legacy-probe-results.md](../archive/steps/universal-legacy-probe-results.md) |

## Recovery

[archive/operator/localnet-recovery.md](../archive/operator/localnet-recovery.md)

## Legacy automation

Historical definition-of-done shell drivers live under `scripts/archive/`. Some `make verify-step10a`
through `verify-step13` targets still invoke them for maintainer regression. External runbooks do
not link to archive paths; use [verification-matrix.md](../reference/verification-matrix.md) for supported gates.

Flow A implementation: [scripts/module-e2e-local.sh](../../scripts/module-e2e-local.sh) (not archive).

## Machine manifest

[context-manifest.json](../context-manifest.json)
