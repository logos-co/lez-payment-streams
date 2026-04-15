# Design Decisions

Implementation-level choices for the payment streams MVP on LEZ.
The spec (`rfc-index/docs/ift-ts/raw/payment-streams.md`)
defines behavioral semantics;
this document covers what the spec leaves open.

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

`InitializeVault` creates both VaultConfig and VaultHolding
atomically in a single instruction.

## PDA derivation

### VaultConfig

`[b"vault_config", owner, vault_id]`

`vault_id` is a user-chosen `u64`.
Duplicate `vault_id` values are rejected
by the `init` check on the PDA account.

### VaultHolding

`[b"vault_holding", vault_config_pda, asset_tag]`

Asset tag is `b"native"` for MVP.
This reserves a path for future single-token vaults.

### StreamConfig

`[b"stream_config", vault_config_pda, stream_id]`

`stream_id` is assigned from `VaultConfig.next_stream_id`,
incremented only on successful `CreateStream`.
Provider is stored as data, not encoded in seeds,
to avoid coupling derivation to external identity formats.

Stream account data does not repeat `vault_id`.
The vault is fixed by `vault_config_pda` in the seed.

## Data types

Provider identity uses `AccountId` (`[u8; 32]`),
which works for both public and private-owned accounts on LEZ.

Stream lifecycle state is an enum:
`ACTIVE = 0`, `PAUSED = 1`, `CLOSED = 2`
(Borsh encodes variants by ordinal).

Numeric field types:

- `rate`: `TokensPerSecond` (`u64`, tokens per second)
- `allocation`, `accrued`: `Balance` (`u128`)
- `accrued_as_of`: `Timestamp` (`u64`), lazy accrual anchor (see **Accrual behavior**)

All match LEZ-native types (`Balance`, `Timestamp`).

`Balance` uses the shared `nssa_core` definition for token quantities.
`TokensPerSecond` and chain timestamps use `u64`:
enough range for realistic rates and second-granularity time
without widening on-chain fields that do not need `u128`.
Accrual multiplies rate by elapsed seconds in `u128` (or `Balance`)
where the product can exceed `u64`,
so widening stays in accrual math
instead of storing an oversized `rate` on the account.

A placeholder timestamp source account
is passed as a read-only instruction account.
To be replaced by a LEZ-native clock once the platform supports it.

Wire layout for the MVP mock clock (`MockTimestamp` in the core crate):
`version` (`u8`, little-endian as a single byte)
followed by `timestamp` (`u64`, little-endian),
nine bytes total.
Tests may synthesize this with `MockTimestamp::to_bytes`
or mutate host-side account data between transactions.

On-chain parsing should use structural checks
and an explicit set of supported wire layouts per version
rather than rejecting all but `version == DEFAULT_VERSION`,
so newer clock layouts can coexist when the program still only needs
the same timestamp fields from the prefix.

## Accounting

VaultHolding stores no application fields beyond `version`.
Actual balance is the LEZ-native account balance.

VaultConfig stores `total_allocated` only.
Unallocated balance: `vault_holding.balance - total_allocated`.
That figure caps both how much you may withdraw without touching streams
and how much you may allocate when opening a stream.
Per-stream accrual stays in StreamConfig.

Multiple streams are allowed per `(vault, provider)`.
The spec does not restrict this on-chain.

### Fund flow

Deposit: owner moves native balance into VaultHolding.

Withdraw: owner moves unallocated funds from VaultHolding
to an explicit target address.
An explicit target supports key rotation and recovery.

Claim: provider receives accrued funds directly from VaultHolding.

### Deposit and withdraw semantics

`Deposit` and `Withdraw` reject `amount = 0`.

`Deposit` moves funds
from an explicit signer-funded source account.

`Deposit` does not modify `vault_config.total_allocated`.

Vault operations that read both vault accounts require
`VaultConfig.version == VaultHolding.version`.

Vault operations with `vault_id` also require
`VaultConfig.vault_id == vault_id`
as defense in depth.

### Balance conservation

Every mutating instruction must preserve:

- `vault_holding.balance >= vault_config.total_allocated`
- `vault_config.total_allocated >= sum(stream.allocation - stream.accrued)`
  across all non-closed streams of the vault

## Authorization

Owner authorizes:
InitializeVault, Deposit, Withdraw,
CreateStream, SyncStream,
PauseStream, ResumeStream, TopUpStream.

CloseStream: either owner or provider.
The handler checks the signer against
`VaultConfig.owner` and `StreamConfig.provider`.

Claim: provider only.

## Accrual behavior

Lazy accrual on mutating stream instructions only
(pause, resume, top-up, close, claim).
Off-chain queries compute current accrual client-side.
When accrued reaches allocation,
the lazy update transitions the stream to PAUSED.

Checked arithmetic for balance mutations
(deposit, withdraw, allocation).
Saturating arithmetic for time-based accrual capping.

Clock versus stored stream time:

`accrued_as_of` on `StreamConfig` is the chain time through which lazy accrual has been folded into `accrued`
(when the cap is not reached, it is advanced to `now` on sync; when depleted from below during the interval, it is the depletion instant and may be before `now`; see `StreamConfig::at_time` in the core crate).
If `now` equals `accrued_as_of`, `at_time` returns the stream unchanged (no elapsed accrual interval).

- If `now` from the clock account is strictly before `accrued_as_of`
  on the stream, the instruction fails.
  This is treated as an error, not as zero accrual.

Implementation shape:

- Lazy accrual is folded with `StreamConfig::at_time(t)` in `lez_payment_streams_core`
  (single source of truth for guest and tests; call site: `stream_config.at_time(t)`).
  `t` is the on-chain time read from the clock account in the guest.
  Guest handlers deserialize, call `at_time`, then apply transfers and serialize.
  They do not duplicate accrual math.

`at_time`-then-operate:

- Instructions that need correct stream state at `now` first compute
  `stream_config.at_time(now)` (updated `StreamConfig` with
  accrued, `accrued_as_of`, and `state`, including depletion).
  Lifecycle and authorization checks run on that snapshot.
  There is one `Paused` variant for both user-initiated pause
  and accrual-induced pause at the allocation cap.
  Returning to `Active` from `Paused` is never implied by the clock alone;
  it requires a later mutating instruction
  (pause, resume, top-up, close, and claim are specified in later plan steps).

Helper semantics for stored state:

- Time-based accrual runs only while the stored state is `Active`.
  For `Paused` or `Closed`, `at_time` does not increase
  accrued balance by elapsed time.

Depletion in one step:

- After applying accrual with saturating cap at `allocation`,
  if the stream is depleted, set `state` to `Paused` in the same output.
  Set `accrued_as_of` to the depletion instant
  `base_as_of + ceil(remained_to_accrue / rate)` in integer seconds,
  where `remained_to_accrue` is headroom at the prior snapshot
  (`allocation - base_accrued` before applying the interval).
  That instant may be **before** `now` when `now` is past it.
  If `remained_to_accrue` is zero, the depletion instant is `base_as_of`.
  If the cap is not reached, advance `accrued_as_of` to `now`.

`SyncStream` instruction:

- First-class mutating instruction (not test-only): vault **owner** signer,
  same vault account layout as `create_stream` (config, holding, stream PDA,
  owner, read-only mock clock).
  After deserializing vault and stream accounts, the guest checks vault
  invariants (version, `vault_id`, owner) then stream alignment with the vault:
  `StreamConfig.version` matches both vault accounts,
  instruction `stream_id` is strictly below `next_stream_id` and matches the
  stored `stream_id`, and `StreamConfig::validate_invariants` passes
  (same structural rules as before calling `at_time`).
  Then it applies `stream_config.at_time(now)` from the clock account and
  writes stream data back.
  Does not move balances or change allocation; use `claim`, `close`, etc.
  for those flows. Later lifecycle instructions also call `at_time` internally.

Testing:

- Exercise `at_time` with unit tests in the core crate,
  and guest-backed `program_tests` via `SyncStream` that persist updated stream data.

`force_mock_timestamp_account` in test helpers may set any timestamp
(including backwards) for negative tests; see its rustdoc. Prefer
`MockTimestamp::advance_by` when building monotonic scenarios.

Test harness hygiene for the mock clock
(monotonic-by-default helpers and escape hatches for negative tests)
is tracked as a separate tightening pass in `plan.md`,
not a blocker for core accrual behavior.

## Versioning

All account layouts include `version: u8` as the first field,
set to 1 for initial version.

PDA labels are plain (`b"vault_config"`, etc.).
Versioning lives in account data, not in labels.
Addresses stay stable across schema versions.

## Out of scope

No on-chain index for MVP.
Both parties know their stream ids
from the off-chain protocol exchange.
