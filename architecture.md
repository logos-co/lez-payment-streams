# Architecture

This document orients a developer reading or reviewing the codebase.
It covers the rationale behind structural choices
and suggests a reading order.
The protocol semantics live in the spec
(`rfc-index/docs/ift-ts/raw/payment-streams.md`).
Operational setup — how to build, run, and test — is in the README.

## Account Model

The program stores state in three LEZ account types:
`VaultConfig`, `VaultHolding`, and `StreamConfig`.

`VaultConfig` carries vault metadata and the authorization anchor.
`VaultHolding` is a dedicated account whose LEZ-native balance is the vault's funds.
Giving vault funds a dedicated account makes `VaultHolding.balance` unambiguously equal
to the vault's treasury.
`VaultHolding` stores only a version byte in its application data.

`StreamConfig` carries per-stream parameters and lazy accrual state.
Vault identity is not stored as a field in `StreamConfig`.
It comes from the PDA seeds at derivation time.

Both vault accounts are required together on every vault-touching instruction.
Their version fields must match.

The spec's "On-Chain Data Model" section lists each field and its type.

## Codebase Layout

Core types and logic live under `lez_payment_streams_core/src/`:

- `stream_config.rs` — `StreamConfig`, `StreamState`, and the `at_time` accrual method
- `vault.rs` — `VaultConfig`, `VaultHolding`, and balance predicates
- `error_codes.rs` — `ErrorCode` enum
- `test_helpers.rs` — fixture builders, state-construction helpers, transaction builders

The guest binary is `methods/guest/src/bin/lez_payment_streams.rs`.
It contains all `#[instruction]` handlers and helper functions called by multiple handlers.

Tests live in `lez_payment_streams_core/src/program_tests/`,
one module per instruction.
`common.rs` holds shared test builders.
`shielded_execution.rs` holds privacy-preserving tests for all instructions.

## Suggested Reading Order

Start in `stream_config.rs`.
The `at_time` method is the core of the program.
Every instruction handler that touches a stream calls it first.
Understanding lazy accrual, depletion, and time regression here
is prerequisite for reading any instruction handler.

Next, read `vault.rs` for balance accounting:
`unallocated`, `checked_total_allocated_after_add`,
and `checked_total_allocated_after_release`.
These appear in every instruction that mutates the vault's `total_allocated`.

Then read the guest binary top to bottom.
The file opens with the shared parsing helpers,
then the instruction handlers in declaration order.

For tests, read the individual-instruction modules first,
then `shielded_execution.rs`.
The PP tests build on the same fixture helpers as the plain tests,
with PP-specific setup steps layered on top.

## Two Clock-Loading Paths

Five instructions need vault, stream, and clock data together.
Two helper functions provide that bundle.

`load_vault_stream_and_clock` handles `pause_stream`, `resume_stream`, and `top_up_stream`.
All three require the vault owner as the transaction signer,
so the helper checks ownership and signature together.

`load_vault_stream_and_clock_with_explicit_owner` handles `close_stream` and `claim`.
`close_stream` accepts either the vault owner or the stream provider as the closing authority.
`claim` is signed by the provider.
Neither instruction can require the owner's signature.
But both still need the owner's identity checked against `VaultConfig.owner`.
This helper separates structural vault validation from the owner equality check
so both verifications run without requiring the owner to sign.

## Test Fixture Pattern

Fixtures are layered structs.
Each level embeds the level below and adds the accounts its instruction created.

`VaultFixture` represents state after `initialize_vault`.
`DepositedVaultFixture` embeds `VaultFixture` and adds a clock account
after one `deposit` and a `force_clock_account` call.
`DepositedVaultWithProviderFixture` embeds `DepositedVaultFixture`
and adds a provider account at zero balance.

Builders for each level live in `test_helpers.rs` and `program_tests/common.rs`.
Builder names follow the pattern `state_with_initialized_vault*`,
`state_deposited_with_clock*`, and `state_deposited_with_clock_and_provider`.
Variants of the same level accept additional parameters such as a custom vault id
or a non-default privacy tier.

For tests that need a stream without exercising `create_stream` as a prerequisite,
the force-insert pattern writes a `StreamConfig` account directly into state
and calls `patch_vault_config` to update `next_stream_id` and `total_allocated` consistently.
PP pause, resume, and top-up tests use this pattern.

Two clock-forcing helpers exist in `test_helpers.rs`.
`force_clock_account_monotonic` asserts in debug builds
that the new `(timestamp, block_id)` pair is strictly after the prior.
`force_clock_account_unchecked` is for time-regression tests
and for cases that reuse the same timestamp.

## Privacy-Preserving Tests

PP tests live in `shielded_execution.rs`.
Every instruction has at least one PP test.
All PP test names contain `pp`.

The `pp_owner_setup` helper builds the shared starting state for owner-private tests:
a Public-tier vault is funded and PP-withdrawn to establish the owner's private commitment,
and a PseudonymousFunder vault is funded via `transfer_native_balance_for_tests`.

## output_index and Multi-Slot Decryption

The PP circuit assigns `output_index` values starting at 0,
incrementing for each private account slot (visibility 1 or 2) in account order.
A decryption call must pass the index matching that slot's position.

For instructions with two private slots —
such as `withdraw` with a private owner and a visibility-2 recipient —
the first private slot receives `output_index = 0`
and the second receives `output_index = 1`.

## PP Deposit and authenticated_transfer_program

`deposit` chains to `authenticated_transfer_program` to move native balance.
A PP `deposit` therefore uses `ProgramWithDependencies` wrapping both programs.
The PP proof covers both in one circuit.

The owner's private commitment used in a PP `deposit` must be
owned by `authenticated_transfer_program`.
A commitment created by a PP `withdraw` from a payment-streams vault
is owned by the payment-streams program.
That ownership works for all other PP instructions but not for `deposit`.
Tests that exercise PP `deposit` must source the owner commitment
from an `authenticated_transfer_program`-owned context.
