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
That workflow does not depend on a particular `cargo test` filter
or a separate test binary.

Primary local loop:

- `RISC0_DEV_MODE=1 cargo test -p lez_payment_streams_core --lib`

Optional: add a `program_tests` filter
(`… --lib program_tests`)
to match only tests under that module
when you want a slightly faster iteration
and are not touching other unit tests
(for example `mock_timestamp`).

After changes to the guest
or to shared types the guest uses,
rebuild the guest ELF before relying on test results,
for example
`cargo risczero build --manifest-path methods/guest/Cargo.toml`
or
`cargo build -p lez_payment_streams-methods`.

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
  Keep `VaultConfig`, `VaultHolding`, `StreamConfig`,
  shared enums, instruction payload types,
  and pure helpers here.
  Avoid guest runtime or account I/O here.
- `methods/src/lib.rs`
  remains generated-methods glue
  and should stay minimal.
- `lez_payment_streams_core/src/program_tests/`
  contains guest-backed `V03State` tests
  (submodules per instruction, plus `serialization` and `common` helpers).
- `lez_payment_streams_core/src/test_helpers.rs`
  contains reusable test harness helpers
  for keypairs,
  state setup,
  guest deployment,
  and transaction builders.

Negative-case tests use a `*_fails` suffix
when the name alone would be ambiguous
(for example `test_withdraw_exceeds_unallocated_fails`).

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
define vault account payloads (`VaultConfig`, `VaultHolding`) in `lez_payment_streams_core/src/lib.rs`
with manual fixed-width `to_bytes` / `from_bytes` (guest-safe; not derive-based Borsh on account data).
Add `initialize_vault` as an `#[instruction]` function.
Declare vault account with `#[account(init, pda = [...])]`
and authority with `#[account(signer)]`.
Write in-process state tests covering `initialize_vault`
(for example `test_initialize_vault_then_reinitialize_fails`).

### 2. Deposit and withdraw

Decision log updates in `design.md`:
- unallocated-balance rule (`vault_holding.balance - total_allocated`)
- withdraw target semantics
- arithmetic safety policy

Impl:
add `deposit` and `withdraw` instruction handlers.
Use `#[account(mut)]` for balance-bearing accounts
and `#[account(signer)]` for owner authorization.

Tests:
`test_deposit`,
`test_withdraw`,
`test_withdraw_exceeds_unallocated_fails`.

### 3. Stream creation

Decision log updates in `design.md`:
- `StreamConfig` fields and types
- stream id assignment policy
- stream PDA derivation and uniqueness

Impl:
define stream account payload (`StreamConfig`) in `lez_payment_streams_core/src/lib.rs`.
Add `create_stream` handler.
Declare stream account with `#[account(init, pda = [...])]`.

Tests:
`test_create_stream`,
`test_create_stream_exceeds_unallocated_fails` must fail.

### 4. Timestamp and accrual

Decision log updates in `design.md`
(see **Data types** for mock clock wire and versioning policy;
see **Accrual behavior** for lazy accrual via `at_time` and testing notes):

- mock timestamp account contract and read-only role
- error if `now` from the clock is strictly before `accrued_as_of` on the stream
- lazy accrual via `StreamConfig::at_time` in shared core; `at_time`-then-operate in handlers
- time-based accrual only while stored state is `Active`
- pause-on-depletion in the same `at_time` step
- `accrued_as_of`: when the cap is hit from below, the exact depletion instant
  (integer-second timeline; may be before `now` when `now` is later);
  further rules are `StreamConfig::at_time` (e.g. unchanged when `now == accrued_as_of`)

Impl:

- Implement lazy accrual as `StreamConfig::at_time(t)` in `lez_payment_streams_core`
  (single source of truth; guest deserializes, calls `stream_config.at_time(now)`, serializes).
  Expose `StreamConfig::validate_invariants` for shared checks (rate, allocation, accrued cap).
- Add `sync_stream`: loads a stream, validates it against the vault accounts and instruction
  arguments (version alignment, `stream_id` bounds and consistency), runs `validate_invariants`,
  then applies `at_time(now)`, writes `StreamConfig` back.

Tests:

- Unit tests on `StreamConfig::at_time` in the core crate.
- Guest-backed `program_tests`:
  `test_accrual_basic`,
  `test_accrual_caps_at_allocation`.

Follow-up (separate plan tightening, not blocking core accrual):

- Test harness hygiene for the mock clock
  (monotonic-by-default helpers and escape hatches for negative tests).

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

When editing the RFC, align terminology with the implementation:
use **unallocated** for `vault_holding.balance - total_allocated`
(not “available balance,” which is easy to confuse with pending-reservation
semantics in other protocols).
Apply the same wording pass to any spec excerpts that predate this naming.

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
Revisit **unallocated** vs legacy “available” wording in the RFC
(see step 9 terminology note).

## Final polish (implementation)

Items below are optional tightening once behavior and tests are stable.

- Revisit how custom program error codes are modeled in Rust.
  Today they are shared `u32` constants (`ERR_*` in `lez_payment_streams_core`).
  If many call sites start matching on codes and exhaustiveness would help,
  consider a `#[repr(u32)] enum` (not driven by the test helper alone).

- CI and contributor ergonomics for the guest binary:
  ensure automated runs build `lez_payment_streams-methods` (or the guest crate)
  before `cargo test -p lez_payment_streams_core`,
  so embedded program tests never use a stale ELF
  (wrong numeric codes or confusing failures after guest edits).

- Policy for `RISC0_DEV_MODE` in automated versus local runs
  (speed vs parity with non-dev proving), documented in README or `design.md`
  when CI exists.

- If the host ever exposes structured program error codes instead of string messages,
  tighten `assert_execution_failed_with_code` or replace it with a typed assertion.

- Formatting and static analysis before release or large merges:
  `cargo fmt --all` (or scoped to changed crates),
  `cargo clippy --workspace --all-targets` (or equivalent scoped invocation),
  and consistency with the project’s recommended Rust style
  (document or link here when that guide exists).

## Out of Scope

- Off-chain protocol
  VaultProof, StreamProof, eligibility proofs, service messaging.
- Integration with a running sequencer for end-to-end demo.
- Multiple tokens per vault (or per vault holding).
- Future token extension path is out of scope.
- Protocol extensions
  auto-pause, delivery receipts, activation fee, auto-claim.
