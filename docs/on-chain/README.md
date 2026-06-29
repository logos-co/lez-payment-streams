# On-chain program

LIP-155 payment streams as a SPEL guest program and shared Rust crate
(`lez-payment-streams-core`). This pillar covers chain semantics and Rust tests, not logoscore
module verification (see [payment streams module](../payment-streams-module/) and
[verification matrix](../verification-matrix.md)).

## Specification

[LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html).
On-chain appendix and branch pins: [step 19](../plan/completed/step-19-lip155-onchain-spec.md),
[feature-branch-pins.md](../feature-branch-pins.md).

## Review guide

[architecture.md](architecture.md) — account model, fold-first stream logic, close vs claim,
program_tests layout.

## Code map

| Path | Role |
| --- | --- |
| `methods/guest/src/bin/lez_payment_streams.rs` | Guest instructions |
| `lez-payment-streams-core/src/` | Vault/stream types, policy, accrual |
| `lez-payment-streams-core/src/program_tests/` | In-process LEZ harness tests |
| `lez-payment-streams-ffi/` | C/FFI boundary for the Logos module |

## Verify (Rust)

```bash
cargo risczero build --manifest-path methods/guest/Cargo.toml
RISC0_DEV_MODE=1 cargo test -p lez-payment-streams-core --lib
```

Optional PP tests: `--features pp-program-tests` (see root [README.md](../../README.md)).

Local deploy: `make build`, `make deploy` with `LEE_WALLET_HOME_DIR` set (operator detail in
[module setup](../payment-streams-module/setup.md)).

## Related pillars

- [Payment streams module](../payment-streams-module/) — exposes chain I/O via LogosAPI
- [Store integration](../store-integration/) — eligibility on Store requests
