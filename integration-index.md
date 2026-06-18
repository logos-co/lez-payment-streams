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

MVP scope: LIP-155 transparent vaults, single user and single provider on one local sequencer,
paid Store mode on the provider. The on-chain SPEL program and core crate are a separate concern;
see [architecture.md](architecture.md).

### Store query dependency

Steps 16–17 need Store query on our delivery forks, not on upstream `master` ([D2](docs/reference/decisions-and-notes.md#d2-delivery-module-hook-design), [N6](docs/reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)):
`logosdelivery_store_query` in `logos-delivery` (Step 15) and `storeQuery(...)` on
`logos-delivery-module` (Step 16). Upstream N6 is no longer a gate for Steps 14–17.
Step 14 (wire) and Step 15 (C hooks + `logosdelivery_store_query`) are complete on the
`logos-delivery` fork; Step 16 (bridge on `logos-delivery-module`) is complete on the module
fork (`bf104a6bfde35ce4fcae5081278d1996ebf5e3c1` on `feat/payment-streams-store-eligibility`).
Step 17 owns full-stack E2E
([N12](docs/reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).

We do not maintain the retired exploratory PR branch
(`feat/liblogosdelivery-query-store` / old `queryStore` exposure) in payment-streams flakes.

### Delivery integration branches

Store eligibility work ships on integration branches forked from current upstream
`master` in the delivery repos. Do not branch from release tags (for example
`logos-delivery-module/v0.1.1` used by `logos-delivery-demo`); tags lag the wire and ABI
changes in Steps 14–16.

Default branch name (use the same string in both repos):
`feat/payment-streams-store-eligibility`.
If that name is taken on a remote, use `feat/lip155-store-eligibility` or
`integration/payment-streams-store` instead, but keep both delivery repos aligned on one name.

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
| N1–N12 | Carry-forward notes | [decisions-and-notes.md](docs/reference/decisions-and-notes.md) (N8 canonical Store bytes; N3a–N3c, N12 Step 16) |

Cross-step APIs without reading full D/N: [`docs/integration-contracts.md`](docs/integration-contracts.md).

## Step map

| Step | Focus | Status |
| --- | --- | --- |
| 1–5 | Rust FFI / core | Complete — `cargo test`, workspace crates |
| 6 | Store query via delivery | Closed — wait-upstream path retired; fork API in Steps 15–16 ([N6](docs/reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)) |
| 7 | Operator install | Runbook: [logos-runtime-guide.md](docs/logos-runtime-guide.md) Part 1 |
| 8 | Universal → Legacy probe | Complete — [step8](docs/step8-universal-legacy-probe-results.md) |
| 9 | Universal module bootstrap | Complete — runtime guide Part 2 |
| 10 | LEZ fixture + wallet | 10a [fixture](docs/step10a-local-chain-fixture.md), 10b [wallet](docs/step10b-wallet-runtime.md) |
| 11 | Module chain I/O | 11a–d runbooks; [N10](docs/reference/decisions-and-notes.md#n10-step-11b-module-writes-decisions) |
| 12 | User eligibility | Complete — [step12](docs/step12-user-eligibility.md), `verify-step12-dod.sh` |
| 13 | Provider verify | Complete — [step13](docs/step13-provider-eligibility.md), `verify-step13-dod.sh` |
| 14 | Store wire (`logos-delivery`) | Complete — `d033a49364f1dda4ee4e5467d828738d01eb7d4c`; [step-14-normative.md](docs/plan/completed/step-14-normative.md) |
| 15 | `liblogosdelivery` hooks | Complete — `e59319d8648c3c3ea9384c592728d5738f623a13`; [step-15-normative.md](docs/plan/completed/step-15-normative.md) |
| 16 | `delivery_module` routing | Complete — `bf104a6bfde35ce4fcae5081278d1996ebf5e3c1`; [step-16.md](docs/plan/upcoming/step-16.md) ([N12](docs/reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)) |
| 17 | E2E demo | Upcoming — full stack script [step-17.md](docs/plan/upcoming/step-17.md) |
| 18 | Basecamp UI | Optional — [step-18.md](docs/plan/upcoming/step-18.md) |

Execution order: Steps 12, 11d, 13, 14, 15, and 16 are complete. Next: 17 on delivery forks
([`docs/AGENT-BRIEF.md`](docs/AGENT-BRIEF.md)). Local demos:
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
`logosdelivery_store_query`, C smoke (`make logosdelivery_eligibility_smoke`) on
`logos-delivery` branch `feat/payment-streams-store-eligibility`.
Verify: [step-15-normative.md](docs/plan/completed/step-15-normative.md).

### Step 16 — `delivery_module` bridge (complete)

DoD: eligibility routing, async `storeQuery`, registration introspection; unit tests on
`logos-delivery-module` branch `feat/payment-streams-store-eligibility`.
Agent packet (decision table + bridge DoD): [step-16.md](docs/plan/upcoming/step-16.md).
Branch pin: [feature-branch-pins.md](docs/feature-branch-pins.md).

## Upcoming steps (pointers)

Do not duplicate full DoD here — read the packet:

- [Step 17](docs/plan/upcoming/step-17.md) — two logical hosts, paid Store mode, demo script ([N12](docs/reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18))
- [Step 18](docs/plan/upcoming/step-18.md) — optional `ui_qml` plugin

## Verify scripts (logoscore path)

Step 15 DoD is verified in the `logos-delivery` fork only
([step-15-normative.md](docs/plan/completed/step-15-normative.md)), not via a script in this repo.

| Script | Step |
| --- | --- |
| `./scripts/verify-step10a-dod.sh` | 10a fixture |
| `./scripts/verify-step10b-dod.sh` | 10b wallet |
| `./scripts/verify-step11a-dod.sh` … `11d` | 11a–d |
| `./scripts/verify-step12-dod.sh` | 12 (`REQUIRE_STREAM_PROOF=1` strict) |
| `./scripts/verify-step13-dod.sh` | 13 (`VERIFY_LOGOSCORE=1` cross-test) |

`make verify-step12`, `make verify-step13` wrap the Step 12–13 scripts.
