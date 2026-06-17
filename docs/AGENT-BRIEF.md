# Agent brief — payment streams integration

Read this file first for integration work in `lez-payment-streams`. Do not load the full
historical plan unless you need audit-level detail.

## Goal

Paid Store queries carry an LIP-155 `EligibilityProof`; the provider verifies against LEZ chain
state and serves only when valid. Rust/FFI holds crypto and policy; `payment_streams_module`
orchestrates wallet I/O; `logos-delivery` gains opaque wire fields and hooks (Steps 14–16).

## Active work (execution order)

Steps 1–13 and 11d are complete in tree. Implement next:

| Step | Repo / area | Agent packet |
| --- | --- | --- |
| 14 | `logos-delivery` Store codec | [plan/upcoming/step-14.md](plan/upcoming/step-14.md) |
| 15 | `liblogosdelivery` C ABI | [plan/upcoming/step-15.md](plan/upcoming/step-15.md) |
| 16 | `logos-delivery-module` routing | [plan/upcoming/step-16.md](plan/upcoming/step-16.md) |
| 17 | E2E demo wiring | [plan/upcoming/step-17.md](plan/upcoming/step-17.md) |
| 18 | Basecamp UI (optional) | [plan/upcoming/step-18.md](plan/upcoming/step-18.md) |

Blocked: Step 16–17 Store query path until upstream `delivery_module` exposes Store query on
`master` ([N6](reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)).

## Read order by task

| Task | Files (in order) |
| --- | --- |
| Delivery / Store wire (14–16) | This brief → [integration-contracts.md](integration-contracts.md) → step packet → [D1](reference/decisions-and-notes.md#d1-store-wire-format) / [D2](reference/decisions-and-notes.md#d2-delivery-module-hook-design) |
| Module eligibility bugfix | [step12-user-eligibility.md](step12-user-eligibility.md) or [step13-provider-eligibility.md](step13-provider-eligibility.md) + contracts |
| Localnet / verify failure | [demo-localnet-recovery.md](demo-localnet-recovery.md) + relevant `verify-step*-dod.sh` |
| Rust FFI / policy only | `lez-payment-streams-core` tests + [step3-policy-and-implementor-notes.md](step3-policy-and-implementor-notes.md) |

## Always-on references

- [integration-index.md](../integration-index.md) — step map, status, links
- [integration-contracts.md](integration-contracts.md) — method names, encodings, wire tags
- [logos-architecture-overview.md](../logos-architecture-overview.md) — hosts, FFI vs LogosAPI

## Historical depth (load on demand)

- [archive/integration-plan-full.md](archive/integration-plan-full.md) — full pre-split plan
- [reference/decisions-and-notes.md](reference/decisions-and-notes.md) — D1–D6, N1–N11
- [plan/completed/step-12-normative.md](plan/completed/step-12-normative.md), [step-13-normative.md](plan/completed/step-13-normative.md)

## Machine manifest

[`context-manifest.json`](context-manifest.json) lists default read order for automation.
