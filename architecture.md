# Architecture

This document orients a developer reviewing the codebase.
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
The current implementation is single-token (native balance only)
(see [Multi-Token Vaults](#multi-token-vaults)).

Because `VaultHolding` is a public account, any party can increase its balance via a direct
native transfer outside the `deposit` instruction.
The solvency invariants are unaffected: `unallocated` grows and streams are unchanged.
For `PseudonymousFunder`-tier vaults,
a transparent transfer creates a traceable on-chain link.
Addressing its privacy implications is wallet responsibility,
as described in the spec's Security and Privacy Considerations section.

`StreamConfig` carries per-stream parameters and lazy accrual state.
Vault identity is not stored as a field in `StreamConfig`;
the PDA derivation already encodes it by including the `VaultConfig` address as a seed.

Both vault accounts are required together on every vault-touching instruction.
Their version fields must match.

Field types and serialization details are in the source files;
the spec's "On-Chain Protocol" section covers the account model at a higher level.

## Codebase Layout

Core types and logic live under `lez_payment_streams_core/src/`:

- `stream_config.rs` — `StreamConfig`, `StreamState`, and the `at_time` accrual method
- `vault.rs` — `VaultConfig`, `VaultHolding`, and balance predicates
- `error_codes.rs` — `ErrorCode` enum
- `instruction.rs` — `Instruction` enum (wire payload for all instructions)
- `test_helpers.rs` — fixture builders, state-construction helpers, transaction builders

`lib.rs` is the shared types and pure-logic boundary.
Keep `VaultConfig`, `VaultHolding`, `StreamConfig`, error codes, and pure helpers here.
Guest runtime code and account I/O belong in the guest binary, not in `lib.rs`.

The guest binary is `methods/guest/src/bin/lez_payment_streams.rs`.
It contains all `#[instruction]` handlers and helper functions called by multiple handlers.

Tests live in `lez_payment_streams_core/src/program_tests/`,
one module per instruction.
Each module contains both transparent and PP tests for that instruction.
`common.rs` holds shared test builders.
`pp_common.rs` holds shared PP infrastructure: fixture builders, key helpers, and setup structs.
Three additional modules cover cross-cutting concerns:
`invariants.rs` for solvency invariant tests,
`serialization.rs` for account layout round-trip checks,
and `privacy_tier_policy.rs` for wallet-enforcement policy tests.

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

For tests, read the individual-instruction modules.
Each module's transparent tests come first, followed by its PP tests.
The PP tests build on the same fixture helpers as the transparent tests,
with PP-specific setup steps layered on top.
`pp_common.rs` provides the shared PP infrastructure referenced across modules.

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

Negative-case tests follow a `*_fails` suffix convention
(e.g., `pause_stream_fails_when_already_paused`).

## Privacy-Preserving Tests

PP tests live alongside the transparent tests in each instruction's module.
Every instruction has at least one PP test.
All PP test names contain `pp`.

Shared PP infrastructure lives in `pp_common.rs`:
fixture builders (`vault_fixture_public_tier_funded_via_deposit`,
`vault_fixture_pseudonymous_funder_funded_via_native_transfer`),
the `fund_private_account_via_pp_withdraw` helper,
key constants and derivation functions for recipient and owner identities,
and setup structs (`PpClaimCloseSetup`, `PpOwnerSetup`) with their builders.

The `pp_owner_setup` helper builds the shared starting state for owner-private tests:
a Public-tier vault is funded and PP-withdrawn to establish the owner's private commitment,
and a PseudonymousFunder vault (vault B) is force-inserted with a pre-funded holding account.

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

## Future Work

The items below are out of scope for the current implementation
but are natural next steps.

### Multi-Token Vaults

The current implementation is single-token (platform-native balance only).
The `VaultHolding` PDA derivation includes an `asset_tag` seed (`"native"`)
to reserve a path for future per-token vaults without breaking the current address space.
Adding token support requires a separate holding account per token type
and corresponding deposit, withdraw, and claim logic for each.
The main structural change is that `VaultConfig.total_allocated` would need to become
a per-token map rather than a single scalar, and `StreamConfig` would need a token field
to identify which holding backs each stream.

### Protocol Extensions

The spec defines the following optional extensions, none of which are implemented:

- Auto-Pause: streams automatically pause after a configurable duration,
  limiting loss if the user goes offline.
- Delivery Receipts: claims require user-signed receipts as proof of service delivery.
- Automatic Claim on Closure: an optional flag that triggers a claim when a stream is closed.
- Activation Fee: a fixed amount accrues immediately when a stream becomes active,
  discouraging abuse of the pause/resume mechanism.
