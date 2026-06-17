# Payment streams integration docs

Operator runbooks (`step*.md`, `logos-runtime-guide.md`) are the commands and verify scripts
to run today. [`../integration-plan-v2.md`](../integration-plan-v2.md) is the full plan
(decisions, DoD, architecture); when they disagree, update the runbook and align the plan —
avoid duplicating long procedural blocks in both places.

| File | Use |
| --- | --- |
| [`../logos-architecture-overview.md`](../logos-architecture-overview.md) | Hosts, modules, FFI vs LogosAPI (repo root) |
| [`../integration-plan-v2.md`](../integration-plan-v2.md) | Master plan (Steps 10–11: fixture, wallet, module chain I/O) |
| [`logos-runtime-guide.md`](logos-runtime-guide.md) | Build `.lgx`, install, logoscore, Steps 7/9/11a–13 loop |
| [`step1-findings-scaffold-rpc.md`](step1-findings-scaffold-rpc.md) | LEZ localnet, deploy, scaffold RPC findings |
| [`step10a-local-chain-fixture.md`](step10a-local-chain-fixture.md) | Step 10a seed script, manifest, reset, wallet re-init |
| [`step10a-handoff-and-follow-up.md`](step10a-handoff-and-follow-up.md) | Step 10a verify failures, sequencer log triage |
| [`step10b-wallet-runtime.md`](step10b-wallet-runtime.md) | Step 10b wallet `.lgx`, install, `open`, verify script |
| [`step11a-chain-reads.md`](step11a-chain-reads.md) | Step 11a wallet-backed reads + verify script |
| [`step11b-chain-writes.md`](step11b-chain-writes.md) | Step 11b `chainAction` writes/status + verify script |
| [`step11c-sign-public-payload.md`](step11c-sign-public-payload.md) | Step 11c `sign_public_payload` Rust FFI + Qt patch + verify script |
| [`step3-policy-and-implementor-notes.md`](step3-policy-and-implementor-notes.md) | Step 3a policy and implementor detail |
| [`step8-universal-legacy-probe-results.md`](step8-universal-legacy-probe-results.md) | Step 8 probe results (+ historical dilemma appendix) |
| [`feature-branch-pins.md`](feature-branch-pins.md) | Wallet flake pins (LEZ main + PR 19) |
| [`archive/legacy-module-bootstrap.md`](archive/legacy-module-bootstrap.md) | Superseded Legacy plugin bootstrap |
| [`archive/superseded-wallet-pr-429-16.md`](archive/superseded-wallet-pr-429-16.md) | Deprecated PR 429 / 16 wallet JSON API |
| [`archive/engineering-notes.md`](archive/engineering-notes.md) | Misc engineering notes (FFI, SPEL PDA) |
