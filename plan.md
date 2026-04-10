# Payment Streams on LEZ - Implementation Plan

This plan covers the SPEL-based LEZ implementation
for payment streams,
as defined in
`rfc-index/docs/ift-ts/raw/payment-streams.md`.

Implementation decisions are tracked in
`lez-payment-streams/design.md`.
RFC promotion is deferred
until decisions are stable.

## Scope

- Production implementation repository.
- Includes account model,
instruction handlers,
validation rules,
and tests.
- Vault semantics are single-token (native).
- Off-chain protocol work is out of scope.

## References

- `rfc-index/docs/ift-ts/raw/payment-streams.md` — protocol semantics.
- `lez-payment-streams/design.md` — implementation decision log.
- `spel/` — SPEL framework and macros.
- `lez-book/` — LEZ Development Guide (mdBook).

## Plan Execution Policy

Before editing the active implementation step,
record or update relevant implementation decisions in
`lez-payment-streams/design.md`.
Only the active step should receive concrete amendments.
Promote decisions to
`rfc-index/docs/ift-ts/raw/payment-streams.md`
in explicit batches.
Multi-token support is a future extension.

## Testing Approach

Use in-process `V03State` tests
with a TDD loop:
start with failing tests,
implement,
rerun to green.

Primary checks:

- `cargo risczero build --manifest-path methods/guest/Cargo.toml`
- `RISC0_DEV_MODE=1 cargo test -p lez_payment_streams_core --lib vault_tests`

Keep Borsh guest-safe:
on guest, avoid `#[derive(BorshSerialize, BorshDeserialize)]`—
use manual serialization.
Shared types are guest-relevant,
so direct derive-based Borsh in shared code isn't expected.

## Code Placement in SPEL Repository

- `methods/guest/src/bin/lez_payment_streams.rs`
  contains the `#[lez_program]` module,
  `#[instruction]` handlers,
  account attributes,
  and thin dispatch glue.
- `lez_payment_streams_core/src/lib.rs`
  is the shared types and pure-logic boundary
  for both guest and host code.
  Keep `VaultLayout`, `StreamLayout`,
  shared enums, instruction payload types,
  and pure helpers here.
  Avoid guest runtime or account I/O here.
- `methods/src/lib.rs`
  remains generated-methods glue
  and should stay minimal.
- `lez_payment_streams_core/src/vault_tests.rs`
  contains behavior tests for instruction flows,
  state transitions,
  and negative cases.
- `lez_payment_streams_core/src/test_helpers.rs`
  contains reusable test harness helpers
  for keypairs,
  state setup,
  guest deployment,
  and transaction builders.

Negative-case tests use a `*_fails` suffix
when the name alone would be ambiguous
(for example `test_withdraw_exceeds_available_fails`).

## Plan

### 1. SPEL scaffold and vault account baseline

Decision log updates in `design.md`:
- canonical PDA seed checklist
- stable external identifiers vs internal counters
- account layout versioning policy
- single-token vault definition
- `total_allocated` is single-token scoped

Setup:
1. Copy host-side test helpers from the learning sandbox.
2. Confirm minimal SPEL guest build
   and trivial `V03State` test pass.

Impl:
define `VaultLayout` in `lez_payment_streams_core/src/lib.rs` (Borsh-encoded).
Add `initialize_vault` as an `#[instruction]` function.
Declare vault account with `#[account(init, pda = [...])]`
and authority with `#[account(signer)]`.
Write in-process state tests covering `initialize_vault`
(for example `test_initialize_vault_then_reinitialize_fails`).

### 2. Deposit and withdraw

Decision log updates in `design.md`:
- available-balance rule
- withdraw target semantics
- arithmetic safety policy

Impl:
add `deposit` and `withdraw` instruction handlers.
Use `#[account(mut)]` for balance-bearing accounts
and `#[account(signer)]` for owner authorization.

Tests:
`test_deposit`,
`test_withdraw`,
`test_withdraw_exceeds_available_fails`.

### 3. Stream creation

Decision log updates in `design.md`:
- `StreamLayout` fields and types
- stream id assignment policy
- stream PDA derivation and uniqueness

Impl:
define `StreamLayout` in `lez_payment_streams_core/src/lib.rs`.
Add `create_stream` handler.
Declare stream account with `#[account(init, pda = [...])]`.

Tests:
`test_create_stream`,
`test_create_stream_exceeds_balance` must fail.

### 4. Timestamp and accrual

Decision log updates in `design.md`:
- mock timestamp account contract
- lazy accrual update policy
- pause-on-depletion behavior

Impl:
implement accrual as a pure helper in guest code.

Tests:
`test_accrual_basic`,
`test_accrual_caps_at_allocation`.

### 5. Pause resume top up

Decision log updates in `design.md`:
- legal transition matrix
- resume failure conditions
- top-up effect on stream state

Impl:
add `pause_stream`,
`resume_stream`,
`top_up_stream` handlers.

Tests:
`test_pause`,
`test_resume`,
`test_resume_zero_remaining` must fail,
`test_topup_resumes`.

### 6. Close and claim

Decision log updates in `design.md`:
- close authorization rules
- claim semantics by state
- unaccrued return accounting on close

Impl:
add `close_stream` and `claim` handlers.

Tests:
`test_close_returns_unaccrued`,
`test_claim_transfers_balance`,
`test_claim_after_close`,
`test_close_already_closed` must fail.

### 7. Negative tests and invariants

Decision log updates in `design.md`:
- invariant checklist
- ownership checks
- overflow and underflow guarantees
- account existence and derivation checks

Impl:
systematic negative tests for wrong caller,
invalid transitions,
overflow or underflow,
and operations on non-existent accounts.

### 8. Shielded execution tests

Decision log updates in `design.md`:
- public and shielded parity assumptions
- timestamp constraints in private flow

Impl:
run working public flows
through `execute_and_prove`
and `transition_from_privacy_preserving_transaction`.
Add tests only,
no new program logic.

### 9. RFC promotion batch

This is the first step that edits the RFC.
Promote stable decisions from `design.md`
to `rfc-index/docs/ift-ts/raw/payment-streams.md`.

Include:
- account model and PDA derivation
- instruction definitions
- validation and invariant rules
- time source and accrual behavior
- execution mode notes

### 10. RFC polish and review

Finalize Security and Privacy Considerations
and References in the RFC.
Review consistency across:
implemented behavior,
`design.md`,
and RFC text.

## Out of Scope

- Off-chain protocol
  VaultProof, StreamProof, eligibility proofs, service messaging.
- Integration with a running sequencer for end-to-end demo.
- Multiple tokens per vault (or per vault holding).
- Future token extension path is out of scope.
- Protocol extensions
  auto-pause, delivery receipts, activation fee, auto-claim.
