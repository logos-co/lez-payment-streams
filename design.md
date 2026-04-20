# Design Decisions

Implementation-level choices for the payment streams MVP on LEZ.
The spec (`rfc-index/docs/ift-ts/raw/payment-streams.md`) defines behavioral semantics.
This document covers what the spec leaves open.

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

A placeholder timestamp source account is passed as a read-only instruction account.
To be replaced by a LEZ-native clock once the platform supports it.

Wire layout for the MVP mock clock (`MockTimestamp` in the core crate): `version` (`u8`, little-endian as a single byte) followed by `timestamp` (`u64`, little-endian), nine bytes total.
Tests may synthesize this with `MockTimestamp::to_bytes` or mutate host-side account data between transactions.

On-chain parsing should use structural checks and an explicit set of supported wire layouts per version rather than rejecting all but `version == DEFAULT_VERSION`, so newer clock layouts can coexist when the program still only needs the same timestamp fields from the prefix.

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

`PauseStream` and `ResumeStream` use the same account layout as `SyncStream` (config, holding, stream, owner signer, read-only mock clock).
Handlers run `StreamConfig::at_time(now)` first, then apply the transition.

`PauseStream` requires the post-`at_time` state to be `Active`.
`ResumeStream` requires `Paused` and `accrued < allocation` (equivalently `unaccrued > 0` after `at_time`; see Balance conservation and invariants).
On successful resume, set `state` to `Active`, set `accrued_as_of` to `now`, and leave `accrued` unchanged so time while paused does not accrue later.
Invalid transitions fail with `ERR_*` (not no-ops).

## Top-up

`TopUpStream` uses the same account layout as `SyncStream` / `PauseStream` / `ResumeStream` (vault config, holding, stream PDA, owner signer, read-only mock clock).

Handlers run `StreamConfig::at_time(now)` first.

- Reject if post-`at_time` state is `CLOSED` (`ERR_STREAM_CLOSED`).
- Reject `vault_total_allocated_increase == 0` (`ERR_ZERO_TOP_UP_AMOUNT`).
- Reserve liquidity the same way as `CreateStream`: increase `StreamConfig.allocation` and `VaultConfig.total_allocated` by the same amount, capped by unallocated vault balance (`vault_holding.balance - total_allocated`).
  No native transfer; use `checked_total_allocated_after_add` in core.
  On stream `allocation` `checked_add` failure, `ERR_ARITHMETIC_OVERFLOW`.

If post-`at_time` state is `Paused`, after the allocation bump the handler calls the same resume transition as `ResumeStream` via `StreamConfig::resume_from_paused_at(now)`: `Active`, `accrued_as_of = now`, `accrued` unchanged (spec: top-up must yield `ACTIVE`; pause wall time must not count as accrual on the next fold).

If state is already `Active`, only allocation and `total_allocated` change.

## CloseStream (lifecycle step 6)

Account order (fixed): `VaultConfig` PDA (mut), `VaultHolding` (mut), stream PDA (mut), owner account (mut, not a signer), `authority` (signer), mock clock (read-only).

Vault checks use a small split: `validate_vault_structural` enforces matching versions, instruction `vault_id`, and related structural rules (`ERR_VERSION_MISMATCH`, `ERR_VAULT_ID_MISMATCH`).
The instruction passes the vault owner as an explicit account; the guest requires that account’s id to equal `VaultConfig.owner` (defense in depth alongside PDA binding).
`validate_vault_owner_signer` and `validate_vault_config` (structural then owner-as-signer) remain the pattern for instructions whose signer must be the vault owner.

Close authorization: the signer must be the vault owner or the stream provider; otherwise `ERR_CLOSE_UNAUTHORIZED`.

Handler shape: deserialize vault and stream, structural vault validation, stream alignment with the vault, then `StreamConfig::close_at_time(now, vault_config.total_allocated)` using the mock clock.
`close_at_time` applies `StreamConfig::at_time(now)` internally, then releases unaccrued liquidity by lowering `total_allocated` via `checked_total_allocated_after_release`.
If `decrease_total_allocated_by` is zero, `total_allocated` is unchanged.
A second close attempt fails with `ERR_STREAM_CLOSED` from `close_at_time` (stream already closed after the accrual fold).

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

- First-class mutating instruction (not test-only): vault owner signer, same vault account layout as `create_stream` (config, holding, stream PDA, owner, read-only mock clock).
  After deserializing vault and stream accounts, the guest checks vault invariants (version, `vault_id`, owner) then stream alignment with the vault: `StreamConfig.version` matches both vault accounts, instruction `stream_id` is strictly below `next_stream_id` and matches the stored `stream_id`, and `StreamConfig::validate_invariants` passes (same structural rules as before calling `at_time`).
  Then it applies `stream_config.at_time(now)` from the clock account and writes stream data back.
  Does not move balances or change allocation; use `claim`, `close`, etc. for those flows.
  Later lifecycle instructions also call `at_time` internally.

Testing:

- Exercise `at_time` with unit tests in the core crate, and guest-backed `program_tests` via `SyncStream` that persist updated stream data.

`force_mock_timestamp_account` in test helpers may set any timestamp (including backwards) for negative tests; see its rustdoc.
Prefer `MockTimestamp::advance_by` when building monotonic scenarios.

Test harness hygiene for the mock clock (monotonic-by-default helpers and escape hatches for negative tests) is tracked as a separate tightening pass in `plan.md`, not a blocker for core accrual behavior.

## Versioning

All account layouts include `version: u8` as the first field, set to 1 for initial version.

PDA labels are plain (`b"vault_config"`, etc.).
Versioning lives in account data, not in labels.
Addresses stay stable across schema versions.

## Out of scope

No on-chain index for MVP.
Both parties know their stream ids from the off-chain protocol exchange.

After MVP behavior is stable, consider state cleanup instruction(s) to reclaim or compact tombstone stream accounts (for example all-zero idle or fully settled rows), subject to LEZ/NSSA account rules.
