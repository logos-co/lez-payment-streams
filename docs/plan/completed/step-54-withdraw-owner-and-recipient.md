# Step 54, withdraw to owner and withdraw

Index: [index.md](../index.md). Status: complete (2026-08-20).
Gate log: [step-54-gate-log.md](step-54-gate-log.md).
Precedent: [Step 44](step-44-payer-and-payee-close.md)
(`CloseStreamByOwner` / `CloseStreamByProvider`).
LIP-155: `rfc-index` / `logos-lips`
`docs/anoncomms/raw/payment-streams.md`.
Spec edits go on `docs/payment-streams-dual-close` and the open PR
[logos-lips#401](https://github.com/logos-co/logos-lips/pull/401).
No second LIP-155 PR.
`chainAction` catalogue: [module/README.md](../../../module/README.md#chainaction-catalogue).
Terms: [names.md](../../reference/names.md) (Withdraw destinations).

UI: `payment_streams_ui` omit-`withdraw_to`, after the module submits
the new ImageID. Stash on `feat/step-21-basecamp-ui`:
`wip: withdraw UI and uniqueness reject` — keep amount/default/inclusion;
drop any `withdraw_to_equals_owner` reject.

## Goal

1. Withdraw to owner — vault owner signs; the owner account is credited.
2. withdraw — vault owner signs; a distinct `withdraw_to` is credited
   (today’s four-slot instruction).
3. Basecamp UI — omit `withdraw_to`; amount defaults to unallocated.

One module `chainAction withdraw`. Dispatch picks the guest instruction
from JSON, the same way `closeStream` picks owner-close vs provider-close.

Appending `WithdrawToOwner` changes the guest ELF (ImageID cut below).
Rebuild, reset localnet, redeploy testnet, flip living pins.
UI submits against the new ImageID.

## Terminology

Destination split, one authorizer (vault owner).
Step 44 is an authorizer split (two peer closers).
Claim already uses one signer-credit slot (provider).

| Term | Meaning |
| --- | --- |
| Vault owner | `VaultConfig.owner`; module JSON `owner` |
| `withdraw_to` | Optional module JSON. Omit, JSON null, or equal to `owner` → withdraw to owner. Distinct → withdraw. |
| Withdraw to owner | Three-slot guest `withdraw_to_owner`; credit the owner slot |
| withdraw | Four-slot guest `withdraw`; explicit distinct `withdraw_to` |
| Tx submitter | Vault owner on both paths |

## Problem

LIP-155 names the vault owner as the Withdraw authorizer.
The Solidity appendix is `withdraw(uint256 amount, address to)`.
`to` may be the owner.

Today’s guest `withdraw` is four slots:

| Slot | Account | Attributes |
| --- | --- | --- |
| 0 | `vault_config` | `mut`, PDA |
| 1 | `vault_holding` | `mut`, PDA |
| 2 | `owner` | `mut`, `signer` |
| 3 | `withdraw_to` | `mut` |

LEZ requires unique account ids in one instruction
(`PreStateAccountIdsNotUnique` /
`"Duplicate account_ids found in message"`).
Same rule for public and private execution.
Visibility-independent, so a private duplicate slot does not help.

When `withdraw_to` equals owner, slots 2 and 3 collide.
The wallet still returns a `tx_hash` (`Message::new_preserialized`
does not check uniqueness). The sequencer never includes the tx.

Program tests and optional E2E only land a distinct `to`
(`SEED_RECIPIENT`, or the stream provider).
`MODULE_E2E_WITHDRAW` defaults to `0`.
Self-payout, the spec-default destination, was never a landing gate.
The module currently defaults omitted `withdraw_to` to owner, which is
the unlistable shape.

Fixing signing flags alone does not clear uniqueness
(same secondary issue as Step 44).

No other current guest instruction has two key-account slots that the
spec allows to be the same person.
Create already rejects `provider == owner`.
Close already split.
Claim credits the provider slot (one id).

## Locked design

Split withdraw into two guest instructions with fixed account lists.
Reuse layouts this program already has.
Do not invent a third account model.

Token dual-ix (`mint` / `mint_with_authority`) is the LEZ uniqueness
precedent for “same authorizer, self vs other.”
Close used role names because it has two peer authorizers.
Here the split is destination:

- Keep `Instruction::Withdraw` at wire index 2 (four-slot, today’s
  semantics: explicit `withdraw_to`).
- Append `Instruction::WithdrawToOwner` at wire index 10 (three-slot).

Do not insert the new variant before `Claim`
(that would shift indices 3–9).

Shared withdraw accounting (same spirit as D44.13):
one helper that checks amount > 0, computes unallocated
(`holding.balance − total_allocated`), rejects amount > unallocated,
debits holding, credits the destination account, and returns AutoClaim.
The privacy circuit requires `AutoClaim::Claimed(Claim::Authorized)`
when a modified account started as `Account::default()` in pre-state
(uncommitted private account receiving funds). Public existing accounts
return `AutoClaim::None`. Use this helper identically from four-slot
`withdraw` and `withdraw_to_owner`. Each instruction loads accounts,
calls the helper, and emits its own `SpelOutput` with the matching
slot count.

Rejected alternatives (do not pursue):

- One instruction with an optional recipient slot — SPEL lists are fixed;
  optional slots do not remove uniqueness.
- Drop the owner slot from four-slot withdraw when `to` is owner —
  PDA binding still needs `account("owner")` on config.
- Module-only or signing-flag fix — LEZ rejects duplicate ids before
  program logic runs.
- Hide the duplicate id in a private slot — uniqueness is
  visibility-independent.
- Chained authenticated-transfer from `vault_holding` to owner —
  holding is payment-streams-owned. Guest already debits holding and
  credits the destination in `claim` and four-slot `withdraw`.
  `deposit` chains AT because it debits a user-owned native account.
  Withdraw to owner credits the owner in-guest.

### Guest instructions

Copy flags from the named precedent.
Do not copy “same as deposit” without checking `mut` / `signer`.

`withdraw_to_owner` (three-slot; match `deposit` slots 0–2, because the
owner native balance is written in this guest):

| Slot | Account | Attributes |
| --- | --- | --- |
| 0 | `vault_config` | `mut`, PDA `[vault_config, owner, vault_id]` |
| 1 | `vault_holding` | `mut`, PDA `[vault_holding, vault_config, native]` |
| 2 | `owner` | `mut`, `signer` |

Credit `owner.account.balance`.
Do not list a fourth account.

`withdraw` (four-slot; keep today’s flags and AutoClaim behaviour):

| Slot | Account | Attributes |
| --- | --- | --- |
| 0 | `vault_config` | `mut`, PDA `[vault_config, owner, vault_id]` |
| 1 | `vault_holding` | `mut`, PDA `[vault_holding, vault_config, native]` |
| 2 | `owner` | `mut`, `signer` |
| 3 | `withdraw_to` | `mut` |

Guest `ErrorCode::WithdrawToEqualsOwner = 6029` (`6028` is
`UnsupportedTokenId`) is D54.5: slice 0 first, using today’s four-slot
`Withdraw` with `withdraw_to == owner` through `build_signed_public_tx`
plus `state.transition_from_public_transaction` in
`program/core/src/program_tests/withdraw.rs`.

- Guest returns 6029 → keep the code in slices 1–2.
- Harness uniqueness (`PreStateAccountIdsNotUnique` / duplicate
  `account_ids`) fires first → do not add 6029. Planner duplicate
  documentation, module uniqueness tests, and the four-slot same-id
  coincidence case stay.

Module JSON has no `withdraw_to_equals_owner` token; that JSON
dispatches to three-slot withdraw to owner.

Wire enum (hard cut). Discriminants are `Instruction` declaration order
(Risc0 Serde). Core, guest, FFI, and module serialize that enum.
Guest SPEL dispatch is name-based (`Instruction::PascalName`).
Guest source order is an IDL-ordering constraint for `make idl`.

| Index | Before | After |
| --- | --- | --- |
| 2 | `Withdraw` | `Withdraw` (unchanged payload `{ vault_id, amount }`) |
| 7 | `CloseStreamByOwner` | unchanged |
| 8 | `Claim` | unchanged |
| 9 | `CloseStreamByProvider` | unchanged |
| 10 | — | `WithdrawToOwner` `{ vault_id, amount }` |

`instruction_wire.rs` asserts those leading words. Round-trip alone
cannot detect a uniform renumbering. Extend
`all_variants_round_trip_via_instruction_words` with a
`WithdrawToOwner` sample.

IDL: `make idl` → `lez-payment-streams-idl.json`.
Place `withdraw_to_owner` next to `withdraw` in the guest file.
Wire discriminants stay enum declaration order.

### Planners

`program/core/src/instruction_accounts.rs`:

- Keep `withdraw_instruction_accounts(..., withdraw_to)` → `[4]`.
- Add `withdraw_to_owner_instruction_accounts(program_id, owner, vault_id)`
  → `[3]`, same ids as `deposit_instruction_accounts`
  (config, holding, owner).
- Type alias `WithdrawToOwnerInstructionAccounts = [AccountId; 3]`
  (or reuse `DepositInstructionAccounts` with a named alias so tests
  read clearly).

A planner unit test must assert that passing `withdraw_to == owner`
into the four-slot planner produces a list with a duplicate
(document the collision) and that the three-slot planner’s three ids
are unique.

### Module and FFI

Keep a single `chainAction withdraw` so omit-`withdraw_to` becomes real
withdraw to owner.
Dispatch is strict (reject mismatches; do not guess).

Identity comparisons are over normalized 32-byte account ids
(D47.7 order: 64-hex else base58).
Raw JSON-string equality is wrong.

| Module JSON | Dispatch |
| --- | --- |
| `withdraw_to` omitted or JSON null | Withdraw to owner (`WithdrawToOwner`, three-slot) |
| `withdraw_to` present, empty or whitespace-only | shared `args_mismatch` (D47.12; do not default to owner) |
| `withdraw_to` present and equal to `owner` (32-byte) | Withdraw to owner (same as omit) |
| `withdraw_to` present and distinct from `owner` | withdraw (`Withdraw`, four-slot, explicit `withdraw_to`) |
| `owner` missing or empty | shared `args_mismatch` (D47.12) |
| `withdraw_to` present but not 64-hex and not base58 | `invalid account id (expect 64-hex or base58)` (kit; before classify) |
| `amount_lo` / `amount_hi` missing or not a u64 | `invalid numeric argument` (`amount_hi` stays mandatory on `withdraw()`; deposit defaults omitted hi to 0) |
| Amount pair valid and zero | guest `ZeroWithdrawAmount` (6006); module does not pre-reject |

Empty checks for `owner` and present-but-empty `withdraw_to` MUST run
before `ownerBytesFromBase58` (same P1 as close).

Callers that already pass a distinct `withdraw_to` (E2E provider,
`SEED_RECIPIENT`) keep four-slot `Withdraw`.

Pre-submit: require a decodable vault config for `owner` + `vault_id`
(same `get_account_public` path as close).
Missing vault → `withdraw_prestate_unavailable`.
If the account decodes and `decodedVault.owner` ≠ normalized JSON `owner`
→ `withdraw_owner_mismatch` (Reserved, same as `close_owner_mismatch`;
PDA seeds usually make this unreachable).
Do not submit a doomed tx.
After a successful decode, set `ctx.hasDecodedVaultConfig = true` and
`ctx.decodedVaultConfig` (same as `closeStream`) so `buildAndSubmit`
reuses the decoded config.

Extract account-list validation from `buildAndSubmit` into a helper
(header next to the writes file) that:

- rejects `accountsHex` whose length is not a multiple of 64
  (`splitAccountsHex` today silently drops a trailing partial chunk)
- then splits into 64-hex ids
- lowercases / treats hex case-insensitively
- returns `duplicate_account_ids` when any id repeats

Call that helper from `buildAndSubmit` before wallet send, so a
malformed length never reaches signing-flag construction.
Unit-test unique and duplicate lists of length 3, 4, 5, and 6
(the live planner widths), plus a `64k + r` malformed length
(reject) and a mixed-case unique list (accept). Doomed-tx backstop;
negative E2E for `duplicate_account_ids` stays Reserved.

Extract withdraw destination classification into a pure helper
(`omit` / `empty` / `equal_owner` / `distinct`) over already-normalized
32-byte ids. Undecodable `withdraw_to` never reaches this helper.
Unit-test equal-owner with Base58 `owner` plus the same id as 64-hex
`withdraw_to` (and the reverse): both classify as withdraw to owner
and the planner emits three unique slots. Equal-owner E2E is slice 6.

Valid current planners already have unique ids:

| Write | Planned ids | Unique when |
| --- | --- | --- |
| `initializeVault` / `deposit` / withdraw to owner | config, holding, owner | always (distinct PDA seeds + user id) |
| `createStream` / pause / resume / top-up / owner-close | config, holding, stream, owner, clock | always (`provider` is an instruction arg on create) |
| provider-close | those plus provider | `provider != owner` (else dispatch uses owner-close) |
| `claim` | six-slot provider list | `provider != owner` (guest 6027 at create) |
| four-slot `withdraw` | config, holding, owner, `withdraw_to` | `withdraw_to` distinct; same-id is doomed |

| Path | `ctx.layout` | serialize / plan | submitter |
| --- | --- | --- | --- |
| Withdraw to owner | `InitOrDeposit3` (owner is slot 2) | new serialize/plan pair below | `owner` |
| withdraw (explicit `withdraw_to`) | new `VaultIxLayout::Withdraw4` | existing `ps_ffi_serialize_withdraw` / `ps_ffi_plan_withdraw` | `owner` |

Today’s `withdraw()` calls `buildAndSubmit` without `VaultSubmitContext`,
so privacy routing falls back to `InitOrDeposit3` and never probes
slot 3. This step always passes `VaultSubmitContext`.
Withdraw to owner reuses `InitOrDeposit3` (owner is slot 2).
Four-slot `withdraw` uses new `VaultIxLayout::Withdraw4`.

`Withdraw4` privacy slots:

| Index | `slotMayHoldPrivateKey` | `pfOwnerSlotByLayout` |
| --- | --- | --- |
| 0, 1 | false | false |
| 2 (owner) | true | true |
| 3 (`withdraw_to`) | true | false |

Update every `switch` on `VaultIxLayout` (policy `.h` / `.cpp`).
Missing enum arms are compile errors; add the arm.
Required tests in `module/tests/test_privacy_submit_policy.cpp`
(same pattern as `init_or_deposit_probes_owner_slot_only`):

- `Withdraw4` probes slots 2 and 3; `pfOwnerSlotByLayout` true only at 2
- `InitOrDeposit3` still probes owner at slot 2 only (withdraw to owner)

`signingRequirementsForAccounts` uses submitter = owner, so three-slot
flags are `[F, F, T]` and four-slot flags are `[F, F, T, F]` when
`withdraw_to` differs.

FFI: two serialize + two plan entry points.
Same change set must update all three FFI surfaces (names match
today’s `withdraw` pair):

1. `module/ffi/src/instruction_abi.rs` —
   `payment_streams_ffi_serialize_withdraw_to_owner_instruction`,
   `payment_streams_ffi_plan_withdraw_to_owner_instruction_accounts`
2. Regenerated `module/ffi/lez_payment_streams_ffi.h` (cbindgen)
3. Hand-maintained bridge
   `module/src/payment_streams_ffi_bridge.h` and
   `module/src/payment_streams_ffi_bridge_writes.c` —
   `ps_ffi_serialize_withdraw_to_owner`,
   `ps_ffi_plan_withdraw_to_owner`

Serialize payloads are both `{ vault_id, amount }` (lo/hi on the C ABI).
Plan for withdraw to owner takes `program_id`, `owner`, `vault_id` only.
Plan for withdraw keeps today’s `withdraw_to` argument.

Module reject tokens. Negative E2E asserts only the Asserted column.

| Condition | Error token | Negative E2E |
| --- | --- | --- |
| Vault missing/undecodable | `withdraw_prestate_unavailable` | Asserted |
| Empty `owner`, or `withdraw_to` key present but empty | `args_mismatch` | Asserted |
| `withdraw_to` present but not 64-hex and not base58 | `invalid account id (expect 64-hex or base58)` | Existing kit token |
| Missing or non-u64 `amount_lo` / `amount_hi` | `invalid numeric argument` | Existing; `amount_hi` mandatory |
| Decoded vault owner ≠ JSON `owner` | `withdraw_owner_mismatch` | Reserved |
| Planned account ids not unique | `duplicate_account_ids` | Reserved |

Do not keep a `withdraw_to_equals_owner` reject.
That case is a valid withdraw to owner.

### Public and private execution

Guest stays visibility-agnostic.
Module routing stays slot-based (D37.9).

| Scenario | Path | Submit | Evidence |
| --- | --- | --- | --- |
| Public vault, omit `withdraw_to` | Withdraw to owner | Public | Required module local + testnet (E2E phase after accrual, before close) |
| Public vault, `withdraw_to` == owner | Withdraw to owner | Public | Second submit in the local public owner-withdraw cell only; unit test for hex vs Base58 |
| Public vault, distinct `withdraw_to` | withdraw (explicit `withdraw_to`) | Public | Required module local thin cell |
| `PseudonymousFunding` vault, omit `withdraw_to` | Withdraw to owner | Private; owner signs and is credited | PP harness + `OWNER_PRIVACY=1` module × local |
| Public vault, private `withdraw_to` | withdraw (explicit `withdraw_to`) | Private (slot 3 private) | Existing PP withdraw tests, keep green |

Store withdraw is out of scope (Store happy path never withdraws).
One Store-local public cell is required as ImageID/PDA insurance (D54.1).
`PROVIDER_PRIVACY` stays out of this step (claim-focused).

### Coincidence tests

For every pair of non-PDA slots on a new or changed instruction, a
harness case where the two ids are equal: the tx includes (one slot
plays both roles), or a named reject / the other instruction is selected.
Distinct-fixture-account success does not cover this.

This step:

1. Omit-`withdraw_to` includes (harness + module E2E). Public cells:
   exact holding decrease, exact owner credit, `total_allocated`
   unchanged. Equal-owner JSON: classify-helper unit test plus slice 6
   second submit on the local public owner-withdraw cell.
2. Four-slot `withdraw_to == owner` never includes (D54.5).
3. Distinct `withdraw_to` still includes.

Later instructions with a second key-account slot copy this trio.

## Spec changes

Edit LIP-155 in slice 9. Until then, describe only here.

Canonical file: `docs/anoncomms/raw/payment-streams.md` in `logos-lips`.
Commit on `docs/payment-streams-dual-close` and push to
[logos-lips#401](https://github.com/logos-co/logos-lips/pull/401).
No new logos-lips PR and no separate `master` branch.

D54.7: those commits close the spec work. Pin the branch tip (full SHA):

- `docs/plan/context-manifest.json` `lip155_spec.branch` →
  `docs/payment-streams-dual-close`
- `lip155_spec.rev` → that tip
- `lip155_spec.contains` → keep `435a6f18` / `f09f9e9e`, add `32d7da4e`
  and the new withdraw commit
- [pins.md](../../reference/pins.md) branch column and the Step 49
  “`master` at …” sentence
- local `rfc-index` at that rev

Today `rev` is already `32d7da4e` (that branch tip) while `branch` still
says `master`. Merge to `master` is out of scope.

Keep LIP abstraction (roles, invariants, LEZ binding).
No SPEL, FFI, or module JSON schemas in the LIP.

Required LIP edits:

1. Guest instructions table.
   Today one row: Withdraw unallocated / `Withdraw` / Vault owner.
   Replace with two rows, parallel to close:

   | On-chain operation | Reference instruction | Authorizer |
   | --- | --- | --- |
   | Withdraw unallocated to owner | `WithdrawToOwner` | Vault owner |
   | Withdraw unallocated (explicit `to`) | `Withdraw` | Vault owner |

2. A short LEZ uniqueness paragraph next to the close split prose
   (the block that already says LEZ encodes either-party close as two
   instructions). State that when `to` is the vault owner, LEZ encodes
   a three-slot instruction that credits the owner slot; when `to` is a
   different account, LEZ encodes a four-slot instruction; the same
   account id MUST NOT appear twice in one instruction.

3. Solidity appendix `withdraw(uint256 amount, address to)`:
   add that `to ==` the owner maps to `WithdrawToOwner`, and a distinct
   `to` maps to `Withdraw`.

4. Replace the existing “Deposit and claim asset paths” paragraph
   (`rfc-index` `payment-streams.md`, the block that today says deposit,
   withdraw, and claim all compose with authenticated-transfer for
   native). After the edit, only deposit composes with the platform
   program (authenticated-transfer for native, Token program for
   non-native). Claim and both withdraw paths credit the destination
   in the payment-streams guest (holding is program-owned).

Do not change accounting rules (`unallocated`, “user MAY withdraw at any
time”).

## Implementation slices

Do these in order. Locked design above is the spec; slices name files
and commands.

Pre-applied (uncommitted; slice 7/11 are flips, not first registration):

| Path | When to commit |
| --- | --- |
| `docs/reference/wire.md` (dispatch block) | Slice 5, after module dispatch exists |
| `docs/reference/names.md` (destination-split terms) | Slice 7 |
| `docs/reproduce/basecamp-ui.md` (deposit AT / in-guest credit) | Slice 7 or 8 |
| [Step 21](step-21-basecamp-ui.md) later-work (withdraw dropped) | Slice 8 |
| `AGENTS.md`, `docs/plan/index.md`, `context-manifest.json` (Step 54 upcoming) | Slice 11 flips these to completed |

### 0. D54.5 seam probe

No new instruction.
File: `program/core/src/program_tests/withdraw.rs`.
Run the four-slot same-id case in Locked design (D54.5). Record the
error. Slices 1–2 add 6029 only if that result keeps it. Slice 3 locks
the same error as a named case.

### 1. Core wire and planners

Files:

- `program/core/src/instruction.rs` — append `WithdrawToOwner { vault_id, amount }`
- `program/core/src/instruction_wire.rs` — golden 2 / 7 / 8 / 9 / 10
- `program/core/src/instruction_accounts.rs` — three-slot planner + alias
- `program/core/src/lib.rs` — re-export
- `program/core/src/error_codes.rs` — `WithdrawToEqualsOwner = 6029`
  only if slice 0 keeps 6029 (D54.5).
  If kept, extend the file header range to `6001`–`6029`.

`cargo test -p lez-payment-streams-core --lib instruction_wire`

### 2. Guest

Files:

- `program/methods/guest/src/bin/lez_payment_streams.rs`

Add `#[instruction] pub fn withdraw_to_owner(...)` with the three-slot
list above.
Extract shared accounting from today’s `withdraw`.
Keep four-slot `withdraw` behaviour. Same-id follows slice 0.

`make idl` writes `program/lez-payment-streams-idl.json`.
Regenerate twice (or diff against git) and commit only when the file
matches the second run. `git status` on that path must be clean after
the intended commit.

### 3. Program and PP tests

Files:

- `program/core/src/program_tests/withdraw.rs` — withdraw to owner success
  (owner balance increases by amount; holding decreases by amount;
  allocated unchanged; nonce transition matches existing four-slot
  withdraw tests);
  four-slot distinct `withdraw_to` still succeeds;
  four-slot same-id: lock the slice-0 result (guest 6029 vs harness
  uniqueness)
- `program/core/src/program_tests/pp_common.rs` / PP helpers —
  a separate three-slot withdraw-to-owner helper (do not overload
  four-slot `withdraw_to` helpers; claim fixtures depend on those).
  Required asserts: one private post-state, decrypted owner balance
  increases by amount, holding decreases by amount, `total_allocated`
  unchanged, nonce transition matches the public three-slot case.
- Keep existing third-party and private-`withdraw_to` tests green

`RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --lib`

### 4. FFI

Files listed under Module and FFI.
Cbindgen the header; then hand-edit the C bridge.
Do not edit only the Rust ABI.

In `module/ffi/src/instruction_abi.rs` `mod tests`, add serialize
round-trips and planner tests for both variants:

- `Withdraw` words leading discriminant 2; accounts hex length `4 × 64`;
  order config, holding, owner, `withdraw_to`
- `WithdrawToOwner` leading discriminant 10; accounts hex length `3 × 64`;
  order config, holding, owner (same ids as deposit for that vault)

### 5. Module dispatch and privacy layout

Files:

- `module/src/payment_streams_module_writes.cpp` — `withdraw()` dispatch
  table; populate `VaultSubmitContext` including
  `hasDecodedVaultConfig` / `decodedVaultConfig` after the pre-read
  (same as `closeStream`); uniqueness helper in `buildAndSubmit`;
  `withdraw_owner_mismatch` after decode
- Uniqueness + classify helpers: header-only (or inline in a `.h` next
  to writes), included by `writes.cpp` and by tests. Do not add
  `payment_streams_module_writes.cpp` to `MODULE_SOURCES` (anonymous
  namespace; Logos deps). Register the new test under `TEST_SOURCES`.
- `module/tests/CMakeLists.txt` — add that test file
- `module/src/payment_streams_privacy_policy.h` / `.cpp` — `Withdraw4`
- `module/tests/test_privacy_submit_policy.cpp` — `Withdraw4` slot
  assertions in Locked design
- `module/src/payment_streams_module_kit.h` / `.cpp` — hex→lo/hi/disable
  helper (slice 8 rules). Tests in this same Qt unit-test target.
- `docs/reference/wire.md` — commit the pre-applied dispatch block after
  this dispatch exists

### 6. E2E and fixtures

Files:

- `verify/module-e2e.sh` — default `MODULE_E2E_WITHDRAW=1` and
  `WITHDRAW_PATH=owner` (omit the `withdraw_to` key completely; do not
  send null or empty). Phase JSON `withdraw_path` is `owner` or
  `withdraw`.
  Keep the withdraw phase where it is today: after accrual, before
  close (unallocated while a stream exists). UI / `module.md` /
  `basecamp-ui.md` stay after-claim leftover; that is a different
  operator frame, not this cell.
  On the local public owner-withdraw cell only: first amount is
  `WITHDRAW_AMOUNT` (default 1) and must leave remainder ≥ 1; then a
  required second submit with `"withdraw_to"` set to the owner id in
  the other encoding (hex if `owner` is Base58). Phase JSON records
  that the equal-owner submit ran. Stub-privacy and testnet omit-only.
  `WITHDRAW_PATH=withdraw` keeps `"withdraw_to":"$PROVIDER"` (today’s
  payload) and skips the equal-owner second submit.
- `verify_withdraw` on public cells: exact holding decrease equal to
  the submitted amount, exact destination (owner or `withdraw_to`)
  balance increase by that amount, `total_allocated` unchanged.
  Stub-privacy / private-owner cells: inclusion plus exact holding
  decrease and unchanged allocation; owner credit is proved by the PP
  harness (D54.1).
- Default `MODULE_E2E_WITHDRAW=1` is script-wide: every
  `module-e2e.sh` caller that does not export `0` runs the withdraw
  phase (`make verify-module-*`, `MODE=module ./verify/e2e.sh`).
  `verify/module-close-negatives.sh` already exports
  `MODULE_E2E_WITHDRAW=0`; keep that.
  `MODULE_E2E_WITHDRAW_NEGATIVES=1` also forces happy withdraw off
  (`MODULE_E2E_WITHDRAW=0`) and asserts
  `withdraw_prestate_unavailable` and `args_mismatch`.
  Store 2×2 stays without withdraw; D54.1 still runs one Store-local
  public cell on the new ELF.
- `verify/seed` / localnet fixture: add a seed withdraw only if a caller
  needs it (today seed does not submit withdraw).
- ImageID cut: sequence below.

### 7. Living docs in this repo

Files:

- `module/README.md` catalogue — omit `withdraw_to` or equal owner →
  withdraw to owner; distinct `withdraw_to` → withdraw;
  walkthrough column: withdraw to owner `yes` if `module.md` gains a step.
  Required-verification phase list currently
  `vault_init`, `deposit`, `create_stream`, `close_stream`, `claim`,
  `module_e2e_complete`. Insert `withdraw` (after accrual, before close)
  once the default flip is on. Note that default `MODULE_E2E_WITHDRAW=1`
  is script-wide (slice 6).
- `docs/reproduce/module.md` — add withdraw to owner as after-claim
  leftover (operator frame). Destination owner, omit `withdraw_to`.
- `docs/reference/names.md` — commit the pre-applied destination-split
  row.
- `docs/reference/matrix.md` / `verify/README.md` — withdraw to owner
  Required on module local and module testnet (D54.1). Store-local
  public on the new ELF; Store-testnet and privacy Store stay off
  this step's gate log.
- [README.md](../../../README.md) Public testnet guest program — new
  ImageID, ELF size, deploy date/commit/tx/block (see ImageID cut)

### 8. Basecamp UI

Files:

- `ui/Main.qml`
- `docs/reproduce/basecamp-ui.md`
- [Step 21](step-21-basecamp-ui.md) (later-work already dropped withdraw)

Amount-cap helper and its tests: slice 5 (`payment_streams_module_kit`).
May reuse stash `wip: withdraw UI and uniqueness reject` after module
dispatch exists. Rework against this step:

- Owner region Withdraw, enabled when the vault exists and unallocated
  is a positive value that fits in `u64` (`amount_hi = 0`).
- Amount prefills live unallocated (`holding − total_allocated`) using
  lossless integer arithmetic (decimal strings or hex), never IEEE
  `Number`. Disable when unallocated is 0, the high limb is nonzero, or
  the value does not fit in `u64`. Prefill overwrites the field only
  while its value equals the last auto-default; any user edit sticks
  thereafter.
  Kit tests (slice 5): `0` (disable), `2^53`, `2^64 − 1`, nonzero high
  limb (disable). QML uses the same three disable rules.
- Payload omits `withdraw_to`. Drop any stashed
  `withdraw_to_equals_owner` dialog. No `withdraw_to` field in v1.
- Live inclusion (public): exact holding decrease equal to the
  submitted amount, `total_allocated` unchanged, public owner native
  balance increased by that amount.
- Demo mode: holding decreases by the withdrawn amount; unallocated
  snapshot follows.
- While a stream is Active or Paused, a note that allocated tokens
  stay locked.
- Walkthrough after Claim: leftover, amount defaults to Unallocated,
  destination is the owner.

Rebuild with `make basecamp-ui-build`.
Gate: one localnet or testnet pass, Demo off:
init → deposit → stream → close → claim → withdraw.

### 9. Spec on the dual-close PR

Checkout `logos-lips` branch `docs/payment-streams-dual-close`
([logos-lips#401](https://github.com/logos-co/logos-lips/pull/401)).
Apply Required LIP edits from Spec changes. Push to that PR.
Then D54.7 pin fields from Spec changes.

### 10. Gate log

Create `docs/plan/completed/step-54-gate-log.md` when closing.
Columns: date, commit, cell, artifact, result, ImageID,
`RISC0_DEV_MODE`, `withdraw_path`.
One row per D54.1 required cell.
Record the new ImageID, ELF size, deploy tx, and block there.

### 11. Close-out

Move this packet to `docs/plan/completed/step-54-withdraw-owner-and-recipient.md`
with the gate log beside it.
Flip upcoming → completed on `docs/plan/index.md`,
`docs/plan/context-manifest.json`, and `AGENTS.md`.
Run the ImageID prefix-grep after the move.

## ImageID cut and identity flip

Appending `Instruction::WithdrawToOwner` changes guest source, so the
release-stripped ELF ImageID changes
([pins.md](../../reference/pins.md) guest release profile).
Public testnet must redeploy (`make deploy-testnet`).
Localnet must reseed (`make seed-fixture` then
`make full-reset-localnet`).

Current living product pin (replace these values; keep them in
completed packets as history):

| Field | Value before this step |
| --- | --- |
| ImageID / `program_id_hex` | `c30781ea9d7cc7b3be36f459ce9094644b984224d3d3119a644bb1b21ba2982a` |
| ELF size | 373916 bytes |
| Freeze commit | `8a0e374a7e7171cd5b60ad20d46b9510b057dfe3` |
| Deploy date | 2026-08-13 |
| Deploy tx | `229dddd92e5184f4a44816ddda711b1eac51476248620a686807e091ffefba8b` |
| Deploy block | 5873 |
| ELF pin filename | `.scaffold/program-bins/lez_payment_streams-c30781ea.bin` (gitignored) |

From guest rebuild until org testnet redeploy plus pin sync,
`CHAIN=testnet` and reproduce Step 3 (fixture ImageID vs `make program-id`)
are mismatched. Advertise testnet green only after the sequence below
and the module-testnet withdraw to owner gate.

### Sequence

1. Clean tracked tree (`git status`). Record `git rev-parse HEAD` as
   the freeze commit before the release `make build`.
2. Docker / release guest build (`make build`) → `spel inspect` /
   `make program-id`. Hard-check that the inspected ImageID equals the
   hex that will be written to fixtures. Record ELF `stat` size.
   Capture `make deploy-testnet` stdout (tx hash, block). Propagate
   commit, ELF size, ImageID, tx, and block from that one run into
   README, fixtures, `bootstrap-testnet.sh`, and the gate log.
3. `make idl` already in slice 2; commit
   `program/lez-payment-streams-idl.json`
   (gains `withdraw_to_owner`; existing instruction names and wire
   indices 0–9 stay).
4. Localnet before any local E2E: `make seed-fixture` rewrites
   gitignored `verify/fixtures/localnet.json` `program_id_hex` and
   vault/stream PDAs from the new ELF. Then `make full-reset-localnet`
   so `.scaffold/snapshots/funded/snapshot.json` `program_id_hex`
   matches (pins.md requires this after an ImageID change).
5. D54.1 local gates green (fast including module unit tests and
   `make check-links`, module withdraw cells, Store-local,
   stub-privacy withdraw to owner).
6. `make deploy-testnet` — deploys the ELF; writes `program_id_hex`
   into `verify/fixtures/testnet.json` and
   `verify/fixtures/testnet-module.json`; copies the ELF to
   `.scaffold/program-bins/lez_payment_streams-<first8>.bin`.
7. Hand-sync the fields `deploy-testnet.sh` does not rewrite
   (`guest_deploy_*`, `guest_elf_bytes`, deploy tx/block, README,
   `bootstrap-testnet.sh` default), then prefix-grep (below).
8. Module testnet withdraw to owner on the new ImageID (D54.1) → append
   the gate log.
9. Slice 11 close-out.

Existing vaults and streams on `c30781ea…` stay bound to that program.
New calls derive PDAs from the new ImageID (same owner + numeric
`vault_id` is a different on-chain config). Module E2E derives vault
and stream PDAs in-process from `program_id_hex` plus owner/`vault_id`.
The committed `verify/fixtures/testnet-module.json` PDA fields stay
stale until the cut edits them: regenerate `vault_config_account_id` /
`vault_holding_account_id` from the new program id, or drop those
unused fields from the module fixture. That fixture has no `stream_*`
keys. Store runs still refresh PDAs per run (Step 33). Re-run
`make bootstrap-testnet` / `make bootstrap-testnet-module` only when
the fixture is missing or owner/provider ids are invalid for the
agent wallet. Drain or abandon leftover under the old id if it still
matters; move fixtures and pins in lockstep with the new deploy.

### Identifiers that change

| Identifier | Why |
| --- | --- |
| ImageID / `program_id_hex` | Hash of the new release ELF |
| `guest_elf_bytes` | New binary size |
| `guest_deploy_date`, `guest_deploy_source_commit` | This deploy |
| `deploy_tx_hash`, `deploy_block_id` | New `deploy-program` |
| ELF pin path | `.scaffold/program-bins/lez_payment_streams-<first8 of new hex>.bin` |
| `vault_config_account_id` | PDA `[vault_config, owner, vault_id]` keyed by program id |
| `vault_holding_account_id` | PDA `[vault_holding, vault_config, native]` keyed by program id |
| `stream_config_account_id` | PDA `[stream_config, vault_config, stream_id]` keyed by program id |
| Localnet `program_id_hex` and those PDAs | Regenerated from the live ELF |
| Funded snapshot `program_id_hex` | `make full-reset-localnet` |

New identifiers this step adds (existing values stay):

| Identifier | Value |
| --- | --- |
| Wire discriminant `WithdrawToOwner` | 10 (`Withdraw` stays 2; close/claim stay 7 / 8 / 9) |
| Guest `ErrorCode::WithdrawToEqualsOwner` | 6029 if D54.5 keeps it (`UnsupportedTokenId` stays 6028) |
| IDL instruction | `withdraw_to_owner` next to `withdraw` |
| FFI entry points | Rust `payment_streams_ffi_*_withdraw_to_owner_instruction*`; C `ps_ffi_serialize_withdraw_to_owner` / `ps_ffi_plan_withdraw_to_owner` |

### Identifiers that stay

| Identifier | Why |
| --- | --- |
| `owner_account_id`, `provider_account_id` | Wallet accounts; reuse via `TESTNET_REUSE_FIXTURE` |
| `clock_10_account_id`, `clock_50_account_id` | LEZ system clock accounts passed into stream ixs; independent of this program id |
| Native `token_id` | All-zeroes (Step 49) |
| `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` | Live testnet AT (`fe96c422…`); deposit chains it. Claim and both withdraw paths credit in-guest. |
| Operator LEZ pin / program-graph LEZ pin | `47eba256…` / `a58fbce2…` in [pins.md](../../reference/pins.md) |
| Module / UI Logos ids | `payment_streams_module`, `payment_streams_ui` |

### Living files to update

`make deploy-testnet` only writes `program_id_hex` into the two JSON
fixtures. Update the rest in the same commit as the pin flip.

Tracked (grep must be clean of live `c30781ea` after the cut):

| File | Fields / note |
| --- | --- |
| [README.md](../../../README.md) Public testnet guest program | ImageID, ELF size, deploy date, freeze commit, deploy tx, block. Living SSOT. |
| `verify/fixtures/testnet-module.json` | `program_id_hex`, `guest_deploy_date`, `guest_deploy_source_commit`, `guest_elf_bytes`. Keep owner/provider. Regenerate or drop unused `vault_config_account_id` / `vault_holding_account_id` (module E2E derives PDAs live). No `stream_*` keys in this fixture. |
| `verify/fixtures/testnet.json.example` | `program_id_hex`, `guest_deploy_*`, `guest_elf_bytes`, `deploy_tx_hash`, `deploy_block_id` |
| `verify/testnet/bootstrap-testnet.sh` | Hardcoded `PROGRAM_ID_HEX` default (module bootstrap already derives via `ps_testnet_program_id_hex`) |
| `verify/store/test_run_e2e_pure.py` | Sample 64-hex in `test_private_id_is_unusable` (prefix grep hits this) |
| `program/lez-payment-streams-idl.json` | Regenerated in slice 2 |

Operator / gitignored (refresh on the machine that deploys):

| Path | Fields / note |
| --- | --- |
| `verify/fixtures/testnet.json` | `program_id_hex` (script); vault/stream PDAs on next Store run |
| `verify/fixtures/localnet.json` | From `make seed-fixture` |
| `.scaffold/program-bins/lez_payment_streams-<new8>.bin` | From deploy / bootstrap pin helper |
| `.scaffold/snapshots/funded/snapshot.json` | `program_id_hex` from `make full-reset-localnet` |

`docs/reproduce/module.md` Step 3 greps `testnet-module.json`; it
follows the fixture once that file is updated.

### Leave as history

Completed packets and gate logs keep the ImageID they verified
(Step 39 `dea010d9…` / `072a26cc…`, Step 44 `ee2cfb74…`,
Step 49/52 `c30781ea…`). Same for
[decisions.md](../../reference/decisions.md) N18 ImageID sentences,
[AGENTS.md](../../../AGENTS.md) Step 39 `dea010d9…`,
`docs/plan/index.md` Step 52 wrap-up line, and `verify/archive/`
defaults. [Step 51](../upcoming/step-51-forum-post.md) fills wrap-up claims from
the Step 52 gate log (that ImageID is wrap-up evidence). Living guest
identity after this step is the README block.

Prefix check after slice 11 (packet is then under `completed/`, which
this glob already excludes). Until then, also exclude this upcoming
file:

```bash
rg -n 'c30781ea' \
  --glob '!docs/plan/completed/**' \
  --glob '!docs/plan/wontfix/**' \
  --glob '!docs/plan/upcoming/step-54-withdraw-owner-and-recipient.md' \
  --glob '!verify/archive/**' \
  --glob '!.scaffold/**' \
  --glob '!**/target/**'
```

Expect README, fixtures, `bootstrap-testnet.sh`, and the pure-Python
sample to show the new hex. Remaining `c30781ea` hits belong in
completed / archive / historical index prose.

## Gate budget (D54.1)

ImageID cut plus withdraw-encoding change.
[Step 52](step-52-wrap-up-verification.md) is the last
full-matrix pin on `c30781ea…`. Precedent: [D49.11](step-49-native-token-spec-alignment.md).
Skip Store-testnet (proof encoding unchanged; Store-local covers new
PDAs) and all `RISC0_DEV_MODE=0` / testnet-private legs (uniqueness is
visibility-independent; PP + stub-privacy cover private routing).

Wall-clock after guest build and localnet reset, `SKIP_BUILD=1` on
later local cells: about 2–3 hours on one Linux host.

What these cells catch: wire golden 2 / 7 / 8 / 9 / 10; uniqueness
backstop on the default lifecycle; three-slot / `Withdraw4` dispatch;
new ImageID/PDAs (localnet reset + Store-local); private withdraw to
owner (PP + stub-privacy `OWNER_PRIVACY=1`).

### Required cells

Run in this order so a cheap fail stops a dearer leg.

| Stage | Cell | Command | Est. |
| --- | --- | --- | --- |
| Fast | clippy | `RISC0_SKIP_BUILD=1 cargo clippy --workspace` | 15–30 min together |
| Fast | unit / PP / FFI | `RISC0_DEV_MODE=1 cargo test --workspace` | (same) |
| Fast | terminology + links | `make check` (`check-terminology` + `check-links`) | (same) |
| Fast | module Qt unit tests | `nix build ./module#checks.x86_64-linux.unit-tests -L` (uniqueness helper, classify helper, `Withdraw4` slots, amount-cap helper) | 5–15 min first Nix; 2–5 later |
| Local public | module withdraw to owner | `MODE=module MODULE_E2E_WITHDRAW=1 ./verify/e2e.sh local run` (omit `withdraw_to`; equal-owner second submit on this cell) | 5–10 min |
| Local public | module withdraw (explicit `withdraw_to`) | `MODE=module MODULE_E2E_WITHDRAW=1 WITHDRAW_PATH=withdraw ./verify/e2e.sh local run` (distinct `withdraw_to`) | 5–10 min |
| Local public | module withdraw negatives | `MODE=module MODULE_E2E_WITHDRAW_NEGATIVES=1 ./verify/e2e.sh local run` | 5–10 min |
| Local public | store eligibility | `MODE=store ./verify/e2e.sh local run` | 15–25 min |
| Local private stub | module withdraw to owner | `RISC0_DEV_MODE=1 MODE=module OWNER_PRIVACY=1 MODULE_E2E_WITHDRAW=1 ./verify/e2e.sh local run` | 15–25 min |
| Testnet public | module withdraw to owner | After deploy + pin flip: `MODE=module MODULE_E2E_WITHDRAW=1 ./verify/e2e.sh testnet run` | 12–20 min |
| UI | Basecamp withdraw to owner | One local pass, Demo off: init → deposit → stream → close → claim → withdraw | operator |

`WITHDRAW_PATH` default is `owner`. Slice 6 adds `WITHDRAW_PATH=withdraw`
and `MODULE_E2E_WITHDRAW_NEGATIVES=1`.

Gate log: date, commit, artifact, result, ImageID, `RISC0_DEV_MODE`,
`withdraw_path`.

### Skip

- Store testnet — proof encoding unchanged; Store-local already ran
  on the new ELF.
- Testnet-private module and Store — 2–3 extra hours; leave for a
  later wrap-up.
- Local `RISC0_DEV_MODE=0` — PP harness plus stub-privacy module
  cover routing.
- Provider-close local and close-negatives — close logic unchanged;
  the default module run already sends a close through the uniqueness
  backstop.
- Qt persist kit and Store lifecycle. Focused module unit tests above
  stay Required.

Optional if the default module run is green and a cell is still
cheap: `CLOSE_ROLE=provider` local (~5–10 min) as a second layout
through the uniqueness backstop. Record it when run; do not block
close on it.

## Explicit non-goals

- A recipient-picker field in v1 UI (omit `withdraw_to` only).
- Changing claim or close.
- Non-native token withdraw (Token program path stays future).
- Full Store × privacy × withdraw product.
- Renaming `Instruction::Withdraw`.
- Full `u128` Withdraw in the UI (v1 is `u64` / `amount_hi = 0`; disable
  when unallocated does not fit).
- Waiting for logos-lips#401 to merge to `master` (D54.7).
- Re-running the Step 52 wrap-up matrix on the new ImageID
  (testnet-private, Store-testnet, local real-prove).

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D54.1 | Gate budget | `fast` (including `make check` and module unit tests) + module local both withdraw paths + negatives + Store-local + stub-privacy withdraw to owner + module testnet withdraw to owner + one Basecamp pass. Skip Store-testnet, testnet-private, local real-prove. |
| D54.2 | Uniqueness backstop | Extracted helper in `buildAndSubmit` on every write. Reject length not a multiple of 64, then unique/duplicate 3–6 slot lists, plus mixed-case accept. Negative E2E token stays Reserved. |
| D54.3 | Layouts | Withdraw to owner reuses `InitOrDeposit3`. Four-slot `withdraw` uses new `Withdraw4` and always passes `VaultSubmitContext`. `Withdraw4` slot tests in `test_privacy_submit_policy.cpp`. |
| D54.4 | Credit path | In-guest debit of `vault_holding` and credit of destination (same as `claim` / four-slot `withdraw`). Deposit chains AT. |
| D54.5 | Four-slot same-id | Slice 0 first. Keep 6029 only if the guest runs. Skip adding 6029 if uniqueness rejects first. Module has no `withdraw_to_equals_owner` token. |
| D54.6 | Vault pre-read | After decode, set `ctx.hasDecodedVaultConfig` and `ctx.decodedVaultConfig`. `decodedVault.owner` vs JSON `owner` → Reserved `withdraw_owner_mismatch`. |
| D54.7 | LIP pin | Withdraw LIP commits on `docs/payment-streams-dual-close` (pushed to #401) close this step. Pin that branch tip: `lip155_spec.branch`, full `rev`, `contains` (`32d7da4e` + withdraw commit), `pins.md` branch column, `rfc-index`. Merge to `master` is out of scope. |

## Done when

D54.1 gate log is complete (every Required cell).
Packet is in `completed/` (slice 11).
ImageID living pins match the deploy (README, `testnet-module.json`,
`testnet.json.example`, `bootstrap-testnet.sh`; operational `c30781ea`
grep clean).

- Three-slot withdraw to owner includes (harness, module local, module
  testnet). Public cells: exact holding decrease, exact owner credit,
  allocated unchanged.
- Four-slot distinct `withdraw_to` includes (`"withdraw_to":"$PROVIDER"`).
- Equal-owner JSON: classify-helper unit test plus second submit on the
  local public owner-withdraw cell.
- Four-slot same-id never includes (slice 0 / D54.5).
- Wire golden 2 / 7 / 8 / 9 / 10.
- LIP-155 Required edits on #401; D54.7 pin of that branch tip.
- Coincidence trio present.
- UI: omit `withdraw_to`, unallocated default, `u64` cap, disable at 0;
  walkthrough in `docs/reproduce/basecamp-ui.md`.
- `names.md` destination-split terms committed.

## Related

- [step-44-payer-and-payee-close.md](step-44-payer-and-payee-close.md)
- [step-49-native-token-spec-alignment.md](step-49-native-token-spec-alignment.md)
  (D49.11 ImageID-cut bar)
- [step-52-wrap-up-verification.md](step-52-wrap-up-verification.md)
  (full-matrix pin; out of scope here)
- [logos-lips#401](https://github.com/logos-co/logos-lips/pull/401)
  (`docs/payment-streams-dual-close`)
- [step-21-basecamp-ui.md](step-21-basecamp-ui.md)
- [step-47-unify-role-terminology.md](step-47-unify-role-terminology.md)
- [module/README.md](../../../module/README.md)
