# On-chain program

[LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html) payment streams as a SPEL guest
program and shared Rust crate (`lez-payment-streams-core`). Vault, stream, close, claim, and account
semantics are defined in the LIP; this doc covers review order and repository layout.

Maintainer LIP appendix work: [plan/completed/step-19-lip155-onchain-spec.md](../plan/completed/step-19-lip155-onchain-spec.md).
Sibling pins: [reference/feature-branch-pins.md](../reference/feature-branch-pins.md).

## Review order

1. `program/core/src/instruction.rs` — wire instruction enum
2. `program/core/src/stream_config.rs`, `vault.rs` — pure transitions
3. `program/methods/guest/src/bin/lez_payment_streams.rs` — handlers in declaration order
4. `program/core/src/program_tests/` — one module per instruction (transparent + PP)

## Code map

| Path | Role |
| --- | --- |
| `program/methods/guest/src/bin/lez_payment_streams.rs` | Guest instructions, SPEL helpers |
| `program/core/src/` | Vault/stream types, policy, accrual |
| `program/core/src/program_tests/` | In-process LEZ harness tests |
| `module/ffi/` | C/FFI boundary for the Logos module |

Core modules: `error_codes.rs`, `test_helpers.rs`. Guest holds parsing, validation, context load,
and writes — not shared pure logic in `lib.rs`.

## Guest helpers

Stream-loading helpers reflect authorization:

- `load_owner_stream_context` — `pause_stream`, `resume_stream`, `top_up_stream`,
  `close_stream_by_owner` (owner auth; five-slot close)
- `load_stream_context_with_owner_binding` — `close_stream_by_provider`, `claim`
  (owner binding without owner auth; provider signs)

## Tests

Layered fixtures: `VaultFixture` → `DepositedVaultFixture` → `DepositedVaultWithProviderFixture`.
Builders in `test_helpers.rs` and `program_tests/common.rs`. Force-insert stream pattern for PP
pause/resume/top-up without `create_stream`. Clock: `force_clock_account_monotonic` vs
`force_clock_account_unchecked` for negative tests (`*_fails` suffix).

Cross-cutting: `common.rs`, `pp_common.rs`, `invariants.rs`, `serialization.rs`,
`privacy_tier_policy.rs` (harness/wallet policy outside guest enforcement).

## Privacy-preserving notes

PP circuit `output_index` follows private slot order. PP `deposit` chains
`authenticated_transfer_program` via `ProgramWithDependencies`; owner commitment for PP deposit
must belong to authenticated transfer, not a PP withdraw from payment-streams.

`VaultConfig.token_id` is recorded at `initialize_vault`.
This guest accepts only the native identity (32 zero octets).
Vault holding PDA seed 3 is that 32-byte `token_id`.
Callers that omit `token_id` on `initializeVault` get native.

## Verify (Rust)

```bash
cargo risczero build --manifest-path program/methods/guest/Cargo.toml
RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --lib
```

Optional: `--features pp-program-tests`.

Local deploy (operators): `make build`, `make deploy` with `LEE_WALLET_HOME_DIR` —
see [payment-streams-module](../payment-streams-module/).

## Related

- [Payment streams module](../payment-streams-module/) — LogosAPI `chainAction`
- [Store eligibility](../reproduce/store-eligibility.md) — paid Store queries
- [Verification matrix](../reference/verification-matrix.md)
