# Design Decisions

Implementation-level choices for the payment streams MVP on LEZ.
The spec (`rfc-index/docs/ift-ts/raw/payment-streams.md`) defines behavioral semantics.
This document covers what the spec leaves open.

## Platform pins (NSSA and SPEL)

Host, guest, and examples use the same LEZ checkout: `nssa_core` / `nssa` (tests) from `logos-execution-zone` git tag `v0.2.0-rc1`, so there is a single `nssa_core` in the graph alongside SPEL.

`lez_payment_streams_core` enables `nssa_core`’s `host` feature so tests can use `EncryptionScheme::decrypt` and viewing-key types when asserting privacy-preserving outputs.

SPEL is pinned to git revision `3457c7431e9b5b88661ed87b53677511ef88d113` on `https://github.com/logos-co/spel.git` (includes `SpelOutput::execute` and the macro rewrite to `execute_with_claims` for `vec![account, …]` patterns).

## Duplicated helpers in the core crate

`lez_payment_streams_core/src/clock_wire.rs` repeats the three system clock `AccountId` constants and the Borsh `ClockAccountData` layout that live in LEZ `clock_core`.
The crate does not list `clock_core` as a dependency so we avoid extra Cargo edges (guest vs host workspaces, git pins, and patch rules) while keeping a single `nssa_core` revision.
If `clock_core` changes upstream, this file must be updated by hand until we switch to a direct dependency from the same LEZ `rev`/`tag` as `nssa_core` (see `plan.md` step 6).

`lez_payment_streams_core/src/test_pda.rs` (test-only) repeats the seed combination rules from `spel-framework-core::pda` (`seed_from_str`, single-seed vs hashed multi-seed), then uses `nssa_core` for `PdaSeed` and `AccountId` derivation.
We avoid a `spel-framework-core` dev-dependency on the core crate because it would introduce a second `nssa_core` (from SPEL’s own LEZ pin) and break type identity with the main dependency.
When the dependency graph allows exactly one `nssa_core`, tests should call SPEL’s helpers instead so host-derived PDAs cannot drift from the guest (see `plan.md` step 6 and step 3).

## Test fixtures (host `program_tests`)

In-process tests build layered fixtures instead of long tuple returns.

1. `VaultFixture` — after `initialize_vault` (from `state_with_initialized_vault*` in `test_helpers.rs`).
2. `DepositedVaultFixture` — embeds `VaultFixture` plus `clock_id` after one `deposit` and `force_clock_account` to `initial_ts` (`state_deposited_with_clock` in `program_tests/common.rs`).
3. `DepositedVaultWithProviderFixture` — embeds `DepositedVaultFixture` plus provider key and `AccountId` when genesis includes the provider at balance zero (`state_deposited_with_clock_and_provider`).

Use the field name `clock_id` for the system clock account passed on the instruction account list (not “mock clock”).

`claim_stream_prelude_synced_at_t1` runs `create_stream` then advances the clock to `t1` and `sync_stream`.
It assumes the same ladder as manual claim tests: `initialize_vault` at block 1 / `Nonce(0)`, `deposit` at block 2 / `Nonce(1)`, `create_stream` at block 3 / `Nonce(2)`, `sync_stream` at block 4 / `Nonce(3)`.

## Privacy-preserving execution (NSSA) and step 5 tests

Host coverage lives in `lez_payment_streams_core/src/program_tests/shielded_execution.rs`.

### NSSA transition rule and visibility

`ValidatedStateDiff::from_privacy_preserving_transaction` rejects a privacy-preserving message when both `new_commitments` and `new_nullifiers` are empty.
The privacy-preserving circuit only produces those for accounts with visibility `1` (authenticated private) or `2` (new private); visibility `0` is public.
Therefore an all-public visibility mask cannot reach a successful PP state transition, even if program execution is otherwise valid.

Visibility semantics follow LEZ `PrivacyPreservingCircuitInput` (`0` public, `1` / `2` private with commitments, nullifiers when spending, ciphertexts).
**Mixed visibility** (public PDAs and clock plus at least one private slot) is the workable default for this program under current NSSA rules.

### PDAs, clock, and npk identities

Vault and stream accounts are PDAs derived from `(program_id, seeds)`; the system clock accounts are platform-defined.
They appear as public (`0`) rows in PP execution.
Private rows use `AccountId::from(npk)` (nullifier public key) and note-style updates; that model does not replace PDA or clock identities without a deeper custody or protocol change.

### What “private” means in the current MVP

Per-slot shielding (for example a withdraw payout encoded as a new private commitment) does not hide that the payment-stream program ran, which vault PDAs were touched, or stream parameters stored on PDAs.

### Public versus shielded parity

The guest’s balance and accrual rules are the same whether transitions are applied via `transition_from_public_transaction` or `transition_from_privacy_preserving_transaction`.
Shielded tests assert the same vault holding and `VaultConfig` invariants as the public `withdraw` ladder for the covered flow.

### Timestamp and clock in PP flows

PP `Message` values carry `timestamp_validity_window` (and block window) from program outputs; `transition_from_privacy_preserving_transaction` must receive a `(block_id, timestamp)` pair that satisfies those windows.
Tests reuse the same system clock ids and `TEST_PUBLIC_TX_TIMESTAMP` convention as public `program_tests` unless a case intentionally exercises bounds.

### Guest rebuild policy

After changes to the guest or to wire types shared with the guest, rebuild the methods crate before relying on `program_tests` (for example `cargo build -p lez_payment_streams-methods`).

### `withdraw` and PP claim metadata

The LEZ privacy-preserving circuit requires that default-owned accounts which change during execution carry an ownership **claim** in the program output so the circuit can set `program_owner` to the executing program before checking “modified but not claimed” invariants.

For `withdraw`, when the payout recipient’s pre-state is `Account::default()`, the guest now returns that row with `AutoClaim::Claimed(Claim::Authorized)` via `SpelOutput::execute_with_claims` so PP execution matches NSSA’s stricter path while public withdrawals to existing default genesis accounts remain unchanged in observable balances.

### Deposit and PP (not covered end-to-end)

`deposit` lists only three top-level accounts (vault config, vault holding, owner) and chains `authenticated_transfer_program` internally.
There is no extra account slot to attach a visibility-`2` row for NSSA’s non-empty commitment or nullifier rule without changing the instruction surface or protocol, so step 5 does not add a full PP `deposit` transition test.
A future design could add an explicit shielding leg or extend the account list if product requirements demand deposit through the PP circuit.

For [`VaultPrivacyTier::PseudonymousFunder`] vaults, tests also treat public `Deposit` as disallowed at the harness (see `test_helpers::transition_public_payment_streams_tx_respecting_privacy_tier`).
PP `withdraw` coverage for that tier uses `transfer_native_balance_for_tests` to move native balance into `VaultHolding` without a public `Deposit` transaction, so the ladder can still reach a funded PP `withdraw` while exercising the same guest rules as the public-path `withdraw` test.

### PP `create_stream` and `sync_stream` with PDA stream accounts

The privacy-preserving circuit ties private visibility rows to nullifier-derived identities.
`CreateStream` initializes the stream PDA with a fixed seed-derived [`AccountId`], which does not match an `npk`-only private row in the current NSSA layout, so this repository does not ship an `execute_and_prove` `create_stream` or `sync_stream` case yet.
Step 6 instead adds harness policy tests that refuse public `create_stream` and public `sync_stream` when the vault is marked [`VaultPrivacyTier::PseudonymousFunder`] (or after patching the stored tier), plus a PP `withdraw` path on such vaults.

## Account types and relationships

A single LEZ program manages three account types:

- VaultConfig:
  authority, stream counter, and `total_allocated` aggregate.
- VaultHolding:
  holds vault funds as LEZ-native account balance.
- StreamConfig:
  per-stream parameters, accrual state, and lifecycle status.

Each vault has exactly one config and one holding.
Each vault may have multiple streams.
Each stream belongs to exactly one vault.

`InitializeVault` creates both VaultConfig and VaultHolding atomically in a single instruction.
It also sets the vault’s [`VaultPrivacyTier`] (see below).

## PDA derivation

### VaultConfig

`[b"vault_config", owner, vault_id]`

`vault_id` is a user-chosen `u64`.
Duplicate `vault_id` values are rejected by the `init` check on the PDA account.

### VaultHolding

`[b"vault_holding", vault_config_pda, asset_tag]`

Asset tag is `b"native"` for MVP.
This reserves a path for future single-token vaults.

### StreamConfig

`[b"stream_config", vault_config_pda, stream_id]`

`stream_id` is assigned from `VaultConfig.next_stream_id`, incremented only on successful `CreateStream`.
Provider is stored as data, not encoded in seeds, to avoid coupling derivation to external identity formats.

Stream account data does not repeat `vault_id`.
The vault is fixed by `vault_config_pda` in the seed.

## Data types

### Vault privacy tier

[`VaultPrivacyTier`] is stored on [`VaultConfig`] and is chosen at `InitializeVault` time only.

Wire values on `InitializeVault` are raw bytes: `0` means [`VaultPrivacyTier::Public`], `1` means [`VaultPrivacyTier::PseudonymousFunder`].
Any other byte fails when the instruction payload is deserialized (before execution), using the same rules as [`VaultPrivacyTier`]'s serde decode.
The numeric code `ERR_INVALID_PRIVACY_TIER` (6027) remains reserved for compatibility but is not returned by the current guest for this path.

On [`VaultConfig`] account data, the tier is the trailing byte after `version`, `owner`, `vault_id`, `next_stream_id`, and `total_allocated` (all little-endian where applicable, same as before this field was added).
No instruction after `InitializeVault` mutates `privacy_tier`; the guest treats it as informational when deciding execution mode (it does not switch between public and PP execution based on this field).

Wallets or hosts that want strict privacy for pseudonymous-funder vaults should refuse ordinary public transitions that touch those vaults; the core test harness exposes helpers such as `assert_public_payment_streams_instruction_allowed` and `transition_public_payment_streams_tx_respecting_privacy_tier` for that policy.

Provider identity uses `AccountId` (`[u8; 32]`), which works for both public and private-owned accounts on LEZ.

Stream lifecycle state is an enum: `ACTIVE = 0`, `PAUSED = 1`, `CLOSED = 2` (Borsh encodes variants by ordinal).

Numeric field types:

- `rate`: `TokensPerSecond` (`u64`, tokens per second)
- `allocation`, `accrued`: `Balance` (`u128`)
- `accrued_as_of`: `Timestamp` (`u64`), lazy accrual anchor (see Accrual behavior)

All match LEZ-native types (`Balance`, `Timestamp`).

`Balance` uses the shared `nssa_core` definition for token quantities.
`TokensPerSecond` and chain timestamps use `u64`: enough range for realistic rates and second-granularity time without widening on-chain fields that do not need `u128`.
Accrual multiplies rate by elapsed seconds in `u128` (or `Balance`) where the product can exceed `u64`, so widening stays in accrual math instead of storing an oversized `rate` on the account.

Time for accrual comes from a read-only system clock account supplied by the client (one of the three LEZ clock accounts, for example `CLOCK_01` for second granularity).
Genesis seeds those clock account ids; the guest rejects any other account id with `ERR_INVALID_CLOCK_ACCOUNT` (6026).
The wire layout matches LEZ `clock_core::ClockAccountData`, Borsh-encoded: `block_id: u64` and `timestamp: u64` (little-endian on the wire as part of Borsh), 16 bytes total.
The program uses the `timestamp` field as the `Timestamp` for `StreamConfig::at_time` and related helpers; `block_id` is validated structurally but not interpreted for MVP stream math.

Three clock granularities exist on the platform (`CLOCK_01`, `CLOCK_10`, `CLOCK_50`); clients choose which clock account to pass.
Finer clocks imply more frequent public updates to that account when used as the read source; coarser clocks can reduce metadata churn at the cost of less precise accrual folds (relevant for privacy or private-proof settings where clock resolution interacts with what observers learn).

`ERR_INVALID_MOCK_TIMESTAMP` (6011) remains defined in the core crate for compatibility but is not emitted on current paths.

On-chain parsing should keep treating unknown or future clock payload extensions as parse failures for this program until a new layout is explicitly supported.

Guest clock loading uses two shared paths: owner-signed stream instructions (`SyncStream`, pause, resume, top-up) load via `load_vault_stream_and_clock` (signer is the vault owner). `CloseStream` and `Claim` use structural vault checks, bind the explicit vault owner account to `VaultConfig.owner`, then load stream state and the clock account via `load_vault_stream_and_clock_with_explicit_owner` (the transaction signer is authority or provider, not necessarily the owner account).

## Accounting

VaultHolding stores no application fields beyond `version`.
Actual balance is the LEZ-native account balance.

VaultConfig stores `total_allocated` only.
Unallocated balance: `vault_holding.balance - total_allocated`.
That figure caps both how much you may withdraw without touching streams and how much you may allocate when opening a stream.
Per-stream accrual stays in StreamConfig.

Multiple streams are allowed per `(vault, provider)`.
The spec does not restrict this on-chain.

### Fund flow

Deposit: owner moves native balance into VaultHolding.

Withdraw: owner moves unallocated funds from VaultHolding to an explicit target address.
An explicit target supports key rotation and recovery.

Claim: provider receives the stream’s current `accrued` balance from VaultHolding, then reduces `StreamConfig.allocation` by that payout and zeros `accrued`.
`VaultConfig.total_allocated` drops by the same amount.
See Balance conservation and invariants below.

### Deposit and withdraw semantics

`Deposit` and `Withdraw` reject `amount = 0`.

`Deposit` moves funds from an explicit signer-funded source account.

`Deposit` does not modify `vault_config.total_allocated`.

Vault operations that read both vault accounts require `VaultConfig.version == VaultHolding.version`.

Vault operations with `vault_id` also require `VaultConfig.vault_id == vault_id` as defense in depth.

### Balance conservation and invariants

All native funds for a vault sit in `VaultHolding`; its balance is `B`.

Definitions (bookkeeping; only `B` and explicit account fields are stored on-chain):

- Unallocated: `B - total_allocated`.
  Caps owner `withdraw` and liquidity for `create_stream` / `top_up_stream`.
- `unaccrued` (per stream): `allocation - accrued` (saturating at zero in implementation).
  By definition `allocation = accrued + unaccrued` for the stream’s current commitment.

Meaning of `allocation`: it is current vault commitment for that stream, not a historical maximum.
Two streams are economically equivalent when they share the same `allocation`, `accrued`, `rate`, and compatible `state`, independent of how much was claimed in the past.

Vault–stream bridge (single aggregate, no separate `total_claimable`):

- `vault_config.total_allocated` MUST equal the sum of `StreamConfig.allocation` over every stream for this vault (all lifecycle states; closed streams with only claimable residue contribute their residual `allocation` until drained).
- Enforce equality by applying the same delta to `total_allocated` whenever any stream’s `allocation` changes.
  Do not depend on rescanning all stream accounts in the guest unless tests or tooling do so offline.

Must hold after every mutating instruction:

- `vault_holding.balance >= vault_config.total_allocated` (solvency).
- `total_allocated` stays in sync with Σ `stream.allocation` as above.

`Claim`: pay `accrued`, then `allocation' = allocation - accrued_paid`, `accrued' = 0`, and decrease `total_allocated` by `accrued_paid`.
`Claim` with `accrued == 0` MUST fail (dedicated `ERR_*`).
“Claim does not change stream state” in the RFC still holds: `Active` / `Paused` / `Closed` are unchanged by `claim`; only balances and commitment fields update.

Idle `Paused` stream: `allocation == 0` and `accrued == 0` is allowed after a full payout—the stream is economically empty but the same PDA may be `top_up`’d again.
`Resume` remains invalid while `allocation - accrued == 0`.
`Closed` is terminal streaming (no further accrual or top-up); do not treat it as the idle zero-commitment case.

`CloseStream` (lifecycle step 6): `close_at_time` applies `at_time` then releases `unaccrued` to the owner: decrease `total_allocated` by `allocation - accrued`, set `state` to `Closed`, and set stream `allocation` to the post-close commitment (only provider-owed liquidity remains—typically `allocation == accrued` until subsequent `claim` zeros both).
Double-close MUST fail.

Optional cash audit (off-chain or future on-chain counters): cumulative `deposit − withdraw − claim_payouts = B` if you define those totals; not required for MVP if each instruction conserves `B` locally.

### Normative spec notes (`payment-streams.md`)

The RFC should be updated in a later promotion batch so normative text matches this implementation:

- Clocks: replace any mock-clock account wording with system clock accounts (client-selected `CLOCK_01` / `CLOCK_10` / `CLOCK_50`), 16-byte Borsh payload, and `ERR_INVALID_CLOCK_ACCOUNT` for wrong id or payload; note security and privacy tradeoffs of clock granularity versus observability of accrual timing.
- Allocation: define `allocation` as current commitment (`accrued + unaccrued`), updated when `claim` pays out (`allocation` decreases by the claimed amount).
- Claim: state that `claim` reduces `allocation` and `total_allocated` by the payout, not only `VaultHolding` balance and `accrued`.
- Resume / “remaining allocation”: align wording with `unaccrued` (`allocation - accrued`): resume requires positive `unaccrued` after `at_time` (equivalently `accrued < allocation` when both are folded).
- Equivalence: streams match when `allocation`, `accrued`, `rate`, and relevant `state` match, not when “original create amount” matches.

Naming in code: `StreamConfig::unaccrued()` is `allocation - accrued`; resume fails with `ERR_RESUME_ZERO_UNACCRUED` when it is zero.

## Authorization

Owner authorizes: InitializeVault, Deposit, Withdraw, CreateStream, SyncStream, PauseStream, ResumeStream, TopUpStream.

CloseStream: either owner or provider.
The handler checks the signer against `VaultConfig.owner` and `StreamConfig.provider`.

Claim: provider only.

## Pause and resume

`PauseStream` and `ResumeStream` use the same account layout as `SyncStream` (config, holding, stream, owner signer, read-only system clock account).
Handlers run `StreamConfig::at_time(now)` first, then apply the transition.

`PauseStream` requires the post-`at_time` state to be `Active`.
`ResumeStream` requires `Paused` and `accrued < allocation` (equivalently `unaccrued > 0` after `at_time`; see Balance conservation and invariants).
On successful resume, set `state` to `Active`, set `accrued_as_of` to `now`, and leave `accrued` unchanged so time while paused does not accrue later.
Invalid transitions fail with `ERR_*` (not no-ops).

## Top-up

`TopUpStream` uses the same account layout as `SyncStream` / `PauseStream` / `ResumeStream` (vault config, holding, stream PDA, owner signer, read-only system clock account).

Handlers run `StreamConfig::at_time(now)` first.

- Reject if post-`at_time` state is `CLOSED` (`ERR_STREAM_CLOSED`).
- Reject `vault_total_allocated_increase == 0` (`ERR_ZERO_TOP_UP_AMOUNT`).
- Reserve liquidity the same way as `CreateStream`: increase `StreamConfig.allocation` and `VaultConfig.total_allocated` by the same amount, capped by unallocated vault balance (`vault_holding.balance - total_allocated`).
  No native transfer; use `checked_total_allocated_after_add` in core.
  On stream `allocation` `checked_add` failure, `ERR_ARITHMETIC_OVERFLOW`.

If post-`at_time` state is `Paused`, after the allocation bump the handler calls the same resume transition as `ResumeStream` via `StreamConfig::resume_from_paused_at(now)`: `Active`, `accrued_as_of = now`, `accrued` unchanged (spec: top-up must yield `ACTIVE`; pause wall time must not count as accrual on the next fold).

If state is already `Active`, only allocation and `total_allocated` change.

## CloseStream

Account order (fixed): `VaultConfig` PDA (mut), `VaultHolding` (mut), stream PDA (mut), owner account (mut, not a signer), `authority` (signer), system clock account (read-only).

Vault checks use a small split: `validate_vault_structural` enforces matching versions, instruction `vault_id`, and related structural rules (`ERR_VERSION_MISMATCH`, `ERR_VAULT_ID_MISMATCH`).
The instruction passes the vault owner as an explicit account; the guest requires that account’s id to equal `VaultConfig.owner` (defense in depth alongside PDA binding).
`validate_vault_owner_signer` and `validate_vault_config` (structural then owner-as-signer) remain the pattern for instructions whose signer must be the vault owner.

Close authorization: the signer must be the vault owner or the stream provider; otherwise `ERR_CLOSE_UNAUTHORIZED`.

Handler shape: deserialize vault and stream, structural vault validation, stream alignment with the vault, then `StreamConfig::close_at_time(now, vault_config.total_allocated)` using the clock account timestamp as `now`.
`close_at_time` applies `StreamConfig::at_time(now)` internally, then releases unaccrued liquidity by lowering `total_allocated` via `checked_total_allocated_after_release`.
If `decrease_total_allocated_by` is zero, `total_allocated` is unchanged.
A second close attempt fails with `ERR_STREAM_CLOSED` from `close_at_time` (stream already closed after the accrual fold).

## Claim

Account order (fixed): `VaultConfig` PDA (mut), `VaultHolding` (mut), stream PDA (mut), vault owner account (mut, not a signer), provider account (mut, signer), system clock account (read-only).
The owner account matches `VaultConfig.owner` (same binding as `close_stream`); index 4 is the stream’s `provider` from `StreamConfig`, which receives the payout.

Only the provider may sign.
The guest checks `signer == StreamConfig.provider`; otherwise `ERR_CLAIM_UNAUTHORIZED` (6025).

Handler shape: structural vault validation (`validate_vault_structural`), deserialize stream, `validate_stream_config_for_vault`, then `StreamConfig::claim_at_time(now, vault_config.total_allocated)` using the clock account timestamp as `now`.
`claim_at_time` folds accrual with `at_time(now)` internally, then pays the full post-fold `accrued` amount, reduces `allocation` and `total_allocated` by that payout, and sets `accrued` to zero without changing `state` (`Active`, `Paused`, or `Closed` unchanged).

Native transfer: debit `VaultHolding.balance` by the payout and credit the provider account, using the same checked arithmetic pattern as `withdraw`.
Host validation requires the provider account to already exist in public state with a non-default `program_owner` before the balance credit (same pattern as funding a withdraw recipient in tests).
If post-fold `accrued == 0`, `claim_at_time` fails with `ERR_ZERO_CLAIM_AMOUNT` (6024).

## Accrual behavior

Lazy accrual on mutating stream instructions only (pause, resume, top-up, close, claim).
Off-chain queries compute current accrual client-side.
When accrued reaches allocation, the lazy update transitions the stream to PAUSED.

Checked arithmetic for balance mutations (deposit, withdraw, allocation).
Saturating arithmetic for time-based accrual capping.

Clock versus stored stream time:

`accrued_as_of` on `StreamConfig` is the chain time through which lazy accrual has been folded into `accrued` (when the cap is not reached, it is advanced to `now` on sync; when depleted from below during the interval, it is the depletion instant and may be before `now`; see `StreamConfig::at_time` in the core crate).
If `now` equals `accrued_as_of`, `at_time` returns the stream unchanged (no elapsed accrual interval).

- If `now` from the clock account is strictly before `accrued_as_of` on the stream, the instruction fails.
  This is treated as an error, not as zero accrual.

Implementation shape:

- Lazy accrual is folded with `StreamConfig::at_time(t)` in `lez_payment_streams_core` (single source of truth for guest and tests; call site: `stream_config.at_time(t)`).
  `t` is the on-chain time read from the clock account in the guest.
  Guest handlers deserialize, call `at_time`, then apply transfers and serialize.
  They do not duplicate accrual math.

`at_time`-then-operate:

- Instructions that need correct stream state at `now` first compute `stream_config.at_time(now)` (updated `StreamConfig` with accrued, `accrued_as_of`, and `state`, including depletion).
  Lifecycle and authorization checks run on that snapshot.
  There is one `Paused` variant for both user-initiated pause and accrual-induced pause at the allocation cap.
  Returning to `Active` from `Paused` is never implied by the clock alone; it requires a later mutating instruction (pause, resume, top-up, close, and claim are specified in later plan steps).

Helper semantics for stored state:

- Time-based accrual runs only while the stored state is `Active`.
  For `Paused` or `Closed`, `at_time` does not increase accrued balance by elapsed time.

Depletion in one step:

- After applying accrual with saturating cap at `allocation`, if the stream is depleted, set `state` to `Paused` in the same output.
  Set `accrued_as_of` to the depletion instant `base_as_of + ceil(remained_to_accrue / rate)` in integer seconds, where `remained_to_accrue` is headroom at the prior snapshot (`allocation - base_accrued` before applying the interval).
  That instant may be before `now` when `now` is past it.
  If `remained_to_accrue` is zero, the depletion instant is `base_as_of`.
  If the cap is not reached, advance `accrued_as_of` to `now`.

`SyncStream` instruction:

- First-class mutating instruction (not test-only): vault owner signer, same vault account layout as `create_stream` (config, holding, stream PDA, owner, read-only system clock account).
  After deserializing vault and stream accounts, the guest checks vault invariants (version, `vault_id`, owner) then stream alignment with the vault: `StreamConfig.version` matches both vault accounts, instruction `stream_id` is strictly below `next_stream_id` and matches the stored `stream_id`, and `StreamConfig::validate_invariants` passes (same structural rules as before calling `at_time`).
  Then it applies `stream_config.at_time(now)` from the clock account and writes stream data back.
  Does not move balances or change allocation; use `claim`, `close`, etc. for those flows.
  Later lifecycle instructions also call `at_time` internally.

Testing:

- Exercise `at_time` with unit tests in the core crate, and guest-backed `program_tests` via `SyncStream` that persist updated stream data.

Harnesses take clock ids from genesis (`harness_clock_01_and_provider_account_ids`), not from a synthetic clock seed keypair.

Clock helpers in `test_helpers` (plan step 4):

- `force_clock_account_unchecked` overwrites the clock payload with no ordering check.
  Use for time-regression and any case that must repeat the same `(timestamp, block_id)` pair.
- `force_clock_account_monotonic` asserts in debug builds that the new `(timestamp, block_id)` is strictly after the previous pair (lexicographic on `(timestamp, block_id)`), then calls unchecked.
  Happy-path tests should use this by default.
- `state_deposited_with_clock` uses monotonic for the post-deposit clock write.
  When `initial_ts == 0`, it uses `block_id == 1` for that first write so the payload is not `(0, 0)`; otherwise the next monotonic write with `block_id == 0` would not advance the pair.

Withdraw to a recipient that does not exist in public state fails host-side before program execution (see `program_tests::withdraw::test_withdraw_recipient_not_present_in_state_fails`), analogous to the claim provider account precondition.

## Spec audit (step 4)

Cross-walked `rfc-index/docs/ift-ts/raw/payment-streams.md` and this document against `methods/guest/src/bin/lez_payment_streams.rs` and `lez_payment_streams_core`.
The RFC file is not edited in this step; normative deltas are listed under RFC proposal candidates for `plan.md` step 8.

### Authorization matrix (wrong signer)

Each row is the account that must sign; tests assert `ERR_VAULT_OWNER_MISMATCH` (6016), `ERR_CLAIM_UNAUTHORIZED` (6025), `ERR_CLOSE_UNAUTHORIZED` (6023), or host witness validation as noted.

| Instruction | Signer | Negative coverage |
| --- | --- | --- |
| `initialize_vault` | owner (third account) | `initialize::test_initialize_vault_wrong_signer_witness_fails` (host Unauthorized) |
| `deposit` | owner | `deposit::test_deposit_owner_mismatch_fails` |
| `withdraw` | owner | `withdraw::test_withdraw_owner_mismatch_fails` |
| `create_stream` | owner | `create_stream::test_create_stream_owner_mismatch_fails` |
| `sync_stream` | owner | `accrual::test_sync_stream_owner_mismatch_fails` |
| `pause_stream` | owner | `pause_stream::test_pause_stream_owner_mismatch_fails` |
| `resume_stream` | owner | `resume_stream::test_resume_stream_owner_mismatch_fails` |
| `top_up_stream` | owner | `top_up::test_top_up_stream_owner_mismatch_fails` |
| `close_stream` | authority (owner or provider) | `close_stream::test_close_stream_unauthorized_fails` |
| `claim` | provider | `claim::test_claim_unauthorized_fails` |

### Arithmetic and boundary coverage (inventory)

Already covered before step 4 (guest or core tests): `next_stream_id` overflow (`create_stream`), `total_allocated` / allocation limits (`create_stream`, `top_up`, `deposit`), top-up allocation overflow (`top_up`), recipient balance overflow on withdraw, zero-amount guards, invalid clock account.

Added or highlighted in step 4:

- `accrual::test_sync_stream_with_timestamp_max_clock_succeeds` exercises `Timestamp::MAX` on the clock wire with `sync_stream`.
- Core `stream_config` and `vault` unit tests continue to cover saturating accrual and checked `total_allocated` helpers.

### Missing or weak tests (addressed in step 4)

- Solvency and conservation over multi-step flows: `program_tests::invariants` (`assert_vault_conservation_invariants` in `program_tests/common.rs`).
- `sync_stream` same-clock idempotence and depletion-via-sync: `accrual` tests `test_sync_stream_twice_same_clock_is_no_op_succeeds`, `test_sync_stream_depletion_via_at_time_paused_succeeds`.
- Withdraw recipient missing from state: `withdraw::test_withdraw_recipient_not_present_in_state_fails`.
- PDA parity documentation: `create_stream::test_derive_stream_pda_stable_succeeds` comments aligned with guest `pda = [...]` order.
- Resume and top-up owner mismatch: dedicated tests above.

### Behavior gaps

None found that required guest or production core changes during this audit; existing behavior matched the checked sections of the RFC and this design doc for the flows under test.

### RFC proposal candidates (step 8 seed)

- Clarify in the RFC that withdraw, like claim, assumes the payout recipient account is already present in the execution environment where the host validates accounts (or document the observable failure mode when it is absent).
- Optional: add a normative solvency invariant (`holding.balance >= total_allocated`, `total_allocated` vs sum of stream allocations) if the RFC should mirror implementation tests.
- Clock granularity and test harness conventions (system clock ids, monotonic time in tests) may warrant a short testing appendix in the RFC if not already covered under Security and Privacy Considerations.

## Versioning

All account layouts include `version: u8` as the first field, set to 1 for initial version.

PDA labels are plain (`b"vault_config"`, etc.).
Versioning lives in account data, not in labels.
Addresses stay stable across schema versions.

## Out of scope

No on-chain index for MVP.
Both parties know their stream ids from the off-chain protocol exchange.

After MVP behavior is stable, consider state cleanup instruction(s) to reclaim or compact stream accounts that only retain a settled closed footprint (for example zero allocation and accrued, or otherwise idle rows), subject to LEZ/NSSA account rules.
