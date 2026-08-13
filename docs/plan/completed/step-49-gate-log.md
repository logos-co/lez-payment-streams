# Step 49 gate log

Upcoming until `testnet-public` (module + store) is recorded after
`make deploy-testnet`.
Packet: [step-49-native-token-spec-alignment.md](../upcoming/step-49-native-token-spec-alignment.md).

Local ImageID-cut dogfood is recorded below.
README [Public testnet guest program](../../../README.md#public-testnet-guest-program)
and [fixtures/testnet-module.json](../../../fixtures/testnet-module.json) still
cite the previous public deploy (`dea010d9…`) until testnet redeploy.

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

## Testnet (not run)

| Handle | Command | Result |
| --- | --- | --- |
| `testnet-public` module | `MODE=module ./scripts/e2e.sh testnet run` | pending (`make deploy-testnet` first) |
| `testnet-public` store | `MODE=store ./scripts/e2e.sh testnet run` | pending |
