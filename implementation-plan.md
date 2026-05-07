# Payment Streams on LEZ - Implementation Plan

This plan covers the SPEL-based LEZ implementation for payment streams.

## Steps

The starting point was a working prototype with core instruction handlers
(`initialize_vault`, `deposit`, `withdraw`, `create_stream`, `pause_stream`, `resume_stream`)
and basic tests, built incrementally before this plan.

### 1. Non-test cleanup

Mechanical cleanup: `cargo fmt`, `cargo clippy`,
extracted shared prologues in guest handlers,
added post-state constructor helpers.

### 2. LEZ upgrade, clock migration, and SPEL execute migration

Migrated from mock timestamp to system clock accounts
(`CLOCK_01`, `CLOCK_10`, `CLOCK_50`; 16-byte Borsh payload).
Migrated guest from `SpelOutput::states_only` / `with_chained_calls`
to `SpelOutput::execute`.
Bumped LEZ, NSSA, and SPEL dependencies together.

### 3. Test fixture extraction

Introduced `VaultFixture`, `DepositedVaultFixture`, `DepositedVaultWithProviderFixture`
layered structs replacing tuple destructuring.
Added scenario builders for common test setups.
Promoted shared constants to module level.

### 4. Spec audit and test hardening

Walked spec against code.
Added tests for: solvency invariants, arithmetic boundaries,
authorization matrix, PDA derivation determinism, clock harness hygiene.

### 5. Shielded execution tests

Added shielded-mode tests through `execute_and_prove` /
`transition_from_privacy_preserving_transaction`
for the initial withdraw flow.

### 6. Privacy tier and host policy

Added `privacy_tier` field (`Public`, `PseudonymousFunder`) to `VaultConfig`,
immutable at creation.
Host and wallet enforce shielded-only policy for `PseudonymousFunder` vaults;
guest stores the tier but cannot detect execution mode.

### 7. Adapter refactor and selective privacy rollout

Centralized PP fixture and harness helpers.
Kept guest handlers thin: parse, apply tier-aware state rules, serialize.
Confirmed public and shielded execution parity under the account model.

### 8. Remaining refactor

`clock_wire.rs` cleanup, `test_pda.rs` consolidation,
`spel_custom` helper, error code cleanup,
unused imports, final `cargo fmt` / `cargo clippy`.

### 9. Remove SyncStream public instruction

Removed `SyncStream` instruction and handler.
Migrated accrual tests to direct `StreamConfig::at_time` unit tests.
Clients compute effective stream state locally by reading `StreamConfig`
and a clock account.

### 10. Privacy-preserving vault operations

Extended shielded test coverage to all instructions
and all private roles (owner vis-1, provider vis-1, recipient vis-2).
Key finding: PP deposit requires `ProgramWithDependencies` with
`authenticated_transfer_program`
because `validate_execution` restricts balance decreases to the owning program.
Owner commitment for PP deposit must therefore be
`authenticated_transfer_program`-owned.

### 11. Documentation pass

Distributed `design.md` content to README, architecture.md, code comments, and spec.
Retired `design.md`.
Merged PP tests into per-instruction modules; added `pp_common.rs` for shared infrastructure.

### 12. RFC restructuring and polish

Restructured spec: renamed Implementation Considerations → On-Chain Protocol
(parallel to Off-Chain Protocol);
expanded Security and Privacy Considerations into six subsections covering
privacy goals, execution modes, unlinkability requirements, verifiability,
and provider privacy.
Applied terminology updates:
"shielded transactions" for the execution mode;
"authorized" covering both cryptographic signature and ZK proof of account control.
Applied Step 12 seeds:
`unallocated` and `allocation` definitions,
resume condition (`unaccrued balance is zero`),
claim semantics (reduces allocation and total_allocated).

## References

- `https://lip.logos.co/ift-ts/raw/payment-streams.html` — protocol semantics.
- `https://github.com/logos-blockchain/logos-execution-zone/pull/403` — system clock accounts.
- `https://github.com/logos-co/spel` — SPEL framework.
- `https://kindainsecurebot.github.io/lez-book/` — LEZ Development Guide.
