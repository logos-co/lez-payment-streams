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
- timestamps: `u64`

All match LEZ-native types (`Balance`, `Timestamp`).

A placeholder timestamp source account
is passed as a read-only instruction account.
To be replaced by a LEZ-native mechanism when available.

## Accounting

VaultHolding stores no application fields beyond `version`.
Actual balance is the LEZ-native account balance.

VaultConfig stores `total_allocated` only.
Available balance: `vault_holding.balance - total_allocated`.
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
CreateStream, PauseStream, ResumeStream, TopUpStream.

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
