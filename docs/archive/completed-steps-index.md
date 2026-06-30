# Completed steps index

Archived step map and completed step summaries.
Replaced by the forward-looking [plan/index.md](../plan/index.md).
This file is for historical reference.

Agent read order: [AGENTS.md](../../AGENTS.md).
Product gates: [verification-matrix.md](../reference/verification-matrix.md).

## Full step map

| Step | Focus | Status |
| --- | --- | --- |
| 1-5 | Rust FFI / core | Complete -- `cargo test`, workspace crates |
| 6 | Store query via delivery | Closed -- fork API in Steps 15-16 ([N6](../reference/integration-decisions.md#n6-delivery-module-store-query-exposure)) |
| 7 | Operator install | Runbook: [logos-runtime-guide.md](steps/logos-runtime-guide.md) Part 1 |
| 8 | Universal to Legacy probe | Complete -- [step8](steps/universal-legacy-probe-results.md) |
| 9 | Universal module bootstrap | Complete -- runtime guide Part 2 |
| 10 | LEZ fixture + wallet | 10a [fixture](steps/local-chain-fixture.md), 10b [wallet](steps/wallet-runtime-runbook.md) |
| 11 | Module chain I/O | 11a-d runbooks; [N10](../reference/integration-decisions.md#n10-step-11b-module-writes-decisions) |
| 12 | User eligibility | Complete -- [step12](steps/user-eligibility-runbook.md), `verify-step12-dod.sh` |
| 13 | Provider verify | Complete -- [step13](steps/provider-eligibility-runbook.md), `verify-step13-dod.sh` |
| 14 | Store wire (`logos-delivery`) | Complete -- [step-14-normative.md](../plan/completed/step-14-normative.md) |
| 15 | `liblogosdelivery` hooks | Complete -- [step-15-normative.md](../plan/completed/step-15-normative.md) |
| 16 | `delivery_module` routing | Complete -- [step-16.md](../plan/completed/step-16.md) |
| 17 | E2E demo (local LEZ) | Complete -- [step-17.md](../plan/completed/step-17.md), [steps/local-store-dual-host-runbook.md](steps/local-store-dual-host-runbook.md) |
| 17b | Localnet snapshot restore | Complete -- [step-17b](../plan/completed/step-17b-localnet-snapshot-restore.md) |
| 18b | rc5 LEZ pin unify | Complete -- [step-18b-rc5-unify-handoff.md](../plan/completed/step-18b-rc5-unify-handoff.md) |
| 18 | Public sequencer E2E | Complete -- [step-18-public-testnet-demo.md](../plan/completed/step-18-public-testnet-demo.md), [steps/public-sequencer-store-runbook.md](steps/public-sequencer-store-runbook.md) |
| 19 | LIP-155 on-chain spec | Complete -- [step-19](../plan/completed/step-19-lip155-onchain-spec.md) |
| 20 | Developer Journey | **Active** -- see [plan/index.md](../plan/index.md) |
| 21 | User Journey: Basecamp UI plugin (optional) | Optional -- see [plan/index.md](../plan/index.md) |
| 22 | User Journey: doc packet (CLI-based) | **Active** -- see [plan/index.md](../plan/index.md) |
| 23 | Public Store provider | Optional -- see [plan/index.md](../plan/index.md) |
| 24 | LEZ `lee` harness @ 510 | Complete -- [step-24](../plan/completed/step-24-lee-harness-upgrade.md) |
| 24b | Rust `lee` / guest unify on rc5 | Complete -- [step-24b-rc5-rust-lee-unify.md](../plan/completed/step-24b-rc5-rust-lee-unify.md) |
| 24c | Simplify demo flow | Complete -- [step-24c-simplify-demo-flow.md](../plan/completed/step-24c-simplify-demo-flow.md) |
| 25 | Demo coordination Logos module | Won't fix -- [step-25](../plan/cancelled/step-25-demo-coordination-module.md) |
| 26 | TestNet v0.2 migration | Complete -- [step-26-testnet-v02-migration.md](../plan/completed/step-26-testnet-v02-migration.md) |
| 27 | Claim fix and verification | Active -- see [plan/index.md](../plan/index.md) |
| 28 | User Journey on TestNet v0.2 | Active -- see [plan/index.md](../plan/index.md) |
| 29 | E2E script UX enhancement | Active -- see [plan/index.md](../plan/index.md) |

## Completed steps -- summary

Steps 1-11: landed in tree; historical DoD via archived `make verify-step10a` ... targets;
product gates in [verification-matrix.md](../reference/verification-matrix.md).

### Step 12 -- User eligibility

DoD: `prepareEligibilityForStoreQuery`, session keys, N8 canonical bytes, persistence v1,
`verify-step12-dod.sh`. Normative excerpt: [step-12-normative.md](../plan/completed/step-12-normative.md).

### Step 13 -- Provider verify

DoD: `verifyEligibilityForStoreQuery`, FFI `parse_eligibility_proof_bytes`, persistence v2
`provider_acceptances`, `verify-step13-dod.sh`. Normative excerpt:
[step-13-normative.md](../plan/completed/step-13-normative.md).

### Step 14 -- Store wire

Normative excerpt and DoD: [step-14-normative.md](../plan/completed/step-14-normative.md).
Branch pin: [feature-branch-pins.md](../reference/feature-branch-pins.md).

### Step 15 -- `liblogosdelivery` hooks

DoD: C verifier/provider registration, inbound wrapper, N8 Nim serializer parity with Rust,
`logosdelivery_store_query`, C smoke on delivery fork.
Verify: [step-15-normative.md](../plan/completed/step-15-normative.md).

### Step 16 -- `delivery_module` bridge

DoD: eligibility routing, async `storeQuery`, registration introspection on module fork.
Agent packet: [step-16.md](../plan/completed/step-16.md).

### Step 17 -- E2E demo local LEZ

DoD: `make verify-step17-back-to-back` -- restore run plus `SKIP_SEED=1` run
(monotonic stream ids); paid Store, missing-proof reject, close then claim.
Runbook: [steps/local-store-dual-host-runbook.md](steps/local-store-dual-host-runbook.md).
Packet: [step-17.md](../plan/completed/step-17.md).

### Step 17b -- Localnet snapshot restore

DoD: funded snapshot + per-run stream; `make prepare-localnet` / `scripts/e2e.sh local prepare`.
Packet: [step-17b-localnet-snapshot-restore.md](../plan/completed/step-17b-localnet-snapshot-restore.md).

### Step 19 -- LIP-155 on-chain spec

Packet: [step-19-lip155-onchain-spec.md](../plan/completed/step-19-lip155-onchain-spec.md).

### Step 24 -- LEZ `lee` harness

Packet: [step-24-lee-harness-upgrade.md](../plan/completed/step-24-lee-harness-upgrade.md).

### Step 18 -- testnet integration (paused 2026-06)

Continue with fully local demo (Step 17). Resume Step 18 when guest deploy size gate clears.
Guest `.bin` ~576 KiB vs public tx cap ~512 KiB; local `make deploy` unaffected.
Scaffolding on `feat/step18-public-testnet`. Do not block Step 20 on testnet.

### Step 18 -- testnet integration (Part B active)

Public sequencer at `https://testnet.lez.logos.co/` (lez jsonrpsee).
Org guest deploy complete for ELF 576576 B; `program_id_hex`
`79b1dd5c441caede8f9f82c30de637aba465f94cc43817b1105c8c48c77d0fc9`.
Remaining: read smoke, per-operator bootstrap, `make verify-step18`. Packet:
[step-18-public-testnet-demo.md](../plan/completed/step-18-public-testnet-demo.md).

## Verify scripts (logoscore path)

Active verification uses the unified script stack
(`scripts/e2e.sh`, `scripts/lifecycle.sh`, `scripts/fixture.sh`, `scripts/lib/common.sh`).
The 2x2 matrix (flow x chain) is documented in
[`verification-matrix.md`](../reference/verification-matrix.md).

| Entry | Cell | Step |
| --- | --- | --- |
| `make verify-module-local` | User Journey (module only) × localnet | 11 / 24c |
| `make verify-step17` | Developer Journey (Store) × localnet | 17 |
| `make verify-step17-back-to-back` | Developer Journey × localnet (continuation on same ledger) | 17 + 24c |
| `make verify-step18` | Developer Journey (Store) × testnet (advanced) | 18 |
| User Journey × testnet | future work (unsupported) | -- |

`make verify-step17` to [scripts/e2e.sh](../../scripts/e2e.sh) `local run` to
[scripts/e2e/run_local_e2e.py](../../scripts/e2e/run_local_e2e.py)
([N17](../reference/integration-decisions.md#n17-demo-orchestration-stays-external-script-2026-06),
[store-integration/README.md](../store-integration/README.md)).
`make verify-module-local` to [scripts/e2e.sh](../../scripts/e2e.sh)
`MODE=module local run` to [scripts/module-e2e-local.sh](../../scripts/module-e2e-local.sh).
`make prepare-localnet` to [scripts/e2e.sh](../../scripts/e2e.sh) `local prepare`.

Completed-step DoD scripts (10a-13) are archived under
[`scripts/archive/`](../../scripts/archive/) and run via the matching `make verify-step1*`
targets; they are historical and not part of the external runbooks.

## Historical runbooks

Step-scoped operator files under `docs/archive/steps/` for reference:

| Area | Examples |
| --- | --- |
| Fixture and wallet | [steps/local-chain-fixture.md](steps/local-chain-fixture.md), [steps/wallet-runtime-runbook.md](steps/wallet-runtime-runbook.md) |
| Module chain I/O | [steps/module-chain-reads-runbook.md](steps/module-chain-reads-runbook.md) through [steps/wallet-510-runbook.md](steps/wallet-510-runbook.md) |
| Eligibility (not external product path) | [steps/user-eligibility-runbook.md](steps/user-eligibility-runbook.md), [steps/provider-eligibility-runbook.md](steps/provider-eligibility-runbook.md) |
| Store E2E detail | [steps/local-store-dual-host-runbook.md](steps/local-store-dual-host-runbook.md), [steps/public-sequencer-store-runbook.md](steps/public-sequencer-store-runbook.md) |
| Runtime install spine | [logos-runtime-guide.md](steps/logos-runtime-guide.md) |
| Discovery / policy | [steps/scaffold-rpc-findings.md](steps/scaffold-rpc-findings.md), [steps/policy-implementor-notes.md](steps/policy-implementor-notes.md), [steps/universal-legacy-probe-results.md](steps/universal-legacy-probe-results.md) |

## Runbook rename ledger

Historical `docs/step*.md` files live under `docs/archive/steps/` with descriptive names.

| Former path | Archive path |
| --- | --- |
| `step17-e2e-local.md` | [steps/local-store-dual-host-runbook.md](steps/local-store-dual-host-runbook.md) |
| `step18-public-sequencer-e2e.md` | [steps/public-sequencer-store-runbook.md](steps/public-sequencer-store-runbook.md) |
| `step10a-local-chain-fixture.md` | [steps/local-chain-fixture.md](steps/local-chain-fixture.md) |
| `step10b-wallet-runtime.md` | [steps/wallet-runtime-runbook.md](steps/wallet-runtime-runbook.md) |
| `step11a`-`step11d` | `module-chain-reads-runbook.md`, `module-chain-writes-runbook.md`, `sign-public-payload-runbook.md`, `wallet-510-runbook.md` |
| `step12` / `step13` | `user-eligibility-runbook.md`, `provider-eligibility-runbook.md` |
| `logos-runtime-guide.md` | [steps/logos-runtime-guide.md](steps/logos-runtime-guide.md) |
| `demo-localnet-recovery.md` | [operator/localnet-recovery.md](operator/localnet-recovery.md) |

## Recovery

[operator/localnet-recovery.md](operator/localnet-recovery.md)