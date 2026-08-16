# Payment streams integration plan

Step program, delivery forks, and upcoming packets.
Agent entry: [`AGENTS.md`](../../AGENTS.md).
Product docs: [README.md](../../README.md), [verification-matrix.md](../reference/matrix.md).

Cross-step APIs: [integration-contracts.md](../reference/wire.md).
Decisions: [integration-decisions.md](../reference/decisions.md).

## Quick links

| Path | Contents |
| --- | --- |
| [AGENTS.md](../../AGENTS.md) | Agent read order, active step |
| [completed/](completed/) | Normative excerpts (12-16), completed step packets (17-19, 22, 24, 26-41, 44–47, 49–50, 52–53) |
| [upcoming/](upcoming/) | Steps 21, 51 |
| [wontfix/](wontfix/) | Not near-term; may return to upcoming — Steps 20, 23, 25, 42, 43, 48 |
| [reference/wire.md](../reference/wire.md) | Cross-step APIs |
| [reference/decisions.md](../reference/decisions.md) | D1-D6, N1-N18 |
| [../archive/completed-steps-index.md](../archive/completed-steps-index.md) | Full step map, completed summaries, verify scripts |

Documentation tracks ([N18](../reference/decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)):

| Track | Status | Steps / links |
| --- | --- | --- |
| Store eligibility | Formal logos-docs publish wontfix; living docs | [store-eligibility.md](../reproduce/store.md), [eligibility.md](../integrate.md); Step 20 ([logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)); Step 46 |
| Eligibility integration guide (Step 35) | Complete (substance in integrate doc) | [eligibility.md](../integrate.md); Step 35 |
| Protocol-only CLI | Complete as historical track; living path | [payment-streams.md](../reproduce/module.md); [logos-docs#370](https://github.com/logos-co/logos-docs/issues/370); Steps 22, 28, 34; Step 46 |
| Protocol UI (Basecamp) | Upcoming | Step 21 |
| Docs unify | Complete | Step 46 |
| Forum post | Upcoming | Step 51; draft [forum-post.md](../external/forum-post.md) |
| Wrap-up verification | Complete | Step 52 |
| Repository structure | Complete | Step 53 |
| Multi-token type alignment (native-only demo) | Complete | Step 49 |
| Consistency and clarity polish | Complete | Step 50 |

Engineering: Steps 26–33 complete; Steps 36–41 complete
([logos-lips#397](https://github.com/logos-co/logos-lips/pull/397) → `master` `435a6f18`;
[logos-lips#379](https://github.com/logos-co/logos-lips/pull/379) → `master` `f09f9e9e`);
Steps 21, 51 upcoming (Basecamp UI; forum post).
Step 52 complete (wrap-up verification).
Step 48 wontfix (program-graph LEZ unify / AT config drop).
Step 46 complete (living docs IA). Step 49 complete (ImageID cut).
Step 50 complete (consistency and clarity polish).
Step 52 recorded the wrap-up matrix on ImageID `c30781ea…`
(unit, localnet, testnet, private execution).
Step 51 publishes the forum post from that gate log.
Step 53 complete (repository structure).
Step 44 complete (payer/payee close). Step 45 complete (deps and patches freeze).
Step 47 complete (role terminology).
Step 20 wontfix (formal Developer Journey logos-docs publish; replaced by Step 46
docs and Step 51 forum).
Step 46 is not blocked on Step 48 (wontfix).
Steps 42–43 wontfix (Testnet v0.3 incentivisation research / spec).
Public hosted Store provider: Step 23 (wontfix).

## Program scope

Logos Delivery Store requests may carry a payment-stream eligibility proof;
the provider verifies against LEZ on-chain state before serving.
Store tag `30` follows RFC 73 (proof on request, status on response) with
LIP-155 as the proof bytes ([D1](../reference/decisions.md#d1-store-wire-format)).
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
| Developer Journey: integrators, Store integration ([logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)) | 20 (wontfix; living docs in Step 46, forum in Step 51) |
| Developer Journey: protocol-agnostic eligibility guide | 35 (complete) |
| User Journey: CLI doc packet ([logos-docs#370](https://github.com/logos-co/logos-docs/issues/370)) | 22 (complete) |
| User Journey: testnet manual walkthrough | 34 (complete) |
| Protocol UI: Basecamp plugin (vaults and streams) | 21 (upcoming) |
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
| E2E payer-close and payee-close | 44 (complete) |
| Revisit dependencies and patches | 45 (complete) |
| Docs unify (reproduce / integrate / README Testing) | 46 (complete) |
| Unify module JSON terminology (`owner` / `provider`) | 47 (complete) |
| Program-graph LEZ unify + drop AT hex config | 48 (wontfix) |
| LIP-155 multi-token type alignment (native-only demo) | 49 (complete) |
| Consistency and clarity polish | 50 (complete) |
| Forum post | 51 (upcoming; after 52) |
| Wrap-up verification | 52 (complete) |
| Repository structure | 53 (complete) |

Step 25 (in-process demo coordinator module) is wontfix
([packet](wontfix/step-25-demo-coordination-module.md)).

Testnet v0.3 incentivisation roadmap:
[incentivisation_v0.3](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3).
Client and provider shielding map to Steps 36–40; Steps 42–43 research
packets are wontfix for now.

### Store query dependency

Steps 16-20 need Store query on our delivery forks, not on upstream `master`
([D2](../reference/decisions.md#d2-delivery-module-hook-design),
[N6](../reference/decisions.md#n6-delivery-module-store-query-exposure)).
Upstream N6 is no longer a gate for Steps 14-20.
Dual-host demo coordination stays in host scripts
([N17](../reference/decisions.md#n17-demo-orchestration-stays-external-script-2026-06)).

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
| 44 | E2E closeStream payer and payee close | [step-44-payer-and-payee-close.md](completed/step-44-payer-and-payee-close.md) |
| 45 | Revisit dependencies and patches | [step-45-dependencies-and-patches.md](completed/step-45-dependencies-and-patches.md) |
| 46 | Docs unify (reproduce / integrate / README Testing) | [step-46-docs-unify-and-forum-post.md](completed/step-46-docs-unify-and-forum-post.md) |
| 47 | Unify role terminology (module, journey, policy, layouts) | [step-47-unify-role-terminology.md](completed/step-47-unify-role-terminology.md) |
| 49 | LIP-155 multi-token type alignment (native-only demo) | [step-49-native-token-spec-alignment.md](completed/step-49-native-token-spec-alignment.md) |
| 50 | Consistency and clarity polish | [step-50-consistency-and-clarity.md](completed/step-50-consistency-and-clarity.md) |
| 52 | Wrap-up verification | [step-52-wrap-up-verification.md](completed/step-52-wrap-up-verification.md) |
| 53 | Repository structure | [step-53-repository-structure.md](completed/step-53-repository-structure.md) |

Gate logs: [step-32-testnet-gate-log.md](completed/step-32-testnet-gate-log.md) (Step 32 D3),
[step-33-testnet-gate-log.md](completed/step-33-testnet-gate-log.md) (Step 33),
[step-39-testnet-gate-log.md](completed/step-39-testnet-gate-log.md) (Step 39),
[step-44-gate-log.md](completed/step-44-gate-log.md) (Step 44),
[step-45-gate-log.md](completed/step-45-gate-log.md) (Step 45),
[step-46-gate-log.md](completed/step-46-gate-log.md) (Step 46),
[step-47-gate-log.md](completed/step-47-gate-log.md) (Step 47),
[step-49-gate-log.md](completed/step-49-gate-log.md) (Step 49),
[step-50-gate-log.md](completed/step-50-gate-log.md) (Step 50),
[step-52-gate-log.md](completed/step-52-gate-log.md) (Step 52),
[step-53-gate-log.md](completed/step-53-gate-log.md) (Step 53).

## Upcoming steps

| Step | Focus | Status |
| --- | --- | --- |
| 21 | Protocol UI: Basecamp plugin | Upcoming — [step-21-basecamp-ui.md](upcoming/step-21-basecamp-ui.md) |
| 51 | Forum post | Upcoming — [step-51-forum-post.md](upcoming/step-51-forum-post.md) |

## Waiting steps

None.

## Wontfix steps

Not planned near-term. May move to `upcoming/` if resurrected.

| Step | Focus | Packet |
| --- | --- | --- |
| 20 | Developer Journey formal logos-docs publish | [step-20-developer-journey.md](wontfix/step-20-developer-journey.md) |
| 23 | Public Store provider | [step-23-public-store-provider.md](wontfix/step-23-public-store-provider.md) |
| 25 | Demo coordination Logos module | [step-25-demo-coordination-module.md](wontfix/step-25-demo-coordination-module.md) |
| 42 | Discovery + payment policy advertisement | [step-42-discovery-payment-policy-advertisement.md](wontfix/step-42-discovery-payment-policy-advertisement.md) |
| 43 | Shared payment pool model research | [step-43-shared-payment-pool-research.md](wontfix/step-43-shared-payment-pool-research.md) |
| 48 | Program-graph LEZ unify + drop AT hex config | [step-48-program-graph-lez-unify.md](wontfix/step-48-program-graph-lez-unify.md) |

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

Record the chosen name in [`feature-branch-pins.md`](../reference/pins.md)
when creating the branch. Both delivery repos must use the same string.

| Repo | Steps | Scope |
| --- | --- | --- |
| `logos-delivery` | 14-15 (done) | Store codec (tag `30`), `liblogosdelivery` hooks, `logosdelivery_store_query` |
| `logos-delivery-module` | 16 (done) | `storeQuery`, eligibility routing; `flake.nix` pins `logos-delivery` to `feat/payment-streams-store-eligibility` ([feature-branch-pins.md](../reference/pins.md)) |

## Components

| Piece | Role |
| --- | --- |
| `lez-payment-streams-core` / `lez-payment-streams-ffi` | Policy, fold, proofs, instruction builders |
| `logos-payment-streams-module` | Universal Qt module, wallet via `logos_execution_zone` |
| `logos-delivery` / `liblogosdelivery` | Store protocol + eligibility hooks (14-15) |
| `logos-delivery-module` | `delivery_module` + routing (16) |
| `verify/e2e.sh`, `verify/store/run_e2e.py` | Store integration: dual-host orchestration (Step 17; in-repo SSOT) |
| `payment_streams_ui` (upcoming) | Protocol UI: Basecamp over `payment_streams_module` only (Step 21). Vaults and streams as a payment mechanism; no Store. |
| `lgs` / `logoscore` / `lgpm` / `lm` | Localnet, host, install, introspection |

Detail: [`logos-architecture-overview.md`](../archive/reference/logos-architecture-overview.md).

## Decisions reference

| ID | Topic | Link |
| --- | --- | --- |
| D1 | Store wire tags | [integration-decisions.md](../reference/decisions.md#d1-store-wire-format) |
| D2 | Delivery hooks | [D2](../reference/decisions.md#d2-delivery-module-hook-design) |
| D3 | Wallet write path | [D3](../reference/decisions.md#d3-wallet-write-path) |
| D4 | Wallet module id | [D4](../reference/decisions.md#d4-wallet-module-runtime-name) |
| D5 | PS module naming | [D5](../reference/decisions.md#d5-new-module-naming) |
| D6 | Universal interface | [D6](../reference/decisions.md#d6-universal-module-interface) |
| N1-N18 | Carry-forward notes | [integration-decisions.md](../reference/decisions.md) |

Cross-step APIs without reading full D/N:
[`integration-contracts.md`](../reference/wire.md).

## Onboarding

### Minimal (implementing Step N)

1. [`AGENTS.md`](../../AGENTS.md)
2. [`reference/wire.md`](../reference/wire.md)
3. Step packet: [`upcoming/`](upcoming/), [`wontfix/`](wontfix/),
   or [`completed/step-N.md`](completed/)
4. [`logos-architecture-overview.md`](../archive/reference/logos-architecture-overview.md)
   when boundaries are unclear

### Full (first time in repo)

Add:
[`../archive/steps/logos-runtime-guide.md`](../archive/steps/logos-runtime-guide.md),
[`../archive/steps/scaffold-rpc-findings.md`](../archive/steps/scaffold-rpc-findings.md),
[`reference/pins.md`](../reference/pins.md),
LIP-155 (`rfc-index/docs/anoncomms/raw/payment-streams.md`,
branch `feat/payment-streams-onchain-part` on `logos-co/logos-lips`).

## Machine manifest

[context-manifest.json](context-manifest.json)
