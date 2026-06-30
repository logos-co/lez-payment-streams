# Payment streams integration plan

Step program, delivery forks, and upcoming packets.
Agent entry: [`AGENTS.md`](../../AGENTS.md).
Product docs: [README.md](../../README.md), [verification-matrix.md](../reference/verification-matrix.md).

Cross-step APIs: [integration-contracts.md](../reference/integration-contracts.md).
Decisions: [integration-decisions.md](../reference/integration-decisions.md).

## Quick links

| Path | Contents |
| --- | --- |
| [AGENTS.md](../../AGENTS.md) | Agent read order, active step |
| [completed/](completed/) | Normative excerpts (12-16), completed step packets |
| [upcoming/](upcoming/) | Step 20; optional 21-22 (User Journey), 23 |
| [cancelled/](cancelled/) | Step 25 won't fix |
| [../reference/integration-contracts.md](../reference/integration-contracts.md) | Cross-step APIs |
| [../reference/integration-decisions.md](../reference/integration-decisions.md) | D1-D6, N1-N18 |
| [../archive/completed-steps-index.md](../archive/completed-steps-index.md) | Full step map, completed summaries, verify scripts |

Developer Journey (Store integration): Step 20 + Step 17 scripts.
User Journey (payment streams UI, optional): Steps 21–22 —
see [N18](../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).

## Program scope

Logos Delivery Store requests may carry a payment-stream eligibility proof;
the provider verifies against LEZ on-chain state before serving.
Store tag `30` follows RFC 73 (proof on request, status on response) with
LIP-155 as the proof bytes ([D1](../reference/integration-decisions.md#d1-store-wire-format)).
Crypto and policy live in Rust (`lez-payment-streams-core`, `lez-payment-streams-ffi`);
orchestration in Universal `payment_streams_module`; Store wire and `liblogosdelivery` hooks
in the delivery repos.

MVP scope: LIP-155 transparent vaults, single user and single provider,
paid Store mode on the provider.

### Program outcomes

| Outcome | Steps |
| --- | --- |
| Runnable integration demo (CLI, Store + eligibility) | 17 (local LEZ), 18 (testnet) |
| LIP-155 on-chain spec (branch pin) | 19 (complete) |
| Developer Journey: integrators, Store integration | 20 (**next**) |
| User Journey: payment streams UI | 21-22 (optional stretch; [N18](../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)) |
| Public hosted Store provider | 23 (optional) |
| LEZ in-process harness (`program_tests`, rc5 `lee`) | 24 + 24b (complete) |
| Deterministic demo lifecycle | 24c (complete) |

Step 25 (in-process demo coordinator module) is **won't fix**
([cancelled packet](cancelled/step-25-demo-coordination-module.md)).

### Store query dependency

Steps 16-20 need Store query on our delivery forks, not on upstream `master`
([D2](../reference/integration-decisions.md#d2-delivery-module-hook-design),
[N6](../reference/integration-decisions.md#n6-delivery-module-store-query-exposure)).
Upstream N6 is no longer a gate for Steps 14-20.
Dual-host demo coordination stays in host scripts
([N17](../reference/integration-decisions.md#n17-demo-orchestration-stays-external-script-2026-06)).

## Upcoming steps

| Step | Focus | Status |
| --- | --- | --- |
| 20 | Developer Journey: Store integration | Active -- [step-20-developer-journey.md](upcoming/step-20-developer-journey.md) |
| 21 | User Journey: Basecamp UI plugin | Optional stretch -- [step-21-basecamp-ui.md](upcoming/step-21-basecamp-ui.md) |
| 22 | User Journey: doc packet | Optional stretch -- [step-22-ui-journey.md](upcoming/step-22-ui-journey.md) |
| 23 | Public Store provider | Optional -- [step-23-public-store-provider.md](upcoming/step-23-public-store-provider.md) |
| 25 | Demo coordination Logos module | Won't fix -- [cancelled/step-25](cancelled/step-25-demo-coordination-module.md) |

## Delivery integration branches

Store eligibility work ships on integration branches forked from current upstream
`master` in the delivery repos. Do not branch from release tags (for example
`logos-delivery-module/v0.1.1` used by `logos-delivery-demo`); tags lag the wire and ABI
changes in Steps 14-16.

Default branch name (use the same string in both repos), in priority order if the name
is taken on a remote:

1. `feat/payment-streams-store-eligibility` (preferred)
2. `feat/lip155-store-eligibility`
3. `integration/payment-streams-store`

Record the chosen name in [`feature-branch-pins.md`](../reference/feature-branch-pins.md)
when creating the branch. Both delivery repos must use the same string.

| Repo | Steps | Scope |
| --- | --- | --- |
| `logos-delivery` | 14-15 (done) | Store codec (tag `30`), `liblogosdelivery` hooks, `logosdelivery_store_query` |
| `logos-delivery-module` | 16 (done) | `storeQuery`, eligibility routing; `flake.nix` pins `logos-delivery` to `feat/payment-streams-store-eligibility` ([feature-branch-pins.md](../reference/feature-branch-pins.md)) |

## Components

| Piece | Role |
| --- | --- |
| `lez-payment-streams-core` / `lez-payment-streams-ffi` | Policy, fold, proofs, instruction builders |
| `logos-payment-streams-module` | Universal Qt module, wallet via `logos_execution_zone` |
| `logos-delivery` / `liblogosdelivery` | Store protocol + eligibility hooks (14-15) |
| `logos-delivery-module` | `delivery_module` + routing (16) |
| `scripts/e2e.sh`, `scripts/e2e/run_local_e2e.py` | Developer Journey: dual-host Store integration orchestration (Step 17, Step 20) |
| `payment_streams_ui` (optional) | User Journey: Basecamp UI over `payment_streams_module` only (Step 21) |
| `lgs` / `logoscore` / `lgpm` / `lm` | Localnet, host, install, introspection |

Detail: [`logos-architecture-overview.md`](../archive/reference/logos-architecture-overview.md).

## Decisions reference

| ID | Topic | Link |
| --- | --- | --- |
| D1 | Store wire tags | [integration-decisions.md](../reference/integration-decisions.md#d1-store-wire-format) |
| D2 | Delivery hooks | [D2](../reference/integration-decisions.md#d2-delivery-module-hook-design) |
| D3 | Wallet write path | [D3](../reference/integration-decisions.md#d3-wallet-write-path) |
| D4 | Wallet module id | [D4](../reference/integration-decisions.md#d4-wallet-module-runtime-name) |
| D5 | PS module naming | [D5](../reference/integration-decisions.md#d5-new-module-naming) |
| D6 | Universal interface | [D6](../reference/integration-decisions.md#d6-universal-module-interface) |
| N1-N18 | Carry-forward notes | [integration-decisions.md](../reference/integration-decisions.md) |

Cross-step APIs without reading full D/N:
[`integration-contracts.md`](../reference/integration-contracts.md).

## Onboarding

### Minimal (implementing Step N)

1. [`AGENTS.md`](../../AGENTS.md)
2. [`../reference/integration-contracts.md`](../reference/integration-contracts.md)
3. Step packet: [`upcoming/step-N.md`](upcoming/) or [`completed/step-N.md`](completed/)
4. [`logos-architecture-overview.md`](../archive/reference/logos-architecture-overview.md)
   when boundaries are unclear

### Full (first time in repo)

Add:
[`../archive/steps/logos-runtime-guide.md`](../archive/steps/logos-runtime-guide.md),
[`../archive/steps/scaffold-rpc-findings.md`](../archive/steps/scaffold-rpc-findings.md),
[`../reference/feature-branch-pins.md`](../reference/feature-branch-pins.md),
LIP-155 (`rfc-index/docs/anoncomms/raw/payment-streams.md`,
branch `feat/payment-streams-onchain-part` on `logos-co/logos-lips`).

## Machine manifest

[context-manifest.json](../context-manifest.json)
