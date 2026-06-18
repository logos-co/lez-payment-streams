# Agent brief — payment streams integration

Read this file first for integration work in `lez-payment-streams`. Use the split doc set below; the old monolithic plan was removed.

## Goal

Paid Store queries carry an LIP-155 `EligibilityProof` (RFC 73 pattern on Store: proof on
request, eligibility status on response); the provider verifies against LEZ chain
state and serves only when valid. Rust/FFI holds crypto and policy; `payment_streams_module`
orchestrates wallet I/O; `logos-delivery` gains opaque wire fields and hooks (Steps 14–16).

## Active work (execution order)

Steps 1–13, 11d, and Steps 14–16 on the delivery forks are complete. Implement next:

| Step | Repo / area | Agent packet |
| --- | --- | --- |
| 17 | E2E demo wiring | [plan/upcoming/step-17.md](plan/upcoming/step-17.md) |
| 18 | Basecamp UI (optional) | [plan/upcoming/step-18.md](plan/upcoming/step-18.md) |

N6 unblocked: `storeQuery` is added directly on our fork of `logos-delivery-module` (Step 16);
upstream N6 is no longer a prerequisite for steps 16–17.

Delivery branches: fork `logos-delivery` and `logos-delivery-module` from upstream
`master` (not release tags); default shared branch name
`feat/payment-streams-store-eligibility`. `logos-delivery-module/flake.nix` pins the
`logos-delivery` input to that branch (see [feature-branch-pins.md](feature-branch-pins.md)).
Summary:
[integration-index.md](../integration-index.md#delivery-integration-branches).

## Read order by task

| Task | Files (in order) |
| --- | --- |
| Delivery / Store wire (16+) | This brief → [integration-contracts.md](integration-contracts.md) → step packet → [D1](reference/decisions-and-notes.md#d1-store-wire-format) / [D2](reference/decisions-and-notes.md#d2-delivery-module-hook-design). Steps 14–16: [step-14-normative.md](plan/completed/step-14-normative.md), [step-15-normative.md](plan/completed/step-15-normative.md), [step-16.md](plan/upcoming/step-16.md). |
| Module eligibility bugfix | [step12-user-eligibility.md](step12-user-eligibility.md) or [step13-provider-eligibility.md](step13-provider-eligibility.md) + contracts |
| Localnet / verify failure | [demo-localnet-recovery.md](demo-localnet-recovery.md) + relevant `verify-step*-dod.sh` |
| Rust FFI / policy only | `lez-payment-streams-core` tests + [step3-policy-and-implementor-notes.md](step3-policy-and-implementor-notes.md) |

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
