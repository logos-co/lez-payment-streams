# Step 38 — Store E2E privacy profiles

Index: [index.md](../index.md). Status: active — planning packet (not started).

Goal: prove Developer Journey Store integration end to end under privacy
profiles, up through full privacy mode (private vault owner and private
provider claim), without changing Delivery wire or eligibility hooks.

This step is verification and harness work only. Product paths already landed
in [Step 36](../completed/step-36-payer-funder-unlinkability.md) and
[Step 37](../completed/step-37-payee-receiver-privacy.md). Delivery continues
to treat `EligibilityProof` as opaque bytes on Store tag `30`.

## Problem

Public Store E2E (`MODE=store`, `scripts/e2e/run_local_e2e.py`) still assumes
public owner and provider accounts and public seeding. Module privacy flags
(`OWNER_PRIVACY`, `PROVIDER_PRIVACY`) already work for single-host module E2E
but do not flow into the Store orchestrator.

Without this step, paid Store queries over `PseudonymousFunder` vaults and
shielded provider claims stay module-only. The dual-host Store path
integrators care about remains public-only.

## Why a separate step

Store × privacy is orchestrator and fixture plumbing, not a new eligibility
protocol. One Developer Journey packet owns both privacy flags on the Store
path. Do not reopen Steps 36 or 37, and do not split into two Store steps
(payer Store vs payee Store): both share private accounts, pre-shield /
dust-shield, `chainAction` instead of public seed, and guest ELF /
`RISC0_DEV_MODE` on proving hosts.

## Inherit from module privacy E2E (do not rediscover)

Reuse the module privacy harness rules from Steps 36–37. Store work is mainly
porting them into `run_local_e2e.py` under dual-host constraints.

| Topic | Module fact to reuse | Store implication |
| --- | --- | --- |
| Flags | `OWNER_PRIVACY` / `PROVIDER_PRIVACY` in `scripts/lib/common.sh`; `PRIVACY=1` aliases owner only | Read the same env in Store orchestrator; no `MODE=privacy` |
| Submit path | D37.9: any private account slot → `submitGenericPrivate` only; PF never public | Prefer `chainAction` for vault/stream lifecycle; public seed must not touch PF vaults |
| AT-init | Public accounts only (D37.11) | Never AT-init private owner or private provider |
| Owner funding | Public funder → `transfer_shielded_owned` into private owner | Run on the user host (holds private owner keys) |
| Provider funding | No `Public/$PROVIDER` pinata; dust `transfer_shielded_owned` into private provider (create-only fails private claim with wallet FFI error 99) | Dust-fund on the host that holds private provider keys (today: provider host — claim uses `cfg_provider`) |
| Claim verify | Private provider: assert `vault_holding` drop, not public `getAccount` balance | Change Store `claim_balance` when `PROVIDER_PRIVACY=1` |
| Mapping | D37.12 encoding smoke already in LogosTests | This step owns dual-host: real peer ad → `registerProviderMapping` → prepare → paid `storeQuery` |
| Proving env | `e2e.sh` already sets `RISC0_DEV_MODE` / `PAYMENT_STREAMS_GUEST_BIN` when any privacy flag is set | Ensure both daemons that prove (user for PF submits / NSK vault proof; provider for private claim) see those env vars |

### Dual-host key placement (locked for this step)

Module E2E is single-host. Store E2E is dual-host. Place keys as follows:

- User host: private vault owner (when `OWNER_PRIVACY=1`), public funder, stream
  create / deposit / close / prepare / `registerProviderMapping`. Needs the
  provider account id (base58) for create and mapping; does not need provider NSK.
- Provider host: private provider account (when `PROVIDER_PRIVACY=1`). Claim
  already runs via `cfg_provider` `chainAction claim` — that host must hold the
  private provider keys and the dust-funded committed note.
- Eligibility verify stays on the provider host; Delivery wire unchanged.

Do not copy full wallet storage between hosts. Export the private provider
account id (and VPK only if a user-host shield path is chosen) into the run
manifest the way public payee ids are shared today.

## Prerequisites

- Step 36 complete (PF vault, `submitGenericPrivate`, `sign_private_payload`,
  module `OWNER_PRIVACY=1`).
- Step 37 complete (`PROVIDER_PRIVACY` claim routing, module E2E, D37.12
  encoding smoke). Phase A (owner-only Store privacy) can start immediately;
  Phase B needs Step 37 (done).
- Public Store local green (`make verify-store-local`).
- Flags and recipes documented in [E2E.md](../../journeys/E2E.md).

## Scope

In scope:

- Store orchestrator honor `OWNER_PRIVACY` / `PROVIDER_PRIVACY` with the
  funding, AT-init, claim-verify, and dual-host rules above.
- Store E2E for `registerProviderMapping` under privacy profiles (D38.7):
  real provider peer ad → map → prepare → paid `storeQuery` → settlement.
- Prefer `payment_streams_module` `chainAction` for privacy lifecycle; refuse
  public-only seed for PF vaults (D38.5).
- Docs and Make aliases for Store privacy recipes; matrix optional then
  required when stable.
- Localnet first; testnet Store × privacy only after local is boringly green.

Out of scope:

- Delivery wire, RFC 73 tags, or `delivery_module` hook shape changes (unless a
  real bug appears).
- Guest transition-logic changes.
- Reopening Steps 36 or 37 DoD (including redoing mapping encoding smoke).
- Reimplementing D37.9 submit selection in the Store harness (module already
  does it when `chainAction` is used).
- Forcing every matrix cell (including testnet) to required on first land.
- Traffic-analysis mitigations beyond the privacy journey docs.

## Implementation plan

### Phase A — Store × owner privacy

1. Teach `run_local_e2e.py` (and ensure helpers) `OWNER_PRIVACY=1`.
2. User wallet: public funder + private vault owner; pre-shield; `initializeVault`
   with `privacy_tier: 1` via `chainAction`.
3. Deposit, create, close via `chainAction` (no transparent seed on PF).
4. Keep provider public; claim stays public-provider path.
5. Expect Store phases (`store_query_success`, `store_query_missing_proof`,
   settlement) plus privacy setup phases such as `pre_shield`.
6. Make alias e.g. `verify-store-local-owner-privacy`; document in E2E.md.

### Phase B — Store × provider privacy and full privacy

1. When `PROVIDER_PRIVACY=1`, create private provider on the provider host,
   put that account id in the manifest, dust-fund it on the provider host,
   set stream `provider_id` and user-host `registerProviderMapping` to that id.
2. Prepare + paid `storeQuery` + settlement; claim via provider-host
   `chainAction` with vault_holding-drop verification.
3. Gate `OWNER_PRIVACY=0 PROVIDER_PRIVACY=1` on Store local.
4. Gate full privacy `OWNER_PRIVACY=1 PROVIDER_PRIVACY=1` on Store local
   (end goal of this step).
5. Extend E2E.md / verification-matrix / scripts README; optional Make aliases.

### Phase C — Optional matrix completion

1. Store × testnet privacy only after local Store privacy is stable (same
   two-pass discipline as other testnet gates when promoting to required).
2. Module × testnet privacy profiles remain optional / out of this packet’s
   critical path (module local privacy already closed under 36/37).
3. Prefer Make targets or a small matrix runner over a third top-level `MODE`.

## Decision log

| Id | Topic | Outcome |
| --- | --- | --- |
| D38.1 | Packet ownership | One Step 38 for Store × privacy; do not fold into 36/37 or split into two Store steps. |
| D38.2 | Delivery impact | No intentional Delivery or wire changes; opaque eligibility stays. |
| D38.3 | Flags | Reuse `OWNER_PRIVACY` / `PROVIDER_PRIVACY`; do not introduce `MODE=privacy`. |
| D38.4 | Rollout | Local Store profiles first; full privacy (both flags) is the step end goal; testnet and required-tier promotion are phased. |
| D38.5 | Seed path | Public seed fixtures must not initialize or fund `PseudonymousFunder` vaults; privacy Store runs use module `chainAction` (or an extended private-aware seed). |
| D38.6 | Phase A vs B | Phase A (owner-only) can start now that Step 36 is complete. Phase B (provider / full) requires Step 37 (complete). |
| D38.7 | `registerProviderMapping` Store E2E | This step owns dual-host Store mapping under privacy profiles. Step 37 owns encoding smoke only (D37.12). Wire mapping in `run_local_e2e.py` before paid Store query when `PROVIDER_PRIVACY=1`. |
| D38.8 | Dual-host + funding inherit | Private owner keys and owner pre-shield on user host. Private provider keys, dust pre-shield, and claim on provider host (`cfg_provider`). AT-init public only. Private-provider claim_balance uses vault_holding drop. |

## Risk

- Half-migrating seed helpers to `chainAction` can leave mixed public/private
  submits that break PF vaults.
- Dual-host proving timeouts and guest ELF / `RISC0_DEV_MODE` missing on the
  provider daemon (private claim) or user daemon (PF + NSK vault proof).
- Dust-funding a private provider across hosts if mistakenly done on the wrong
  wallet (no committed note → private claim FFI error 99).
- Promoting privacy cells to required too early multiplies flake surface.

Mitigation: phase A then B on localnet only; keep public Store required; treat
privacy Store as optional until green and boring; fund private accounts on the
host that holds their keys.

## Verification

| Gate | Command (illustrative) | Pass criteria |
| --- | --- | --- |
| Public Store regression | `make verify-store-local` | Unchanged green. |
| Store owner privacy | `MODE=store CHAIN=local OWNER_PRIVACY=1 ./scripts/e2e.sh local run` | Paid Store query + settlement over PF vault; public provider. |
| Store provider privacy | `MODE=store CHAIN=local PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` | Mapping to private provider id; paid Store query; claim via vault_holding drop. |
| Store full privacy | `MODE=store CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` | Both profiles; mapping + query + settlement. |
| Module privacy regression | `make verify-module-local-privacy` and `make verify-module-local-provider-privacy` | User Journey privacy unchanged. |

Exact Make alias names may follow the module pattern
(`verify-store-local-owner-privacy`, etc.).

## Deliverables

- [ ] Store orchestrator respects `OWNER_PRIVACY` and `PROVIDER_PRIVACY`
  (including dual-host key placement and dust pre-shield, D38.8).
- [ ] Phase A: Store local × `OWNER_PRIVACY=1` green.
- [ ] Phase B: Store local × `PROVIDER_PRIVACY=1` and both flags green, including
  `registerProviderMapping` → prepare → paid `storeQuery` → settlement (D38.7).
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
  (Phase B), with `registerProviderMapping` wired to the private payee id and
  claim confirmed via vault_holding drop.
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

- User Journey product work (Steps 36 and 37), including redoing
  `registerProviderMapping` encoding smoke (D37.12) or D37.9 submit selection.
- logos-docs publication.
- Expanding privacy into a third required matrix axis on first land.

## Related

- [step-36-payer-funder-unlinkability.md](../completed/step-36-payer-funder-unlinkability.md) —
  owner privacy product; module `OWNER_PRIVACY`.
- [step-37-payee-receiver-privacy.md](../completed/step-37-payee-receiver-privacy.md) —
  provider privacy product; module `PROVIDER_PRIVACY`; D37.9–D37.12.
- [step-33-store-e2e-fresh-vault.md](../completed/step-33-store-e2e-fresh-vault.md) —
  Store fresh vault and sizing baseline.
- [E2E.md](../../journeys/E2E.md) — recipe SSOT; privacy as profile overlays.
- [PRIVACY_ENHANCED_JOURNEY.md](../../journeys/PRIVACY_ENHANCED_JOURNEY.md) —
  privacy narrative.
- [integration-contracts.md](../../reference/integration-contracts.md) —
  Store eligibility API surface (unchanged by design).
- [verification-matrix.md](../../reference/verification-matrix.md) — required
  tiers vs optional privacy gates.
