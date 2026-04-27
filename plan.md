# Payment Streams on LEZ - Implementation Plan

This plan covers the SPEL-based LEZ implementation
for payment streams,
as defined in
`rfc-index/docs/ift-ts/raw/payment-streams.md`.

Implementation decisions were tracked in
`lez-payment-streams/design.md`, which was retired in step 11.
Its content was redistributed to:
`lez-payment-streams/README.md` (code map, test commands, fixture hierarchy, PP coverage),
`rfc-index/docs/ift-ts/raw/payment-streams.md` Implementation Considerations (PDA derivation,
account types, balance accounting, time source, authorization, privacy tier, PP execution model),
and targeted code comments in guest and core source files.

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
- `lez-payment-streams/README.md` — code map, test commands, fixture hierarchy, PP coverage (design.md retired in step 11; content redistributed here and to the RFC).
- logos-execution-zone PR 403 `https://github.com/logos-blockchain/logos-execution-zone/pull/403` — system clock accounts.
- `spel/` — SPEL framework and macros.
- SPEL PR 126 `https://github.com/logos-co/spel/pull/126` — unified `SpelOutput::execute()` (auto-claim from account attributes); deprecates `states_only` / `with_chained_calls`.
- `lez-book/` — LEZ Development Guide (mdBook).

## Plan Execution Policy

Steps 1–11 are complete.
For step 12 and beyond, promote decisions directly to
`rfc-index/docs/ift-ts/raw/payment-streams.md`.
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
and are not touching other unit tests.

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

## Completed Work

Reference summary of work already landed.
Decision bullets for each item are recorded in `design.md`.

- SPEL scaffold and vault baseline:
  `VaultConfig` and `VaultHolding` payloads with manual `to_bytes` / `from_bytes`,
  `initialize_vault` handler with PDA-derived vault accounts.
- Deposit and withdraw:
  `deposit` and `withdraw` handlers,
  unallocated-balance rule (`vault_holding.balance - total_allocated`),
  owner authorization.
- Stream creation:
  `StreamConfig` payload,
  `create_stream` handler with stream PDA,
  stream id assignment policy.
- Timestamp and accrual:
  mock timestamp account,
  lazy accrual via `StreamConfig::at_time` in shared core,
  `sync_stream` handler,
  `StreamConfig::validate_invariants`,
  pause-on-depletion,
  depletion-instant handling for `accrued_as_of`.
- Pause, resume, top-up handlers with legal-transition enforcement.
- Close stream with unaccrued return.
- Claim with state-dependent semantics.
- Systematic negative tests and invariants:
  wrong caller, invalid transitions, overflow or underflow,
  operations on non-existent accounts.

## Plan

Numbered steps below replace the remaining work.
Steps are listed in execution order.

### Ordering overview

```mermaid
flowchart TD
    S1[Step 1 non-test cleanup] --> S2[Step 2 LEZ clock and SPEL execute]
    S2 --> S3[Step 3 test fixtures]
    S3 --> S4[Step 4 spec audit and test hardening]
    S4 --> S5[Step 5 shielded execution complete]
    S5 --> S6[Step 6 privacy_tier and host policy]
    S6 --> S7[Step 7 adapter refactor and selective privacy rollout]
    S7 --> S8[Step 8 remaining refactor]
    S8 --> S9[Step 9 remove SyncStream]
    S9 --> S10[Step 10 PP vault operations]
    S10 --> S11[Step 11 documentation pass]
    S11 --> S12[Step 12 RFC proposal and promotion]
    S4 -. accumulates .-> S8
    S6 -. accumulates .-> S12
    S7 -. accumulates .-> S12
```

### 1. Non-test cleanup

Mechanical, low-risk, preserves external surface.
Touches core and guest only.

- `cargo fmt --all`.
- `cargo clippy --workspace --all-targets`,
  apply only mechanical fixes
  (unused imports, needless clones, redundant closures).
- Extract the repeated prologue in
  `methods/guest/src/bin/lez_payment_streams.rs`
  shared by `close_stream`, `claim`, and `top_up_stream`.
  Shape: two or three variants keyed on auth rule
  (owner-signed, authority-signed, provider-signed),
  reusing or generalizing the existing `load_vault_stream_and_clock`.
- Add a small post-state constructor helper
  (for example `states_only_5(vault_config, holding, stream, signer, clock)`).

Do not touch in this step:
public names on `VaultConfig`, `VaultHolding`, `StreamConfig`, `Instruction`, `ERR_*`;
core math signatures
(`at_time`, `close_at_time`, `claim_at_time`, `resume_from_paused_at`, `validate_invariants`);
module boundaries;
`assert_execution_failed_with_code` semantics.

### 2. LEZ upgrade, clock migration, and SPEL `execute` migration

Retire `lez_payment_streams_core/src/mock_timestamp.rs`
and all `MockTimestamp` references
in favor of the system clock accounts from
logos-execution-zone PR 403
(`CLOCK_01`, `CLOCK_10`, `CLOCK_50`; 16-byte `(block_id, timestamp)` payload).

In the same dependency bump,
migrate the guest from deprecated `SpelOutput::states_only` / `with_chained_calls`
to `SpelOutput::execute` (SPEL PR 126):
one pinned SPEL revision across `methods/guest`, `lez_payment_streams_core`, and `examples`,
verified together with the LEZ/NSSA upgrade.

Decision log updates in `design.md`:

- system clock accounts supersede `MockTimestamp`
- clock granularity policy (guest accepts any of the three; client picks)
- retirement of `ERR_INVALID_MOCK_TIMESTAMP` and `SEED_MOCK_CLOCK`
- private-proof invalidation rationale for coarser clocks
- SPEL `execute` migration and pinned SPEL git revision (PR 126 included in the pin)

#### 2.1 Upgrade LEZ, NSSA, and SPEL dependencies

- Bump the LEZ and NSSA dependencies
  to a revision that includes PR 403 merged.
- Bump SPEL (`spel-framework`, `spel-framework-core`, and any other SPEL crates in the workspace)
  to a git revision that includes PR 126 merged (`SpelOutput::execute`).
  Pin the same `rev` on every SPEL git dependency so macros and core stay aligned.
- Resolve versions so the chosen SPEL revision is compatible with the chosen LEZ/NSSA stack;
  if a single combination fails CI, adjust pins or split only as a last resort.
- Verify in the resolved version:
  - `V03State::new_with_genesis_accounts` seeds the three clock accounts.
  - `CLOCK_01_ID`, `CLOCK_10_ID`, `CLOCK_50_ID` (or equivalents)
    are exported and reachable from guest and host.
  - The clock program id is reachable for ownership checks.
- Smoke test: existing suite still passes after the bump
  (build the guest, then run
  `RISC0_DEV_MODE=1 cargo test -p lez_payment_streams_core --lib`).
  Fix any upstream breakage in isolation before starting the rest of the migration.

#### 2.2 Guest and core changes

- Delete `MockTimestamp` or shrink it to a test-only payload constructor
  (`fn clock_payload(block_id: u64, timestamp: u64) -> Vec<u8>`).
- Migrate guest handlers from `SpelOutput::states_only` / `with_chained_calls`
  to `SpelOutput::execute` per SPEL PR 126;
  refactor or remove `states_only_five_owner_stream_sync_layout`
  so the guest uses the unified API (macro-generated claims) end to end.
- In `methods/guest/src/bin/lez_payment_streams.rs`,
  replace `parse_mock_timestamp` with `parse_clock_account`
  that reads the 16-byte `(block_id, timestamp)` layout
  and returns the `timestamp` as `Timestamp`.
- Add clock identity validation inside `parse_clock_account`.
  Pick one of:
  - owner check against the clock program id
    (`account.program_owner == CLOCK_PROGRAM_ID`), or
  - allowlist against the three system clock account ids.
  Emit a new error code `ERR_INVALID_CLOCK_ACCOUNT`
  (append after 6025; do not renumber existing codes).
- Retire `ERR_INVALID_MOCK_TIMESTAMP` (6011).
  Leave the constant reserved and unused.
- `StreamConfig::at_time` and the other math in
  `lez_payment_streams_core/src/stream_config.rs`
  stay unchanged.
- Do not alter `Instruction` variants in this step.
  Client chooses clock granularity
  by which clock account id it includes in `account_ids`.

Note (clock types in core).
`lez_payment_streams_core/src/clock_wire.rs` duplicates the system clock
`AccountId` constants and `ClockAccountData` Borsh layout from LEZ `clock_core`
instead of depending on that crate,
to avoid Cargo friction (guest vs host workspaces, git pins, and patch rules)
while the LEZ and SPEL stack was aligned.
Revisit in step 8 (or earlier if the graph simplifies)
and prefer a direct `clock_core` dependency from the same LEZ `rev`/`tag` as `nssa_core`
so definitions stay synchronized with upstream.

Note (test PDA helpers).
`lez_payment_streams_core/src/test_pda.rs` mirrors the seed combination rules from
`spel-framework-core::pda` (`seed_from_str`, multi-seed hashing, then `PdaSeed` /
`AccountId` via `nssa_core`).
It does not reimplement NSSA’s PDA-to-id mapping.
The duplication avoids adding `spel-framework-core` as a dev-dependency on
`lez_payment_streams_core`, which would pull a second `nssa_core` revision
(SPEL’s LEZ pin) and break type identity with the crate’s main `nssa_core` dep.
Revisit in step 8 (or with step 3 fixture work) and drop `test_pda.rs` once SPEL and LEZ
pins guarantee a single `nssa_core` in the test graph.

#### 2.3 Test harness changes

- Replace `force_mock_timestamp_account` in
  `lez_payment_streams_core/src/test_helpers.rs`
  with `force_clock_account(state, clock_id, block_id, timestamp)` that:
  - writes the 16-byte payload,
  - sets `program_owner` to the clock program id
    so the guest identity check passes.
- In `lez_payment_streams_core/src/harness_seeds.rs`,
  retire `SEED_MOCK_CLOCK`
  and add constants or helpers that surface
  the three system clock account ids.
- Update `lez_payment_streams_core/src/program_tests/common.rs`
  fixtures (`state_deposited_with_clock*`)
  to take a clock account id from the system clocks,
  not a keypair-derived id.
- Tests should bind the harness clock as `clock_account_id`
  (and clock timestamps as `clock_initial_ts`, etc.),
  not legacy “mock clock” local names.

#### 2.4 Design doc and proposal list updates

- In `design.md`,
  replace the "placeholder timestamp source account" paragraph under Data types
  with a section describing the system clock accounts,
  the 16-byte layout,
  granularity trade-offs,
  and the guest-side identity check.
- Add entries to the RFC-proposal list (seed for step 10):
  replace "mock timestamp source" wording with system clocks;
  document the granularity trade-off
  in Security and Privacy Considerations.

### 3. Test fixture extraction

Runs over migrated code so helpers are keyed on the new clock reality.

- Introduce a `VaultFixture` struct returned by `state_with_initialized_vault*`
  (replacing the 7-tuple destructuring) with fields
  `state`, `program_id`, `owner_key`, `owner_id`,
  `vault_id`, `vault_config`, `vault_holding`.
  Add `provider` and `clock_id` where fixtures provide them.
- Add a scenario builder for
  "vault initialized, deposit made, clock set, stream created,
  clock advanced, synced at `t1`".
  The three tests in
  `lez_payment_streams_core/src/program_tests/claim.rs`
  collapse down to the differing tail after this helper lands.
- Generalize `first_stream_accounts` so it builds `StreamIxAccounts`
  directly from a `VaultFixture` plus stream PDA.
- Consolidate per-test constants (`allocation`, `rate`, `t0`, `t1`)
  that three or more tests in the same module share,
  promoted to module-level `const`s.
- Sort and deduplicate `use` blocks across test modules.

### 4. Spec audit and test hardening

Walk `rfc-index/docs/ift-ts/raw/payment-streams.md`
and `design.md` against the code.
Produce a running three-bucket list during the audit:

1. missing or weak tests (add in place),
2. behavior gaps (fix in place, minimal change),
3. RFC-proposal candidates (append to the step 10 list).

Specific items to check that are not fully covered today:

- Solvency and conservation invariants as scenario tests:
  `vault_holding.balance >= vault_config.total_allocated` and
  `total_allocated == Σ stream.allocation` after arbitrary legal sequences.
- Arithmetic boundaries at `u128::MAX`, `u64::MAX`, `Timestamp::MAX`,
  `next_stream_id` overflow, `total_allocated` overflow,
  stream `allocation` overflow on top-up.
- Authorization matrix: one wrong-signer negative case per instruction.
  Consider a parameterized module.
- `sync_stream` edge cases:
  `now == accrued_as_of` is a no-op fold;
  depletion via sync;
  time regression fails.
- Withdraw recipient existence precondition,
  parallel to the documented provider-account precondition on claim.
- Deterministic PDA derivation test that asserts the host helper
  in `lez_payment_streams_core/src/test_helpers.rs`
  matches the guest `#[account(pda = [...])]` seed declarations.
- Clock harness hygiene:
  monotonic-by-default helpers
  and an explicit escape hatch for negative tests,
  forward from the earlier mock-clock follow-up,
  now keyed on the system clock.

### 5. Shielded execution tests

Shielded-mode tests now cover representative flows through
`execute_and_prove`
and
`transition_from_privacy_preserving_transaction`
under the current account model.
No protocol redesign happened in this step.

Benefits from step 2 because the system clock
matches the private-proof invalidation model that PR 403 was designed around.

Decision log updates in `design.md`:

- public and shielded parity assumptions
- timestamp constraints in private flow
  (clock granularity choice per instruction)

Normative detail for NSSA visibility, PP transition rules,
the `withdraw` claim metadata change,
and deposit PP limitations is recorded under
Privacy-preserving execution (NSSA) and step 5 tests
in `design.md`.

RFC-level privacy goals,
how the current guest contradicts them,
and protocol directions beyond step 5
are spelled out in the section
Transition to private execution
after the numbered Plan steps.

### 6. privacy_tier and host policy

Goal:
keep the current protocol semantics and math,
while persisting an explicit `privacy_tier` on `VaultConfig`
and documenting a realistic pseudonymous unlinkability story.

Introduce `privacy_tier` at vault creation time (immutable for life).
The stored tier is a small discriminant with labels for product and docs,
not a consensus-enforced PP gate.

- `Public`:
  may execute via public or PP transactions.
  No owner-funding unlinkability guarantee.
- `PseudonymousFunder`:
  intended for PP-only lifecycle
  (vault and stream instructions) under our host or wallet.
  The unlinkability target is primary public funder key
  versus vault and stream activity,
  not hiding that a vault has a controller `AccountId` on-chain.

#### Chosen MVP approach (implementation target)

- `VaultConfig.owner` remains the authorization anchor.
  Operationally it should be a dedicated private-account `AccountId`
  (npk-derived in the LEZ sense), not the user’s main public wallet id,
  when unlinkability from that main key matters.
- Authorization in guest code stays the same shape:
  `owner.account_id` must match `VaultConfig.owner`
  and owner-gated instructions keep `#[account(signer)]`.
  Under PP, control of a `PrivateOwned` owner row
  is established by the wallet and NSSA witness material,
  not by “the same mechanical public tx signature,”
  but the guest still checks the same equality as today.
- Funding discipline:
  users should pre-shield spendable balance
  before funding a `PseudonymousFunder` vault,
  then use PP-shaped flows so liquidity enters `VaultHolding`
  without a public leg that debits a doxxed public account
  in the same correlation surface as the vault.
  A public shielding hop from Alice’s public account
  still creates Alice → private persona;
  delay and hygiene reduce naive timing correlation only.
- `vault_config` row visibility:
  if the config account stays a public row in mixed PP,
  `VaultConfig.owner` bytes remain observer-visible
  as a persistent pseudonym,
  not “secret owner metadata.”
  Unlinkability is therefore pseudonymity relative to the main funder key,
  given no on-chain bridge between them.

#### Where enforcement lives (pinned NSSA and LEZ fact)

On the pinned stack,
the inner program guest receives only
`program_id`, `caller_program_id`, `pre_states`, and `instruction_data`
via `Program::write_inputs`.
It does not receive the visibility mask;
that mask is an input to the outer privacy-preserving circuit only.

Therefore:

- PP-only for `PseudonymousFunder` vaults is enforced by our host or wallet
  (refuse to build or submit public transitions that touch those vaults),
  and by tests that assert that behavior for our harness.
- The application guest must not be described as enforcing
  “public vs PP execution mode,”
  because it has no trustworthy execution-mode signal in its input env.
- The guest still stores and checks `privacy_tier` in `VaultConfig`
  for on-chain semantic rules
  (for example immutability, or invariants that depend on tier),
  not for detecting PP.

#### What we do not claim without further platform work

- We do not guarantee that arbitrary third-party submitters
  cannot attempt a public path:
  that would require consensus-level rules
  (for example a generic account or program policy hook in NSSA),
  not payment-streams guest logic alone.

Implementation work:

- Extend `VaultConfig` with a `privacy_tier` field (`Public` or `PseudonymousFunder`).
- Thread `privacy_tier` through initialization and validation helpers.
- Guest: tier-aware checks only where derivable from state bytes
  (not from execution mode).
- Host or test harness:
  refuse public transitions for `PseudonymousFunder` vault fixtures;
  document the same policy for product wallets.
- Add positive PP tests for PseudonymousFunder-tier flows
  (and extend fixtures for private-owner personas where needed).
- Add negative harness tests that a public transition attempt
  for a `PseudonymousFunder` vault is rejected before or without
  committing the unwanted linkage policy
  (exact hook depends on where the harness can intercept).

Decision log updates in `design.md`:

- exact `privacy_tier` semantics and immutability
- privacy guarantee scope:
  pseudonymous controller,
  pre-shield and PP-only operational requirements,
  first-hop shielding caveat
- guest vs host vs (optional future) consensus enforcement boundaries,
  citing inner-program inputs vs outer-circuit `visibility_mask`

### 7. Adapter refactor and selective privacy rollout

Goal:
maximize reuse of business logic
while separating host transaction construction
from guest state transition code.

Refactor shape:

- Keep stream math,
  lifecycle transitions,
  and invariants in shared core.
- Keep guest handlers thin:
  parse pre-state,
  apply state-derived `privacy_tier` rules only,
  invoke shared helpers,
  serialize post-state.
- Centralize fixture and harness helpers that build PP account lists,
  visibility masks,
  and witness inputs for `PseudonymousFunder` vault scenarios
  (including private-owner rows where tests require them).
- Introduce small adapter helpers for account parsing and output assembly
  where it reduces duplication without hiding execution-mode facts
  inside the guest.
- Avoid duplicated accrual or conservation logic across modes.

Selective privacy rollout under current account model:

- `PseudonymousFunder` tier:
  our host or wallet enforces PP-only interaction;
  guest remains agnostic to public versus PP envelope.
- `Public` tier:
  public and PP execution remain allowed.
  PP can provide selective confidentiality
  but does not imply owner-funding unlinkability
  if the vault was created or funded on a public path.
- Stream-vault linkage remains visible in the current PDA model.
  This step does not attempt commitment-native custody.

### 8. Remaining refactor

With public,
shielded,
and `privacy_tier` policy suites green,
reshape where audit findings and reviewer perspective now justify it.

Candidates not resolved in earlier steps:

- Replace `lez_payment_streams_core/src/clock_wire.rs` with a dependency on LEZ
  `clock_core` (same git `rev`/`tag` as `nssa_core`) once pins and workspaces allow,
  removing duplicated clock ids and `ClockAccountData` layout.
- Remove `lez_payment_streams_core/src/test_pda.rs` in favor of
  `spel-framework-core::pda` (or equivalent) in tests once dev-dependencies resolve to
  exactly one `nssa_core`, so host PDA helpers cannot drift from SPEL.
- Collapse contextual `spel_custom(code, "message")` call sites via a small helper
  (design informed by the full call-site set after step 4).
- Decide `ERR_*` as `#[repr(u32)]` enum vs keep `u32` constants,
  gated on whether tests actually match on many codes.
- Error code gaps (e.g. legacy reserved numbers no longer emitted):
  either keep them documented as permanently reserved in `design.md`,
  or plan a single breaking renumbering pass that produces a dense `6001..` table
  and updates every consumer that matches on numeric codes
  (not mixed with feature work; treat as an explicit compatibility event).
- Any public-name renames on types or fields,
  applied in a single sweep together with the reviewer doc in step 9.
- Unused imports: run the clippy/unused-import pass workspace-wide,
  tighten explicit `use` lists,
  and remove blanket allows that only exist to hide them
  (for example `#[allow(unused_imports)]` on the guest program module).
- Final `cargo fmt` and
  `cargo clippy --workspace --all-targets`
  sweep before handoff.

### 9. Remove SyncStream public instruction

The lazy accrual fold (`StreamConfig::at_time`) is already called as the
first step in every lifecycle instruction.
A public `sync_stream` instruction adds no computation that a client cannot
perform locally by reading `StreamConfig` and a clock account and calling
`at_time(now)` directly.
Removing the public entry point shrinks the instruction surface without
removing any expressible behavior.

Work:

- Remove the `SyncStream` variant from `Instruction` in
  `lez_payment_streams_core/src/instruction.rs`.
- Remove the `sync_stream` handler from
  `methods/guest/src/bin/lez_payment_streams.rs`.
- Remove `signed_sync_stream` and any `SyncStream`-specific helpers from
  `lez_payment_streams_core/src/test_helpers.rs`.
- Migrate `lez_payment_streams_core/src/program_tests/accrual.rs`:
  replace the harness-backed integration tests with unit tests directly on
  `StreamConfig::at_time`.
  The fold logic is unchanged; only the test vehicle changes.
  Cover: normal accrual, depletion auto-pause, time regression,
  idempotent double-fold at the same timestamp, multi-stream isolation.
- Remove the `accrual` entry from
  `lez_payment_streams_core/src/program_tests/mod.rs`.
- Add a note in `design.md` documenting the removal decision and the
  off-chain fold pattern
  (read `StreamConfig` + `ClockAccountData`, call `at_time(now)`)
  as the intended client-side substitute.

Do not change:

- `StreamConfig::at_time` or any other core fold logic.
- Any other instruction handler or its test file.

### 10. Privacy-preserving vault operations

**Prerequisite: Step 5 must be complete.**
`shielded_execution.rs`, the constants `RECIPIENT_NSK` / `RECIPIENT_VSK` / `EPK_SCALAR`,
`run_pp_withdraw_to_private_recipient`, and the
`execute_and_prove` / `try_from_circuit_output` / `WitnessSet::for_message` call patterns
all come from Step 5.
Phase 1 cannot start until that infrastructure exists.

Extends PP test coverage from the current `withdraw`-only baseline to the
full instruction set.
No new NSSA or SPEL platform work is required.
Changes are primarily new tests and test infrastructure,
with one conditional guest change depending on Phase 1 findings.

**Hypothesis**: no guest code changes are required.
All signing accounts are already included in handler output vectors.
The PP circuit applies `private_account_nonce_increment(nsk)`
to the guest's post-state outside the guest;
the `#[account(signer)]` vs `#[account(mut, signer)]` distinction
does not affect output inclusion or circuit re-commitment.
Phase 1 validates or falsifies this.
If Phase 1 falsifies it,
add `#[account(mut, signer)]` to the owner parameter of
`initialize_vault`, `create_stream`, `pause_stream`, `resume_stream`,
and `top_up_stream`,
and to `authority` in `close_stream`,
then continue.

#### Phase 1: validate visibility-1 signer mechanics

Uses the existing RECIPIENT identity (RECIPIENT_NSK / RECIPIENT_VSK),
which already has a commitment after a PP withdraw,
as the private provider / authority.
No new constants or fixture helpers needed.

Each test sets up its own state independently — there is no shared fixture object.
Both tests run the same three-step ladder inline (or via an extracted helper):
extract a `pp_claim_close_fixture(block_deposit, block_stream) -> (VaultFixture, StreamId)`
that both tests call if the setup proves long enough to be worth factoring out.

Ladder (per test):
1. `vault_fixture_public_tier_funded_via_deposit()`
2. PP-withdraw 50 to `recipient_npk()` — creates RECIPIENT commitment
   (reuse `run_pp_withdraw_to_private_recipient`; this step is
   identical to the existing `test_withdraw_private_recipient_pp_transition_succeeds` setup)
3. Create stream (public tx): `provider = AccountId::from(&recipient_npk())`

Tests:

`test_pp_claim_private_provider_succeeds`
- visibility_mask: `[0, 0, 0, 0, 1, 0]` (provider at index 4)
- `private_nsks = vec![RECIPIENT_NSK]`, `membership_proofs = vec![None]`
- `try_from_circuit_output(public_ids, [], [], output)`:
  public_ids = vault_config, vault_holding, stream_config, owner, clock;
  empty public nonces (nullifier handles replay for visibility-1);
  empty private recipients (no visibility-2 accounts)
- `WitnessSet::for_message(&message, proof, &[])` — no public signers
- Asserts: vault_holding balance reduced by payout,
  provider's new commitment decryptable to updated balance,
  stream_config updated

`test_pp_close_stream_private_provider_authority_succeeds`
- Same ladder; RECIPIENT closes the stream as `authority`
  (provider satisfies the authority check)
- visibility_mask: `[0, 0, 0, 0, 1, 0]` (authority at index 4)
- Same private_nsks / membership_proofs pattern
- Asserts: stream state is Closed,
  unaccrued balance returned to public owner

Open question resolved in Phase 1:
does `try_from_circuit_output(ids, [], [], output)` work
when the only signer is visibility-1 (empty public-nonce slice)?

Concrete fallback if it does not:
set the signer account (provider / authority) as visibility-0 instead of visibility-1.
This makes the signer's `account_id` publicly visible but keeps all other PP machinery
(commitment generation, ciphertext, new nullifier) intact.
Document the finding in `design.md` as a known limitation:
"owner-signer identity is revealed in the current platform version."
Defer full identity hiding to a platform-level fix (out of scope for this step).

#### Phase 2: PP deposit

The `deposit` handler already chains to `authenticated_transfer_program`,
which owns the user's native account and is the only program
authorized by `validate_execution` to decrease its balance.
The PP circuit supports single chained calls:
`execute_and_prove` proves each program in the chain sequentially
and adds each receipt as a RISC0 assumption via `env_builder.add_assumption`.

Client-side change only.
Add a new helper to `shielded_execution.rs` alongside the existing `load_guest_program()`:

```rust
pub fn load_payment_streams_with_auth_transfer() -> ProgramWithDependencies {
    let payment_streams = load_guest_program();
    let auth_transfer = Program::authenticated_transfer_program();
    ProgramWithDependencies::new(payment_streams, [(auth_transfer.id(), auth_transfer)].into())
}
```

The PP deposit test calls this helper instead of `load_guest_program()`.
No guest code changes.

The deduplicated account set seen by the circuit is
`[vault_config, vault_holding, owner]`
in the order accounts first appear across both program outputs —
the same three-account layout as the public deposit.

Test: `test_pp_deposit_private_owner_succeeds`
- Ladder: private owner funded from a separate funding vault via PP withdraw;
  vault under test initialized with `owner_account_id = AccountId::from(&owner_npk())`
  and funded via `transfer_native_balance_for_tests`
- visibility_mask: `[0, 0, 1]`
  (vault_config, vault_holding public; owner visibility-1)
- `private_nsks = vec![OWNER_NSK]`
- Asserts: vault_holding.balance increased by the deposit amount;
  owner's new commitment decryptable to reduced balance
- Note in test: the deposit amount is publicly visible
  from vault_holding's balance change;
  vault_holding is a public PDA and its balance appears in public_post_states

#### Phase 3: owner-signer instructions

Covers `create_stream`, `pause_stream`, `resume_stream`, `top_up_stream`,
and a PP-owner variant of `withdraw`.
The vault owner must be a private identity whose AccountId
is derived from a nullifier public key,
matching the PDA seed `account("owner")`.

New constants (in `shielded_execution.rs`):
```rust
const OWNER_NSK: NullifierSecretKey = [0x7c; 32];
const OWNER_VSK: Scalar             = [0x8d; 32];
const OWNER_FUND_EPK_SCALAR: Scalar = [4u8; 32];
```

New helpers:
- Rename `run_pp_withdraw_to_private_recipient`
  → `fund_private_account_via_pp_withdraw(fx, npk, nsk, esk, amount, block)`.
  The two existing callers —
  `test_withdraw_private_recipient_pp_transition_succeeds` and
  `test_pp_withdraw_private_recipient_pseudonymous_funded_vault_succeeds` —
  become thin wrappers that pass the RECIPIENT constants into the new form.
  No behaviour change; just a signature generalization.
- `vault_fixture_with_npk_derived_owner(funding_amount) → VaultFixture`:
  creates a vault where `owner_account_id = AccountId::from(&owner_npk())`;
  calls `initialize_vault` in a public tx
  (the private owner is a valid public signer for this one step);
  funds via `transfer_native_balance_for_tests`,
  `PseudonymousFunder` tier.

Shared ladder:
1. `vault_fixture_public_tier_funded_via_deposit()` as vault_A (funding vault)
2. `fund_private_account_via_pp_withdraw(&mut fx_A, owner_npk(), ..., 50, block)`
   → creates OWNER commitment in state
3. `vault_fixture_with_npk_derived_owner(400)` as vault_B

Tests (all on vault_B; visibility_mask `[0, 0, 0, 1, 0]` unless noted):
- `test_pp_create_stream_private_owner_succeeds`
  (owner at index 3, clock at index 4 public)
- `test_pp_pause_stream_private_owner_succeeds`
- `test_pp_resume_stream_private_owner_succeeds`
- `test_pp_top_up_stream_private_owner_succeeds`
- `test_pp_withdraw_private_owner_succeeds`:
  owner visibility-1 (spending from private balance),
  recipient visibility-2;
  visibility_mask `[0, 0, 1, 2]`

All Phase 3 tests:
`private_nsks = vec![OWNER_NSK]`, `membership_proofs = vec![None]`.

#### Phase 4: PP initialize_vault

Owner must have a commitment before their vault exists.
Uses the same two-step ladder
(fund private owner from vault_A, then PP-initialize vault_B).

Test: `test_pp_initialize_vault_private_owner_succeeds`
- visibility_mask: `[0, 0, 1]`
  (vault_config and vault_holding init as visibility-0; owner visibility-1)
- Completes full PP coverage across all instructions.

#### Decision log updates in `design.md`

- Update PP coverage table after each phase.
- Record that Option B (PP deposit via ChainedCall) is implemented,
  deposit amount remains visible,
  and `ProgramWithDependencies` is the mechanism.
- Record Phase 1 finding on visibility-1 signer mechanics
  and whether the guest annotation hypothesis held.

### 11. Documentation pass

No separate reviewer guide document.
Distribute documentation across three artifacts:
the spec's on-chain Implementation Considerations section (step 11),
`README.md`,
and code comments.
Retire `design.md` once its content is redistributed;
drop pure implementation history ("in step X we did Y")
and preserve only what describes the system as it stands.

Note: `design.md`'s PP section grew substantially in step 10 (Phase 1–4 decision log,
`output_index` semantics, auth_transfer ownership constraint, force-insert test pattern,
full coverage table).
The redistribution below accounts for this; do not underestimate the scope.

#### README

- Code map: which crate and file owns which concern
  (guest binary, core lib, test harness, examples).
- How to run tests:
  command, `RISC0_DEV_MODE` flag,
  when to rebuild the guest ELF and the command to do so.
- Platform pins: LEZ tag, SPEL revision.
- Test fixture hierarchy:
  `VaultFixture` → `DepositedVaultFixture` → `DepositedVaultWithProviderFixture`.
- PP coverage table: instruction × private role (owner vis-1, provider vis-1, recipient vis-2);
  all instructions covered end-to-end after step 10.
- PP test harness: `fund_private_account_via_pp_withdraw` and `pp_owner_setup` for private-owner
  ladders; force-insert pattern (`patch_vault_config` + direct account write) for stream setup in
  pause / resume / top-up tests; `load_payment_streams_with_auth_transfer` for chained-program
  (deposit) tests.
- Clock helpers: `force_clock_account_monotonic` for happy-path tests,
  `_unchecked` for time-regression tests.

#### Code comments

Apply at the locations listed below.
Do not add comments that restate what well-named identifiers already express.

- `asset_tag = b"native"` reserves a path for future per-token vaults
  → VaultHolding PDA derivation in the guest.
- Why `rate × elapsed` is computed as `u128` rather than stored wide
  → multiplication site in `at_time`.
- Depletion instant formula (ceiling division)
  → formula site in `at_time`.
- What `accrued_as_of` means when depleted versus not
  → `StreamConfig.accrued_as_of` field doc.
- Why `accrued` is left unchanged on resume
  but `accrued_as_of` is reset to `now`
  (wall time while paused must not accrue later)
  → `resume_from_paused_at`.
- Two clock-loading paths and when each applies
  (`load_vault_stream_and_clock` vs `_with_explicit_owner`)
  → those function definitions.
- `block_id` validated structurally but not used for stream math
  → `parse_clock_account`.
- Unknown clock payload extensions treated as parse failures
  → `parse_clock_account`.
- Owner as explicit non-signer account in `CloseStream` and `Claim`
  (structural binding; defense in depth alongside PDA)
  → those handler entry points.
- `AutoClaim` for default-owned withdraw recipient
  (PP circuit requirement for modified-but-not-claimed accounts)
  → `withdraw` handler.
- PP deposit uses `ProgramWithDependencies` with `authenticated_transfer_program`
  as the chained dependency;
  the deposit amount is publicly visible because vault_holding is a public PDA
  → `deposit` handler and `shielded_execution.rs`.
- `output_index` starts at 0 and increments for each private account slot (vis-1 or vis-2)
  in account order; decryption must pass the matching index or it fails with `DataTooBigError`
  → PP decryption call sites in `shielded_execution.rs`.

#### Spec on-chain section

The spec's Implementation Considerations section
is the primary home for implementation-level design decisions.
This work is owned by step 11;
step 10 only catalogues what needs to move there from `design.md`.

Material to promote (fills current spec placeholders):

- PDA seeds for all three account types,
  with rationale for non-obvious choices
  (provider in data not seeds;
  `stream_id` from counter not client;
  `asset_tag` reserves future token path).
- Field types with rationale
  (`Balance = u128`, `Timestamp / rate = u64`,
  widening in accrual math).
- Full authorization matrix (all nine instructions × required signer).
- Why `CloseStream` and `Claim` pass owner as an explicit non-signing account.
- Balance accounting definitions and invariants
  (`unallocated`, `unaccrued`, `allocation` as current commitment,
  two solvency invariants, how `total_allocated` stays in sync).
- Time source: system clock accounts, 16-byte layout, allowlist validation,
  granularity tradeoffs, off-chain fold pattern.
- Accrual semantics: lazy fold, `at_time`-then-operate pattern,
  time regression as error, depletion auto-pause.
- Privacy tier: `VaultPrivacyTier` values and immutability,
  what `PseudonymousFunder` does and does not guarantee,
  where enforcement lives (host/wallet, not guest),
  PP limitations (deposit amount visible;
  vault_holding is a public PDA so balance changes are on-chain).
- PP execution model specifics (from step 10 decision log):
  mixed-visibility pattern (public PDAs + at least one private slot);
  `ProgramWithDependencies` for chained-program PP calls (deposit uses auth_transfer);
  owner commitment must be auth_transfer-owned for PP deposit
  (`validate_execution` blocks balance decreases on accounts not owned by the executing program);
  seed-derived PDAs work as vis-0 public rows in mixed-visibility PP calls
  (`create_stream`, `initialize_vault` confirmed in Phases 3–4).
- Validation rules: zero-amount guards, version matching, `vault_id` defense in depth.
- Double-close behavior (errors, not idempotent)
  and `CloseStream` / `Claim` authorization specifics
  (fills current spec placeholders).

Do not duplicate the state machine diagram or lifecycle narrative
already in the abstract protocol section of the spec;
reference it and note only implementation divergences.

### 12. RFC proposal, promotion, and polish

Consolidate the RFC-proposal list accumulated in steps 2 through 11,
apply it in one batch to
`rfc-index/docs/ift-ts/raw/payment-streams.md`,
and finalize the document.

Deliverable: a reviewed, self-consistent RFC with a rationale paragraph
per normative change.

Known seeds:

- `unallocated` terminology
  (replace any lingering “available balance” wording).
- `allocation` as current commitment (`accrued + unaccrued`).
- `resume` wording around `unaccrued`
  (resume requires `accrued < allocation`).
- Equivalence criterion: streams match when
  `allocation`, `accrued`, `rate`, and `state` match,
  not when “original create amount” matches.
- Claim reduces `allocation` and `total_allocated` by the payout,
  not only `VaultHolding` balance and `accrued`.
- System clock replaces the mock timestamp source.
  The RFC currently states “This MVP uses a mock timestamp source
  until a LEZ-native timestamp mechanism is finalized.”
  Replace with the real mechanism:
  client selects one of `CLOCK_01`, `CLOCK_10`, `CLOCK_50`;
  guest validates the account id and reads the 16-byte Borsh payload
  (`block_id: u64`, `timestamp: u64`);
  clock granularity tradeoffs (accrual precision vs observability)
  added to Security and Privacy Considerations.
- `CloseStream` idempotency and authorization.
  The RFC currently has a placeholder:
  “The final text should define who can close and idempotency behavior.”
  Replace with: either vault owner or stream provider may close;
  attempting to close an already-CLOSED stream fails
  (double-close is not idempotent — it errors).
- `SyncStream` instruction removed.
  Not present on the public instruction surface.
  Providers and other interested parties compute effective stream state
  off-chain by reading `StreamConfig` and a clock account and calling
  `at_time(now)` locally — no write transaction required.
  Remove any draft RFC text that described or proposed `SyncStream`.

Additional seeds from steps 6, 7, and 10:

- explicit `privacy_tier` (`Public`, `PseudonymousFunder`) and immutability for life
- user-facing privacy contract:
  pseudonymous `VaultConfig.owner` (typically private-account id),
  pre-shield funding hygiene,
  PP-only operation for `PseudonymousFunder` vaults on our host or wallet
- guest vs host vs consensus enforcement:
  inner program does not receive `visibility_mask`;
  PP-only policy is host or wallet enforced unless a future generic NSSA hook exists
- honest caveats:
  public `vault_config` row still exposes `owner` bytes;
  public shielding hop links funder to private persona;
  LEZ proofs do not assert “never funded from public key Alice”
- selective confidentiality language for PP on `Public`-tier vaults

Promotion and polish:

- Apply all seeds as a single batch diff against
  `rfc-index/docs/ift-ts/raw/payment-streams.md`.
  Cover: account model and PDA derivation, instruction definitions,
  validation and invariant rules, time source and accrual behavior,
  execution mode notes.
- Finalize Security and Privacy Considerations and References.
- Review consistency across implemented behavior,
  `design.md` (or its successor reviewer doc from step 10),
  and RFC text.

## Transition to private execution

This section ties the payment-streams RFC privacy posture
to the present SPEL guest and NSSA PP model.
It supersedes the earlier step 5 privacy-evolution bullets.

### What one code path means in practice

The phrase
write guest logic once and execute publicly or privately
means business logic reuse,
not identical privacy or visibility outcomes.

Shared across public and private execution:

- accrual and balance math
- lifecycle state transitions
- authorization predicates over plaintext state inside the zkVM
  (`AccountWithMetadata`, `VaultConfig.owner`, and so on)

Potentially different between execution contexts:

- account visibility and witness material outside the inner program
- what linkage an external observer can infer from on-chain artifacts
- how post-state is represented in public versus encrypted rows

### NSSA split: inner program vs outer privacy circuit

On the pinned stack,
application program execution (including SPEL guests)
is proven with inputs written by `Program::write_inputs`:
program id, optional caller id, `pre_states`, instruction words.

The visibility mask is supplied only to the outer
privacy-preserving circuit input (`PrivacyPreservingCircuitInput`),
together with program outputs and private-account key material.

So the payment-streams guest cannot branch on the mask
or reliably infer “this run was submitted as PP versus public”
from its executor input envelope alone.

### Why PDAs and public rows still matter for linkage

In the current account model,
vault and stream identities are PDA-derived
from seeds and program id.
Observers can always rebuild vault ↔ stream structure
from public account ids and updates.

What we target for the MVP is narrower:

- break or avoid primary public funder key ↔ vault activity
  when the user follows the PseudonymousFunder-tier playbook below.

PP mixed visibility can hide some legs (for example payouts),
but does not erase a public `vault_config` row
that still carries plaintext `VaultConfig` bytes including `owner`.

### Chosen MVP direction (what we implement)

#### privacy_tier values

- `Public`:
  same as today for public-first workflows.
  PP remains allowed for selective confidentiality;
  it does not retroactively hide a public funder link.
- `PseudonymousFunder`:
  operational PP-only policy under our host or wallet,
  plus funding hygiene described below.
  Arbitrary third-party clients are out of scope
  unless consensus adds a generic enforcement hook later.

#### Pseudonymous controller

- Keep `VaultConfig.owner` as the on-chain authorization anchor.
- For unlinkability from a main public wallet,
  use a dedicated private-account `AccountId`
  (npk-derived in the LEZ sense) as `owner`,
  created and operated through wallet PP APIs,
  not through `spel` public signing for that identity.
- Authorization in the guest stays the same checks;
  under PP, private row authorization is handled by
  NSSA and wallet witness rules,
  while the guest still verifies `owner.account_id == VaultConfig.owner`.

#### Funding and bridging

- Pre-shield spendable balance before vault funding when the goal is
  “no public leg from a doxxed public account into this vault story.”
- A public transfer from Alice’s public account into a private persona
  still creates Alice → persona on-chain;
  subsequent PseudonymousFunder-tier vault ops may not add a new vault-specific edge
  from Alice, but they do not erase that first hop.
- Delay between shielding and vault use is operational hygiene only,
  not a cryptographic unlinker.

#### What LEZ proving does and does not assert

- PP proofs enforce authorization, conservation,
  and privacy-circuit rules for the chosen account rows.
- They do not by themselves assert
  “this `owner` id has never been funded from public key Alice.”
  That property is behavioral and architectural:
  pre-shield, PP-only policy on our stack,
  and avoiding public bridges afterward.

#### User-facing contract

- We offer pseudonymous vault control
  and conditional separation from a primary public funder
  when users follow pre-shielding and PP-only operation
  on our host or wallet.
- We do not claim full anonymity of all metadata,
  hidden `owner` on-chain while `vault_config` is public plaintext,
  or protection against other submitters without consensus rules.

### Longer-term privacy direction

If stronger unlinkability is required later,
the protocol may evolve toward commitment-native custody,
private or committed `vault_config` fields,
or platform-level account policy for PP-only rows.

That is out of scope for the current plan
and remains a separate design track.

#### Why commitment-native owner is deferred, not adopted now

A natural question is whether to replace plaintext `VaultConfig.owner`
with a commitment up front,
so that pseudonymity is cryptographic rather than behavioral.
The primitive itself is not what makes this hard:
LEZ npk derivation already gives per-vault unlinkable owner ids
from a master seed,
and the `PseudonymousFunder` tier above is built on that fact.
The blockers are in the account model surrounding the owner field,
not in how the owner bytes are produced.

Concretely:

- Authorization currently reduces to byte equality
  (`signer.account_id == VaultConfig.owner`),
  verifiable by the guest with no extra machinery.
  A commitment-based owner turns this into
  proof-of-preimage for `Commit(owner_id, r)`,
  which the inner program cannot check on its own.
  It must flow through NSSA's private-row witness mechanism,
  and the pinned NSSA does not expose
  a generic application-level proof-of-preimage hook—
  private-row auth only covers control of npk-derived account ids.
  Adding such a hook is a platform change
  that has to be coordinated with NSSA and SPEL,
  not a local guest edit.
- `VaultConfig` is a public PDA row
  whose non-owner fields
  (`total_allocated`, `next_stream_id`, version, privacy_tier)
  and every mutation timestamp are observable.
  Stream PDAs are seeded from `vault_config_pda`,
  so the vault ↔ stream graph is reconstructable regardless of owner.
  Hiding just the owner bytes while leaving the rest public
  delivers partial benefit for disproportionate cost:
  the useful stronger posture requires
  vault and stream state to live as commitments or notes end to end,
  not as public PDAs with one obfuscated field.
- Step 10 Phase 3 ships PP `create_stream` with the stream PDA as a vis-0 public row,
  confirming that fixed-seed PDAs work in mixed-visibility PP calls.
  That path is not blocked by PDA identity;
  the remaining gap is that `VaultConfig.owner` is stored as plaintext
  in the public `vault_config` row, so hiding the owner still requires
  commitment-native vault state, not just PP execution.
  That is the prerequisite for commitment-native vaults,
  and it runs ahead of the current platform surface.
- Surface area is large.
  A commitment-native redesign touches the account model,
  PDA derivation, authorization check,
  client-side state (the user has to persist `(owner_id, randomness)`
  to re-derive their own vault PDA),
  every test fixture and harness helper,
  the reviewer writeup planned in step 9,
  and the RFC's Account Model and Privacy Considerations.
  The current layered plan
  (steps 6–7 add the tier and policy,
  steps 11–12 promote to the RFC)
  depends on those pieces being stable.

The tradeoff therefore favors shipping the pseudonymous tier now
with an honestly documented boundary
(`PseudonymousFunder` + pre-shield funding, owner is a plaintext pseudonym,
vault ↔ stream graph stays public),
collecting reviewer feedback on that posture,
and opening a separate design track for commitment-native custody
once the NSSA/SPEL platform gaps are closed.
The alternative—folding commitment-native custody into the MVP—
either blocks progress on upstream coordination
or produces a half-private design
that has to be reworked when the platform catches up.

Clock accounts remain platform-defined time anchors.
Even strong privacy modes assume public time validity,
not a hidden global clock.

## Cross-cutting Deliverables

Apply alongside the steps above, not as a dedicated phase.

- CI job that builds `lez_payment_streams-methods`
  before running `cargo test`
  so program tests never use a stale ELF.
- Documented `RISC0_DEV_MODE` policy for CI versus local runs.
- Decision whether to tighten `assert_execution_failed_with_code`
  to typed errors once the host exposes them.

## Out of Scope

- Off-chain protocol
  VaultProof, StreamProof, eligibility proofs, service messaging.
- Integration with a running sequencer for end-to-end demo.
- Multiple tokens per vault (or per vault holding).
- Future token extension path is out of scope.
- Protocol extensions
  auto-pause, delivery receipts, activation fee, auto-claim.

## Potential Future Refactoring: Merge PP Tests into Per-Instruction Files

Currently all PP tests live in `shielded_execution.rs`.
That file is large and grows with each new PP test.
This section describes a potential refactoring to co-locate each PP test
with its corresponding transparent tests.

### Goal

Move each PP test into the same file as the transparent tests for the same instruction.
`shielded_execution.rs` would be deleted.
A new `pp_common.rs` module would hold shared PP infrastructure.

### Motivation

A reviewer reading `withdraw.rs` would see both public-mode and PP-mode tests for `withdraw`
in one place, making the full behavior of an instruction easier to survey.
The reason two test modes exist per instruction is not duplication:
public tests verify the instruction logic,
PP tests verify that the same instruction functions correctly when accounts are private.

### Proposed Structure

#### `pp_common.rs` (new module)

Holds everything shared across multiple instruction PP tests:

- `pub use` re-exports of nssa/nssa_core PP types
  (`execute_and_prove`, `ProgramWithDependencies`, `Message`, `WitnessSet`,
  `PrivacyPreservingTransaction`, `Program`, `V03State`,
  `Account`, `AccountId`, `AccountWithMetadata`,
  `Commitment`, `EncryptionScheme`, `EphemeralPublicKey`, `Scalar`, `ViewingPublicKey`,
  `MembershipProof`, `NullifierPublicKey`, `NullifierSecretKey`, `SharedSecretKey`, `BlockId`).
- Shared identity constants and key-derivation functions:
  `RECIPIENT_NSK`, `RECIPIENT_VSK`, `EPK_SCALAR`, `recipient_npk()`, `recipient_vpk()`,
  `OWNER_NSK`, `OWNER_VSK`, `owner_npk()`, `owner_vpk()`.
- Shared helpers:
  `account_meta()`,
  `vault_fixture_public_tier_funded_via_deposit()`,
  `vault_fixture_pseudonymous_funder_funded_via_native_transfer()`,
  `fund_private_account_via_pp_withdraw()`,
  `run_pp_withdraw_to_private_recipient()`,
  `load_payment_streams_with_auth_transfer()`.
- Shared type `PpWithdrawReceipt`.

Note: items from `test_helpers` are `pub(crate)` and cannot be `pub use`-d from `pp_common`.
Each instruction file must import those directly from `crate::test_helpers`.

#### Per-instruction files

Each file adds `use super::pp_common::*;` at the top
and appends its PP test(s) and any instruction-local PP constants at the bottom.

Local constant groups by destination:

- `withdraw.rs`:
  `test_withdraw_private_recipient_pp_transition_succeeds`,
  `test_pp_withdraw_private_recipient_pseudonymous_funded_vault_succeeds`,
  `test_pp_withdraw_private_owner_succeeds`
  (Phase 3 constants: `PP3_SIGNER_EPK_SCALAR`, `PP3_RECIPIENT_NSK/VSK/EPK_SCALAR`,
  `PP3_WITHDRAW_AMOUNT`, `PP3_OWNER_FUND_EPK_SCALAR`, `PP3_OWNER_FUND_AMOUNT`
  and the `pp_owner_setup` helper — shared with pause/resume/top_up/create_stream).
- `claim.rs`:
  `test_pp_claim_private_provider_succeeds`
  (Phase 1 constants and `PpClaimCloseSetup` / `pp_claim_close_setup` — shared with `close_stream.rs`).
- `close_stream.rs`:
  `test_pp_close_stream_private_provider_authority_succeeds`.
- `deposit.rs`:
  `test_pp_deposit_private_owner_succeeds`
  (Phase 2 constants: `OWNER_FUND_EPK_SCALAR`, `PP_DEPOSIT_EPK_SCALAR`,
  `PP_OWNER_FUND_AMOUNT`, `PP_DEPOSIT_AMOUNT`).
- `create_stream.rs`:
  `test_pp_create_stream_private_owner_succeeds`.
- `pause_stream.rs`:
  `test_pp_pause_stream_private_owner_succeeds`.
- `resume_stream.rs`:
  `test_pp_resume_stream_private_owner_succeeds`.
- `top_up.rs`:
  `test_pp_top_up_stream_private_owner_succeeds`.
- `initialize.rs`:
  `test_pp_initialize_vault_private_owner_succeeds`
  (Phase 4 constants: `PP4_FUND_EPK_SCALAR`, `PP4_INIT_EPK_SCALAR`, `PP4_OWNER_FUND_AMOUNT`).

Constants and helpers shared across pause, resume, top-up, create-stream, and withdraw
(`PpOwnerSetup`, `pp_owner_setup`, Phase 3 constants) could either live in `pp_common.rs`
or be defined in whichever file is read first and re-exported from there.
The former avoids tight coupling between instruction files.

#### `mod.rs`

Add `mod pp_common;`.
Remove `mod shielded_execution;` once all tests are migrated.
Update the module-level doc comment to note that each instruction file
contains both transparent and PP tests.

### Implementation Notes

- Proceed in stages, one instruction at a time.
  After each move, run:
  ```bash
  RISC0_DEV_MODE=1 cargo test -p lez_payment_streams_core --lib program_tests
  ```
  before proceeding to the next.
- Keep `mod shielded_execution;` in `mod.rs` until all tests are moved.
  Delete the file only at the end.
- Start with `initialize.rs` (Phase 4, self-contained, no shared state with other tests).
- Move `claim.rs` and `close_stream.rs` together (they share `PpClaimCloseSetup`).
- Move Phase 3 tests together (they share `PpOwnerSetup` and `pp_owner_setup`).
- `architecture.md` should be updated after the migration:
  revise the "Privacy-Preserving Tests" section to reflect the new layout.
