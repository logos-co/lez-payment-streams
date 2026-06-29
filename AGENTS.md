# lez-payment-streams — agent context

Logos payment-streams: LIP-155 on-chain program, `payment_streams_module`, and Store eligibility integration.
Do not load obsolete monolithic plans; use the split doc set below.

Human-oriented overview: [README.md](README.md).

## Goal

Paid Store queries carry an LIP-155 `EligibilityProof` (RFC 73 pattern on Store: proof on
request, eligibility status on response); the provider verifies against LEZ chain
state and serves only when valid. Rust/FFI holds crypto and policy; `payment_streams_module`
orchestrates wallet I/O; `logos-delivery` gains opaque wire fields and hooks (Steps 14–16).

Payment streams is a universal LEZ payment protocol; Store eligibility is one integration
use case (N18 Track A). Optional Basecamp UI (N18 Track B) covers protocol-only flows, not Store.

Program outcomes and track split:
[N18](docs/reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).
Program index: [docs/development-map/program-index.md](docs/development-map/program-index.md).

Terminology (Flow A/B vs Track A/B):
[docs/reference/naming-conventions.md](docs/reference/naming-conventions.md).

## Active work (execution order)

Steps 1–13, 11d, Steps 14–16 on the delivery forks, Steps 17–17b, 18, 18b, 19, 24, 24b, and 24c are complete.

Active work: Step 20 (N18 Track A — Store integration developer journey; verification Flow B).

| Step | Repo / area | Agent packet |
| --- | --- | --- |
| 20 | Developer journey — N18 Track A (Store integration) | [plan/upcoming/step-20-developer-journey.md](docs/plan/upcoming/step-20-developer-journey.md) — script E2E + step-by-step dual-host commands ([N17](docs/reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06), [N18](docs/reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)) |
| 23 | Public Store provider (optional) | [plan/upcoming/step-23-public-store-provider.md](docs/plan/upcoming/step-23-public-store-provider.md) |

Reference (complete): [step-18-public-testnet-demo.md](docs/plan/completed/step-18-public-testnet-demo.md),
[step-18b-rc5-unify-handoff.md](docs/plan/completed/step-18b-rc5-unify-handoff.md),
[step-24c-simplify-demo-flow.md](docs/plan/completed/step-24c-simplify-demo-flow.md),
[step-24b-rc5-rust-lee-unify.md](docs/plan/completed/step-24b-rc5-rust-lee-unify.md).

Optional stretch (if time): Step 21 payment streams Basecamp UI, Step 22 UI journey doc —
N18 Track B only; payee claim assumes stream ids shared out of band.

Closed: Step 25 — [plan/cancelled/step-25-demo-coordination-module.md](docs/plan/cancelled/step-25-demo-coordination-module.md).

Completed packets (reference): [step-17.md](docs/plan/completed/step-17.md),
[step-17b](docs/plan/completed/step-17b-localnet-snapshot-restore.md),
[step-19](docs/plan/completed/step-19-lip155-onchain-spec.md),
[step-24](docs/plan/completed/step-24-lee-harness-upgrade.md).

Step 20 documents N18 Track A (Store integrator journey), not Flow A module-only verification.
Steps 21–22 document N18 Track B only. Step 25 won't fix ([N17](docs/reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06)).

N6 unblocked: `storeQuery` is on our `logos-delivery-module` fork (Step 16); upstream N6 is not
a gate for Steps 17–20.

Delivery branches: fork `logos-delivery` and `logos-delivery-module` from upstream
`master` (not release tags); default shared branch name
`feat/payment-streams-store-eligibility`. `logos-delivery-module/flake.nix` pins the
`logos-delivery` input to that branch (see [feature-branch-pins.md](docs/feature-branch-pins.md)).
Summary:
[program-index.md](docs/development-map/program-index.md#delivery-integration-branches).

## Read order by task

| Task | Files (in order) |
| --- | --- |
| Product overview | [README.md](README.md) → pillar hub under `docs/on-chain/`, `docs/payment-streams-module/`, or `docs/store-integration/` |
| Verification gates | [verification-matrix.md](docs/verification-matrix.md) → [scripts/README.md](scripts/README.md) |
| Delivery / Store wire (16+) | [integration-contracts.md](docs/reference/integration-contracts.md) → step packet → [D1](docs/reference/decisions-and-notes.md#d1-store-wire-format) / [D2](docs/reference/decisions-and-notes.md#d2-delivery-module-hook-design). Steps 14–16: [step-14-normative.md](docs/plan/completed/step-14-normative.md), [step-15-normative.md](docs/plan/completed/step-15-normative.md), [step-16.md](docs/plan/completed/step-16.md). |
| Module eligibility bugfix (historical) | [step12-user-eligibility.md](docs/step12-user-eligibility.md) or [step13-provider-eligibility.md](docs/step13-provider-eligibility.md) + contracts |
| Localnet / verify failure | [demo-localnet-recovery.md](docs/demo-localnet-recovery.md) + [verification-matrix.md](docs/verification-matrix.md) |
| Step 17b snapshot / fast reuse | [plan/completed/step-17b-localnet-snapshot-restore.md](docs/plan/completed/step-17b-localnet-snapshot-restore.md) + [N15](docs/reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19) |
| Step 17 E2E dual-host (done) | [plan/completed/step-17.md](docs/plan/completed/step-17.md) + [store-integration/runbook-localnet.md](docs/store-integration/runbook-localnet.md) + [N13](docs/reference/decisions-and-notes.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18) |
| Step 18b rc5 unify | [step-18b-rc5-unify-handoff.md](docs/plan/completed/step-18b-rc5-unify-handoff.md) then [step-18-public-testnet-demo.md](docs/plan/completed/step-18-public-testnet-demo.md) |
| Step 18 public sequencer | [plan/completed/step-18-public-testnet-demo.md](docs/plan/completed/step-18-public-testnet-demo.md) + [store-integration/runbook-testnet.md](docs/store-integration/runbook-testnet.md) |
| Step 20 developer journey (N18 Track A) | [plan/upcoming/step-20-developer-journey.md](docs/plan/upcoming/step-20-developer-journey.md) + [store-integration/runbook-localnet.md](docs/store-integration/runbook-localnet.md) + [N17](docs/reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06) + [N18](docs/reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06) |
| Step 21–22 payment streams UI (N18 Track B, optional) | [step-21-basecamp-ui.md](docs/plan/upcoming/step-21-basecamp-ui.md), [step-22-ui-journey.md](docs/plan/upcoming/step-22-ui-journey.md) + [N18](docs/reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06) |
| Step 23 hosted provider | [plan/upcoming/step-23-public-store-provider.md](docs/plan/upcoming/step-23-public-store-provider.md) |
| Rust FFI / policy only | `lez-payment-streams-core` tests + [step3-policy-and-implementor-notes.md](docs/step3-policy-and-implementor-notes.md) |
| LIP on-chain spec (19, done) | [step-19-lip155-onchain-spec.md](docs/plan/completed/step-19-lip155-onchain-spec.md) + [feature-branch-pins.md](docs/feature-branch-pins.md) |
| LEZ harness / `program_tests` (24, done) | [step-24-lee-harness-upgrade.md](docs/plan/completed/step-24-lee-harness-upgrade.md) + [docs/on-chain/architecture.md](docs/on-chain/architecture.md) |
| Rust LEZ pin unify rc5 (24b, done) | [step-24b-rc5-rust-lee-unify.md](docs/plan/completed/step-24b-rc5-rust-lee-unify.md) + [N16](docs/reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06) |
| Demo lifecycle simplify (24c, complete) | [step-24c-simplify-demo-flow.md](docs/plan/completed/step-24c-simplify-demo-flow.md) |
| Doc packet (20 / 22) | Step 20 = N18 Track A; Step 22 = N18 Track B (after 21) + [logos-docs doc packet template](https://github.com/logos-co/logos-docs/blob/main/resources/templates/doc-packet.md) |

## Always-on references

| Category | Files |
| --- | --- |
| Verification | [docs/verification-matrix.md](docs/verification-matrix.md), [scripts/README.md](scripts/README.md) |
| APIs / wire | [docs/reference/integration-contracts.md](docs/reference/integration-contracts.md) |
| Program | [docs/development-map/program-index.md](docs/development-map/program-index.md) |
| Architecture | [docs/reference/logos-architecture-overview.md](docs/reference/logos-architecture-overview.md), [docs/on-chain/architecture.md](docs/on-chain/architecture.md) |
| Terminology | [docs/reference/naming-conventions.md](docs/reference/naming-conventions.md) |

## Historical depth (load on demand)

- [reference/decisions-and-notes.md](docs/reference/decisions-and-notes.md) — D1–D6, N1–N18
- [plan/completed/step-12-normative.md](docs/plan/completed/step-12-normative.md) through step-15 normative excerpts
- Step runbooks and policy notes under [development-map/README.md](docs/development-map/README.md)

## Machine manifest

[`docs/context-manifest.json`](docs/context-manifest.json) lists default read order for automation.
