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
Developer Journey generalization — rewrite as a protocol-agnostic eligibility guide:
[Step 35](docs/plan/upcoming/step-35-developer-journey-generalization.md) (active).

User Journey — CLI doc packet: [Step 22](docs/plan/upcoming/step-22-ui-journey.md) (active, independent of UI).
User Journey — testnet manual walkthrough: [Step 34](docs/plan/upcoming/step-34-user-journey-manual-walkthrough.md) (active).
User Journey on TestNet v0.2: [Step 28](docs/plan/completed/step-28-user-journey-testnet.md) (complete).
Raw TODOs (unscheduled ideas): [docs/plan/raw-todos/](docs/plan/raw-todos/).
User Journey UI (optional) — Basecamp plugin: [Step 21](docs/plan/upcoming/step-21-basecamp-ui.md).

### E2E and migration (Steps 26-33)

Steps 26-31 and 33 are complete
([26](docs/plan/completed/step-26-testnet-v02-migration.md),
[27](docs/plan/completed/step-27-claim-fix-verification.md),
[28](docs/plan/completed/step-28-user-journey-testnet.md),
[29](docs/plan/completed/step-29-e2e-script-ux.md),
[30](docs/plan/completed/step-30-static-dependency-migration.md),
[31](docs/plan/completed/step-31-dependencies-upgrade.md),
[33](docs/plan/completed/step-33-store-e2e-fresh-vault.md)).
Active engineering packet:

| Step | Focus | Can Start |
|------|-------|-----------|
| [32](docs/plan/upcoming/step-32-auth-transfer-unify-store-claim.md) | AT-init unify + Store claim phase (D3 testnet gate pending) | **Now** — maintainer gate in [step-32-testnet-gate-log.md](docs/plan/completed/step-32-testnet-gate-log.md) |

Optional: [step-23-public-store-provider.md](docs/plan/upcoming/step-23-public-store-provider.md).
Cancelled: [step-25-demo-coordination-module.md](docs/plan/cancelled/step-25-demo-coordination-module.md).

### Privacy-preserving payment streams (Steps 36-37)

Active engineering packet:

| Step | Focus | Can Start |
|------|-------|-----------|
| [36](docs/plan/upcoming/step-36-payer-funder-unlinkability.md) | Payer funder unlinkability via LEZ private execution | After Step 32 lands or in parallel if the close-then-claim contract is preserved |
| [37](docs/plan/upcoming/step-37-payee-receiver-privacy.md) | Payee receiver privacy via LEZ private execution | After Step 36 or in the same PR, so it reuses the shared `submitGenericPrivate` helper |

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
