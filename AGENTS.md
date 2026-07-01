# lez-payment-streams — agent context

Human entry: [README.md](README.md) → [docs/README.md](docs/README.md) → pillar README.

LIP-155 on-chain program, `payment_streams_module`, and Store eligibility integration.
Plan packets: [docs/plan/](docs/plan/).

## Goal

Paid Store queries carry LIP-155 `EligibilityProof` (RFC 73 on Store tag `30`); provider verifies
LEZ state before serving. Rust/FFI holds policy; `payment_streams_module` orchestrates wallet I/O;
delivery forks add wire fields and hooks.

Universal payment protocol vs Store integration use case. Developer Journey (Store integration);
optional User Journey (payment streams UI) is protocol-only, not Store.

Program index: [docs/plan/index.md](docs/plan/index.md).
Terminology: [docs/reference/naming-conventions.md](docs/reference/naming-conventions.md).

## Active work

### Documentation and integration

Developer Journey — Store integration. Packet:
[docs/plan/upcoming/step-20-developer-journey.md](docs/plan/upcoming/step-20-developer-journey.md).
In-repo SSOT draft: [docs/store-integration/README.md](docs/store-integration/README.md).

User Journey — CLI doc packet: [Step 22](docs/plan/upcoming/step-22-ui-journey.md) (active, independent of UI).
User Journey UI (optional) — Basecamp plugin: [Step 21](docs/plan/upcoming/step-21-basecamp-ui.md).

### TestNet v0.2 migration (Steps 28-29) and static dependency migration (Step 30)

Steps 26-27 are complete
([26](docs/plan/completed/step-26-testnet-v02-migration.md),
[27](docs/plan/completed/step-27-claim-fix-verification.md)).
These steps parallelize and can proceed while documentation matures:

| Step | Focus | Can Start |
|------|-------|-----------|
| [28](docs/plan/upcoming/step-28-user-journey-testnet.md) | User Journey on TestNet v0.2 | **Now** — Steps 26-27 complete; testnet v0.2 upgrade pending for testnet-side claim |
| [29](docs/plan/upcoming/step-29-e2e-script-ux.md) | E2E script UX enhancement | **Now** — use `CHAIN=local` |
| [30](docs/plan/upcoming/step-30-static-dependency-migration.md) | Static dependency migration | **Now** — Step 26 complete (was gated); parallelizes with 28-29 |

Optional: [step-23-public-store-provider.md](docs/plan/upcoming/step-23-public-store-provider.md).
Cancelled: [step-25-demo-coordination-module.md](docs/plan/cancelled/step-25-demo-coordination-module.md).

## Verification

Canonical: `scripts/e2e.sh` — [docs/reference/verification-matrix.md](docs/reference/verification-matrix.md),
[scripts/README.md](scripts/README.md).

Maintainer lifecycle regression: `make verify-store-local-lifecycle` /
[scripts/archive/verify-store-local-lifecycle.sh](scripts/archive/verify-store-local-lifecycle.sh).

## Read order by task

| Task | Files |
| --- | --- |
| Product overview | README → docs hub → pillar README |
| Verify | verification-matrix → scripts/README |
| Store wire | integration-contracts → integration-decisions D1/D2 |
| Localnet failure | archive/operator/localnet-recovery.md |
| Developer journey | step-20 packet + store-integration README |
| Rust / guest | docs/on-chain/README.md |

## Always-on references

| Category | Files |
| --- | --- |
| Verification | docs/reference/verification-matrix.md, scripts/README.md |
| APIs | docs/reference/integration-contracts.md |
| Program | docs/plan/index.md |
| Decisions | docs/reference/integration-decisions.md |

Historical runbooks: [docs/archive/](docs/archive/).

## Machine manifest

[docs/context-manifest.json](docs/context-manifest.json)
