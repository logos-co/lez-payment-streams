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
| [completed/](completed/) | Normative excerpts (12-16), completed step packets (17-19, 24, 26-31, 33) |
| [upcoming/](upcoming/) | Steps 20, 22, 32, 34, 35, 36, 37 (active); optional 21 (UI), 23 |
| [cancelled/](cancelled/) | Step 25 won't fix |
| [../reference/integration-contracts.md](../reference/integration-contracts.md) | Cross-step APIs |
| [../reference/integration-decisions.md](../reference/integration-decisions.md) | D1-D6, N1-N18 |
| [../archive/completed-steps-index.md](../archive/completed-steps-index.md) | Full step map, completed summaries, verify scripts |

Developer Journey (Store integration): Step 20 + Step 17 scripts.
User Journey: Step 22 (active, CLI-based doc packet); Step 34 (active, in-repo manual walkthrough).
User Journey UI (optional): Step 21 (Basecamp plugin) — if shipped, Step 22 may include UI content.
TestNet v0.2 migration: Steps 26-28 (complete).
E2E narrative UX: Step 29 (complete).
Delivery fork rebase + wallet bump: Step 31 (complete).
Static dependency migration: Step 30 (complete; D6 revisit condition closed).
Store fresh vault per run: Step 33 (complete).
AT-init unify + Store claim phase: Step 32 (active; D3 testnet gate pending).
See [N18](../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).

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
| Runnable integration demo (CLI, Store + eligibility) | 17 (local LEZ), 18 (testnet, historical) |
| LIP-155 on-chain spec (branch pin) | 19 (complete) |
| Developer Journey: integrators, Store integration | 20 (**next**) |
| Developer Journey: protocol-agnostic eligibility guide | 35 (active) |
| User Journey: CLI doc packet | 22 (active) |
| User Journey: testnet manual walkthrough | 34 (active) |
| User Journey UI: Basecamp plugin (optional) | 21 (optional) |
| Public hosted Store provider | 23 (optional) |
| LEZ in-process harness (`program_tests`, rc5 `lee`) | 24 + 24b (complete) |
| Deterministic demo lifecycle | 24c (complete) |
| TestNet v0.2 migration | 26 (complete) |
| Claim fix: both journeys, both chains | 27 (complete; testnet v0.2 re-test deferred) |
| User Journey on TestNet v0.2 | 28 (complete) |
| E2E script UX enhancement | 29 (complete) |
| Static dependency migration | 30 (complete) |
| Delivery fork rebase + wallet module bump | 31 (complete) |
| AT-init unify + Store claim phase | 32 (active; D3 gate pending) |
| Store E2E fresh vault + testnet sizing | 33 (complete) |
| Payer funder unlinkability via LEZ private execution | 36 (complete) |
| Payee receiver privacy via LEZ private execution | 37 (active) |

Step 25 (in-process demo coordinator module) is **won't fix**
([cancelled packet](cancelled/step-25-demo-coordination-module.md)).

### Store query dependency

Steps 16-20 need Store query on our delivery forks, not on upstream `master`
([D2](../reference/integration-decisions.md#d2-delivery-module-hook-design),
[N6](../reference/integration-decisions.md#n6-delivery-module-store-query-exposure)).
Upstream N6 is no longer a gate for Steps 14-20.
Dual-host demo coordination stays in host scripts
([N17](../reference/integration-decisions.md#n17-demo-orchestration-stays-external-script-2026-06)).

## Completed step packets

| Step | Focus | Packet |
| --- | --- | --- |
| 26 | TestNet v0.2 migration | [step-26-testnet-v02-migration.md](completed/step-26-testnet-v02-migration.md) |
| 27 | Claim fix and verification | [step-27-claim-fix-verification.md](completed/step-27-claim-fix-verification.md) |
| 28 | User Journey on TestNet v0.2 | [step-28-user-journey-testnet.md](completed/step-28-user-journey-testnet.md) |
| 29 | E2E script UX enhancement | [step-29-e2e-script-ux.md](completed/step-29-e2e-script-ux.md) |
| 30 | Static dependency migration | [step-30-static-dependency-migration.md](completed/step-30-static-dependency-migration.md) |
| 31 | Delivery fork rebase + wallet bump | [step-31-dependencies-upgrade.md](completed/step-31-dependencies-upgrade.md) |
| 33 | Store E2E fresh vault + testnet sizing | [step-33-store-e2e-fresh-vault.md](completed/step-33-store-e2e-fresh-vault.md) |
| 36 | Payer funder unlinkability via LEZ private execution | [step-36-payer-funder-unlinkability.md](completed/step-36-payer-funder-unlinkability.md) |

Gate logs: [step-32-testnet-gate-log.md](completed/step-32-testnet-gate-log.md) (Step 32 D3),
[step-33-testnet-gate-log.md](completed/step-33-testnet-gate-log.md) (Step 33).

## Upcoming steps

| Step | Focus | Status |
| --- | --- | --- |
| 20 | Developer Journey: Store integration | Active -- [step-20-developer-journey.md](upcoming/step-20-developer-journey.md) |
| 21 | User Journey: Basecamp UI plugin (optional) | Optional -- [step-21-basecamp-ui.md](upcoming/step-21-basecamp-ui.md) |
| 22 | User Journey: doc packet (CLI-based) | Active -- [step-22-ui-journey.md](upcoming/step-22-ui-journey.md) |
| 23 | Public Store provider | Optional -- [step-23-public-store-provider.md](upcoming/step-23-public-store-provider.md) |
| 32 | AT-init unify + Store claim phase | Active (signed off; D3 gate pending) -- [step-32-auth-transfer-unify-store-claim.md](upcoming/step-32-auth-transfer-unify-store-claim.md) |
| 34 | User Journey manual walkthrough (testnet CLI) | Active -- [step-34-user-journey-manual-walkthrough.md](upcoming/step-34-user-journey-manual-walkthrough.md) |
| 35 | Developer Journey generalization (protocol-agnostic eligibility guide) | Active -- [step-35-developer-journey-generalization.md](upcoming/step-35-developer-journey-generalization.md) |
| 37 | Payee receiver privacy via LEZ private execution | Active -- [step-37-payee-receiver-privacy.md](upcoming/step-37-payee-receiver-privacy.md) |

Ideas not yet steps: [raw-todos/](raw-todos/).

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
