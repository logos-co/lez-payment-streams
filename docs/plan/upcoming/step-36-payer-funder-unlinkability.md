# Step 36 — payer funder unlinkability via LEZ private execution

Index: [index.md](../index.md). Status: **active** — planning and implementation packet.

Goal: productize LEZ private execution for the paying client side of
payment streams so the vault owner identity is shielded from the network.
This delivers the LIP-155 primary privacy goal, funder unlinkability:
separating the user's primary public key from on-chain vault and stream
activity.

## Problem

The MVP runs everything in transparent execution. Vault and stream accounts
are public PDAs, so a `Public`-tier vault owner is the user's primary public
key, and every vault and stream operation links that key on chain. The
on-chain program already carries a `VaultPrivacyTier::PseudonymousFunder`
tier and the harness already proves out the privacy-preserving (PP) flows,
but the module never submits a shielded transaction. The gap is wiring and
enforcement, not cryptography.

## Architectural context

LEZ separates state into public and private accounts. Private accounts are
stored locally, publish commitments and nullifiers on chain, and use
nullifier keys (NSK/NPK) for authorization and viewing keys (VSK/VPK) for
proving and decryption. The same guest bytecode runs for transparent and
shielded execution; the difference is which signing identities and transfer
endpoints appear on chain.

Existing scaffolding in this repo:

- `VaultPrivacyTier { Public, PseudonymousFunder }` in
  `lez-payment-streams-core/src/vault.rs`. The guest stores the tier
  but cannot observe execution mode; the doc comment on `VaultPrivacyTier`
  states the wallet enforces shielded-only policy for
  `PseudonymousFunder` vaults.
- Harness-side enforcement tests in
  `lez-payment-streams-core/src/program_tests/privacy_tier_policy.rs`
  (public touches of a `PseudonymousFunder` vault are rejected at the
  harness, not the guest).
- PP test infrastructure in
  `lez-payment-streams-core/src/program_tests/pp_common.rs`, covering PP
  deposit, PP withdraw to a private recipient (`pp_common.rs`,
  `fund_private_account_via_pp_withdraw`), and PP claim and close with a
  private signer slot (`pp_common.rs`, `pp_claim_close_setup`).

Upstream PP submit already exists and is tested:

- `wallet_ffi_send_generic_private_transaction` in
  `logos-execution-zone/lez/wallet-ffi/src/generic_transaction.rs`,
  declared in `lez/wallet-ffi/wallet_ffi.h`.
- `wallet_ffi_resolve_private_account` in
  `lez/wallet-ffi/src/keys.rs`, declared in `wallet_ffi.h`.
- Wallet module wrapper `LEZCoreModule::send_generic_private_transaction`
  in `logos-execution-zone-module/src/lez_core_module.cpp`, declared
  in `lez_core_module.h`.
- End-to-end test `test_wallet_ffi_transfer_generic_private` in
  `logos-execution-zone/integration_tests/tests/wallet_ffi.rs`.

The caller supplies resolved private `FfiAccountIdentity` list, instruction
words, and `FfiProgramWithDependencies` (main ELF plus dependency ELFs). The
wallet internally handles proving, commitment and nullifier emission, and
encrypted post-states; the FFI takes no `signing_requirements` list and no
recipient VPK or ECDH scalar.

Two constraints from the LIP and the guest:

- The deposit amount is always public. `vault_holding` is a public PDA and
  its balance change appears in public post-states (the `deposit` handler
  comment in `methods/guest/src/bin/lez_payment_streams.rs` on the public
  `vault_holding` post-state). Shielding hides the funder identity, not
  amounts or the vault-to-stream graph.
- One transparent touch on a `PseudonymousFunder` vault permanently links
  its owner on chain; later shielded operations cannot remove that linkage.
  Wallet-side refusal of transparent touches is therefore the real
  enforcement boundary, since the guest cannot enforce execution mode.

The in-protocol identity (the vault owner NPK) is public and globally
linkable across all vaults and streams that share it. Funder unlinkability
is the unlinkability of that NPK to the user's primary public key, achieved
by a one-way shielding step whose downstream nullifier is unlinkable to the
shielding commitment. Amount and timing correlation across the shielding
boundary can weaken this heuristically; it is a side channel, not a break of
the nullifier scheme.

## Prerequisites

- [Step 32](../upcoming/step-32-auth-transfer-unify-store-claim.md)
  close-then-claim lifecycle (signed off; D3 gate pending) or equivalent.
- [Step 33](../completed/step-33-store-e2e-fresh-vault.md) fresh-vault
  behavior only if the local E2E is exercised through the fresh-vault path.
- `PAYMENT_STREAMS_GUEST_BIN` and the `logos_execution_zone` wallet module
  with the private-transaction surface (`send_generic_private_transaction`)
  available.
- The repo-local `sign_public_payload` patch (N1) already applied to the
  wallet module wrapper.

## Private account identifiers

A private account is identified by its 32-byte `AccountId`.
That id is derived from a nullifier public key (NPK) plus an identifier.
The `payment_streams_module` never sees the secret nullifier key (NSK);
`logos_execution_zone` resolves the private account via
`wallet_ffi_resolve_private_account` and the wallet proves with the NSK
inside the privacy-preserving circuit.

For `PseudonymousFunder` vaults, the `VaultConfig.owner` field is the NPK-derived
account id of the vault owner.
The funding private account used for the PP deposit must be derived from the same
NPK as the vault owner.
`payment_streams_module` does not enforce this equality; it is a wallet-side
prerequisite that the User Journey must document.

## Scope decisions (resolved)

| Decision | Outcome |
| --- | --- |
| Guest transition logic | No change. The guest is visibility-agnostic. The only guest-side given is the existing `PseudonymousFunder` tier and the NPK-derived owner invariant from the LIP. |
| Shielded-only enforcement | Moves from the test harness into `payment_streams_module` (refuse public submit for `PseudonymousFunder` vaults). The guest stays unenforcing by design. |
| Step 37 dependency | The shared PP submit wiring is a prerequisite for Step 37, which reuses the same private submit path. Step 36 does not depend on Step 37. |
| Cross-relationship vault rotation | Documented as a hygiene recommendation, not enforced. |
| Pre-shielding | Out of scope of `payment_streams_module`; documented as a wallet-CLI prerequisite. The module adds an NPK preflight check in the PP `deposit` path. |

## JSON schema for private `chainAction` operations

For `PseudonymousFunder` vaults, operations that touch the vault route through
`submitGenericPrivate` and use private account ids for the vault owner and the
funding source.

A private account id can be passed as 64-character hex or as base58.
`payment_streams_module` resolves the account based on the vault privacy tier:

- For `Public` vaults, it resolves `signer`/`owner` as a public account.
- For `PseudonymousFunder` vaults, it resolves `signer`/`owner` as a private
  account via `wallet_ffi_resolve_private_account`.

Resolved convention: keep the existing field names (`signer`, `owner`,
`provider`) and let the vault tier determine the resolution path. Do not
introduce a separate `private_signer` field. This is the simplest option and is
consistent with the existing `chainAction` API where the user already passes an
account id in `signer` and the module handles wallet resolution.

| Operation | `signer` / `owner` / `provider` value | Resolution |
| --- | --- | --- |
| `initializeVault` | vault owner account id | Private for `PseudonymousFunder`, public for `Public` |
| `deposit` | funding account id | Private for `PseudonymousFunder`; must share the vault owner NPK |
| `createStream`, `pauseStream`, `resumeStream`, `topUpStream`, `closeStream` | vault owner account id | Private for `PseudonymousFunder`, public for `Public` |
| `claim` | `owner` = vault owner account id; `provider` = provider account id | `owner` private for `PseudonymousFunder`; `provider` public unless Step 37 private claim is used |

## Implementation plan

1. Wallet module surface. Confirm
   `send_generic_private_transaction` is callable from the PS module via
   Qt dynamic dispatch. It is multi-arg (`account_ids`, `instruction`,
   `program_elf`, `program_dependencies`) and takes ELF bytes plus
   dependency ELFs, not a `program_id_hex`. Decide whether to add a
   repo-local `send_generic_private_transaction_json` convenience patch
   mirroring the N10 public JSON wrapper, or call the multi-arg method
   directly. Resolved as D36.2.

2. PS module writes. Add `submitGenericPrivate` alongside
   `submitGenericPublic` in
   `logos-payment-streams-module/src/payment_streams_module_writes.cpp`,
   next to `submitGenericPublicViaFfi`.
   Supply the guest ELF (`PAYMENT_STREAMS_GUEST_BIN`). For PP `deposit`, also
   supply the `authenticated_transfer` dependency ELF; other PP operations do
   not need it unless the guest requires additional dependencies. Resolve
   private accounts via `wallet_ffi_resolve_private_account` (called inside
   `logos_execution_zone`, not directly from the PS module).

3. Tier routing. Read `VaultConfig.privacy_tier` (already decoded in
   `payment_streams_module_impl.cpp` where `VaultConfig` is decoded). For
   `PseudonymousFunder`, route to `submitGenericPrivate` and refuse
   `submitGenericPublic`. This is where the shielded-only rule leaves the
   harness and enters the module.

4. `chainAction` signer fields. Overload the existing `signer` / `owner`
   fields with account ids (hex or base58). The module resolves them as
   public or private based on the vault privacy tier. See D36.3.

5. Pre-shielding flow. Keep pre-shielding as a generic wallet-CLI
   public-to-private transfer, documented as a prerequisite in
   `PRIVACY_ENHANCED_JOURNEY.md`. Add a module-level NPK preflight check in
   the PP `deposit` path: resolve the vault owner private account and the
   funding private account through `get_private_account_keys`, compare their
   NPKs, and reject the deposit if they differ. This catches the most common
   user error without wrapping the transfer inside the module.

6. Eligibility signing. `VaultProof.owner_signature` must be signed by the
   private account's NSK. Implement the repo-local `sign_private_payload`
   patch (D36.4) on the wallet wrapper, mirroring the existing
   `sign_public_payload` patch (N1).

7. Tests. Add PP `program_tests` for the full `PseudonymousFunder`
   lifecycle (init, PP deposit, create_stream, pause, resume, top_up,
   close, claim) at the module submit layer. Add a module-level test that
   public submit is refused for `PseudonymousFunder` vaults. Add a module-level
   test that the PP `deposit` preflight rejects a funding account whose NPK
   does not match the vault owner NPK. Add unit tests for the new
   privacy-enhanced journey flow in `logos-payment-streams-module/tests/` and
   in `lez-payment-streams-core` as needed. Run PP tests with
   `RISC0_DEV_MODE=1`.

8. Journey doc. Add a payer-side shielding and PP lifecycle walkthrough to
   `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md`. Do not modify
   `USER_JOURNEY.md` or `DEVELOPER_JOURNEY.md`.

## Decision log

| Id | Topic | Outcome |
| --- | --- | --- |
| D36.1 | Pre-shielding scope | Keep pre-shielding as a generic wallet public-to-private transfer, out of scope of `payment_streams_module`. Add a module-level NPK preflight check in the PP `deposit` path to catch mismatches. Document the prerequisite and the NPK-equality invariant in `PRIVACY_ENHANCED_JOURNEY.md`. |
| D36.2 | Private submit IPC shape | Call the multi-arg `send_generic_private_transaction` directly via Qt dynamic dispatch. No `send_generic_private_transaction_json` repo-local patch unless investigation shows the private path requires a JSON envelope. |
| D36.3 | `chainAction` private account id encoding | Overload the existing `signer` / `owner` / `provider` fields. The module resolves the account as public or private based on the vault privacy tier. Step 37 reuses the same convention. |
| D36.4 | Owner off-chain signing | Add a repo-local `sign_private_payload` on the patched wallet wrapper that retrieves the NSK for an owned private account and produces a Schnorr signature over the digest. This is the Step 36 schedule risk. |
| D36.5 | Cross-relationship vault rotation | Documented as hygiene, not implemented. |
| D36.6 | Amount and timing correlation | Documented as known traffic-analysis limitations, not implemented. |

## Risk

The schedule risk is the `sign_private_payload` patch (D36.4).
Upstream `wallet_ffi` exposes no raw signing primitive for private accounts.
The existing `sign_public_payload` patch can only sign with public-account keys.
Extending it to retrieve an owned private account's NSK and produce a Schnorr
signature without exporting the NSK across the FFI boundary is feasible but
non-trivial.

Why it is the long pole: the patch touches both the Rust FFI layer
(`wallet_ffi_sign_private_payload`) and the C++ wallet wrapper
(`sign_private_payload`), and must be built, tested, and landed before the module
routing changes can produce a verifiable `VaultProof.owner_signature`.

Other options considered and rejected:

- Skip eligibility proofs for `PseudonymousFunder` vaults. This breaks the paid
  Store use case for private vaults, so it is not viable.
- Replace the off-chain signature with a zero-knowledge ownership proof. This
  is outside the current LIP and much larger scope than a signing patch.
- Produce the signature inside a PP transaction and use that transaction as
  the proof. This changes the eligibility protocol and is not compatible with
  the existing `VaultProof.owner_signature` design.

Mitigation: split the patch into a Rust FFI addition (`wallet_ffi_sign_private_payload`)
and a C++ wrapper slot (`sign_private_payload`), mirroring the existing public patch.
If the patch proves large, land it as a separate PR before the module routing changes.

## Verification

| Gate | Command | Pass criteria |
| --- | --- | --- |
| PP program tests | `RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core` | Existing PP tests plus new `PseudonymousFunder` lifecycle tests pass. |
| Module private submit | Unit test in `logos-payment-streams-module/tests/` (to be added) | Public submit rejected for `PseudonymousFunder` vaults; private submit accepted. |
| User Journey local | `MODE=module CHAIN=local ./scripts/e2e.sh local run` | Full `PseudonymousFunder` lifecycle succeeds. |
| Public tier regression | `make verify-module-local` | `Public`-tier flows unchanged and green. |

## Deliverables

- [ ] `submitGenericPrivate` implemented in `payment_streams_module_writes`
  and called for every `PseudonymousFunder` vault operation that touches the vault.
- [ ] Public submit refused for `PseudonymousFunder` vaults at the module.
- [ ] PP deposit from a pre-shielded private account succeeds on localnet.
- [ ] PP `deposit` preflight rejects a funding account whose NPK does not match
  the vault owner NPK.
- [ ] Full `PseudonymousFunder` lifecycle executable via shielded submits.
- [ ] `VaultProof.owner_signature` signed by the private owner NSK verifies
  under the provider FFI.
- [ ] PP `program_tests` pass with `RISC0_DEV_MODE=1`.
- [ ] Module-level test that public submit is rejected for `PseudonymousFunder` vaults.
- [ ] Localnet E2E `MODE=module CHAIN=local ./scripts/e2e.sh local run`
  passes for a `PseudonymousFunder` vault.
- [ ] No regression on the `Public` tier.
- [ ] `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` documents the pre-shielding
  prerequisite, the NPK-equality invariant, cross-relationship vault rotation
  as hygiene, and the known traffic-analysis limitations.
- [ ] [index.md](../index.md) upcoming table and program outcomes list
  Step 36.
- [ ] [AGENTS.md](../../AGENTS.md) active-work pointer lists Step 36.

## Definition of done

- [ ] `submitGenericPrivate` ships and is routed for `PseudonymousFunder`.
- [ ] Transparent submit of a `PseudonymousFunder` vault is rejected by the
  module.
- [ ] PP deposit, create_stream, pause, resume, top_up, close, and claim
  all succeed via shielded submits on localnet.
- [ ] PP `deposit` preflight rejects a funding account whose NPK does not match
  the vault owner NPK.
- [ ] Eligibility proof over a `PseudonymousFunder` vault verifies.
- [ ] `Public`-tier flows unchanged and green.
- [ ] Unit tests for the privacy-enhanced journey flow pass.
- [ ] `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` documents the payer-side
  pre-shielding prerequisite, the NPK-equality invariant, and the known
  traffic-analysis limitations.
- [ ] Step 36 listed in `index.md` and `AGENTS.md`.

## Known limitations

- Deposit amounts are public because `vault_holding` is a public PDA.
- Amount and timing correlation across the shielding boundary are side channels
  that can weaken unlinkability heuristically; they are not breaks of the
  nullifier scheme.
- Cross-relationship vault rotation is documented as hygiene, not enforced.

## Not in scope

- Payee receiver privacy (Step 37).
- Guest transition-logic changes.
- Forcing cross-relationship vault rotation.
- Traffic-analysis mitigations beyond documentation.
- logos-docs publication.
- Store integration and eligibility hooks (Developer Journey track; no wire or `delivery_module` changes).

## Resolved

All decisions are recorded in the [Decision log](#decision-log) above:
D36.1 (wallet-CLI pre-shielding + module NPK preflight), D36.2 (multi-arg
private submit), D36.3 (overload existing `chainAction` fields), D36.4
(`sign_private_payload` patch), D36.5 (vault rotation as hygiene), and D36.6
(amount/timing correlation documented as limitations).

If E2E automation or UX proves painful in practice, revisit a thin
`payment_streams_module` helper that wraps the wallet transfer. The helper would
look like a new `chainAction` operation, e.g. `shieldAndDeposit`, taking
`public_account_id`, `private_account_id`, `vault_id`, `amount_lo`,
`amount_hi`, calling wallet `transfer_shielded_owned`, then submitting the PP
deposit.

## Related

- [step-37-payee-receiver-privacy.md](step-37-payee-receiver-privacy.md) —
  payee side, reuses this step's PP submit wiring.
- [integration-decisions.md](../../reference/integration-decisions.md) —
  N1 (off-chain signing), N5 (provider identity mapping), N10 (module
  writes).
- [PRIVACY_ENHANCED_JOURNEY.md](../../journeys/PRIVACY_ENHANCED_JOURNEY.md) —
  payer-side pre-shielding and PP lifecycle walkthrough.
- [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) and
  [DEVELOPER_JOURNEY.md](../../journeys/DEVELOPER_JOURNEY.md) — unchanged by
  this step.
- LIP-155 Security and privacy considerations — funder unlinkability.
