# Payment streams integration index

Canonical short index for the Logos payment-streams Store demo. Agents: repo root
[`AGENTS.md`](AGENTS.md), then [`docs/AGENT-BRIEF.md`](docs/AGENT-BRIEF.md). Operators: step runbooks in [`docs/README.md`](docs/README.md).

The integration doc set is split under [`docs/plan/`](docs/plan/README.md),
[`docs/integration-contracts.md`](docs/integration-contracts.md), and
[`docs/reference/decisions-and-notes.md`](docs/reference/decisions-and-notes.md).
[`integration-plan.md`](integration-plan.md) redirects here.

## Task summary

Logos Delivery Store requests may carry a payment-stream eligibility proof; the provider verifies
against LEZ on-chain state before serving. Store tag `30` follows RFC 73 (proof on request,
status on response) with LIP-155 as the proof bytes ([D1](docs/reference/decisions-and-notes.md#d1-store-wire-format)).
Crypto and policy live in Rust (`lez-payment-streams-core`,
`lez-payment-streams-ffi`); orchestration in Universal `payment_streams_module`; Store wire and
`liblogosdelivery` hooks in the delivery repos (Steps 14–16).

MVP scope: LIP-155 transparent vaults, single user and single provider, paid Store mode on the
provider. Demonstration is split into two tracks ([N18](docs/reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)):
**Track A** — integrator integration demo (Step 17 script + Step 20 developer journey): payment
streams eligibility with Logos Delivery Store. **Track B** — optional payment streams-only UI
(Steps 21–22): vault/stream/claim flows in Basecamp; service and stream discovery out of band.
Step 17 is the local LEZ gate (`make verify-step17`). Step 18 uses public testnet for chain I/O while Store and relay stay on two local `logoscore`
hosts; org guest deploy is on chain. Step 18b (rc5 operational pin) is complete on `master`
([N16](docs/reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06)). On-chain
guest: Step 19 (`rfc-index` branch `feat/payment-streams-onchain-part`); see
[architecture.md](architecture.md).

## Program outcomes

| Outcome | Steps |
| --- | --- |
| Runnable integration demo (CLI, Store + eligibility) | 17 local LEZ ([N17](docs/reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06)); 18 public sequencer + local P2P (rc5 tooling) |
| LIP-155 on-chain spec (branch pin) | 19 (complete) |
| Developer journey — Track A (integrators, Store integration) | 20 (**next**) |
| Payment streams UI + user journey — Track B (protocol only, optional) | 21–22 (optional stretch; [N18](docs/reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)) |
| Public hosted Store provider (optional) | 23 |
| LEZ in-process harness (`program_tests`, rc5 `lee`) | 24 + 24b (complete) |
| Deterministic demo lifecycle (fresh stream, explicit prepare id) | 24c local complete — [step-24c-simplify-demo-flow.md](docs/plan/upcoming/step-24c-simplify-demo-flow.md) |

Step 25 (in-process demo coordinator module) is **won't fix**
([cancelled packet](docs/plan/cancelled/step-25-demo-coordination-module.md)).

Details in step packets under [`docs/plan/upcoming/`](docs/plan/upcoming/),
[`docs/plan/completed/`](docs/plan/completed/), and [`docs/plan/cancelled/`](docs/plan/cancelled/).

### Store query dependency

Steps 16–20 need Store query on our delivery forks, not on upstream `master` ([D2](docs/reference/decisions-and-notes.md#d2-delivery-module-hook-design), [N6](docs/reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)):
`logosdelivery_store_query` in `logos-delivery` (Step 15) and `storeQuery(...)` on
`logos-delivery-module` (Step 16). Upstream N6 is no longer a gate for Steps 14–20.
Step 14 (wire) and Step 15 (C hooks + `logosdelivery_store_query`) are complete on the
`logos-delivery` fork; Step 16 (bridge on `logos-delivery-module`) is complete on the module
fork (`9361e49` on `feat/payment-streams-store-eligibility`; eligibility bridge threading for Step 17).
Step 17 owns full-stack E2E
([N12](docs/reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).
Dual-host demo coordination stays in host scripts
([N17](docs/reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06)).
Step 18 Part B follows rc5 on `master` ([N16](docs/reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06)).

We do not maintain the retired exploratory PR branch
(`feat/liblogosdelivery-query-store` / old `queryStore` exposure) in payment-streams flakes.

### Delivery integration branches

Store eligibility work ships on integration branches forked from current upstream
`master` in the delivery repos. Do not branch from release tags (for example
`logos-delivery-module/v0.1.1` used by `logos-delivery-demo`); tags lag the wire and ABI
changes in Steps 14–16.

Default branch name (use the same string in both repos), in priority order if the name is taken
on a remote:

1. `feat/payment-streams-store-eligibility` (preferred)
2. `feat/lip155-store-eligibility`
3. `integration/payment-streams-store`

Record the chosen name in [`feature-branch-pins.md`](docs/feature-branch-pins.md) when creating
the branch. Both delivery repos must use the same string.

| Repo | Steps | Scope |
| --- | --- | --- |
| `logos-delivery` | 14–15 (done) | Store codec (tag `30`), `liblogosdelivery` hooks, `logosdelivery_store_query` |
| `logos-delivery-module` | 16 (done) | `storeQuery`, eligibility routing; `flake.nix` pins `logos-delivery` to `feat/payment-streams-store-eligibility` ([feature-branch-pins.md](docs/feature-branch-pins.md)) |

Suggested workflow: Steps 14–15 are on `logos-delivery` branch
`feat/payment-streams-store-eligibility`. Step 16 is on `logos-delivery-module` (same branch
name). The module repo points its flake `logos-delivery` input at that branch; commit
`flake.lock` when the delivery branch moves.
Optional explicit rev rows in [`feature-branch-pins.md`](docs/feature-branch-pins.md) for Step 17
needs reproducible `lgpm` installs. Wallet pins in that doc are unchanged.

## Onboarding

### Minimal (implementing Step N)

1. [`docs/AGENT-BRIEF.md`](docs/AGENT-BRIEF.md)
2. [`docs/integration-contracts.md`](docs/integration-contracts.md)
3. Step packet: [`docs/plan/upcoming/step-N.md`](docs/plan/upcoming/) or
   [`docs/plan/completed/step-N.md`](docs/plan/completed/) (Step 16) or runbook under `docs/step*.md`
4. [`logos-architecture-overview.md`](logos-architecture-overview.md) when boundaries are unclear

### Full (first time in repo)

Add: [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md), [`docs/step1-findings-scaffold-rpc.md`](docs/step1-findings-scaffold-rpc.md),
[`docs/feature-branch-pins.md`](docs/feature-branch-pins.md), LIP-155 (`rfc-index/docs/anoncomms/raw/payment-streams.md`, branch `feat/payment-streams-onchain-part` on `logos-co/logos-lips`).

## Components (sketch)

| Piece | Role |
| --- | --- |
| `lez-payment-streams-core` / `lez-payment-streams-ffi` | Policy, fold, proofs, instruction builders |
| `logos-payment-streams-module` | Universal Qt module, wallet via `logos_execution_zone` |
| `logos-delivery` / `liblogosdelivery` | Store protocol + eligibility hooks (14–15) |
| `logos-delivery-module` | `delivery_module` + routing (16) |
| `scripts/demo-e2e-local.sh`, `scripts/e2e/run_local_e2e.py` | Track A: dual-host Store integration orchestration (Step 17, Step 20) |
| `payment_streams_ui` (optional) | Track B: Basecamp UI over `payment_streams_module` only (Step 21) |
| `lgs` / `logoscore` / `lgpm` / `lm` | Localnet, host, install, introspection |

Detail: [`logos-architecture-overview.md`](logos-architecture-overview.md).

## Decisions and notes

| ID | Topic | Section |
| --- | --- | --- |
| D1 | Store wire tags | [decisions-and-notes.md](docs/reference/decisions-and-notes.md#d1-store-wire-format) |
| D2 | Delivery hooks | [D2](docs/reference/decisions-and-notes.md#d2-delivery-module-hook-design) |
| D3 | Wallet write path (491 / PR 19) | [D3](docs/reference/decisions-and-notes.md#d3-wallet-write-path) |
| D4 | Wallet module id | [D4](docs/reference/decisions-and-notes.md#d4-wallet-module-runtime-name) |
| D5 | PS module naming | [D5](docs/reference/decisions-and-notes.md#d5-new-module-naming) |
| D6 | Universal interface | [D6](docs/reference/decisions-and-notes.md#d6-universal-module-interface) |
| N1–N18 | Carry-forward notes | [decisions-and-notes.md](docs/reference/decisions-and-notes.md) (N16 rc5 operational pin; N17 script orchestration; N18 Track A vs B) |

Cross-step APIs without reading full D/N: [`docs/integration-contracts.md`](docs/integration-contracts.md).

## Step map

| Step | Focus | Status |
| --- | --- | --- |
| 1–5 | Rust FFI / core | Complete — `cargo test`, workspace crates |
| 6 | Store query via delivery | Closed — fork API in Steps 15–16 ([N6](docs/reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)) |
| 7 | Operator install | Runbook: [logos-runtime-guide.md](docs/logos-runtime-guide.md) Part 1 |
| 8 | Universal → Legacy probe | Complete — [step8](docs/step8-universal-legacy-probe-results.md) |
| 9 | Universal module bootstrap | Complete — runtime guide Part 2 |
| 10 | LEZ fixture + wallet | 10a [fixture](docs/step10a-local-chain-fixture.md), 10b [wallet](docs/step10b-wallet-runtime.md) |
| 11 | Module chain I/O | 11a–d runbooks; [N10](docs/reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions) |
| 12 | User eligibility | Complete — [step12](docs/step12-user-eligibility.md), `verify-step12-dod.sh` |
| 13 | Provider verify | Complete — [step13](docs/step13-provider-eligibility.md), `verify-step13-dod.sh` |
| 14 | Store wire (`logos-delivery`) | Complete — [step-14-normative.md](docs/plan/completed/step-14-normative.md) |
| 15 | `liblogosdelivery` hooks | Complete — [step-15-normative.md](docs/plan/completed/step-15-normative.md) |
| 16 | `delivery_module` routing | Complete — [step-16.md](docs/plan/completed/step-16.md) |
| 17 | E2E demo (local LEZ) | Complete — [step-17.md](docs/plan/completed/step-17.md), [step17-e2e-local.md](docs/step17-e2e-local.md) |
| 17b | Localnet snapshot restore | Complete — [step-17b](docs/plan/completed/step-17b-localnet-snapshot-restore.md), [N15](docs/reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19) |
| 18b | rc5 LEZ pin unify | Complete — [step-18b-rc5-unify-handoff.md](docs/plan/upcoming/step-18b-rc5-unify-handoff.md), [N16](docs/reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06) |
| 18 | Public sequencer E2E | Part B — [step-18-public-testnet-demo.md](docs/plan/upcoming/step-18-public-testnet-demo.md), [step18-public-sequencer-e2e.md](docs/step18-public-sequencer-e2e.md) |
| 19 | LIP-155 on-chain spec | Complete — [step-19](docs/plan/completed/step-19-lip155-onchain-spec.md) |
| 20 | Developer journey (Track A — Store integration) | After Step 18 DoD — [step-20-developer-journey.md](docs/plan/upcoming/step-20-developer-journey.md) |
| 21 | Payment streams Basecamp UI (Track B) | Optional stretch — [step-21-basecamp-ui.md](docs/plan/upcoming/step-21-basecamp-ui.md) |
| 22 | Payment streams UI journey (Track B) | Optional stretch — [step-22-ui-journey.md](docs/plan/upcoming/step-22-ui-journey.md) |
| 23 | Public Store provider | Optional — [step-23-public-store-provider.md](docs/plan/upcoming/step-23-public-store-provider.md) |
| 24 | LEZ `lee` harness @ 510 | Complete — [step-24](docs/plan/completed/step-24-lee-harness-upgrade.md) |
| 24b | Rust `lee` / guest unify on rc5 | Complete — [step-24b-rc5-rust-lee-unify.md](docs/plan/completed/step-24b-rc5-rust-lee-unify.md) |
| 24c | Simplify demo flow (fresh stream / explicit prepare) | Local complete — [step-24c-simplify-demo-flow.md](docs/plan/upcoming/step-24c-simplify-demo-flow.md) (testnet Phase 4 open) |
| 25 | Demo coordination Logos module | Won't fix — [cancelled/step-25](docs/plan/cancelled/step-25-demo-coordination-module.md), [N17](docs/reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06) |

Execution order: Steps through 17, 17b, 18b, 19, 24, 24b, and **24c local gate** are complete.
Parallel: Step 18 Part B DoD on testnet. Then Step 20 (Track A). Optional 23; optional stretch Steps 21–22 (Track B). Step 25 won't fix.
Entry: [`docs/AGENT-BRIEF.md`](docs/AGENT-BRIEF.md). Local demos:
[`demo-localnet-recovery.md`](docs/demo-localnet-recovery.md).

Doc index: [`docs/README.md`](docs/README.md). Plan layout: [`docs/plan/README.md`](docs/plan/README.md).

## Completed steps (summary)

Steps 1–11: landed in tree; verify via `make verify-step10a` … `verify-step12` and step runbooks under [`docs/`](docs/README.md).

### Step 12 — User eligibility (complete)

DoD: `prepareEligibilityForStoreQuery`, session keys, N8 canonical bytes, persistence v1,
`verify-step12-dod.sh`. Normative excerpt: [step-12-normative.md](docs/plan/completed/step-12-normative.md).

### Step 13 — Provider verify (complete)

DoD: `verifyEligibilityForStoreQuery`, FFI `parse_eligibility_proof_bytes`, persistence v2
`provider_acceptances`, `verify-step13-dod.sh`. Normative excerpt: [step-13-normative.md](docs/plan/completed/step-13-normative.md).

### Step 14 — Store wire (complete)

Normative excerpt and DoD: [step-14-normative.md](docs/plan/completed/step-14-normative.md).
Branch pin: [feature-branch-pins.md](docs/feature-branch-pins.md).

### Step 15 — `liblogosdelivery` hooks (complete)

DoD: C verifier/provider registration, inbound wrapper, N8 Nim serializer parity with Rust,
`logosdelivery_store_query`, C smoke on delivery fork.
Verify: [step-15-normative.md](docs/plan/completed/step-15-normative.md).

### Step 16 — `delivery_module` bridge (complete)

DoD: eligibility routing, async `storeQuery`, registration introspection on module fork.
Agent packet: [step-16.md](docs/plan/completed/step-16.md).

### Step 17 — E2E demo local LEZ (complete)

DoD: `make verify-step17-back-to-back` — restore run plus `SKIP_SEED=1` run (monotonic stream ids); paid Store, missing-proof reject, close then claim.
Runbook: [step17-e2e-local.md](docs/step17-e2e-local.md). Packet:
[step-17.md](docs/plan/completed/step-17.md).

### Step 17b — Localnet snapshot restore (complete)

DoD: funded snapshot + per-run stream; `make prepare-localnet` / `demo-localnet-prepare.sh`.
Packet: [step-17b-localnet-snapshot-restore.md](docs/plan/completed/step-17b-localnet-snapshot-restore.md).

### Step 19 — LIP-155 on-chain spec (complete)

Packet: [step-19-lip155-onchain-spec.md](docs/plan/completed/step-19-lip155-onchain-spec.md).

### Step 24 — LEZ `lee` harness (complete)

Packet: [step-24-lee-harness-upgrade.md](docs/plan/completed/step-24-lee-harness-upgrade.md).

### Step 18 — testnet integration (paused, 2026-06)

Continue with fully local demo (Step 17). Resume Step 18 when guest deploy size gate clears.
Guest `.bin` ~576 KiB vs public tx cap ~512 KiB; local `make deploy` unaffected.
Scaffolding on `feat/step18-public-testnet`. Do not block Step 20 on testnet.

### Step 18 — testnet integration (Part B active)

Public sequencer at `https://testnet.lez.logos.co/` (lez jsonrpsee). Org guest deploy complete
for ELF 576576 B; `program_id_hex` `79b1dd5c441caede8f9f82c30de637aba465f94cc43817b1105c8c48c77d0fc9`.
Remaining: read smoke, per-operator bootstrap, `make verify-step18`. Packet:
[step-18-public-testnet-demo.md](docs/plan/upcoming/step-18-public-testnet-demo.md).

## Upcoming steps (pointers)

- [Step 20](docs/plan/upcoming/step-20-developer-journey.md) — Track A developer journey (**next**)
- [Step 21](docs/plan/upcoming/step-21-basecamp-ui.md) — Track B payment streams UI (optional)
- [Step 22](docs/plan/upcoming/step-22-ui-journey.md) — Track B UI journey doc (optional, after 21)
- [Step 18](docs/plan/upcoming/step-18-public-testnet-demo.md) — public LEZ E2E (paused)
- [Step 23](docs/plan/upcoming/step-23-public-store-provider.md) — optional hosted provider

Closed: [Step 25 cancelled](docs/plan/cancelled/step-25-demo-coordination-module.md).

Completed (pointers): [Step 17](docs/plan/completed/step-17.md),
[Step 17b](docs/plan/completed/step-17b-localnet-snapshot-restore.md),
[Step 19](docs/plan/completed/step-19-lip155-onchain-spec.md),
[Step 24](docs/plan/completed/step-24-lee-harness-upgrade.md).

## Verify scripts (logoscore path)

| Script | Step |
| --- | --- |
| `./scripts/verify-step10a-dod.sh` | 10a fixture |
| `./scripts/verify-step10b-dod.sh` | 10b wallet |
| `./scripts/verify-step11a-dod.sh` … `11d` | 11a–d |
| `./scripts/verify-step12-dod.sh` | 12 |
| `./scripts/verify-step13-dod.sh` | 13 |
| `./scripts/demo-localnet-prepare.sh` | 17b / 17 fixture |
| `./scripts/demo-e2e-local.sh` | 17 dual-host E2E |
| `make verify-step17-back-to-back` | 17 + 24c — restore run then continuation on same ledger |

`make verify-step17` → [scripts/demo-e2e-local.sh](scripts/demo-e2e-local.sh) →
[scripts/e2e/run_local_e2e.py](scripts/e2e/run_local_e2e.py) ([N17](docs/reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06),
[step17-e2e-local.md](docs/step17-e2e-local.md)). `make prepare-localnet` →
[scripts/demo-localnet-prepare.sh](scripts/demo-localnet-prepare.sh).
