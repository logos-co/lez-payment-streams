# Step 38 — Store E2E privacy profiles

Index: [index.md](../index.md). Status: active — planning packet (not started).

Goal: prove Developer Journey Store integration end to end under privacy
profiles, up through full privacy mode (private vault owner and private
provider claim), without changing Delivery wire or eligibility hooks.

This step is verification and harness work. Product paths for owner and
provider privacy land in Steps 36 and 37. Delivery continues to treat
`EligibilityProof` as opaque bytes on Store tag `30`.

## Problem

Public Store E2E (`MODE=store`, `scripts/e2e/run_local_e2e.py`) still assumes
public owner and provider accounts and public seeding
(`seed_localnet_fixture`, public `initialize_vault` / deposit). Module privacy
profiles (`OWNER_PRIVACY`, `PROVIDER_PRIVACY`) do not flow into the Store
orchestrator.

Without this step, paid Store queries over `PseudonymousFunder` vaults and
shielded provider claims are only covered at the module or Rust unit level.
The dual-host Store path that integrators care about stays public-only.

## Why a separate step

- Step 36 is complete User Journey product work; reopening it for Store
  harness would mix journeys.
- Step 37 is User Journey payee privacy product work; Store E2E must not
  block closing that packet.
- Store × privacy is mostly orchestrator and fixture plumbing, not a new
  eligibility protocol. One Developer Journey packet owns both privacy flags
  on the Store path.

Do not split into two Store steps (payer Store vs payee Store). Both share
the same prepare path (private accounts, pre-shield, `chainAction` instead of
public seed, guest ELF / `RISC0_DEV_MODE`).

## Architectural context

### What Delivery sees

From Delivery and `delivery_module`, public vs private payment-streams mode
makes no difference. Hooks still call
`prepareEligibilityProof…` / `verifyEligibilityForStoreQuery` and carry opaque
proof bytes. Privacy changes stay inside `payment_streams_module` and the LEZ
wallet (submit path, NPK / `sign_private_payload` for vault proofs, private
provider claim).

### Independent privacy choices

Owner privacy and provider privacy are independent product choices:

| Flag | Who chooses | Effect |
| --- | --- | --- |
| `OWNER_PRIVACY=1` | Payer at vault init | `PseudonymousFunder` vault; vault-touching ops via private submit; eligibility vault proof signed with NSK |
| `PROVIDER_PRIVACY=1` | Provider at stream create / claim | `provider_id` is a private account; claim credits that account via private submit |

`PRIVACY=1` remains an alias for `OWNER_PRIVACY=1` only. It must not mean
“full privacy.”

Four combos matter:

1. owner public, provider public — today’s required Store baseline
2. owner private, provider public — Step 36 product + this step’s first Store gate
3. owner public, provider private — Step 37 product + this step
4. owner private, provider private — full privacy mode (end goal of this step)

### Not a required 2×2×2 matrix

The required verification matrix stays journey × chain (module|store ×
local|testnet). Privacy is a profile overlay, not a third `MODE`.

Do not require all journey × chain × owner × provider cells as equal CI jobs
on day one. Roll out local Store profiles first, then optional testnet, and
promote gates in [verification-matrix.md](../../reference/verification-matrix.md)
only when stable.

## Prerequisites

- [Step 36](../completed/step-36-payer-funder-unlinkability.md) complete
  (`submitGenericPrivate`, PF vault, `sign_private_payload`, module
  `OWNER_PRIVACY=1`).
- [Step 37](step-37-payee-receiver-privacy.md) complete for phases that need
  `PROVIDER_PRIVACY` (private provider claim routing and module E2E).
  Phase A (owner-only Store privacy) can start after Step 36 even if Step 37
  is still open.
- Public Store local green (`make verify-store-local`) as the baseline this
  step must not regress.
- Shared E2E flags already defined in `scripts/lib/common.sh` and documented
  in [E2E.md](../../journeys/E2E.md).

## Scope

In scope:

- Store orchestrator honor `OWNER_PRIVACY` and `PROVIDER_PRIVACY` the same way
  module E2E does (create private accounts, pre-shield, `privacy_tier=1`,
  private provider where flagged).
- Prefer `payment_streams_module` `chainAction` for vault and stream lifecycle
  under privacy profiles; do not use public-only seed paths for PF vaults.
- Set `RISC0_DEV_MODE` and `PAYMENT_STREAMS_GUEST_BIN` on hosts that prove
  private submits.
- Ensure user-host wallet holds private owner keys used for prepare-proof;
  provider host verifies only.
- Docs and Make aliases for Store privacy recipes; verification-matrix optional
  then required promotion as cells stabilize.
- Localnet first; testnet Store × privacy only after local is boringly green.

Out of scope:

- Changes to logos-delivery Store wire, RFC 73 tags, or `delivery_module`
  eligibility hook shapes (unless a real bug appears).
- Guest transition-logic changes.
- Reopening Steps 36 or 37 DoD.
- Forcing every matrix cell (including testnet) to required on first land.
- Traffic-analysis mitigations beyond documentation already in the privacy
  journey.

## Implementation plan

### Phase A — Store × owner privacy (depends on Step 36)

1. Teach `run_local_e2e.py` (and ensure helpers) a privacy profile path when
   `OWNER_PRIVACY=1`.
2. User wallet: public funder + private vault owner; `transfer_shielded_owned`
   pre-shield; `initializeVault` with `privacy_tier: 1` via `chainAction`.
3. Deposit, create, close, claim via module `chainAction` so PF vaults never
   take transparent seed submit.
4. Keep provider public for this phase.
5. Expect existing Store phases (`store_query_success`,
   `store_query_missing_proof`, settlement) plus privacy setup phases such as
   `pre_shield`.
6. Make target e.g. `verify-store-local-owner-privacy`; document in E2E.md.

### Phase B — Store × provider privacy and full privacy (depends on Step 37)

1. When `PROVIDER_PRIVACY=1`, create private provider, set stream `provider_id`
   and `registerProviderMapping` to that account id, claim via private submit.
2. Gate `OWNER_PRIVACY=0 PROVIDER_PRIVACY=1` on Store local.
3. Gate full privacy `OWNER_PRIVACY=1 PROVIDER_PRIVACY=1` on Store local
   (end-to-end goal of this step).
4. Extend E2E.md / verification-matrix / scripts README; optional Make aliases
   for provider-only and full privacy.

### Phase C — Optional matrix completion

1. Module × testnet privacy profiles if not already closed under 36/37.
2. Store × testnet privacy only after local Store privacy is stable (same
   two-pass discipline as other testnet gates when promoting to required).
3. Prefer Make targets or a small matrix runner over a third top-level `MODE`.

## Decision log

| Id | Topic | Outcome |
| --- | --- | --- |
| D38.1 | Packet ownership | One Step 38 for Store × privacy; do not fold into 36/37 or split into two Store steps. |
| D38.2 | Delivery impact | No intentional Delivery or wire changes; opaque eligibility stays. |
| D38.3 | Flags | Reuse `OWNER_PRIVACY` / `PROVIDER_PRIVACY`; do not introduce `MODE=privacy`. |
| D38.4 | Rollout | Local Store profiles first; full privacy (both flags) is the step end goal; testnet and required-tier promotion are phased. |
| D38.5 | Seed path | Public seed fixtures must not initialize or fund `PseudonymousFunder` vaults; privacy Store runs use module `chainAction` (or an extended private-aware seed). |
| D38.6 | Phase A vs B | Owner-only Store privacy can land before Step 37 closes; provider and full privacy wait on Step 37. |

## Risk

- Store prepare and seed helpers are deeply public-tier today; half-migrating
  to `chainAction` can leave mixed public/private submits that break PF vaults.
- Dual-host proving timeouts and guest ELF path on the user daemon are easy to
  miss when only module single-host was tested.
- Promoting too many privacy cells to required too early multiplies flake
  surface (dual-host + proving + testnet).

Mitigation: phase A then B on localnet only; keep public Store required; treat
privacy Store as optional until green and boring.

## Verification

| Gate | Command (illustrative) | Pass criteria |
| --- | --- | --- |
| Public Store regression | `make verify-store-local` | Unchanged green. |
| Store owner privacy | `MODE=store CHAIN=local OWNER_PRIVACY=1 ./scripts/e2e.sh local run` | Paid Store query + settlement over PF vault; public provider. |
| Store provider privacy | `MODE=store CHAIN=local PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` | Paid Store query + shielded provider claim. |
| Store full privacy | `MODE=store CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` | Both profiles together; end goal of this step. |
| Module privacy regression | `make verify-module-local-privacy` and Step 37 provider module gates | No User Journey privacy regression. |

Exact Make alias names may follow the module naming pattern
(`verify-store-local-owner-privacy`, etc.).

## Deliverables

- [ ] Store orchestrator respects `OWNER_PRIVACY` and `PROVIDER_PRIVACY`.
- [ ] Phase A: Store local × `OWNER_PRIVACY=1` green.
- [ ] Phase B: Store local × `PROVIDER_PRIVACY=1` and both flags green (full
  privacy mode).
- [ ] Public Store local remains green.
- [ ] [E2E.md](../../journeys/E2E.md) documents Store privacy profile recipes.
- [ ] [verification-matrix.md](../../reference/verification-matrix.md) lists
  Store privacy gates (optional until promoted).
- [ ] [scripts/README.md](../../scripts/README.md) and Make aliases for the
  Store privacy commands.
- [x] [index.md](../index.md) and [AGENTS.md](../../AGENTS.md) list Step 38.
- [ ] Optional Phase C: testnet Store privacy recorded or explicitly deferred
  with rationale.

## Definition of done

- [ ] Paid Store queries succeed for a `PseudonymousFunder` vault owner on
  localnet (Phase A).
- [ ] Paid Store queries succeed with private provider claim on localnet
  (Phase B).
- [ ] Full privacy mode Store local green:
  `OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`.
- [ ] No Delivery wire or hook shape change required for the above (or any
  unexpected change is documented as a decision).
- [ ] Public Store and module privacy regressions green.
- [ ] Docs and plan index updated; packet moved to `docs/plan/completed/` when
  phases A and B pass.

## Known limitations

- Amounts on `vault_holding` remain public.
- Store × privacy does not itself mitigate timing or amount correlation across
  the shielding boundary.
- Testnet Store × privacy may lag local gates.

## Not in scope

- User Journey product work (Steps 36 and 37).
- logos-docs publication.
- Expanding privacy into a third required matrix axis on first land.

## Related

- [step-36-payer-funder-unlinkability.md](../completed/step-36-payer-funder-unlinkability.md) —
  owner privacy product; module `OWNER_PRIVACY`.
- [step-37-payee-receiver-privacy.md](step-37-payee-receiver-privacy.md) —
  provider privacy product; module `PROVIDER_PRIVACY`.
- [step-33-store-e2e-fresh-vault.md](../completed/step-33-store-e2e-fresh-vault.md) —
  Store fresh vault and sizing baseline.
- [E2E.md](../../journeys/E2E.md) — recipe SSOT; privacy as profile overlays.
- [PRIVACY_ENHANCED_JOURNEY.md](../../journeys/PRIVACY_ENHANCED_JOURNEY.md) —
  privacy narrative.
- [integration-contracts.md](../../reference/integration-contracts.md) —
  Store eligibility API surface (unchanged by design).
- [verification-matrix.md](../../reference/verification-matrix.md) — required
  tiers vs optional privacy gates.
