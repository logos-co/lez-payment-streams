# Step 37 — payee receiver privacy via LEZ private execution

Index: [index.md](../index.md). Status: **active** — planning and implementation packet.

Goal: productize LEZ private execution for the receiving service provider
side of payment streams so the provider can claim accrued funds to a
shielded address. This delivers the LIP-155 secondary privacy goal,
provider receiving privacy: limiting linkage between on-chain claims and
the provider's real receiving addresses.

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
| Provider-side signing | No patch needed. Claim authorization happens inside the PP circuit; the wallet signs with the NSK during proving. |
| Identifier consolidation | Documented as hygiene, not implemented. |
| Automatic-claim-on-closure | Documented as a timing-correlation trade-off, not a hard incompatibility. |

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
| `provider_private_identifier` | Optional identifier (hex, 16 bytes) to reuse the same `(npk, identifier)` chain for consolidation. If omitted, the wallet chooses one. |

The field shape reuses the Step 36 private account id convention (D36.3). The
only claim-specific addition is the optional `provider_private_identifier` for
consolidation. The `claim` account list is mixed: public non-signing PDAs
(vault_config, vault_holding, stream_config, clock) plus the private provider
signer; if the vault is `PseudonymousFunder`, the owner is also a private
non-signing account.

## Implementation plan

1. Shared dependency. Reuse the `submitGenericPrivate` helper added in
   Step 36. Step 37 only adds the `chainAction claim` private path that calls
   the helper; it does not duplicate the helper. See D37.1.

2. Provider key publication. This mirrors the public-mode identity sharing
   pattern. Document the provider publishing an NPK/VPK pair via
   `logos_execution_zone get_private_account_keys` and the user creating the
   stream with the NPK-derived `provider_id`. Confirm `registerProviderMapping`
   (N5) carries NPK-derived `provider_id` values end to end; the mapping logic
   is unchanged, only the account id type changes.

3. PP claim path. Add a private-claim path to `chainAction` in
   `logos-payment-streams-module/src/payment_streams_module_writes.cpp`
   that routes `claim` through `submitGenericPrivate` with the provider as a
   private signer slot. Resolve the provider private account via
   `wallet_ffi_resolve_private_account` (called inside `logos_execution_zone`).
   The field shape reuses the Step 36 convention (D36.3); only the optional
   `provider_private_identifier` is claim-specific.

4. Provider mapping. Confirm `registerProviderMapping` maps the
   provider libp2p `PeerId` to the NPK-derived `provider_id`, not to a
   public account. The `PeerId` stays Store-routing only (N5).

5. Identifier hygiene. Document that reusing one `(npk, identifier)` for the
   claim chain keeps consolidation in a single private account chain. GMS
   shared private accounts are out of scope.

6. Timing. Document that automatic-claim-on-closure is a
   timing-correlation trade-off when used with receiver privacy; recommend
   claim batching or delay as alternatives.

7. Tests. Extend the PP `program_tests` claim coverage to a private
   recipient at the module submit layer. Add a module-level test that
   claim routes to a private recipient and that the public `vault_holding`
   drop is visible while the destination is hidden. Add unit tests for the
   new privacy-enhanced journey flow in `logos-payment-streams-module/tests/`
   and in `lez-payment-streams-core` as needed. Run with `RISC0_DEV_MODE=1`.

8. Journey doc. Extend `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` with the
   payee-side claiming walkthrough. Do not modify `USER_JOURNEY.md` or
   `DEVELOPER_JOURNEY.md`.

## Decision log

| Id | Topic | Outcome |
| --- | --- | --- |
| D37.1 | Shared helper with Step 36 | Step 36 adds `submitGenericPrivate` in `payment_streams_module_writes.cpp`. Step 37 reuses it. Do not duplicate the helper. |
| D37.2 | Provider key publication | Mirrors the public-mode identity sharing pattern. Provider publishes NPK/VPK via `logos_execution_zone get_private_account_keys`. The user derives the NPK-derived account id and creates the stream with that `provider_id`. The module only routes `claim` through `submitGenericPrivate`. |
| D37.3 | `registerProviderMapping` encoding | N5 maps `PeerId` to the 32-byte stream payee `AccountId`. For a shielded provider this is the NPK-derived id. This is an encoding verification, not a design change. |
| D37.4 | Identifier consolidation | Documented as hygiene. Reuse one `(npk, identifier)` for the claim chain to avoid linear spend cost. GMS shared custody is out of scope. |
| D37.5 | Automatic-claim-on-closure | Documented as a timing-correlation trade-off: it forces the shielded payout to coincide with the close event. The destination stays shielded; the amount is already public. |
| D37.6 | `chainAction claim` field shape | Reuse the same private account id convention as Step 36 (D36.3). The only claim-specific addition is an optional `provider_private_identifier` for consolidation. |
| D37.7 | Provider-side signing patch | None needed. Claim authorization happens inside the PP circuit; the wallet signs with the NSK during proving. |

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
| Payee Journey local | `MODE=module CHAIN=local ./scripts/e2e.sh local run` with a shielded provider | Payee claim to a private receiving account succeeds. |
| Transparent regression | `make verify-module-local` | Transparent claim path unchanged and green. |

## Deliverables

- [ ] Private-claim path in `chainAction` routing `claim` through
  `submitGenericPrivate` with a private provider signer slot.
- [ ] `claim` credits the provider's private receiving account; the
  `vault_holding` public drop is visible and the destination is hidden.
- [ ] `registerProviderMapping` carries NPK-derived `provider_id`.
- [ ] PP claim `program_tests` pass with `RISC0_DEV_MODE=1`.
- [ ] Module-level test that `claim` routes to a private recipient and that the
  destination is hidden.
- [ ] Localnet E2E payee claim to a shielded address succeeds.
- [ ] No regression on transparent claim.
- [ ] `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` extended with the payee-side
  claiming walkthrough, provider key publication, identifier consolidation
  hygiene, and the automatic-claim-on-closure timing trade-off.
- [ ] [index.md](../index.md) upcoming table and program outcomes list
  Step 37.
- [ ] [AGENTS.md](../../AGENTS.md) active-work pointer lists Step 37.

## Definition of done

- [ ] Provider can claim accrued funds to a private receiving account not
  tied to its primary identity.
- [ ] Shielded claim succeeds via `submitGenericPrivate`; destination
  hidden, amount visible.
- [ ] `registerProviderMapping` carries NPK-derived `provider_id`.
- [ ] Transparent claim path unchanged and green.
- [ ] Unit tests for the privacy-enhanced journey flow pass.
- [ ] `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` extended with the payee-side
  claiming walkthrough, provider key publication, identifier consolidation
  hygiene, and the automatic-claim-on-closure timing trade-off.
- [ ] Step 37 listed in `index.md` and `AGENTS.md`.

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
- Store integration and eligibility hooks (Developer Journey track; no wire or `delivery_module` changes).

## Resolved

All decisions are recorded in the [Decision log](#decision-log) above:
D37.1 (shared helper), D37.2 (provider key publication), D37.3
(`registerProviderMapping` encoding), D37.4 (identifier consolidation),
D37.5 (automatic-claim-on-closure trade-off), D37.6 (claim field shape reuses
Step 36 convention), and D37.7 (no provider-side signing patch).

## Related

- [step-36-payer-funder-unlinkability.md](../completed/step-36-payer-funder-unlinkability.md) —
  payer side, supplies the shared PP submit wiring.
- [integration-decisions.md](../../reference/integration-decisions.md) —
  N5 (provider identity mapping), N10 (module writes).
- [PRIVACY_ENHANCED_JOURNEY.md](../../journeys/PRIVACY_ENHANCED_JOURNEY.md) —
  payee-side private-claim walkthrough.
- [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) and
  [DEVELOPER_JOURNEY.md](../../journeys/DEVELOPER_JOURNEY.md) — unchanged by
  this step.
- LIP-155 Security and privacy considerations — receiver privacy.
