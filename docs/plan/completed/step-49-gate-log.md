# Step 49 gate log

Upcoming until ImageID-cut dogfood (`local-public` module + store, then
`testnet-public`) is recorded.
Packet: [step-49-native-token-spec-alignment.md](../upcoming/step-49-native-token-spec-alignment.md).

Operator override 2026-08-13: skip `testnet-public` and `deploy-testnet` in this
pass (too long). Packet stays under `upcoming/`.
Do not treat README / fixture ImageID `dea010d9…` as the new guest.

## Spec pin (D49.9)

| Artifact | Value |
| --- | --- |
| `logos-lips` | `master` / `32d7da4e2cd28e1b0c7e69e46505e883b224442c` |
| Contains | Step 40 `435a6f18`, Step 41 `f09f9e9e` |
| Path | `docs/anoncomms/raw/payment-streams.md` |

## Fast verification (2026-08-13)

Repo commit `534704e` on `feat/step-49-native-token-spec-alignment`.
`RISC0_DEV_MODE=1`.

| Cell | Command | Result | Notes |
| --- | --- | --- | --- |
| clippy | `RISC0_SKIP_BUILD=1 cargo clippy --workspace` | pass | style warnings only (pre-existing FFI docs / arg counts) |
| tests | `RISC0_DEV_MODE=1 cargo test --workspace` | pass | core 152, FFI 33. Guest `UnsupportedTokenId` (6028) on non-native init |
| terminology | `make check-terminology` | pass | |

Guest used by program tests (methods embed, not Docker deploy ELF):

| Field | Value |
| --- | --- |
| Path | `target/riscv-guest/lez_payment_streams-methods/lez_payment_streams-guest/riscv32im-risc0-zkvm-elf/release/lez_payment_streams.bin` |
| ELF size | 372712 |
| ImageID (hex bytes) | `b06ef0194867ac8e2c9b3e9194b6c6f5f1999be57ca3f59447727a8b72cbb557` |

Docker deploy ELF at `methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin`
was not rebuilt (366868 bytes, still `dea010d9…`).
`make build` / `make program-id` needed before local-public or testnet.

## ImageID-cut dogfood (D49.11)

| Handle | Command | Result |
| --- | --- | --- |
| `local-public` module | `MODE=module ./scripts/e2e.sh local run` | pending (`make build` + snapshot reset) |
| `local-public` store | `MODE=store ./scripts/e2e.sh local run` | pending |
| `testnet-public` module | `MODE=module ./scripts/e2e.sh testnet run` | skipped this pass |
| `testnet-public` store | `MODE=store ./scripts/e2e.sh testnet run` | skipped this pass |

README [Public testnet guest program](../../../README.md#public-testnet-guest-program)
and [fixtures/testnet.json](../../../fixtures/testnet.json) stay on the previous
deploy until `make deploy-testnet`.
