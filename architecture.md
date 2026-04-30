# Architecture

This document is for a reviewer who wants to understand the codebase quickly.
It explains where protocol semantics live,
how the implementation is split,
and which distinctions matter during review.

The protocol definition lives in the
[LIP-155 payment-streams spec](https://lip.logos.co/ift-ts/raw/payment-streams.html).
Build, run, and test instructions live in `README.md`.

## Review order

Read the codebase in this order:

1. The LIP-155 spec for protocol requirements.
2. `lez_payment_streams_core/src/stream_config.rs` for lazy accrual.
3. `lez_payment_streams_core/src/vault.rs` for vault accounting.
4. `methods/guest/src/bin/lez_payment_streams.rs` for instruction handlers.
5. `lez_payment_streams_core/src/program_tests/` for execution coverage.

Two ideas appear throughout the implementation.

First, stream instructions follow a fold-first rule.
Any instruction that touches a stream first computes
the stream state at the current clock time,
then applies its own transition.

Second, privacy tier is stored on-chain,
but shielded-only use of `VaultPrivacyTier::PseudonymousFunder`
is not enforced by the guest.
That restriction is wallet or harness policy.
The guest cannot tell whether execution was transparent or shielded.

## Account model

The program stores three account types:
`VaultConfig`, `VaultHolding`, and `StreamConfig`.

`VaultConfig` stores vault metadata
and the authorization anchor.
Its `owner` field is the identity checked by owner-gated instructions.

`VaultHolding` is a dedicated account
whose native balance is the vault treasury.
It stores only a version byte in its data.
Using a dedicated holding account makes
`VaultHolding.balance` equal to vault funds without ambiguity.

`StreamConfig` stores per-stream parameters
and lazy accrual state.
It does not store vault identity as a field.
The vault binding is already present in the stream PDA seeds.

Both vault accounts are required together
on every vault-touching instruction.
Their version fields must match.

The current implementation uses native balance only.
The `VaultHolding` PDA still includes the `"native"` seed
to preserve a clean extension path for future per-token vaults.

Because `VaultHolding` is a public account,
any party can transfer native funds into it directly
without calling `deposit`.
This does not break solvency.
It only increases unallocated liquidity.
For `PseudonymousFunder` vaults,
such a transfer creates a public link.
Handling that privacy consequence is a wallet concern,
not guest logic.

## Core semantics

The semantic center of the program is `StreamConfig::at_time`.
It computes accrual and auto-pauses depleted streams.

A reviewer should treat every stream instruction as:

1. load accounts and clock
2. fold stream state to `now`
3. apply the instruction-specific transition
4. update vault accounting if stream allocation changed

This pattern is used by `pause_stream`,
`resume_stream`,
`top_up_stream`,
`close_stream`,
and `claim`.

Vault accounting is centered on `VaultConfig.total_allocated`.
That value must stay aligned with the sum of all stream allocations.
It is updated through
`checked_total_allocated_after_add`
and `checked_total_allocated_after_release`.

The close and claim paths are easy to confuse.
They differ in an important way.

Closing a stream can be initiated by either the vault owner
or the stream provider.
It releases only the unaccrued remainder back to the vault.
Accrued funds may remain on the closed stream for later claim.

Claiming a stream is provider-specific.
It pays out accrued funds
and reduces both stream allocation
and vault `total_allocated`
by the payout amount.

## Code layout

Core types and pure logic live in `lez_payment_streams_core/src/`.

- `stream_config.rs` contains stream state,
  accrual logic,
  and stream-local transitions.
- `vault.rs` contains vault state
  and vault allocation bookkeeping helpers.
- `error_codes.rs` defines `ErrorCode`.
- `instruction.rs` defines the wire-level instruction enum.
- `test_helpers.rs` contains harness builders and transaction helpers.

`lib.rs` is the shared pure-logic boundary.
Shared account types,
error codes,
and pure helpers belong there.
Guest runtime code does not.

The guest binary is
`methods/guest/src/bin/lez_payment_streams.rs`.
It contains:

- small SPEL error helpers
- account parsing helpers
- validation helpers
- context-loading helpers
- account-write helpers
- instruction handlers in declaration order

Tests live in `lez_payment_streams_core/src/program_tests/`.
There is one module per instruction.
Transparent and privacy-preserving cases live side by side.
That mirrors the program model:
the business logic is shared,
while visibility and submission policy differ.

Cross-cutting test modules are:

- `common.rs` for shared transaction builders and fixtures
- `pp_common.rs` for shared privacy-preserving setup
- `invariants.rs` for vault solvency checks
- `serialization.rs` for account layout round trips
- `privacy_tier_policy.rs` for wallet or harness policy checks

## Guest helper structure

The guest has two stream-loading helpers.
They exist because authorization differs across instructions.

`load_owner_stream_context`
is used by `pause_stream`,
`resume_stream`,
and `top_up_stream`.
These instructions require the vault owner authorization.

`load_stream_context_with_explicit_owner`
is used by `close_stream`
and `claim`.
Those instructions still need the owner identity checked
against `VaultConfig.owner`,
but the owner does not necessarily authorize.
`close_stream` accepts either the owner
or the provider as the authority.
`claim` must be authorized by the provider.

The duplication between these helpers is intentional.
It keeps the two authorization paths obvious during review.

## Test structure

The test harness uses layered fixtures.
Each layer adds the accounts created by one more setup step.

`VaultFixture`
represents state after `initialize_vault`.

`DepositedVaultFixture`
adds one deposit and a clock account.

`DepositedVaultWithProviderFixture`
adds a provider account for claim-related flows.

Fixture builders live in `test_helpers.rs`
and `program_tests/common.rs`.
Builder names follow the fixture ladder:
`state_with_initialized_vault*`,
`state_deposited_with_clock*`,
and `state_deposited_with_clock_and_provider`.

Some tests need a stream
without exercising `create_stream`.
Those tests use a force-insert pattern.
They write a `StreamConfig` directly into harness state
and patch `VaultConfig`
so `next_stream_id` and `total_allocated` stay consistent.
This is used in several privacy-preserving pause,
resume,
and top-up tests.

Two clock helpers matter during review:

`force_clock_account_monotonic`
is the normal helper.
It asserts that the new clock value moves forward.

`force_clock_account_unchecked`
exists for negative tests,
especially time regression
or repeated timestamps.

Negative test names use the `*_fails` suffix.

## Privacy-preserving coverage

Every instruction has at least one privacy-preserving (PP) test.
PP tests live in the same instruction module
as the transparent tests.

`pp_common.rs` provides shared PP infrastructure.
It contains fixture builders,
recipient and owner identity helpers,
and setup structs used by multiple instruction modules.

The privacy-tier policy tests need special interpretation.
They do not test guest enforcement.
They test harness or wallet behavior
that refuses transparent transitions
for `PseudonymousFunder` vaults.

This distinction is important.
The guest stores privacy tier in `VaultConfig`,
but privacy-tier submission policy lives outside the guest.

## PP-specific notes

The PP circuit assigns `output_index`
in account order
for private slots.
If an instruction creates two private outputs,
the first private slot has index `0`
and the second has index `1`.

`deposit` is special.
It chains into `authenticated_transfer_program`
to move native balance.
A PP `deposit` therefore proves both programs together
through `ProgramWithDependencies`.

The owner commitment used for a PP `deposit`
must belong to `authenticated_transfer_program`.
A commitment created by a PP `withdraw`
from the payment-streams program
cannot be reused for PP `deposit`,
because it has the wrong program owner.

## Future work

The current implementation intentionally excludes
several natural extensions.

### Multi-token vaults

The present code supports native balance only.
A token-aware version would need:

- one holding account per token
- token-aware deposit, withdraw, and claim logic
- `VaultConfig.total_allocated` changed from a scalar
  to a per-token structure
- a token field on `StreamConfig`

### Optional protocol extensions

The spec defines several optional extensions
that are not implemented here:

- auto-pause
- delivery receipts
- automatic claim on closure
- activation fee
