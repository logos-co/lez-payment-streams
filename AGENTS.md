# lez-payment-streams — agent context

Human entry: [README.md](README.md) → [docs/README.md](docs/README.md) → pillar README.

LIP-155 on-chain program, `payment_streams_module`, and Store eligibility integration.
Plan packets: [docs/plan/](docs/plan/).

## Goal

Paid Store queries carry LIP-155 `EligibilityProof` (RFC 73 on Store tag `30`); provider verifies
LEZ state before serving. Rust/FFI holds policy; `payment_streams_module` orchestrates wallet I/O;
delivery forks add wire fields and hooks.

Universal payment protocol vs Store integration use case. Developer Journey (Store integration)
is upcoming ([Step 20](docs/plan/upcoming/step-20-developer-journey.md);
[logos-docs#369](https://github.com/logos-co/logos-docs/issues/369)).
User Journey (CLI / module, protocol-only, not Store) is complete
([logos-docs#370](https://github.com/logos-co/logos-docs/issues/370)).
Forum progress report: [Step 46](docs/plan/upcoming/step-46-forum-post-user-journey.md).
User Journey UI (Step 21) is upcoming; public hosted Store (Step 23) is wontfix.

Program index: [docs/plan/index.md](docs/plan/index.md).
Terminology: [docs/reference/naming-conventions.md](docs/reference/naming-conventions.md).

## Active work

### Documentation and integration

Developer Journey — Store integration (upcoming):
[Step 20](docs/plan/upcoming/step-20-developer-journey.md);
[logos-docs#369](https://github.com/logos-co/logos-docs/issues/369).
In-repo SSOT draft: [docs/store-integration/README.md](docs/store-integration/README.md).
Developer Journey generalization (protocol-agnostic eligibility guide):
[Step 35](docs/plan/completed/step-35-developer-journey-generalization.md) (complete).

User Journey (complete — CLI / module, not Store):
[logos-docs#370](https://github.com/logos-co/logos-docs/issues/370);
[Step 22](docs/plan/completed/step-22-ui-journey.md) (logos-docs packet);
[Step 34](docs/plan/completed/step-34-user-journey-manual-walkthrough.md) (in-repo walkthrough);
[Step 28](docs/plan/completed/step-28-user-journey-testnet.md) (module×testnet E2E).
In-repo SSOT: [docs/journeys/USER_JOURNEY.md](docs/journeys/USER_JOURNEY.md).
Upcoming: [Step 20](docs/plan/upcoming/step-20-developer-journey.md) (Developer Journey Store publish),
[Step 21](docs/plan/upcoming/step-21-basecamp-ui.md) (Basecamp UI),
[Step 44](docs/plan/upcoming/step-44-payer-and-payee-close.md) (payer and payee close),
[Step 45](docs/plan/upcoming/step-45-dependencies-and-patches.md) (deps and patches),
[Step 46](docs/plan/upcoming/step-46-forum-post-user-journey.md) (forum progress report),
[Step 47](docs/plan/upcoming/step-47-unify-role-terminology.md) (role terminology: module, journey, policy, layouts).
Wontfix: [Step 23](docs/plan/wontfix/step-23-public-store-provider.md) (public Store provider),
[Step 25](docs/plan/wontfix/step-25-demo-coordination-module.md) (demo coordinator).
Raw TODOs (unscheduled ideas): [docs/plan/raw-todos/](docs/plan/raw-todos/).

### E2E and migration (Steps 26-33)

Steps 26–33 are complete
([26](docs/plan/completed/step-26-testnet-v02-migration.md),
[27](docs/plan/completed/step-27-claim-fix-verification.md),
[28](docs/plan/completed/step-28-user-journey-testnet.md),
[29](docs/plan/completed/step-29-e2e-script-ux.md),
[30](docs/plan/completed/step-30-static-dependency-migration.md),
[31](docs/plan/completed/step-31-dependencies-upgrade.md),
[32](docs/plan/completed/step-32-auth-transfer-unify-store-claim.md),
[33](docs/plan/completed/step-33-store-e2e-fresh-vault.md)).
Step 32 D3 closed via Step 39 strict-claim Store/module testnet evidence; default
`E2E_CLAIM_OPTIONAL=0`; `demo_claim` alias removed
([gate log](docs/plan/completed/step-32-testnet-gate-log.md)).

### Privacy-preserving payment streams (Steps 36-39)

Steps 36–39 complete (product + local Store privacy + testnet privacy E2E):
[step-36-payer-funder-unlinkability.md](docs/plan/completed/step-36-payer-funder-unlinkability.md),
[step-37-payee-receiver-privacy.md](docs/plan/completed/step-37-payee-receiver-privacy.md),
[step-38-store-privacy-e2e.md](docs/plan/completed/step-38-store-privacy-e2e.md),
[step-39-testnet-privacy-e2e.md](docs/plan/completed/step-39-testnet-privacy-e2e.md).
Gate log: [step-39-testnet-gate-log.md](docs/plan/completed/step-39-testnet-gate-log.md)
(module + Store full privacy on ImageID `072a26cc…`, `RISC0_DEV_MODE=0`,
`E2E_CLAIM_OPTIONAL=0`). Do not reopen Phase 1–3 greens (D39.23). CPU prove is
valid without NVIDIA (D39.26).

Complete (docs): [Step 40](docs/plan/completed/step-40-lip155-privacy-workflow-spec.md) —
privacy-preserving user and provider workflows in LIP-155
([logos-lips#397](https://github.com/logos-co/logos-lips/pull/397) → `master` `435a6f18`).

### Testnet v0.3 incentivisation research (Steps 41-43)

Roadmap: [incentivisation_v0.3](https://roadmap.logos.co/anoncomms/roadmap/testnet_v0.3/incentivisation_v0.3).
Client/provider shielding map to Steps 36–40.
Step 40 complete (privacy workflow on LIP-155):
[step-40-lip155-privacy-workflow-spec.md](docs/plan/completed/step-40-lip155-privacy-workflow-spec.md)
([logos-lips#397](https://github.com/logos-co/logos-lips/pull/397) → `master` `435a6f18`).
Step 41 complete (multi-token provider policy on LIP-155, F8/U9):
[step-41-non-native-token-policy-spec.md](docs/plan/completed/step-41-non-native-token-policy-spec.md)
([logos-lips#379](https://github.com/logos-co/logos-lips/pull/379) → `master` `f09f9e9e`).
Research/spec packets moved to wontfix:

- [Step 42](docs/plan/wontfix/step-42-discovery-payment-policy-advertisement.md) —
  discovery + payment policy advertisement (F6, F7, U8); Discovery support.
- [Step 43](docs/plan/wontfix/step-43-shared-payment-pool-research.md) —
  shared payment pool investigation (F9–F11, U10).

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
| Developer journey | upcoming/step-20 packet + store-integration README; logos-docs#369 |
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
