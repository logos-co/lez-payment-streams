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

The public leg of the pre-shield transfer is a one-time transparent touch that
links the user's primary public key to the shielding commitment. It is outside
the "one transparent touch" rule above, which applies to vault and stream
operations, not to the initial funding step. Funder unlinkability assumes the
shielding commitment is unlinkable to the downstream nullifiers used by the
vault owner account.

## Prerequisites

- The close-then-claim contract from
  [Step 32](../upcoming/step-32-auth-transfer-unify-store-claim.md) (signed off;
  D3 testnet gate pending). Step 36 can start in parallel with Step 32 as long as
  the close-then-claim instruction shape and account layout do not change.
- [Step 33](../completed/step-33-store-e2e-fresh-vault.md) fresh-vault behavior only
  if the local E2E is exercised through the fresh-vault path.
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
The PP `deposit` debits the vault owner account directly (the guest derives the
`vault_config` PDA from the owner account id), so the funding private account must
be the same account id as the vault owner. Pre-shielding therefore moves funds
into the vault owner private account before the vault is initialized, not into a
separate account under the same NPK.
The User Journey must document this invariant. The module enforces it by checking
that the `signer` account id passed to `deposit` matches the on-chain
`VaultConfig.owner` before the transaction reaches the guest.

## Scope decisions (resolved)

| Decision | Outcome |
| --- | --- |
| Guest transition logic | No change. The guest is visibility-agnostic. The only guest-side given is the existing `PseudonymousFunder` tier and the NPK-derived owner invariant from the LIP. |
| Shielded-only enforcement | Moves from the test harness into `payment_streams_module` (refuse public submit for `PseudonymousFunder` vaults). The guest stays unenforcing by design. |
| Step 37 dependency | The shared PP submit wiring is a prerequisite for Step 37, which reuses the same private submit path. Step 36 does not depend on Step 37. |
| Cross-relationship vault rotation | Documented as a hygiene recommendation, not enforced. |
| Pre-shielding | Out of scope of `payment_streams_module`; documented as a wallet-CLI prerequisite. The module checks that the PP `deposit` signer equals `VaultConfig.owner`. |

## JSON schema for private `chainAction` operations

For `PseudonymousFunder` vaults, operations that touch the vault route through
`submitGenericPrivate` and use the vault owner private account id as the signer
(or as the non-signing owner slot for `closeStream` and `claim`).

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

`initializeVault` is the only operation that creates the vault, so it cannot
read an existing `VaultConfig.privacy_tier`. It must accept a new
`privacy_tier` JSON field (`0` for `Public`, `1` for `PseudonymousFunder`).
When the tier is `PseudonymousFunder`, the module routes through
`submitGenericPrivate` and sets the guest `VaultConfig.privacy_tier` to `1`.
All subsequent operations read the stored tier from chain state and route
accordingly.

| Operation | `signer` / `owner` / `provider` value | Resolution |
| --- | --- | --- |
| `initializeVault` | vault owner account id | Private for `PseudonymousFunder`, public for `Public` |
| `deposit` | vault owner account id | Private for `PseudonymousFunder`; must equal `VaultConfig.owner` |
| `createStream`, `pauseStream`, `resumeStream`, `topUpStream` | vault owner account id | Private for `PseudonymousFunder`, public for `Public` |
| `closeStream` | `signer` = vault owner account id; `authority` = provider account id | `signer` private for `PseudonymousFunder`; `authority` can be public or private. The owner slot is non-signing. |
| `claim` | `owner` = vault owner account id; `provider` = provider account id | `owner` private for `PseudonymousFunder`; `provider` can be public or private. The transaction is private because the vault owner is private, not because the provider is. |

### Tier-based account resolution

For `PseudonymousFunder` vaults, every account slot that must be private is
resolved as private and hard-fails if it cannot be resolved as private. The
module never falls back to public resolution for a slot that the vault tier says
must be private. This is the enforcement of the "one transparent touch" rule at
the module boundary.

Resolution rule:

1. Determine the expected identity type for each account slot from the operation
   and the vault privacy tier (see the table below).
2. For slots expected to be private, call
   `wallet_ffi_resolve_private_account(account_id)`. If it fails, return an error
   immediately; do not try public resolution.
3. For slots expected to be public, call
   `wallet_ffi_resolve_public_account(account_id, needs_sign)` with the correct
   signing flag.
4. For public read-only PDA slots, pass `needs_sign = false`.

Foreign private accounts (e.g., a payer's private account seen by the provider)
are not supported until Step 37.

### Per-operation `FfiAccountIdentity` list

The list order matches the guest instruction account layout in
`lez-payment-streams-core/src/instruction_accounts.rs`. For `PseudonymousFunder`
vaults, the owner and signer slots are private; for `Public` vaults, they are
public. Public PDAs are always `PublicNoSign`.

| Operation | Guest account order | Identity per slot (PseudonymousFunder) | Notes |
| --- | --- | --- | --- |
| `initializeVault` | vault_config, vault_holding, owner | PublicNoSign, PublicNoSign, PrivateOwned signer | `owner` is the vault owner private account. |
| `deposit` | vault_config, vault_holding, owner | PublicNoSign, PublicNoSign, PrivateOwned signer | `owner` must equal `VaultConfig.owner`; the guest debits it. |
| `withdraw` | vault_config, vault_holding, owner, recipient | PublicNoSign, PublicNoSign, PrivateOwned signer, PrivateOwned or PublicNoSign | `recipient` is private when withdrawing to a shielded address. |
| `createStream`, `pauseStream`, `resumeStream`, `topUpStream` | vault_config, vault_holding, stream_config, owner, clock | PublicNoSign, PublicNoSign, PublicNoSign, PrivateOwned signer, PublicNoSign | `owner` is the vault owner private account. |
| `closeStream` | vault_config, vault_holding, stream_config, owner, authority, clock | PublicNoSign, PublicNoSign, PublicNoSign, PrivateOwned non-signer, Public or PrivateOwned signer, PublicNoSign | `authority` is the provider; `owner` is not a signer for this instruction. |
| `claim` | vault_config, vault_holding, stream_config, owner, provider, clock | PublicNoSign, PublicNoSign, PublicNoSign, PrivateOwned non-signer, Public or PrivateOwned signer, PublicNoSign | `provider` is the signer; `owner` is private non-signer for a `PseudonymousFunder` vault. |

For `Public` vaults, replace `PrivateOwned` owner slots with `Public` signer or
`PublicNoSign` non-signer as appropriate, and make the private recipient in
`withdraw` a public account.

## Implementation plan

1. Wallet module surface. Call the multi-arg
   `send_generic_private_transaction` directly via Qt dynamic dispatch.
   The signature is
   `send_generic_private_transaction(account_identities, instruction_words,
   program_elf, program_dependencies)`. It needs an `FfiAccountIdentity`
   list, not a JSON envelope, so the JSON-wrapper escape hatch is rejected.
   Resolved as D36.2.

2. PS module writes. Add `submitGenericPrivate` alongside
   `submitGenericPublic` in
   `logos-payment-streams-module/src/payment_streams_module_writes.cpp`,
   next to `submitGenericPublicViaFfi`.
   Build the `FfiAccountIdentity` list per instruction (private owner +
   public or private signer + public non-signing PDAs) and call
   `send_generic_private_transaction` with the guest ELF
   (`PAYMENT_STREAMS_GUEST_BIN`) and the instruction words.
   For PP `deposit`, also supply the `authenticated_transfer` dependency
   ELF; other PP operations do not need it (see dependency matrix below).

3. Tier routing. For `initializeVault`, read the `privacy_tier` field from the
   `chainAction` JSON. For all other operations, read `VaultConfig.privacy_tier`
   from the decoded vault config on chain (already decoded in
   `payment_streams_module_writes.cpp`). For `PseudonymousFunder`, route to
   `submitGenericPrivate` and refuse `submitGenericPublic`. This is where the
   shielded-only rule leaves the harness and enters the module.

4. `chainAction` signer fields. Overload the existing `signer` / `owner`
   fields with account ids (hex or base58). The module resolves them as
   public or private based on the vault privacy tier. See D36.3.

5. Pre-shielding flow. Keep pre-shielding as a generic wallet-CLI
   public-to-private transfer, documented as a prerequisite in
   `PRIVACY_ENHANCED_JOURNEY.md`. The E2E script pins
   `logos_execution_zone transfer_shielded_owned`; the exact operator CLI syntax
   stays in the journey doc with a "verify syntax" caveat. Add a module-level
   deposit-signer check in the PP `deposit` path: the `signer` account id must
   equal the on-chain `VaultConfig.owner`, because the guest derives the
   `vault_config` PDA from that account id and debits it. This rejects the most
   common user error (passing the wrong private account) without wrapping the
   transfer inside the module.

6. Eligibility signing. `VaultProof.owner_signature` must be signed by the
   private account's NSK. Implement the repo-local `sign_private_payload`
   patch (D36.4) on the wallet wrapper, mirroring the existing
   `sign_public_payload` patch (N1). Inside Rust, retrieve the NSK from
   `wallet.storage().key_chain().private_account(account_id).key_chain.private_key_holder.nullifier_secret_key`,
   construct a `PrivateKey`, and call `Signature::new`. The signed digest is
   exactly `vault_owner_auth_canonical_payload_digest` from
   `lez-payment-streams-core/src/off_chain/canonical.rs`, and the verification
   public key is the owner's NPK. The provider already verifies this with the
   existing Rust helper `verify_stream_proposal_vault_signature`; no new
   provider-side FFI is needed.

7. Tests. Add Rust `program_tests` in `lez-payment-streams-core` for the full
   `PseudonymousFunder` lifecycle (init, PP deposit, create_stream, pause,
   resume, top_up, close, claim). These tests model the module submit layer
   by calling `execute_and_prove` directly with the correct account identity
   mix. Add a harness-side test that a public transition touching a
   `PseudonymousFunder` vault is rejected (extend
   `src/program_tests/privacy_tier_policy.rs`). Add C++ module-level tests in
   `logos-payment-streams-module/tests/` that public submit is refused and that
   the PP `deposit` signer check rejects mismatched account ids. Add unit tests for
   the privacy-enhanced journey flow. Run PP tests with `RISC0_DEV_MODE=1`.

8. Journey doc. Add a payer-side shielding and PP lifecycle walkthrough to
   `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md`. Do not modify
   `USER_JOURNEY.md` or `DEVELOPER_JOURNEY.md`.

9. E2E script. Add an owner-privacy profile to `scripts/module-e2e.sh` (or a new
   `scripts/module-e2e-privacy.sh`) that creates a private owner and a public
   provider, pre-shields funds via `transfer_shielded_owned`, initializes a
   `PseudonymousFunder` vault, and runs the full lifecycle. Keep the existing
   public flow as the default. Use `OWNER_PRIVACY=1` for this profile
   (`PRIVACY=1` remains a compatibility alias). Provider privacy is a separate
   flag (`PROVIDER_PRIVACY`) owned by Step 37.

### Dependency ELF matrix

The `FfiProgramWithDependencies` passed to `send_generic_private_transaction`
contains the main payment-streams ELF and a map of dependency ELFs. Only the
operations that chain into the `authenticated_transfer` program need that
dependency ELF.

| Operation | Needs `authenticated_transfer` ELF | Notes |
| --- | --- | --- |
| `initializeVault` | No | Creates the vault PDAs. |
| `deposit` | Yes | Debits the owner via `authenticated_transfer`. |
| `withdraw` | No | Credits the recipient via the guest without chaining. |
| `createStream`, `pauseStream`, `resumeStream`, `topUpStream` | No | Only touches payment-streams accounts. |
| `closeStream` | No | Releases allocation; no external transfer. |
| `claim` | No | Credits the provider via the guest without chaining (post-Step-27 fixture shape). |

## Decision log

| Id | Topic | Outcome |
| --- | --- | --- |
| D36.1 | Pre-shielding scope | Keep pre-shielding as a generic wallet public-to-private transfer, out of scope of `payment_streams_module`. Add a module-level check in the PP `deposit` path that the `signer` account id equals the on-chain `VaultConfig.owner`, because the guest debits the owner account directly. Document the prerequisite in `PRIVACY_ENHANCED_JOURNEY.md`. |
| D36.2 | Private submit IPC shape | Call the multi-arg `send_generic_private_transaction` directly via Qt dynamic dispatch. No `send_generic_private_transaction_json` repo-local patch; the private path takes an `FfiAccountIdentity` list, not a JSON envelope. |
| D36.3 | `chainAction` private account id encoding | Overload the existing `signer` / `owner` / `provider` fields. The module resolves the account as public or private based on the vault privacy tier. Step 37 reuses the same convention. |
| D36.4 | Owner off-chain signing | Add a repo-local `sign_private_payload` wallet wrapper that calls a new Rust FFI `wallet_ffi_sign_private_payload`. Inside Rust, retrieve the owned private account's NSK from `wallet.storage().key_chain().private_account(account_id).key_chain.private_key_holder.nullifier_secret_key`, construct a `PrivateKey`, and call `Signature::new`. The NSK never crosses the FFI boundary. This is the Step 36 schedule risk. |
| D36.5 | Cross-relationship vault rotation | Documented as hygiene, not implemented. |
| D36.6 | Amount and timing correlation | Documented as known traffic-analysis limitations, not implemented. |

## Risk

The schedule risk is the `sign_private_payload` patch (D36.4).
Upstream `wallet_ffi` exposes no raw signing primitive for private accounts.
The existing `sign_public_payload` patch can only sign with public-account keys.
The NSK for an owned private account is reachable inside Rust at
`wallet.storage().key_chain().private_account(account_id).key_chain.private_key_holder.nullifier_secret_key`;
from there we can construct a `PrivateKey` and call `Signature::new`. The NSK
never crosses the FFI boundary, so the patch is a straightforward mirror of the
public one but it must still be written, tested, and landed before the module
routing changes can produce a verifiable `VaultProof.owner_signature`.

Why it is the long pole: the patch touches both the Rust FFI layer
(`wallet_ffi_sign_private_payload`) and the C++ wallet wrapper
(`sign_private_payload`), and must be built and landed before eligibility
signing over a `PseudonymousFunder` vault can be verified.

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

| Gate | Command | Pass criteria | Status |
| --- | --- | --- | --- |
| PP program tests | `RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --features pp-program-tests` | Existing PP tests plus `PseudonymousFunder` lifecycle tests pass. | Pass |
| Public tier regression | `make verify-module-local` | `Public`-tier flows unchanged and green. | Pass (re-run after guest ImageID + privacy E2E script) |
| User Journey local | `MODE=module CHAIN=local OWNER_PRIVACY=1 ./scripts/e2e.sh local run` (`PRIVACY=1` alias) | Full `PseudonymousFunder` lifecycle succeeds, including pause/resume/top_up. | Pass (guest fold-seconds + localnet redeploy) |
| Module private submit | `nix build .#unit-tests` in `logos-payment-streams-module/` | Public submit rejected for `PseudonymousFunder` vaults; deposit signer mismatch rejected. | Pass (`payment_streams_privacy_policy` + 6 LogosTest cases) |
| Eligibility (private owner) | Rust unit test + `sign_private_payload` module wiring | `VaultProof.owner_signature` over a `PseudonymousFunder` vault verifies via `verify_stream_proposal_vault_signature`. | Pass (`pseudonymous_funder_vault_proof_signature_verifies_with_nsk`; wallet NSK sign patch wired) |

## Deliverables

Done:

- [x] `submitGenericPrivate` implemented in `payment_streams_module_writes`
  and called for every `PseudonymousFunder` vault operation that touches the vault.
- [x] Public submit refused for `PseudonymousFunder` vaults at the module
  (tier routing always uses the private path).
- [x] PP deposit from a pre-shielded private account succeeds on localnet.
- [x] PP `deposit` signer check rejects a funding account whose account id does
  not match `VaultConfig.owner`.
- [x] Shielded submit path covers the full `PseudonymousFunder` lifecycle
  (init, deposit, create, pause, resume, top_up, close, claim) in module routing
  and PP `program_tests`.
- [x] PP `program_tests` pass with `RISC0_DEV_MODE=1`.
- [x] `scripts/module-e2e.sh` owner-privacy profile (`OWNER_PRIVACY=1`,
  `PRIVACY=1` alias) creates a private owner, a public provider, pre-shields
  funds, and runs the privacy-enhanced lifecycle.
- [x] No regression on the `Public` tier (`make verify-module-local`).
- [x] `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` documents the pre-shielding
  prerequisite, the deposit signer invariant, cross-relationship vault rotation
  as hygiene, and the known traffic-analysis limitations.
- [x] [index.md](../index.md) upcoming table and program outcomes list
  Step 36.
- [x] [AGENTS.md](../../AGENTS.md) active-work pointer lists Step 36.
- [x] Localnet E2E green:
  `MODE=module CHAIN=local OWNER_PRIVACY=1 ./scripts/e2e.sh local run`
  including pause, resume, and top_up
  (guest `chain_timestamp_to_fold_seconds`; ImageID `072a26cc…`).
- [x] Eligibility proof over a `PseudonymousFunder` vault verifies with
  `sign_private_payload` and `verify_stream_proposal_vault_signature`
  (`pseudonymous_funder_vault_proof_signature_verifies_with_nsk`).
- [x] Module-level C++ tests in `logos-payment-streams-module/tests/`:
  public submit refused; PP deposit signer mismatch rejected
  (`nix build .#unit-tests`; `decideVaultSubmitPath`).
- [x] Plan housekeeping: packet moved under `docs/plan/completed/`.

## Definition of done

Done:

- [x] `submitGenericPrivate` ships and is routed for `PseudonymousFunder`.
- [x] Transparent submit of a `PseudonymousFunder` vault is rejected by the
  module (private-path routing).
- [x] PP `deposit` signer check rejects a funding account whose account id does
  not match `VaultConfig.owner`.
- [x] `Public`-tier flows unchanged and green.
- [x] PP lifecycle / privacy-tier unit tests pass
  (`pp-program-tests`, `privacy_tier_policy`).
- [x] `docs/journeys/PRIVACY_ENHANCED_JOURNEY.md` documents the payer-side
  pre-shielding prerequisite, the deposit signer invariant, and the known
  traffic-analysis limitations.
- [x] `scripts/module-e2e.sh` (`OWNER_PRIVACY=1`) creates a private owner and
  public provider and drives the `PseudonymousFunder` lifecycle.
- [x] Step 36 listed in `index.md` and `AGENTS.md`.
- [x] PP deposit, create_stream, pause, resume, top_up, close, and claim
  all succeed via shielded submits on localnet.
  `claim` may use a public or private provider account; the transaction is
  shielded because the vault owner is private.
- [x] Eligibility proof over a `PseudonymousFunder` vault verifies.
- [x] Module-level C++ tests for public-submit refusal and deposit signer
  mismatch.
- [x] Step packet moved to `docs/plan/completed/`.


## Follow-ups (post-complete)

- E2E privacy flags were split so owner and provider choices stay independent:
  `OWNER_PRIVACY` (this step) and `PROVIDER_PRIVACY` (Step 37). `PRIVACY=1`
  remains an alias for `OWNER_PRIVACY=1`.
- Store × privacy profiles are out of scope here; planned as a Developer Journey
  verification step (Step 38) that reuses the same two flags.
- Recipe SSOT for the owner-privacy module cell: [E2E.md](../../journeys/E2E.md).

## Known limitations

- Deposit and claim amounts are public because `vault_holding` is a public PDA.
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
D36.1 (wallet-CLI pre-shielding + module deposit-signer check), D36.2 (multi-arg
private submit), D36.3 (overload existing `chainAction` fields), D36.4
(`sign_private_payload` patch), D36.5 (vault rotation as hygiene), and D36.6
(amount/timing correlation documented as limitations).

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
