# Payment streams integration index

Canonical short index for the Logos payment-streams Store demo. Agents: repo root
[`AGENTS.md`](AGENTS.md), then [`docs/AGENT-BRIEF.md`](docs/AGENT-BRIEF.md). Operators: step runbooks in [`docs/README.md`](docs/README.md).

The full pre-split plan lives in [`docs/archive/integration-plan-full.md`](docs/archive/integration-plan-full.md).
[`integration-plan.md`](integration-plan.md) redirects here.

## Task summary

Logos Delivery Store requests may carry a payment-stream eligibility proof; the provider verifies
against LEZ on-chain state before serving. Crypto and policy live in Rust (`lez-payment-streams-core`,
`lez-payment-streams-ffi`); orchestration in Universal `payment_streams_module`; Store wire and
`liblogosdelivery` hooks in the delivery repos (Steps 14–16).

MVP scope: LIP-155 transparent vaults, single user and single provider on one local sequencer,
paid Store mode on the provider. The on-chain SPEL program and core crate are a separate concern;
see [architecture.md](architecture.md).

### Store query dependency

Steps 16–18 depend on upstream Store query on `logos-delivery-module` `master` ([N6](docs/reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)).
Active integration work on Steps 14–15 proceeds in parallel. We do not maintain a local
`queryStore` PR fork.

## Onboarding

### Minimal (implementing Step N)

1. [`docs/AGENT-BRIEF.md`](docs/AGENT-BRIEF.md)
2. [`docs/integration-contracts.md`](docs/integration-contracts.md)
3. Step packet: [`docs/plan/upcoming/step-N.md`](docs/plan/upcoming/) or completed runbook under `docs/step*.md`
4. [`logos-architecture-overview.md`](logos-architecture-overview.md) when boundaries are unclear

### Full (first time in repo)

Add: [`docs/logos-runtime-guide.md`](docs/logos-runtime-guide.md), [`docs/step1-findings-scaffold-rpc.md`](docs/step1-findings-scaffold-rpc.md),
[`docs/feature-branch-pins.md`](docs/feature-branch-pins.md), LIP-155 (`rfc-index/.../payment-streams.md`).

## Components (sketch)

| Piece | Role |
| --- | --- |
| `lez-payment-streams-core` / `lez-payment-streams-ffi` | Policy, fold, proofs, instruction builders |
| `logos-payment-streams-module` | Universal Qt module, wallet via `logos_execution_zone` |
| `logos-delivery` / `liblogosdelivery` | Store protocol + eligibility hooks (14–15) |
| `logos-delivery-module` | `delivery_module` + routing (16) |
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
| N1–N11 | Carry-forward notes | [decisions-and-notes.md](docs/reference/decisions-and-notes.md) (N8 = canonical Store bytes) |

Cross-step APIs without reading full D/N: [`docs/integration-contracts.md`](docs/integration-contracts.md).

## Step map

| Step | Focus | Status |
| --- | --- | --- |
| 1–5 | Rust FFI / core | Complete — `cargo test`, workspace crates |
| 6 | Store query via delivery | Closed — wait upstream (N6) |
| 7 | Operator install | Runbook: [logos-runtime-guide.md](docs/logos-runtime-guide.md) Part 1 |
| 8 | Universal → Legacy probe | Complete — [step8](docs/step8-universal-legacy-probe-results.md) |
| 9 | Universal module bootstrap | Complete — runtime guide Part 2 |
| 10 | LEZ fixture + wallet | 10a [fixture](docs/step10a-local-chain-fixture.md), 10b [wallet](docs/step10b-wallet-runtime.md) |
| 11 | Module chain I/O | 11a–d runbooks; [N10](docs/reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions) |
| 12 | User eligibility | Complete — [step12](docs/step12-user-eligibility.md), `verify-step12-dod.sh` |
| 13 | Provider verify | Complete — [step13](docs/step13-provider-eligibility.md), `verify-step13-dod.sh` |
| 14 | Store wire (`logos-delivery`) | Upcoming — [step-14.md](docs/plan/upcoming/step-14.md) |
| 15 | `liblogosdelivery` hooks | Upcoming — [step-15.md](docs/plan/upcoming/step-15.md) |
| 16 | `delivery_module` routing | Upcoming — [step-16.md](docs/plan/upcoming/step-16.md) |
| 17 | E2E demo | Upcoming — [step-17.md](docs/plan/upcoming/step-17.md) |
| 18 | Basecamp UI | Optional — [step-18.md](docs/plan/upcoming/step-18.md) |

Execution order: Steps 12, 11d, and 13 are complete. Next: 14 → 15 → 16 (then 17 when Store query
lands). Local demos: [`demo-localnet-recovery.md`](docs/demo-localnet-recovery.md).

Doc index: [`docs/README.md`](docs/README.md). Plan layout: [`docs/plan/README.md`](docs/plan/README.md).

## Completed steps (summary)

Steps 1–11: landed in tree; verify via `make verify-step10a` … `verify-step12` and step runbooks.
Historical step prose: [archive](docs/archive/integration-plan-full.md#integration-steps).

### Step 12 — User eligibility (complete)

DoD: `prepareEligibilityForStoreQuery`, session keys, N8 canonical bytes, persistence v1,
`verify-step12-dod.sh`. Normative excerpt: [step-12-normative.md](docs/plan/completed/step-12-normative.md).

### Step 13 — Provider verify (complete)

DoD: `verifyEligibilityForStoreQuery`, FFI `parse_eligibility_proof_bytes`, persistence v2
`provider_acceptances`, `verify-step13-dod.sh`. Normative excerpt: [step-13-normative.md](docs/plan/completed/step-13-normative.md).

## Upcoming steps (pointers)

Do not duplicate full DoD here — read the packet:

- [Step 14](docs/plan/upcoming/step-14.md) — `logos-delivery` codec
- [Step 15](docs/plan/upcoming/step-15.md) — C verifier/provider callbacks
- [Step 16](docs/plan/upcoming/step-16.md) — `setEligibilityVerifier` / `setEligibilityProvider`
- [Step 17](docs/plan/upcoming/step-17.md) — two logical hosts, paid Store mode
- [Step 18](docs/plan/upcoming/step-18.md) — optional `ui_qml` plugin

## Verify scripts (logoscore path)

| Script | Step |
| --- | --- |
| `./scripts/verify-step10a-dod.sh` | 10a fixture |
| `./scripts/verify-step10b-dod.sh` | 10b wallet |
| `./scripts/verify-step11a-dod.sh` … `11d` | 11a–d |
| `./scripts/verify-step12-dod.sh` | 12 (`REQUIRE_STREAM_PROOF=1` strict) |
| `./scripts/verify-step13-dod.sh` | 13 (`VERIFY_LOGOSCORE=1` cross-test) |

`make verify-step12`, `make verify-step13` wrap the Step 12–13 scripts.
