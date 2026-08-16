# Step 52 gate log

Packet: [step-52-wrap-up-verification.md](step-52-wrap-up-verification.md).
D52.1 coverage (`fast` + local public + local private stub + local real-prove
+ testnet public + testnet private) is recorded below.
Program identity: README [Public testnet guest program](../../../README.md#public-testnet-guest-program)
and [verify/fixtures/testnet-module.json](../../../verify/fixtures/testnet-module.json).
Living runner on this tree: `./verify/e2e.sh`.

## Identity

| Field | Value |
| --- | --- |
| Product ImageID (hex bytes) | `c30781ea9d7cc7b3be36f459ce9094644b984224d3d3119a644bb1b21ba2982a` |
| Product ELF size | 373916 |
| Product freeze commit | `8a0e374a7e7171cd5b60ad20d46b9510b057dfe3` |
| Product ELF pin | `.scaffold/program-bins/lez_payment_streams-c30781ea.bin` (gitignored; recovered 2026-08-15 by isolated `make build` at the freeze commit) |
| Local harness guest | `cdc9bfea4fdb6490a99929619bfb2c0eefd36a936668b1b3a22f684e66b44f0c` (live guest; local real-prove only) |
| logoscore | `pre-release-66c4194` (`66c4194ca6d3b556866cf22f5da912d235885dd8`) |
| LEZ pin | `47eba256479f6f785acbd138834340703cd03401` |
| Branch | `feat/step-53-repository-structure` |
| AT program | `fe96c4228babbe8bc578e3e25b884cacb07f8c86541f27ed676789875eef875a` |

Testnet cells used the pinned product ELF.
Local real-prove used the live local guest.
CPU prove (D39.26).

## Fast (2026-08-14)

| Cell | Command | Result | Notes |
| --- | --- | --- | --- |
| clippy | `RISC0_SKIP_BUILD=1 cargo clippy --workspace` | pass | `/tmp/step52-verify/clippy.log` |
| tests | `RISC0_DEV_MODE=1 cargo test --workspace` | pass | `/tmp/step52-verify/cargo-test.log` |
| terminology | `make check-terminology` | pass | also `make check` |
| Qt kit | `nix build ./module#checks.x86_64-linux.unit-tests -L` | pass | `/tmp/step52-verify/nix-tests.log` |

## Local public (2026-08-14)

`RISC0_DEV_MODE=1`. `SKIP_BUILD=1`.

| Cell | Artifact | Result |
| --- | --- | --- |
| module lifecycle | `.scaffold/e2e/artifacts/module-e2e-20260814T144631.log` | pass |
| provider-close | `.scaffold/e2e/artifacts/module-e2e-20260814T145407.log` | pass |
| close / create reject tokens | `.scaffold/e2e/artifacts/module-e2e-20260814T150159.log` | pass |
| Store eligibility | `.scaffold/e2e/artifacts/e2e-20260814T153211.log` | pass |

## Local private stub (2026-08-14)

`RISC0_DEV_MODE=1 OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`.

| Cell | Artifact | Result |
| --- | --- | --- |
| module | `.scaffold/e2e/artifacts/module-e2e-20260814T154533.log` | pass |
| Store | `.scaffold/e2e/artifacts/e2e-20260814T174058.log` | pass |

## Local real-prove (2026-08-15)

`RISC0_DEV_MODE=0 OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 E2E_CLAIM_OPTIONAL=0 LOCALNET_BLOCK_TIME=45s`.
Guest `cdc9bfea…`.

| Cell | Artifact | Result | Notes |
| --- | --- | --- | --- |
| module (1st 45s) | `.scaffold/e2e/artifacts/module-e2e-local-realprove-45s.log` | fail | claim inclusion 110s; CLOCK_50 `rem=0` after prove crossed the tick |
| module (retry) | `.scaffold/e2e/artifacts/module-e2e-local-realprove-45s-retry.log` | pass | claim `0e8a2566…`; vault_drop 400; wall ~3.1 h |

## Testnet public (2026-08-14)

`SKIP_BUILD=1`. Product ImageID `c30781ea…`.

| Cell | Artifact | Result |
| --- | --- | --- |
| module | `.scaffold/e2e/artifacts/module-e2e-20260814T180419.log` | pass |
| Store | `.scaffold/e2e/artifacts/e2e-20260814T190338.log` | pass |

## Testnet private (2026-08-15 to 2026-08-16)

`RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 SKIP_BUILD=1`.
Pinned ELF `c30781ea…`. Funded via `./verify/testnet/fund-testnet-accounts.sh`.

| Cell | Artifact | Result | Notes |
| --- | --- | --- | --- |
| module (ImageID skew) | `.scaffold/e2e/artifacts/module-e2e-20260814T212606.log` | fail | live guest `cdc9bfea…` vs testnet `c30781ea…`; FFI 7 at `initializeVault` |
| module (segfault) | `.scaffold/e2e/artifacts/module-e2e-testnet-private.log` | fail | logoscore segfault after AT-init; pre-shield hex resolve empty |
| module (retry) | `.scaffold/e2e/artifacts/module-e2e-testnet-private-retry.log` | pass | claim `34e48ed2…`; vault_drop 400; wall ~3.6 h |
| Store (RPC 20s) | `.scaffold/e2e/artifacts/e2e-testnet-store-private.log` | fail | `create_account_private` CLI RPC_FAILED at default 20s while seed clone flushed |
| Store (retry) | `.scaffold/e2e/artifacts/e2e-testnet-store-private-retry.log` | pass | query 200, missing-proof reject, close, claim `2547744f…`; vault_drop 400; wall ~3.05 h |

Store paid query `store_query_success` status 200; `store_query_missing_proof` ok.

## Harness notes

- logoscore `pre-release-66c4194` required so `LOGOSCORE_RPC_TIMEOUT_MS` reaches `core_service`.
  Pass that env on the daemon and on real-prove `logoscore call` only (D39.24).
- Local real-prove uses `LOCALNET_BLOCK_TIME=45s` so a CLOCK_50 epoch outlasts CPU prove.
- Claim prove rejects a stale CLOCK_50 `rem=0` and waits for a fresh window.
- Store real-prove `logoscore_cmd` now raises the CLI RPC budget for every `call`,
  not only `chainAction`, so `create_account_private` survives a large seed clone flush.

## Step 51

Forum wrap-up claims may be filled from this log.
Do not treat earlier fail artifacts as green rows.
