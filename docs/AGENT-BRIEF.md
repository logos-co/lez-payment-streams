# Agent brief — payment streams integration

Read this file first for integration work in `lez-payment-streams`. Use the split doc set below; the old monolithic plan was removed.

## Goal

Paid Store queries carry an LIP-155 `EligibilityProof` (RFC 73 pattern on Store: proof on
request, eligibility status on response); the provider verifies against LEZ chain
state and serves only when valid. Rust/FFI holds crypto and policy; `payment_streams_module`
orchestrates wallet I/O; `logos-delivery` gains opaque wire fields and hooks (Steps 14–16).

Program outcomes after Step 16: runnable CLI demo (17–18), LIP-155 on-chain spec on `main`
(19), developer journey doc packet (20); optional Basecamp UI and UI journey (21–22). Index:
[integration-index.md](../integration-index.md#program-outcomes).

## Active work (execution order)

Steps 1–13, 11d, and Steps 14–16 on the delivery forks are complete. Implement next in order:

| Step | Repo / area | Agent packet |
| --- | --- | --- |
| 17 | E2E demo (local LEZ) | [plan/upcoming/step-17.md](plan/upcoming/step-17.md) |
| 18 | Public testnet demo | [plan/upcoming/step-18-public-testnet-demo.md](plan/upcoming/step-18-public-testnet-demo.md) |
| 19 | LIP-155 on-chain spec | [plan/upcoming/step-19-lip155-onchain-spec.md](plan/upcoming/step-19-lip155-onchain-spec.md) |
| 20 | Developer journey doc packet | [plan/upcoming/step-20-developer-journey.md](plan/upcoming/step-20-developer-journey.md) |
| 21 | Basecamp UI (optional) | [plan/upcoming/step-21-basecamp-ui.md](plan/upcoming/step-21-basecamp-ui.md) |
| 22 | UI journey doc packet (optional) | [plan/upcoming/step-22-ui-journey.md](plan/upcoming/step-22-ui-journey.md) |

Step 19 may run in parallel with 17–18. Step 20 should follow 17 and 18 when the journey
targets testnet v0.2. Steps 21–22 only if shipping a UI journey.

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
| Rust FFI / policy only | `lez-payment-streams-core` tests + [step3-policy-and-implementor-notes.md](step3-policy-and-implementor-notes.md) |
| LIP on-chain spec (19) | [step-19-lip155-onchain-spec.md](plan/upcoming/step-19-lip155-onchain-spec.md) + [architecture.md](../architecture.md) |
| Doc packet (20 / 22) | Step packet + [logos-docs doc packet template](https://github.com/logos-co/logos-docs/blob/main/resources/templates/doc-packet.md) |

## Always-on references

- [integration-index.md](../integration-index.md) — step map, status, links
- [integration-contracts.md](integration-contracts.md) — method names, encodings, wire tags
- [logos-architecture-overview.md](../logos-architecture-overview.md) — hosts, FFI vs LogosAPI

## Historical depth (load on demand)

- [reference/decisions-and-notes.md](reference/decisions-and-notes.md) — D1–D6, N1–N12 (Step 16: N3a–N3c, N12 scope)
- [plan/completed/step-12-normative.md](plan/completed/step-12-normative.md), [step-13-normative.md](plan/completed/step-13-normative.md), [step-14-normative.md](plan/completed/step-14-normative.md), [step-15-normative.md](plan/completed/step-15-normative.md)
- Step runbooks and policy notes under [docs/README.md](README.md)

## Machine manifest

[`context-manifest.json`](context-manifest.json) lists default read order for automation.
