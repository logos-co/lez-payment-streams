# Agent brief — payment streams integration

Read this file first for integration work in `lez-payment-streams`. Use the split doc set below; the old monolithic plan was removed.

## Goal

Paid Store queries carry an LIP-155 `EligibilityProof` (RFC 73 pattern on Store: proof on
request, eligibility status on response); the provider verifies against LEZ chain
state and serves only when valid. Rust/FFI holds crypto and policy; `payment_streams_module`
orchestrates wallet I/O; `logos-delivery` gains opaque wire fields and hooks (Steps 14–16).

Program outcomes after Step 16: runnable CLI demo (17 local LEZ, 18 public sequencer + local P2P),
LIP-155 on-chain spec (19, branch pin), developer journey doc packet (20); optional Basecamp UI
and UI journey (21–22); optional public Store provider (23); LEZ harness upgrade (24, done). Index:
[integration-index.md](../integration-index.md#program-outcomes).

## Active work (execution order)

Steps 1–13, 11d, Steps 14–16 on the delivery forks, Steps 17–17b, 19, and 24 are complete.
Implement next in order:

| Step | Repo / area | Agent packet |
| --- | --- | --- |
| 18 | Public sequencer E2E (local Store) | [plan/upcoming/step-18-public-testnet-demo.md](plan/upcoming/step-18-public-testnet-demo.md) |
| 20 | Developer journey doc packet | [plan/upcoming/step-20-developer-journey.md](plan/upcoming/step-20-developer-journey.md) |
| 21 | Basecamp UI (optional) | [plan/upcoming/step-21-basecamp-ui.md](plan/upcoming/step-21-basecamp-ui.md) |
| 22 | UI journey doc packet (optional) | [plan/upcoming/step-22-ui-journey.md](plan/upcoming/step-22-ui-journey.md) |
| 23 | Public Store provider (optional) | [plan/upcoming/step-23-public-store-provider.md](plan/upcoming/step-23-public-store-provider.md) |

Completed packets (reference): [step-17.md](plan/completed/step-17.md),
[step-17b](plan/completed/step-17b-localnet-snapshot-restore.md),
[step-19](plan/completed/step-19-lip155-onchain-spec.md),
[step-24](plan/completed/step-24-lee-harness-upgrade.md).

Step 20 should follow 17 and 18 when the journey targets testnet v0.2 (Step 23 not required).
Steps 21–22 only if shipping a UI journey; Step 23 only if shipping a hosted paid-Store provider.

N6 unblocked: `storeQuery` is added directly on our fork of `logos-delivery-module` (Step 16);
upstream N6 is no longer a prerequisite for Steps 17–20.

Delivery branches: fork `logos-delivery` and `logos-delivery-module` from upstream
`master` (not release tags); default shared branch name
`feat/payment-streams-store-eligibility`. `logos-delivery-module/flake.nix` pins the
`logos-delivery` input to that branch (see [feature-branch-pins.md](feature-branch-pins.md)).
Summary:
[integration-index.md](../integration-index.md#delivery-integration-branches).

## Read order by task

| Task | Files (in order) |
| --- | --- |
| Delivery / Store wire (16+) | This brief → [integration-contracts.md](integration-contracts.md) → step packet → [D1](reference/decisions-and-notes.md#d1-store-wire-format) / [D2](reference/decisions-and-notes.md#d2-delivery-module-hook-design). Steps 14–16: [step-14-normative.md](plan/completed/step-14-normative.md), [step-15-normative.md](plan/completed/step-15-normative.md), [step-16.md](plan/completed/step-16.md). |
| Module eligibility bugfix | [step12-user-eligibility.md](step12-user-eligibility.md) or [step13-provider-eligibility.md](step13-provider-eligibility.md) + contracts |
| Localnet / verify failure | [demo-localnet-recovery.md](demo-localnet-recovery.md) + relevant `verify-step*-dod.sh` |
| Step 17b snapshot / fast reuse | [plan/completed/step-17b-localnet-snapshot-restore.md](plan/completed/step-17b-localnet-snapshot-restore.md) + [N15](reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19) (`FULL_RESET=1` after `make build`) |
| Step 17 E2E dual-host (done) | [plan/completed/step-17.md](plan/completed/step-17.md) + [step17-e2e-local.md](step17-e2e-local.md) + [N13](reference/decisions-and-notes.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18) |
| Step 18 public sequencer | [plan/upcoming/step-18-public-testnet-demo.md](plan/upcoming/step-18-public-testnet-demo.md) + Step 17 runbook (local P2P unchanged) |
| Step 23 hosted provider | [plan/upcoming/step-23-public-store-provider.md](plan/upcoming/step-23-public-store-provider.md) |
| Rust FFI / policy only | `lez-payment-streams-core` tests + [step3-policy-and-implementor-notes.md](step3-policy-and-implementor-notes.md) |
| LIP on-chain spec (19, done) | [step-19-lip155-onchain-spec.md](plan/completed/step-19-lip155-onchain-spec.md) + [feature-branch-pins.md](feature-branch-pins.md) LIP-155 pin |
| LEZ harness / `program_tests` (24, done) | [step-24-lee-harness-upgrade.md](plan/completed/step-24-lee-harness-upgrade.md) + [architecture.md](../architecture.md) |
| Doc packet (20 / 22) | Step packet + [logos-docs doc packet template](https://github.com/logos-co/logos-docs/blob/main/resources/templates/doc-packet.md) |

## Always-on references

- [integration-index.md](../integration-index.md) — step map, status, links
- [integration-contracts.md](integration-contracts.md) — method names, encodings, wire tags
- [logos-architecture-overview.md](../logos-architecture-overview.md) — hosts, FFI vs LogosAPI

## Historical depth (load on demand)

- [reference/decisions-and-notes.md](reference/decisions-and-notes.md) — D1–D6, N1–N15 (Step 16: N3a–N3c; Step 17: N13–N14; Step 17b: N15)
- [plan/completed/step-12-normative.md](plan/completed/step-12-normative.md), [step-13-normative.md](plan/completed/step-13-normative.md), [step-14-normative.md](plan/completed/step-14-normative.md), [step-15-normative.md](plan/completed/step-15-normative.md)
- Step runbooks and policy notes under [docs/README.md](README.md)

## Machine manifest

[`context-manifest.json`](context-manifest.json) lists default read order for automation.
