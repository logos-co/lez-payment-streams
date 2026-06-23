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
provider. Step 17 uses a local LEZ sequencer; Step 18 uses public testnet v0.2 for chain access
while Store and relay stay on two local `logoscore` hosts. Optional Step 23 hosts a public paid
Store provider on the mesh. The on-chain guest is documented in Step 19 (`rfc-index` branch
`feat/payment-streams-onchain-part`); see [architecture.md](architecture.md).

## Program outcomes

| Outcome | Steps |
| --- | --- |
| Runnable demo (CLI) | 17 local LEZ gate; 18 public sequencer + local P2P; 25 in-process demo coordinator module |
| LIP-155 on-chain spec (branch pin) | 19 (complete) |
| Developer journey (logos-docs doc packet) | 20 (depends on 25) |
| Basecamp UI + UI journey (optional) | 21–22 (depend on 25) |
| Public hosted Store provider (optional) | 23 |
| LEZ in-process harness (`lee` @ 510, `program_tests`) | 24 (complete) |

Details in step packets under [`docs/plan/upcoming/`](docs/plan/upcoming/) and
[`docs/plan/completed/`](docs/plan/completed/) for Steps 19 and 24; do not duplicate DoD here.

### Store query dependency

Steps 16–20 need Store query on our delivery forks, not on upstream `master` ([D2](docs/reference/decisions-and-notes.md#d2-delivery-module-hook-design), [N6](docs/reference/decisions-and-notes.md#n6-delivery-module-store-query-exposure)):
`logosdelivery_store_query` in `logos-delivery` (Step 15) and `storeQuery(...)` on
`logos-delivery-module` (Step 16). Upstream N6 is no longer a gate for Steps 14–20.
Step 14 (wire) and Step 15 (C hooks + `logosdelivery_store_query`) are complete on the
`logos-delivery` fork; Step 16 (bridge on `logos-delivery-module`) is complete on the module
fork (`9361e49` on `feat/payment-streams-store-eligibility`; eligibility bridge threading for Step 17).
Step 17 owns full-stack E2E
([N12](docs/reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).
Step 25 retires the external Python orchestrator and moves demo coordination into an in-process
Logos module; until Step 25 lands, `scripts/e2e/run_local_e2e.py` remains the Step 17/18 CI path.

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
| `payment_streams_demo_coordinator` | In-process demo coordinator (Step 25, upcoming); replaces `scripts/e2e/run_local_e2e.py` |
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
| N1–N15 | Carry-forward notes | [decisions-and-notes.md](docs/reference/decisions-and-notes.md) (N8 canonical Store bytes; N3a–N3c, N12 Step 16; N15 Step 17b snapshot) |

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
| 16 | `delivery_module` routing | Complete — `bf104a6…`; [step-16.md](docs/plan/completed/step-16.md) |
| 17 | E2E demo (local LEZ) | Complete — [step-17.md](docs/plan/completed/step-17.md), [step17-e2e-local.md](docs/step17-e2e-local.md), [N13](docs/reference/decisions-and-notes.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18), [N14](docs/reference/decisions-and-notes.md#n14-step-17-paid-query-verify-rejects-2026-06-19) |
| 17b | Localnet snapshot restore | Complete — [step-17b-localnet-snapshot-restore.md](docs/plan/completed/step-17b-localnet-snapshot-restore.md), [N15](docs/reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19); `demo-localnet-prepare.sh`, `make prepare-localnet`; `FULL_RESET=1` after guest ImageID change |
| 18 | Public sequencer E2E (local Store) | Upcoming — [step-18-public-testnet-demo.md](docs/plan/upcoming/step-18-public-testnet-demo.md) |
| 19 | LIP-155 on-chain spec | Complete — [step-19-lip155-onchain-spec.md](docs/plan/completed/step-19-lip155-onchain-spec.md) (`feat/payment-streams-onchain-part` @ `345c8eef`) |
| 20 | Developer journey doc packet | Upcoming (depends on 25) — [step-20-developer-journey.md](docs/plan/upcoming/step-20-developer-journey.md) |
| 21 | Basecamp UI | Optional (depends on 25) — [step-21-basecamp-ui.md](docs/plan/upcoming/step-21-basecamp-ui.md) |
| 22 | UI journey doc packet | Optional (depends on 21) — [step-22-ui-journey.md](docs/plan/upcoming/step-22-ui-journey.md) |
| 23 | Public Store provider | Optional — [step-23-public-store-provider.md](docs/plan/upcoming/step-23-public-store-provider.md) |
| 24 | LEZ `lee` harness (NSSA → 510) | Complete — [step-24-lee-harness-upgrade.md](docs/plan/completed/step-24-lee-harness-upgrade.md) |
| 25 | Demo coordination Logos module | Upcoming — [step-25-demo-coordination-module.md](docs/plan/upcoming/step-25-demo-coordination-module.md) |

Execution order: Steps 12, 11d, 13, 14, 15, 16, 17, 17b, 19, and 24 are complete. Next: 18,
then 25, then 20. Step 20 depends on 25 (the developer journey documents the coordinator
module's `runDemo` entry). Optional 21–22 after 25 if shipping UI docs; optional 23 if shipping
a hosted paid-Store provider. Entry:
[`docs/AGENT-BRIEF.md`](docs/AGENT-BRIEF.md). Local demos:
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
Agent packet: [step-16.md](docs/plan/completed/step-16.md). Locked rev:
[feature-branch-pins.md](docs/feature-branch-pins.md).

### Step 17 — E2E demo local LEZ (complete)

DoD: `make verify-step17` — paid Store, missing-proof reject, claim `tx_hash`.
Runbook: [step17-e2e-local.md](docs/step17-e2e-local.md). Packet:
[step-17.md](docs/plan/completed/step-17.md). Prepare via Step 17b; after guest rebuild run
`FULL_RESET=1` before E2E ([N13](docs/reference/decisions-and-notes.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18), [N14](docs/reference/decisions-and-notes.md#n14-step-17-paid-query-verify-rejects-2026-06-19)).

### Step 17b — Localnet snapshot restore (complete)

DoD: funded snapshot + per-run stream `0`; `make prepare-localnet` / `demo-localnet-prepare.sh`.
After guest ImageID or LEZ pin change: `FULL_RESET=1` rebuild required ([N15](docs/reference/decisions-and-notes.md#n15-step-17b-localnet-snapshot-restore-2026-06-19)).
Packet: [step-17b-localnet-snapshot-restore.md](docs/plan/completed/step-17b-localnet-snapshot-restore.md).

### Step 19 — LIP-155 on-chain spec (complete)

DoD: `## On-Chain Protocol` and `## Implementation Considerations` on `logos-lips` branch
`feat/payment-streams-onchain-part` (`345c8eef`); merge to spec `main` not required for
integration closure. Packet: [step-19-lip155-onchain-spec.md](docs/plan/completed/step-19-lip155-onchain-spec.md).

### Step 24 — LEZ `lee` harness (complete)

DoD: single LEZ rev `62d9ba10…`, `lee`/`lee_core` host harness, transparent `program_tests`,
vendored SPEL on `lee_core`; verify 10a/12/13 and Step 17 E2E. Packet:
[step-24-lee-harness-upgrade.md](docs/plan/completed/step-24-lee-harness-upgrade.md).

## Upcoming steps (pointers)

Do not duplicate full DoD here — read the packet:

- [Step 18](docs/plan/upcoming/step-18-public-testnet-demo.md) — public LEZ sequencer; local dual-host Store E2E
- [Step 25](docs/plan/upcoming/step-25-demo-coordination-module.md) — in-process Logos module replacing the external Python orchestrator
- [Step 23](docs/plan/upcoming/step-23-public-store-provider.md) — optional hosted Store provider on public mesh
- [Step 20](docs/plan/upcoming/step-20-developer-journey.md) — logos-docs developer journey (depends on 25)
- [Step 21](docs/plan/upcoming/step-21-basecamp-ui.md) — optional `ui_qml` plugin (depends on 25)
- [Step 22](docs/plan/upcoming/step-22-ui-journey.md) — optional UI journey doc packet (depends on 21)

Completed (pointers): [Step 17](docs/plan/completed/step-17.md),
[Step 17b](docs/plan/completed/step-17b-localnet-snapshot-restore.md) (after guest rebuild use
`FULL_RESET=1` prepare),
[Step 19](docs/plan/completed/step-19-lip155-onchain-spec.md),
[Step 24](docs/plan/completed/step-24-lee-harness-upgrade.md).

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
| `./scripts/demo-localnet-prepare.sh` | 17b / 17 fixture (restore + stream) |
| `./scripts/demo-e2e-local.sh` | 17 (dual-host Store E2E) |

`make verify-step12`, `make verify-step13` wrap the Step 12–13 scripts.
Step 17: `make verify-step17` → [scripts/demo-e2e-local.sh](scripts/demo-e2e-local.sh) (see [step17-e2e-local.md](docs/step17-e2e-local.md)). Fixture prepare (17b): `make prepare-localnet` → [scripts/demo-localnet-prepare.sh](scripts/demo-localnet-prepare.sh). Installs all modules via nix `#lgx` + `lgpm`; optional `liblogosdelivery` overlay or hermetic `SKIP_LIBLOGOSDELIVERY_OVERLAY=1` ([N13](docs/reference/decisions-and-notes.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18), [hermetic run](docs/step17-e2e-local.md#hermetic-run-hand-off)).
Step 25 (upcoming) retargets `make verify-step17` and `make verify-step18` to invoke `payment_streams_demo_coordinator.runDemo` instead of the Python orchestrator; artifact path and phase row shape stay stable.
