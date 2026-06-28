# Agent brief — payment streams integration

Read this file first for integration work in `lez-payment-streams`. Use the split doc set below; the old monolithic plan was removed.

## Goal

Paid Store queries carry an LIP-155 `EligibilityProof` (RFC 73 pattern on Store: proof on
request, eligibility status on response); the provider verifies against LEZ chain
state and serves only when valid. Rust/FFI holds crypto and policy; `payment_streams_module`
orchestrates wallet I/O; `logos-delivery` gains opaque wire fields and hooks (Steps 14–16).

Payment streams is a **universal LEZ payment protocol**; Store eligibility is **one integration
use case** (Track A). Optional Basecamp UI (Track B) covers protocol-only flows, not Store.

Program outcomes and track split: [N18](reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).
Index: [integration-index.md](../integration-index.md#program-outcomes).

## Active work (execution order)

Steps 1–13, 11d, Steps 14–16 on the delivery forks, Steps 17–17b, 18b, 19, 24, 24b, and 24c are complete.

Step 18 Part B (public testnet E2E with local Store) uses the same operational LEZ pin as Step 17
(rc5 on `master`). Active work: Step 18 Part B testnet DoD, Step 20 (Track A). Step 20 testnet
journey rows need Step 18 DoD.

| Step | Repo / area | Agent packet |
| --- | --- | --- |
| 18 | Public sequencer E2E (local Store) | [plan/upcoming/step-18-public-testnet-demo.md](plan/upcoming/step-18-public-testnet-demo.md) |
| 20 | Developer journey — **Track A** (Store integration) | [plan/upcoming/step-20-developer-journey.md](plan/upcoming/step-20-developer-journey.md) — script E2E + step-by-step dual-host commands ([N17](reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06), [N18](reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)) |
| 23 | Public Store provider (optional) | [plan/upcoming/step-23-public-store-provider.md](plan/upcoming/step-23-public-store-provider.md) |

Reference (24c, complete): [step-24c-simplify-demo-flow.md](plan/completed/step-24c-simplify-demo-flow.md).
Reference (complete): [step-24b-rc5-rust-lee-unify.md](plan/completed/step-24b-rc5-rust-lee-unify.md),
[plan/upcoming/step-18b-rc5-unify-handoff.md](plan/upcoming/step-18b-rc5-unify-handoff.md),
[N16](reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06).

**Optional stretch (if time):** Step 21 payment streams Basecamp UI, Step 22 UI journey doc —
**Track B** protocol only ([N18](reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)); payee claim assumes stream ids shared **out of band**.

Closed: Step 25 — [plan/cancelled/step-25-demo-coordination-module.md](plan/cancelled/step-25-demo-coordination-module.md).

Completed packets (reference): [step-17.md](plan/completed/step-17.md),
[step-17b](plan/completed/step-17b-localnet-snapshot-restore.md),
[step-19](plan/completed/step-19-lip155-onchain-spec.md),
[step-24](plan/completed/step-24-lee-harness-upgrade.md).

Step 20 documents Track A only. Steps 21–22 document Track B only; cross-link Step 20 for Store
integration. Step 25 won't fix ([N17](reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06)).

N6 unblocked: `storeQuery` is on our `logos-delivery-module` fork (Step 16); upstream N6 is not
a gate for Steps 17–20.

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
| Step 17b snapshot / fast reuse | [plan/completed/step-17b-localnet-snapshot-restore.md](plan/completed/step-17b-localnet-snapshot-restore.md) + [N15](reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19) |
| Step 17 E2E dual-host (done) | [plan/completed/step-17.md](plan/completed/step-17.md) + [step17-e2e-local.md](step17-e2e-local.md) + [N13](reference/decisions-and-notes.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18) |
| Step 18b rc5 unify | [step-18b-rc5-unify-handoff.md](plan/upcoming/step-18b-rc5-unify-handoff.md) (complete) then [step-18-public-testnet-demo.md](plan/upcoming/step-18-public-testnet-demo.md) |
| Step 18 public sequencer | [plan/upcoming/step-18-public-testnet-demo.md](plan/upcoming/step-18-public-testnet-demo.md) + [step18-public-sequencer-e2e.md](step18-public-sequencer-e2e.md) |
| Step 20 developer journey (Track A) | [plan/upcoming/step-20-developer-journey.md](plan/upcoming/step-20-developer-journey.md) + [step17-e2e-local.md](step17-e2e-local.md) + [N17](reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06) + [N18](reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06) |
| Step 21–22 payment streams UI (Track B, optional) | [step-21-basecamp-ui.md](plan/upcoming/step-21-basecamp-ui.md), [step-22-ui-journey.md](plan/upcoming/step-22-ui-journey.md) + [N18](reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06) |
| Step 23 hosted provider | [plan/upcoming/step-23-public-store-provider.md](plan/upcoming/step-23-public-store-provider.md) |
| Rust FFI / policy only | `lez-payment-streams-core` tests + [step3-policy-and-implementor-notes.md](step3-policy-and-implementor-notes.md) |
| LIP on-chain spec (19, done) | [step-19-lip155-onchain-spec.md](plan/completed/step-19-lip155-onchain-spec.md) + [feature-branch-pins.md](feature-branch-pins.md) |
| LEZ harness / `program_tests` (24, done) | [step-24-lee-harness-upgrade.md](plan/completed/step-24-lee-harness-upgrade.md) + [architecture.md](../architecture.md) |
| Rust LEZ pin unify rc5 (24b, done) | [step-24b-rc5-rust-lee-unify.md](plan/completed/step-24b-rc5-rust-lee-unify.md) + [N16](reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06) |
| Demo lifecycle simplify (24c, complete) | [step-24c-simplify-demo-flow.md](plan/completed/step-24c-simplify-demo-flow.md) |
| Doc packet (20 / 22) | Step 20 = Track A; Step 22 = Track B (after 21) + [logos-docs doc packet template](https://github.com/logos-co/logos-docs/blob/main/resources/templates/doc-packet.md) |

## Always-on references

- [integration-index.md](../integration-index.md) — step map, status, links
- [integration-contracts.md](integration-contracts.md) — method names, encodings, wire tags
- [logos-architecture-overview.md](../logos-architecture-overview.md) — hosts, FFI vs LogosAPI

## Historical depth (load on demand)

- [reference/decisions-and-notes.md](reference/decisions-and-notes.md) — D1–D6, N1–N18
- [plan/completed/step-12-normative.md](plan/completed/step-12-normative.md), [step-13-normative.md](plan/completed/step-13-normative.md), [step-14-normative.md](plan/completed/step-14-normative.md), [step-15-normative.md](plan/completed/step-15-normative.md)
- Step runbooks and policy notes under [docs/README.md](README.md)

## Machine manifest

[`context-manifest.json`](context-manifest.json) lists default read order for automation.
