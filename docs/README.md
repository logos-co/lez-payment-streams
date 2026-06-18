# Payment streams integration docs

Operator runbooks (`step*.md`, `logos-runtime-guide.md`) are the commands and verify scripts
to run today. [`integration-index.md`](../integration-index.md) is the short step map and status.
[`integration-contracts.md`](integration-contracts.md) and
[`reference/decisions-and-notes.md`](reference/decisions-and-notes.md) hold cross-step rules.
Agents: [`AGENT-BRIEF.md`](AGENT-BRIEF.md) first; repo root [`AGENTS.md`](../AGENTS.md) for tooling entry.

When a runbook and the index disagree, update the runbook and align the index — avoid duplicating
long procedural blocks in both places.

| File | Use |
| --- | --- |
| [`../AGENTS.md`](../AGENTS.md) | Repo-root agent context (vendor-neutral) |
| [`AGENT-BRIEF.md`](AGENT-BRIEF.md) | Agent read order and active-step packets |
| [`integration-contracts.md`](integration-contracts.md) | LogosAPI names, encodings, wire tags |
| [`context-manifest.json`](context-manifest.json) | Machine read-order manifest |
| [`../integration-index.md`](../integration-index.md) | Master index (steps, status, verify scripts) |
| [`../integration-plan.md`](../integration-plan.md) | Redirect to integration index |
| [`archive/implementation-plan-on-chain.md`](archive/implementation-plan-on-chain.md) | Archived on-chain SPEL guest milestone list |
| [`plan/README.md`](plan/README.md) | Completed vs upcoming plan excerpts |
| [`../logos-architecture-overview.md`](../logos-architecture-overview.md) | Hosts, modules, FFI vs LogosAPI |
| [`logos-runtime-guide.md`](logos-runtime-guide.md) | Build `.lgx`, install, logoscore, Steps 7/9/11a–13 loop |
| [`step1-findings-scaffold-rpc.md`](step1-findings-scaffold-rpc.md) | LEZ localnet, deploy, scaffold RPC findings |
| [`step10a-local-chain-fixture.md`](step10a-local-chain-fixture.md) | Step 10a seed script, manifest, reset, wallet re-init |
| [`step10a-handoff-and-follow-up.md`](step10a-handoff-and-follow-up.md) | Step 10a verify failures, sequencer log triage |
| [`step10b-wallet-runtime.md`](step10b-wallet-runtime.md) | Step 10b wallet `.lgx`, install, `open`, verify script |
| [`step11a-chain-reads.md`](step11a-chain-reads.md) | Step 11a wallet-backed reads + verify script |
| [`step11b-chain-writes.md`](step11b-chain-writes.md) | Step 11b `chainAction` writes/status + verify script |
| [`step11c-sign-public-payload.md`](step11c-sign-public-payload.md) | Step 11c `sign_public_payload` Rust FFI + Qt patch + verify script |
| [`step11d-wallet-510.md`](step11d-wallet-510.md) | Step 11d LEZ 510 wallet pin bump, deploy FFI, verify script |
| [`step12-user-eligibility.md`](step12-user-eligibility.md) | Step 12 user eligibility (`verify-step12-dod.sh`) |
| [`step13-provider-eligibility.md`](step13-provider-eligibility.md) | Step 13 provider verify (`verify-step13-dod.sh`) |
| [`step17-e2e-local.md`](step17-e2e-local.md) | Step 17 dual-host Store E2E (`make verify-step17`; hermetic run section) |
| [`step3-policy-and-implementor-notes.md`](step3-policy-and-implementor-notes.md) | Step 3a policy and implementor detail |
| [`step8-universal-legacy-probe-results.md`](step8-universal-legacy-probe-results.md) | Step 8 probe results (+ historical dilemma appendix) |
| [`feature-branch-pins.md`](feature-branch-pins.md) | Wallet flake pins (LEZ 510 + PR 19 wrapper) |
| [`demo-localnet-recovery.md`](demo-localnet-recovery.md) | Blank-slate localnet policy |
| [`archive/legacy-module-bootstrap.md`](archive/legacy-module-bootstrap.md) | Superseded Legacy plugin bootstrap |
| [`archive/superseded-wallet-pr-429-16.md`](archive/superseded-wallet-pr-429-16.md) | Deprecated PR 429 / 16 wallet JSON API |
| [`archive/engineering-notes.md`](archive/engineering-notes.md) | Misc engineering notes (FFI, SPEL PDA) |
