# Step 50 gate log

Packet: [step-50-consistency-and-clarity.md](step-50-consistency-and-clarity.md).
Status: complete. Wrap-up matrix is [Step 52](../upcoming/step-52-wrap-up-verification.md)
(D52.1). No ImageID cut (D50.1).

Branch: `feat/step-50-consistency-and-clarity`.
Repo commit at close: `b6d8dba` plus this close.

## Workstreams

| Stream | Landed |
| --- | --- |
| C persist merge | `7919fe7` kit `mergePersistedDiskKeys`; Qt unit tests in `test_persist_merge.cpp` |
| B, D kit + clock + `getVaultStatus` | `fe26115` `payment_streams_kit`; C++ fold `>= 1e12`; `test_clock_fold.cpp`; unused `streamId` dropped from private C++ `getVaultStatus` |
| E inclusion wait | `30e8cea` `scripts/e2e/await_tx.py`; `chain_poll.sh` shells to it |
| A maintainer surfaces | `71cfb84` generated Makefile help / `.PHONY`; derive-and-fail `TESTNET_PROGRAM_ID_HEX` |
| F FFI ping | `ee7d85e` cbindgen header; `c_header_smoke.c` calls `payment_streams_ffi_decode_vault_config_bytes` |

## Fast verification (2026-08-13)

`RISC0_DEV_MODE=1`. No E2E (D50.7).

| Cell | Command | Result | Notes |
| --- | --- | --- | --- |
| tests | `RISC0_DEV_MODE=1 cargo test --workspace` | pass | core 152, FFI 33 |
| terminology | `make check-terminology` | pass on committed tree | uncommitted Step 21 packet still has one backtick-quoted prohibition line (`payer` / `payee` as field names not to use). Accepted as meta. |
| Qt kit tests | `logos-payment-streams-module` flake `tests` (`test_persist_merge.cpp`, `test_clock_fold.cpp`) | not run at close | sources and `tests/CMakeLists.txt` land with the kit commits |

## After this step

Step 52 runs D52.1 on the current ImageID (privacy + testnet).
Step 51 publishes the forum post from that gate log.
