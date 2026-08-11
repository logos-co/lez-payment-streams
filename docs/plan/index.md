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
| [completed/](completed/) | Normative excerpts (12-16), completed step packets (17-19, 22, 24, 26-41, 47) |
| [upcoming/](upcoming/) | Steps 20–21, 44–46 |
| [waiting/](waiting/) | (empty) |
| [wontfix/](wontfix/) | Not near-term; may return to upcoming — Steps 23, 25, 42, 43 |
| [../reference/integration-contracts.md](../reference/integration-contracts.md) | Cross-step APIs |
| [../reference/integration-decisions.md](../reference/integration-decisions.md) | D1-D6, N1-N18 |
| [../archive/completed-steps-index.md](../archive/completed-steps-index.md) | Full step map, completed summaries, verify scripts |

Journeys ([N18](../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)):

| Journey | Status | Steps / links |
| --- | --- | --- |
| Developer Journey (Store integration) | Upcoming | Step 20 ([logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)) + Step 17 scripts |
| Developer Journey generalization | Complete | Step 35 |
| User Journey (CLI / module) | Complete | [logos-docs#370](https://github.com/logos-co/logos-docs/issues/370); Steps 22, 28, 34; [USER_JOURNEY.md](../journeys/USER_JOURNEY.md) |
| User Journey UI (Basecamp) | Upcoming | Step 21 |
| Forum progress report | Upcoming | Step 46 |

Engineering: Steps 26–33 complete; Steps 36–41 complete
([logos-lips#397](https://github.com/logos-co/logos-lips/pull/397) → `master` `435a6f18`;
[logos-lips#379](https://github.com/logos-co/logos-lips/pull/379) → `master` `f09f9e9e`);
Steps 20–21, 44–46 upcoming (Developer Journey publish; Basecamp UI; payer/payee close;
deps/patches; forum progress report). Step 47 complete (role terminology).
Steps 42–43 wontfix (Testnet v0.3 incentivisation research / spec).
Public hosted Store provider: Step 23 (wontfix).

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
| Developer Journey: integrators, Store integration ([logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)) | 20 (upcoming) |
| Developer Journey: protocol-agnostic eligibility guide | 35 (complete) |
| User Journey: CLI doc packet ([logos-docs#370](https://github.com/logos-co/logos-docs/issues/370)) | 22 (complete) |
| User Journey: testnet manual walkthrough | 34 (complete) |
| User Journey UI: Basecamp plugin | 21 (upcoming) |
| Public hosted Store provider | 23 (wontfix) |
| LEZ in-process harness (`program_tests`, rc5 `lee`) | 24 + 24b (complete) |
| Deterministic demo lifecycle | 24c (complete) |
| TestNet v0.2 migration | 26 (complete) |
| Claim fix: both journeys, both chains | 27 (complete; testnet v0.2 re-test deferred) |
| User Journey on TestNet v0.2 | 28 (complete) |
| E2E script UX enhancement | 29 (complete) |
| Static dependency migration | 30 (complete) |
| Delivery fork rebase + wallet module bump | 31 (complete) |
| AT-init unify + Store claim phase | 32 (complete) |
| Store E2E fresh vault + testnet sizing | 33 (complete) |
| Payer funder unlinkability via LEZ private execution | 36 (complete) |
| Payee receiver privacy via LEZ private execution | 37 (complete) |
| Store E2E privacy profiles (full privacy mode) | 38 (complete) |
| Testnet privacy E2E after native guest deploy | 39 (complete) |
| LIP-155 privacy-preserving workflow in the specification | 40 (complete; [logos-lips#397](https://github.com/logos-co/logos-lips/pull/397) → `master` `435a6f18`) |
| Non-native token support in provider payment policies (F8, U9) | 41 (complete; [logos-lips#379](https://github.com/logos-co/logos-lips/pull/379) → `master` `f09f9e9e`) |
| Discovery + payment policy advertisement (F6, F7, U8) | 42 (wontfix; Discovery support) |
| Shared payment pool model research (F9–F11, U10) | 43 (wontfix) |
| E2E payer-close and payee-close | 44 (upcoming) |
| Revisit dependencies and patches | 45 (upcoming) |
| Forum progress report (plain English + command spoiler) | 46 (upcoming) |
| Unify module JSON terminology (`owner` / `provider`) | 47 (complete) |

Step 25 (in-process demo coordinator module) is wontfix
([packet](wontfix/step-25-demo-coordination-module.md)).

Testnet v0.3 incentivisation roadmap:
[incentivisation_v0.3](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3).
Client and provider shielding map to Steps 36–40; Steps 42–43 research
packets are wontfix for now.

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
| 22 | User Journey doc packet ([logos-docs#370](https://github.com/logos-co/logos-docs/issues/370)) | [step-22-ui-journey.md](completed/step-22-ui-journey.md) |
| 28 | User Journey on TestNet v0.2 | [step-28-user-journey-testnet.md](completed/step-28-user-journey-testnet.md) |
| 29 | E2E script UX enhancement | [step-29-e2e-script-ux.md](completed/step-29-e2e-script-ux.md) |
| 30 | Static dependency migration | [step-30-static-dependency-migration.md](completed/step-30-static-dependency-migration.md) |
| 31 | Delivery fork rebase + wallet bump | [step-31-dependencies-upgrade.md](completed/step-31-dependencies-upgrade.md) |
| 32 | AT-init unify + Store claim phase | [step-32-auth-transfer-unify-store-claim.md](completed/step-32-auth-transfer-unify-store-claim.md) |
| 33 | Store E2E fresh vault + testnet sizing | [step-33-store-e2e-fresh-vault.md](completed/step-33-store-e2e-fresh-vault.md) |
| 34 | User Journey manual walkthrough (testnet CLI) | [step-34-user-journey-manual-walkthrough.md](completed/step-34-user-journey-manual-walkthrough.md) |
| 35 | Developer Journey generalization | [step-35-developer-journey-generalization.md](completed/step-35-developer-journey-generalization.md) |
| 36 | Payer funder unlinkability via LEZ private execution | [step-36-payer-funder-unlinkability.md](completed/step-36-payer-funder-unlinkability.md) |
| 37 | Payee receiver privacy via LEZ private execution | [step-37-payee-receiver-privacy.md](completed/step-37-payee-receiver-privacy.md) |
| 38 | Store E2E privacy profiles (full privacy mode) | [step-38-store-privacy-e2e.md](completed/step-38-store-privacy-e2e.md) |
| 39 | Testnet privacy E2E after native guest deploy | [step-39-testnet-privacy-e2e.md](completed/step-39-testnet-privacy-e2e.md) |
| 40 | LIP-155 privacy-preserving workflow in the specification | [step-40-lip155-privacy-workflow-spec.md](completed/step-40-lip155-privacy-workflow-spec.md) |
| 41 | Non-native token provider payment policies (F8, U9) | [step-41-non-native-token-policy-spec.md](completed/step-41-non-native-token-policy-spec.md) |
| 47 | Unify role terminology (module, journey, policy, layouts) | [step-47-unify-role-terminology.md](completed/step-47-unify-role-terminology.md) |

Gate logs: [step-32-testnet-gate-log.md](completed/step-32-testnet-gate-log.md) (Step 32 D3),
[step-33-testnet-gate-log.md](completed/step-33-testnet-gate-log.md) (Step 33),
[step-39-testnet-gate-log.md](completed/step-39-testnet-gate-log.md) (Step 39).
[step-47-gate-log.md](completed/step-47-gate-log.md) (Step 47).

## Upcoming steps

| Step | Focus | Status |
| --- | --- | --- |
| 20 | Developer Journey: Store integration | Upcoming — Store publish track; [logos-docs#369](https://github.com/logos-co/logos-docs/issues/369) — [step-20-developer-journey.md](upcoming/step-20-developer-journey.md) |
| 21 | User Journey: Basecamp UI plugin | Upcoming — [step-21-basecamp-ui.md](upcoming/step-21-basecamp-ui.md) |
| 44 | E2E closeStream payer and payee close | Upcoming — [step-44-payer-and-payee-close.md](upcoming/step-44-payer-and-payee-close.md) |
| 45 | Revisit dependencies and patches | Upcoming — [step-45-dependencies-and-patches.md](upcoming/step-45-dependencies-and-patches.md) |
| 46 | Forum progress report | Upcoming — [step-46-forum-post-user-journey.md](upcoming/step-46-forum-post-user-journey.md) |

## Waiting steps

None.

## Wontfix steps

Not planned near-term. May move to `upcoming/` if resurrected.

| Step | Focus | Packet |
| --- | --- | --- |
| 23 | Public Store provider | [step-23-public-store-provider.md](wontfix/step-23-public-store-provider.md) |
| 25 | Demo coordination Logos module | [step-25-demo-coordination-module.md](wontfix/step-25-demo-coordination-module.md) |
| 42 | Discovery + payment policy advertisement | [step-42-discovery-payment-policy-advertisement.md](wontfix/step-42-discovery-payment-policy-advertisement.md) |
| 43 | Shared payment pool model research | [step-43-shared-payment-pool-research.md](wontfix/step-43-shared-payment-pool-research.md) |

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
| `payment_streams_ui` (upcoming) | User Journey: Basecamp UI over `payment_streams_module` only (Step 21) |
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
3. Step packet: [`upcoming/`](upcoming/), [`waiting/`](waiting/), [`wontfix/`](wontfix/),
   or [`completed/step-N.md`](completed/)
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
