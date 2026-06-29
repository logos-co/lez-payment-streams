# lez-payment-streams — agent context

Human entry: [README.md](README.md) → [docs/README.md](docs/README.md) → pillar README.

LIP-155 on-chain program, `payment_streams_module`, and Store eligibility integration.
Plan packets: [docs/plan/](docs/plan/).

## Goal

Paid Store queries carry LIP-155 `EligibilityProof` (RFC 73 on Store tag `30`); provider verifies
LEZ state before serving. Rust/FFI holds policy; `payment_streams_module` orchestrates wallet I/O;
delivery forks add wire fields and hooks.

Universal payment protocol vs Store integration use case (N18 Track A). Optional Basecamp UI
(N18 Track B) is protocol-only, not Store.

Program index: [docs/development-map/program-index.md](docs/development-map/program-index.md).
Terminology: [docs/reference/naming-conventions.md](docs/reference/naming-conventions.md).

## Active work

Developer journey — Store integration (N18 Track A). Packet:
[docs/plan/upcoming/step-20-developer-journey.md](docs/plan/upcoming/step-20-developer-journey.md).
In-repo SSOT draft: [docs/store-integration/README.md](docs/store-integration/README.md).

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
| Program | docs/development-map/program-index.md |
| Decisions | docs/reference/integration-decisions.md |

Historical runbooks: [docs/archive/](docs/archive/).

## Machine manifest

[docs/context-manifest.json](docs/context-manifest.json)
