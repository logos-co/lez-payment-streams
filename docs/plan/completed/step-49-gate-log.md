# Step 49 gate log

Packet: [step-49-native-token-spec-alignment.md](../upcoming/step-49-native-token-spec-alignment.md).
D49.11 ImageID-cut bar (`fast` + `local-public` + `testnet-public`) is recorded below.
Program identity: README [Public testnet guest program](../../../README.md#public-testnet-guest-program)
and [fixtures/testnet-module.json](../../../fixtures/testnet-module.json).

## Spec pin (D49.9)

| Artifact | Value |
| --- | --- |
| `logos-lips` | `master` / `32d7da4e2cd28e1b0c7e69e46505e883b224442c` |
| Contains | Step 40 `435a6f18`, Step 41 `f09f9e9e` |
| Path | `docs/anoncomms/raw/payment-streams.md` |

## Docker guest (2026-08-13)

`make build` on `feat/step-49-native-token-spec-alignment`.

| Field | Value |
| --- | --- |
| Path | `methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin` |
| ELF size | 373916 |
| ImageID (hex bytes) | `c30781ea9d7cc7b3be36f459ce9094644b984224d3d3119a644bb1b21ba2982a` |

`FULL_RESET=1 SKIP_BUILD=1 ./scripts/e2e.sh local prepare` rewrote
gitignored `fixtures/localnet.json` (`program_id_hex` + vault PDAs) and the
funded snapshot.

Methods-embed ELF used by `cargo test` remains a separate blob (372712 bytes,
ImageID `b06ef019…`).

## Fast verification (2026-08-13)

Repo commit `534704e`. `RISC0_DEV_MODE=1`.

| Cell | Command | Result | Notes |
| --- | --- | --- | --- |
| clippy | `RISC0_SKIP_BUILD=1 cargo clippy --workspace` | pass | style warnings only |
| tests | `RISC0_DEV_MODE=1 cargo test --workspace` | pass | core 152, FFI 33. Guest `UnsupportedTokenId` (6028) on non-native init |
| terminology | `make check-terminology` | pass | |

## Local-public dogfood (D49.11)

`RISC0_DEV_MODE=1`. Live AT hex `fe96c422…` exported into logoscore
(`scripts/module-e2e.sh`, `scripts/e2e.sh`).

| Date (UTC) | Cell | Artifact | Result | ImageID |
| --- | --- | --- | --- | --- |
| 2026-08-13 | module-local | `.scaffold/e2e/artifacts/module-e2e-20260813T152726.log` | pass | `c30781ea…` |
| 2026-08-13 | module-local-provider-close | `.scaffold/e2e/artifacts/module-e2e-20260813T153432.log` | pass | `c30781ea…` |
| 2026-08-13 | module-local-close-negatives | `.scaffold/e2e/artifacts/module-e2e-20260813T154141.log` | pass | `c30781ea…` |
| 2026-08-13 | store-local | `.scaffold/e2e/artifacts/e2e-20260813T161711.log` | pass | `c30781ea…` |

Store artifact includes `store_query_success`, `store_query_missing_proof`, and
`claim`. Paid query status 200.

## Testnet deploy (2026-08-13)

`make deploy-testnet` on `feat/step-49-native-token-spec-alignment`.

| Field | Value |
| --- | --- |
| ImageID (hex bytes) | `c30781ea9d7cc7b3be36f459ce9094644b984224d3d3119a644bb1b21ba2982a` |
| ELF size | 373916 |
| Deploy tx | `229dddd92e5184f4a44816ddda711b1eac51476248620a686807e091ffefba8b` |
| Block | 5873 |
| Freeze commit | `8a0e374a7e7171cd5b60ad20d46b9510b057dfe3` |

Fixture sweep: `24c1f48`.

## Testnet-public dogfood (D49.11)

`SKIP_BUILD=1`. Live AT hex `fe96c422…`. Default proving (`RISC0_DEV_MODE` unset).

| Date (UTC) | Cell | Artifact | Result | Notes |
| --- | --- | --- | --- | --- |
| 2026-08-13 | module-testnet | `.scaffold/e2e/artifacts/module-e2e-20260813T171430.log` | pass | vault 0, stream 0. `MODULE_E2E_SKIP_FUND=1` |
| 2026-08-13 | store-testnet | `.scaffold/e2e/artifacts/e2e-20260813T172600.log` | fail | `PROOF_INVALID` / session public key unknown. `rediscoverStreams` persist wiped the disk seed |
| 2026-08-13 | store-testnet | `.scaffold/e2e/artifacts/e2e-20260813T173906.log` | pass | after `12f93bd`. vault 0 stream 2. query status 200, missing-proof reject, close, claim |

Store pass artifact includes `store_query_success`, `store_query_missing_proof`, and `claim`.
