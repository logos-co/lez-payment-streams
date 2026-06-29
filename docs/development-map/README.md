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

Product verification gates: [verification-matrix.md](../verification-matrix.md) (not step DoD scripts).

## Historical runbooks

Step-scoped operator files remain under `docs/` for reference:

| Area | Examples |
| --- | --- |
| Fixture and wallet | [step10a-local-chain-fixture.md](../step10a-local-chain-fixture.md), [step10b-wallet-runtime.md](../step10b-wallet-runtime.md) |
| Module chain I/O | [step11a-chain-reads.md](../step11a-chain-reads.md) through [step11d-wallet-510.md](../step11d-wallet-510.md) |
| Eligibility (not external product path) | [step12-user-eligibility.md](../step12-user-eligibility.md), [step13-provider-eligibility.md](../step13-provider-eligibility.md) |
| Store E2E detail | [step17-e2e-local.md](../step17-e2e-local.md), [step18-public-sequencer-e2e.md](../step18-public-sequencer-e2e.md) |
| Runtime install spine | [logos-runtime-guide.md](../logos-runtime-guide.md) |
| Discovery / policy | [step1-findings-scaffold-rpc.md](../step1-findings-scaffold-rpc.md), [step3-policy-and-implementor-notes.md](../step3-policy-and-implementor-notes.md), [step8-universal-legacy-probe-results.md](../step8-universal-legacy-probe-results.md) |

## Recovery

[demo-localnet-recovery.md](../demo-localnet-recovery.md)

## Legacy automation

Historical definition-of-done shell drivers live under `scripts/archive/`. Some `make verify-step10a`
through `verify-step13` targets still invoke them for maintainer regression. External runbooks do
not link to archive paths; use [verification-matrix.md](../verification-matrix.md) for supported gates.

Flow A implementation: [scripts/module-e2e-local.sh](../../scripts/module-e2e-local.sh) (not archive).

## Machine manifest

[context-manifest.json](../context-manifest.json)
