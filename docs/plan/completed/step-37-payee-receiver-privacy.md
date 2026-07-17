# Step 37 — payee receiver privacy via LEZ private execution

Index: [index.md](../index.md). Status: **complete**.

Goal: productize LEZ private execution for the receiving service provider
side of payment streams so the provider can claim accrued funds to a
shielded address. This delivers the LIP-155 secondary privacy goal,
provider receiving privacy: limiting linkage between on-chain claims and
the provider's real receiving addresses.

Owner privacy and provider privacy are independent product choices. A payer
may open a `Public` or `PseudonymousFunder` vault; independently, a provider
may claim to a public or private receiving account. E2E reflects that with
two flags: `OWNER_PRIVACY` (Step 36) and `PROVIDER_PRIVACY` (this step).
`PRIVACY=1` is only an alias for `OWNER_PRIVACY=1` and must not be overloaded
to mean “full privacy.”

## Problem

Today the provider claims accrued funds to a public account (`provider`
signer in the `claim` instruction), and each transparent claim links that
stream to a visible receiving address. The guest `claim` already supports a
private signer slot and the harness already proves out PP claim and PP
withdraw to a private recipient, but the module never submits a shielded
claim. As with Step 36, the gap is wiring, not cryptography.

## Architectural context

The provider side mirrors the payer side. The in-protocol identity is
`StreamConfig.provider`, an NPK-derived `AccountId` (N5). This mirrors the
public-mode flow where the provider shares a public account id that the user
puts into `provider_id`. In private mode, the provider shares an NPK/VPK pair
(or the NPK-derived account id directly) and the user creates the stream with
that `provider_id`. Claims route through shielded transactions to a private
receiving account derived from the same NPK.

Existing scaffolding in this repo:

- PP withdraw to a private recipient in
  `lez-payment-streams-core/src/program_tests/pp_common.rs`,
  `fund_private_account_via_pp_withdraw`, including ephemeral ECDH
  against the recipient VPK and the `AutoClaim::Claimed(Claim::Authorized)`
  path for a default-owned recipient.
- PP claim and close with a private signer slot (visibility-1) in
  `pp_common.rs`, `pp_claim_close_setup`.
- The guest `claim` instruction in
  `methods/guest/src/bin/lez_payment_streams.rs`, the `claim` handler,
  credits `provider.account.balance`; for a shielded claim the `provider`
  account is a private account supplied as a private signer slot. The
  guest transition logic is unchanged.

Upstream PP submit is the same path as Step 36:
`wallet_ffi_send_generic_private_transaction` and
`wallet_ffi_resolve_private_account` in `logos-execution-zone/lez/wallet-ffi`,
wrapped by `LEZCoreModule::send_generic_private_transaction` in
`logos-execution-zone-module/src/lez_core_module.cpp`.

Three constraints:

- Claim amounts are public. `vault_holding` is a public PDA, so its balance
  drop on claim is visible. Shielding hides the destination, not the
  amount.
- There is no automatic balance consolidation. N claims to the same
  provider NPK with distinct identifiers produce N separate private
  accounts, and spending all of them needs N inputs, so proof and
  transaction cost grows linearly with N. Identifier allocation matters.
- Timing correlation. The LIP's automatic-claim-on-closure extension
  merges close and payout into one transaction, which forces the
  shielded payout to coincide with the close event and removes the
  provider's claim-timing and batching lever. It is not incompatible with
  receiver privacy (the destination stays shielded and the accrued amount
  is already public from stream state), but it is a timing-correlation
  trade-off.

The in-protocol `provider_id` (NPK) is public and globally linkable across
all streams that share it. Provider receiving privacy is the
unlinkability of that NPK to the provider's primary public key, achieved
by claiming to shielded addresses not tied to the primary identity. The
user who created the stream necessarily knows the `provider_id` and can
link multiple streams to the same provider; shielding does not change
that, because the user saw `provider_id` at stream creation.

## Prerequisites

- [Step 32](../upcoming/step-32-auth-transfer-unify-store-claim.md)
  close-then-claim lifecycle (signed off; D3 gate pending) or equivalent.
- `PAYMENT_STREAMS_GUEST_BIN` and the `logos_execution_zone` wallet module
  with the private-transaction surface (`send_generic_private_transaction`)
  available.
- Step 37 should be implemented after Step 36, or in the same PR, so it can
  reuse the shared `submitGenericPrivate` helper. Step 37 does not duplicate
  the helper.

## Private account identifiers for the provider

A provider private account is identified by its 32-byte `AccountId`, derived
from the provider's NPK plus an identifier.
The user who creates the stream sets `provider_id` to the NPK-derived account id.
When the provider claims, the wallet proves with the matching NSK inside the
privacy-preserving circuit.

Claiming to distinct identifiers under the same NPK produces distinct private
accounts. Consolidation is a hygiene choice made by the provider; the module
exposes it through the `chainAction claim` JSON schema.

## Scope decisions (resolved)

| Decision | Outcome |
| --- | --- |
| Guest transition logic | No change. `test_pp_claim_private_provider_succeeds` in `lez-payment-streams-core/src/program_tests/claim.rs` already runs the standard `Instruction::Claim` with the provider as a private authorized account (visibility-1 slot). The existing slot suffices. |
| Step 36 dependency | Step 37 does not need a `PseudonymousFunder` vault and is not blocked by Step 36. It reuses the `submitGenericPrivate` helper added in Step 36, so it should land after Step 36 or in the same PR. |
| E2E flag | Drive the payee path with `PROVIDER_PRIVACY=1` on `MODE=module` (independent of `OWNER_PRIVACY`). Primary gate is `OWNER_PRIVACY=0 PROVIDER_PRIVACY=1`; also keep one combo run with both flags set. |
| Submit-path selection | Explicit account-slot resolutions, then explicit submit API (D37.9). Do not infer submit mode from vault tier alone, and do not try-public-then-private. |
| Provider-side signing | No patch needed. Claim authorization happens inside the PP circuit; the wallet signs with the NSK during proving. |
| Identifier consolidation | Documented as hygiene, not implemented in-module. Reuse one private provider account id; no claim-time identifier field (D37.10). |
| Automatic-claim-on-closure | Documented as a timing-correlation trade-off, not a hard incompatibility. |
| E2E AT-init / funding | Mirror owner-privacy asymmetry (D37.11): AT-init public accounts only; private provider is `create_account_private` only (no AT, no `Public/$PROVIDER` pinata). |
| `registerProviderMapping` verify | Encoding smoke in this step (D37.12); Store E2E and Store integration owned by [Step 38](../upcoming/step-38-store-privacy-e2e.md). |

## JSON schema for private `chainAction claim`

For provider receiver privacy, the `claim` operation routes through
`submitGenericPrivate` with the provider as a private signer slot.

Field names:

| Field | Meaning |
| --- | --- |
| `owner` | Vault owner account id (hex or base58). Resolution follows the vault tier: public for `Public`, private for `PseudonymousFunder`. The vault PDA itself is always public. |
| `provider` | Provider private account id (hex or base58) that matches the `provider_id` stored in `StreamConfig`. |
| `vault_id` | Vault id. |
| `stream_id` | Stream id. |

The field shape reuses the Step 36 private account id convention (D36.3).
There is no `provider_private_identifier` on `claim` (D37.10). Consolidation
hygiene is: create one private provider account and pass that same account id
as `provider` on `createStream` and `claim`. Guest `claim` already requires
`provider.account_id == StreamConfig.provider`, so the payee account is fixed
at stream creation. Richer identifier tooling is tracked in
[private-account-identifier-management.md](../raw-todos/private-account-identifier-management.md).

The `claim` account list is mixed: public non-signing PDAs
(vault_config, vault_holding, stream_config, clock) plus the private provider
signer; if the vault is `PseudonymousFunder`, the owner is also a private
non-signing account.

## Submit-path rule (D37.9)

Do not choose public vs private submit from vault `privacy_tier` alone, and do
not guess with fallbacks (no try-public-then-private).

For every vault-touching `chainAction` prepare path:

1. Build the planned account list with an explicit per-slot resolution
   (`private`, `public_sign`, or `public_no_sign`) from known facts for that
   op (which accounts are private identities, who must sign).
2. Choose the wallet API by a hard rule on those slots:
   - if any slot resolution is `private` → call only `submitGenericPrivate`
   - otherwise → call only `submitGenericPublic`
3. Keep vault-tier policy as a separate invariant where needed (for example
   refuse a public submit that touches a `PseudonymousFunder` vault). That is
   not a substitute for step 1–2.

Consequences locked by this rule:

- `createStream` — `provider_id` is instruction data only; it is not an
  account slot. On a `Public` vault with a public owner signer, create stays
  `submitGenericPublic` even when `provider_id` is a private account id.
- `claim` — provider is a signer slot. When that provider is a private
  account, mark the slot `private` and call only `submitGenericPrivate`,
  including on a `Public` vault (`OWNER_PRIVACY=0 PROVIDER_PRIVACY=1`).
- `closeStream` — if `authority` is present and is a private account, mark
  that slot `private` and call only `submitGenericPrivate`. Payer-only close
  (omit authority) on a public vault with a public owner stays public submit.

Classify private vs public account identity using the wallet’s private-account
surface (resolve / keychain) for the account ids the caller already passed.
Do not silently upgrade a public submit after failure.

## Implementation plan

1. Shared dependency. Reuse the `submitGenericPrivate` helper added in
   Step 36. Step 37 does not duplicate the helper. See D37.1.

2. Provider key publication. This mirrors the public-mode identity sharing
   pattern. Document the provider publishing an NPK/VPK pair via
   `logos_execution_zone get_private_account_keys` and the user creating the
   stream with the NPK-derived `provider_id`. Mapping logic is unchanged
   (N5); only the account id type changes. Store dual-host use of the
   mapping is Step 38 (D37.12).

3. Submit-path refactor (D37.9). In
   `payment_streams_module_writes.cpp`, replace vault-tier-only submit
   selection with explicit slot resolutions then explicit
   `submitGenericPrivate` / `submitGenericPublic`. Update claim (and close
   when authority is private) so a private provider on a public vault takes
   the private path. Preserve the PF vault “no public submit” invariant as a
   separate check. Reuse Step 36 account-id conventions (D36.3).

4. Provider mapping encoding (D37.12). Add a module unit / LogosTest that
   `registerProviderMapping(peerId, privateProviderB58)` succeeds and that
   prepare / map lookup yields the same 32-byte `provider_id` as
   `StreamConfig.provider` (base58 → bytes, no dual-host Store). Module
   lifecycle E2E does not require Store mapping. Paid Store query and
   Store integration with that mapping are [Step 38](../upcoming/step-38-store-privacy-e2e.md).

5. Identifier hygiene. Document that reusing one private provider account id
   (one `(npk, identifier)` chosen at account creation) for create and claim
   keeps consolidation in a single private account chain. Do not add a claim
   JSON identifier field (D37.10). GMS shared private accounts are out of
   scope. Future identifier-management ideas:
   [private-account-identifier-management.md](../raw-todos/private-account-identifier-management.md).

6. Timing. Document that automatic-claim-on-closure is a
   timing-correlation trade-off when used with receiver privacy; recommend
   claim batching or delay as alternatives.

7. Tests. Extend the PP `program_tests` claim coverage to a private
   recipient at the module submit layer. Add module-level tests that claim
   (and close-with-private-authority if exercised) select private submit from
   slot resolutions, not from vault tier alone, and that `vault_holding` drop
   is visible while the destination is hidden. Run with `RISC0_DEV_MODE=1`.

8. Journey doc. Extend `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` with the
   payee-side claiming walkthrough. Do not modify `USER_JOURNEY.md` or
   `DEVELOPER_JOURNEY.md`.

9. E2E profile. Wire `PROVIDER_PRIVACY=1` in `scripts/module-e2e.sh` (create
   private provider, stream `provider_id` = that account, claim via private
   submit). AT-init and funding follow D37.11 (public parties only for AT /
   pinata; private provider create-only). Keep `OWNER_PRIVACY` behavior from
   Step 36 unchanged and combinable. Update [E2E.md](../../journeys/E2E.md)
   and [verification-matrix.md](../../reference/verification-matrix.md) for
   the provider-privacy module cell. Store × privacy remains out of scope
   ([Step 38](../upcoming/step-38-store-privacy-e2e.md)).

## Decision log

| Id | Topic | Outcome |
| --- | --- | --- |
| D37.1 | Shared helper with Step 36 | Step 36 adds `submitGenericPrivate` in `payment_streams_module_writes.cpp`. Step 37 reuses it. Do not duplicate the helper. |
| D37.2 | Provider key publication | Mirrors the public-mode identity sharing pattern. Provider publishes NPK/VPK via `logos_execution_zone get_private_account_keys`. The user derives the NPK-derived account id and creates the stream with that `provider_id`. The module only routes `claim` through `submitGenericPrivate`. |
| D37.3 | `registerProviderMapping` encoding | N5 maps `PeerId` to the 32-byte stream payee `AccountId`. For a shielded provider this is the NPK-derived id. No design change to the mapping API. Verification depth is D37.12. |
| D37.4 | Identifier consolidation | Documented as hygiene. Reuse one private provider account id for the claim chain to avoid linear spend cost. GMS shared custody is out of scope. |
| D37.5 | Automatic-claim-on-closure | Documented as a timing-correlation trade-off: it forces the shielded payout to coincide with the close event. The destination stays shielded; the amount is already public. |
| D37.6 | `chainAction claim` field shape | Same private account id convention as Step 36 (D36.3): `owner`, `provider`, `vault_id`, `stream_id` only. |
| D37.7 | Provider-side signing patch | None needed. Claim authorization happens inside the PP circuit; the wallet signs with the NSK during proving. |
| D37.8 | E2E flags | Use `PROVIDER_PRIVACY=1` for this step. Do not overload `PRIVACY` / `OWNER_PRIVACY`. Owner and provider privacy remain independently toggleable. |
| D37.9 | Submit-path selection | Explicit per-slot resolutions, then hard rule: any `private` slot → only `submitGenericPrivate`; else only `submitGenericPublic`. No vault-tier-only inference and no public/private fallback. Vault-tier “PF forbids public submit” stays a separate invariant. |
| D37.10 | No claim-time identifier field | Do not add `provider_private_identifier` on `claim`. Guest binds payee to `StreamConfig.provider`; Step 36 already uses account ids only. Future identifier tooling: [raw-todos/private-account-identifier-management.md](../raw-todos/private-account-identifier-management.md). |
| D37.11 | E2E AT-init / funding for private provider | Mirror Step 36 owner-privacy asymmetry. AT-init only public accounts (`wallet auth-transfer init` / `register_public_account` on `Public/$acct`). Never AT-init a private provider. Public owner (when `OWNER_PRIVACY=0`) keeps pinata as today. Private provider: no `Public/$PROVIDER` pinata; dust `transfer_shielded_owned` into the private provider so claim has a committed private note (create-only without a note fails private submit). Combo `OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`: AT neither private party; funder → pre-shield owner and dust-shield provider. |
| D37.12 | `registerProviderMapping` verify depth | Step 37 owns encoding smoke only: register with NPK-derived base58 and assert prepare / map lookup uses the correct 32-byte `provider_id` (no dual-host Store). Step 38 owns Store E2E and Store integration (`registerProviderMapping` before paid `storeQuery`, settlement). Do not build a Store-shaped harness in this step. |

### D37.9 rationale: explicit slots then explicit submit API

Step 36 routed private submit from vault `PseudonymousFunder` tier. That is
insufficient for Step 37’s primary gate (public vault, private provider
claim): tier alone would keep claim on `submitGenericPublic`.

Guessing (for example “if `resolve_private` succeeds, upgrade to private”) or
try-public-then-private invites silent wrong paths. The chosen rule matches
how the private wallet API already consumes `account_slots`: the module states
resolutions, then calls the required prepare/submit function. The wallet
executes that path; it does not invent mode behind the module.

### D37.11 rationale: AT-init and funding asymmetry

AT-init sets public-account `program_owner` to `authenticated_transfer` so
deposit can chain an AT debit. That API is `Public/$acct` only; private
accounts do not use it. Non-zero pinata balance is a public fee/spendable-funds
concern, not a private-account create prerequisite. Mirroring Step 36 keeps
script branching small and avoids inventing dust pre-shield or fake
`Public/$PROVIDER` topups until a concrete failure requires them.

### D37.12 rationale: encoding smoke vs Store E2E

`registerProviderMapping` is host-local N5 glue (`PeerId` → payee `AccountId`).
Module lifecycle E2E never needs Store routing. Encoding bugs (base58 decode,
wrong field wiring into prepare) are cheap to catch in a LogosTest and stay
durable when Step 38 adds dual-host Store. Building a temporary Store path in
Step 37 would be throwaway against Step 38’s real orchestrator work.

### D37.2 rationale: provider key publication

The decision is to keep provider key publication as a wallet-CLI prerequisite,
not as a `payment_streams_module` method. This mirrors the public-mode flow
where the provider shares a public account id (via the discovery layer or
out-of-band) and the user creates the stream with that `provider_id`. In
private mode, the provider publishes the NPK/VPK pair via
`logos_execution_zone get_private_account_keys`, and the user derives the
NPK-derived account id from it.

Pros of wallet-CLI prerequisite:

- Key management is the wallet's responsibility.
- No new module code.
- The provider already uses the wallet for claims, so the same tool publishes keys.
- The discovery/mapping logic (`registerProviderMapping`) stays unchanged; only
  the account id type changes.

Cons of wallet-CLI prerequisite:

- The user must export keys and pass them to the stream creator out of band.
- The module cannot validate that `provider_id` matches the provider's wallet keys.

Pros of a module method:

- Module could generate and return the NPK-derived `provider_id` directly.
- Easier to automate in scripts.

Cons of a module method:

- Turns the module into a key-management utility, which is outside its scope.
- The wallet already exposes `create_private_accounts_key` and
  `get_private_account_keys` via `logos_execution_zone`.

The lean is wallet-CLI because key publication is a generic wallet concern, not
a payment-streams-specific operation. The same `registerProviderMapping`
logic used in public mode can be reused with an NPK-derived `provider_id`.

## Risk

Step 37 is lower risk than Step 36 because no new wallet signing patch is needed.
The only risk is landing before Step 36: if the shared `submitGenericPrivate`
helper does not exist yet, Step 37 would have to add it and then Step 36 would
reuse it, which reverses the intended dependency.

Mitigation: implement Step 36 first, or implement both in the same PR so the
helper is shared from the start.

## Verification

| Gate | Command | Pass criteria |
| --- | --- | --- |
| PP program tests | `RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core` | `test_pp_claim_private_provider_succeeds` and new module-layer private-claim tests pass. |
| Module private claim | Unit test in `logos-payment-streams-module/tests/` (to be added) | `claim` routes to `submitGenericPrivate` when `provider` resolves to a private account; destination is hidden, `vault_holding` drop is visible. |
| Mapping encoding (D37.12) | Module unit / LogosTest (to be added) | `registerProviderMapping` with NPK-derived provider base58 succeeds; prepare / map lookup yields the same 32-byte `provider_id`. No Store query. |
| Payee Journey local | `MODE=module CHAIN=local PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` | Payee claim to a private receiving account succeeds (`OWNER_PRIVACY=0`). |
| Combo local | `MODE=module CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` | Both profiles together succeed. |
| Owner-privacy regression | `make verify-module-local-privacy` | Step 36 `OWNER_PRIVACY=1` path unchanged and green. |
| Transparent regression | `make verify-module-local` | Transparent claim path unchanged and green. |

## Deliverables

- [x] Private-claim path in `chainAction` routing `claim` through
  `submitGenericPrivate` with a private provider signer slot (D37.9).
- [x] `claim` credits the provider's private receiving account; the
  `vault_holding` public drop is visible and the destination is hidden.
- [x] `registerProviderMapping` encoding smoke (D37.12): LogosTests cover
  NPK-derived base58 store and hex match. Store E2E for mapping is Step 38.
- [x] PP claim `program_tests` already covered (`test_pp_claim_private_provider_succeeds`).
- [x] Module-level submit-path LogosTests (any private slot → private submit).
- [x] Localnet E2E with `PROVIDER_PRIVACY=1` and combo `OWNER_PRIVACY=1
  PROVIDER_PRIVACY=1` green.
- [x] No regression on transparent claim or `OWNER_PRIVACY=1`.
- [x] `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` extended with the payee-side
  claiming walkthrough.
- [x] [index.md](../index.md) and [AGENTS.md](../../AGENTS.md) updated.

## Definition of done

- [x] Provider can claim accrued funds to a private receiving account not
  tied to its primary identity.
- [x] Shielded claim succeeds via `submitGenericPrivate`; destination
  hidden, amount visible.
- [x] `registerProviderMapping` encoding smoke green (D37.12); Store mapping
  E2E deferred to Step 38.
- [x] Transparent claim path unchanged and green.
- [x] Unit tests for the privacy-enhanced journey flow pass.
- [x] `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` extended with the payee-side
  claiming walkthrough, provider key publication, identifier consolidation
  hygiene, and the automatic-claim-on-closure timing trade-off.
- [x] Step 37 listed complete in `index.md` and `AGENTS.md`.

## Known limitations

- Claim amounts are public because `vault_holding` is a public PDA.
- The user who created the stream necessarily knows the `provider_id` and can
  link multiple streams to the same provider; this is a user-side limitation,
  not a break of the nullifier scheme.
- N claims with distinct identifiers under the same NPK produce N separate
  private accounts; consolidation is hygiene, not enforced.
- Automatic-claim-on-closure forces the shielded payout to coincide with the
  close event; it is a timing-correlation trade-off, not a hard incompatibility.

## Not in scope

- Payer funder unlinkability (Step 36).
- Guest transition-logic changes.
- GMS shared private account implementation.
- Forcing cross-relationship provider NPK rotation.
- logos-docs publication.
- Store integration, Store E2E, and dual-host use of `registerProviderMapping`
  (Developer Journey track; no wire or `delivery_module` changes). Owned by
  [Step 38](../upcoming/step-38-store-privacy-e2e.md), reusing `OWNER_PRIVACY` /
  `PROVIDER_PRIVACY` (D37.12).

## Resolved

All decisions are recorded in the [Decision log](#decision-log) above:
D37.1 (shared helper), D37.2 (provider key publication), D37.3
(`registerProviderMapping` encoding), D37.4 (identifier consolidation),
D37.5 (automatic-claim-on-closure trade-off), D37.6 (claim field shape reuses
Step 36 convention), D37.7 (no provider-side signing patch), D37.8
(independent `PROVIDER_PRIVACY` E2E flag), D37.9 (explicit slots then
explicit submit API), D37.10 (no claim-time identifier field), D37.11
(E2E AT-init / funding mirrors owner-privacy asymmetry), and D37.12
(`registerProviderMapping` encoding smoke here; Store E2E in Step 38).

D37.9 also locks:

- create on a public vault with a private `provider_id` stays public submit
  (`provider` is not an account slot).
- claim with a private provider uses private submit even on a public vault.
- close with a private `authority` uses private submit; payer-only close on a
  public vault stays public submit.

D37.11 also locks:

- AT-init is for public AT program ownership (deposit debit path), not for
  private accounts; skip AT for private provider the same way Step 36 skips
  AT for private owner.
- Do not pinata-topup a private provider as `Public/$PROVIDER`.
- Private provider gets dust `transfer_shielded_owned` (committed note for
  claim); never pinata as `Public/$PROVIDER`.

D37.12 also locks:

- Step 37: encoding smoke only (register + prepare / map lookup bytes).
- Step 38: Store E2E and Store integration with that mapping.
- No Store-shaped harness or paid `storeQuery` gate in this step.

## Related

- [step-36-payer-funder-unlinkability.md](step-36-payer-funder-unlinkability.md) —
  payer side, supplies the shared PP submit wiring.
- [step-38-store-privacy-e2e.md](../upcoming/step-38-store-privacy-e2e.md) —
  Store E2E privacy profiles (depends on this step for provider/full privacy).
- [integration-decisions.md](../../reference/integration-decisions.md) —
  N5 (provider identity mapping), N10 (module writes).
- [PRIVACY_ENHANCED_JOURNEY.md](../../journeys/PRIVACY_ENHANCED_JOURNEY.md) —
  payee-side private-claim walkthrough.
- [private-account-identifier-management.md](../raw-todos/private-account-identifier-management.md) —
  deferred identifier-management ideas.
- [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) and
  [DEVELOPER_JOURNEY.md](../../journeys/DEVELOPER_JOURNEY.md) — unchanged by
  this step.
- LIP-155 Security and privacy considerations — receiver privacy.
