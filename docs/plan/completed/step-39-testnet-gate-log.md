# Step 39 — testnet privacy gate execution log

Append-only record for native guest redeploy and required privacy gates on
public testnet.
SSOT:
[step-39-testnet-privacy-e2e.md](../upcoming/step-39-testnet-privacy-e2e.md).

Locked highlights: real proving on public testnet privacy gates — `RISC0_DEV_MODE=0`
(D39.4 amended 2026-07-22); soft proving only for Phase 1; module full then
Store full (D39.7); one green (D39.8); agent tries deploy, flag+stop on
failure (D39.9); Docker ELF + ImageID from `make program-id` (D39.10); fixture
sync not full bootstrap (D39.11); strict claim on privacy gates —
`E2E_CLAIM_OPTIONAL=0` (D39.13); agent reports greens, human alone moves packet
to completed or writes off (D39.15); Y-equal no-op contingency (D39.16);
funding defaults then bump (D39.18). Soft stubs on public testnet are not DoD
(D39.19). Shield/dust for real prove uses `wallet auth-transfer send`
(D39.22); do not reopen Phase 1–3 greens (D39.23); local real-prove module
smoke before testnet Phase 4 (D39.24).

## Deploy

| Field | Value |
| --- | --- |
| Freeze commit | `c1f5b605705a8d8d2030d2c547ec7b9b9e77236a` |
| Deploy date | 2026-07-22 |
| ImageID Y (hex) | `072a26cc9865e95679012e53f2b1861b71f488b5e90da93611459afbc7adcfc7` |
| ELF size (bytes) | 362044 |
| ELF path | `methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin` |
| Operator | Agent (`make build` → `make deploy-testnet` + prefix checks) |

## Required commands

Public regression (default claim optional):

```bash
./scripts/fund-testnet-accounts.sh   # defaults OWNER_TARGET=550 PROVIDER_MIN=50
SKIP_BUILD=1 MODULE_E2E_SKIP_FUND=1 make verify-module-testnet
SKIP_BUILD=1 make verify-store-testnet
```

Phase 3b (after wallet-CLI shield harness; isolation only — not Phase 1 reopen):

```bash
SKIP_BUILD=1 RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 \
  MODE=module CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
  ./scripts/e2e.sh local run
```

Privacy testnet (real proving; strict claim). Order matters; fund before each;
requires Phase 3b green:

```bash
./scripts/fund-testnet-accounts.sh
SKIP_BUILD=1 MODULE_E2E_SKIP_FUND=1 RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 \
  MODE=module CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
  ./scripts/e2e.sh testnet run

./scripts/fund-testnet-accounts.sh
SKIP_BUILD=1 RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 \
  MODE=store CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
  ./scripts/e2e.sh testnet run
```

Funding-short once: `OWNER_TARGET=700 PROVIDER_MIN=100 ./scripts/fund-testnet-accounts.sh`.

## Not DoD (do not greenwash)

Soft proving against public testnet (`RISC0_DEV_MODE=1`) is rejected by the
sequencer (FakeReceipt verify). Local soft proving remains valid for Phase 1
only. Historical soft attempts below are fail rows, not close criteria.
Do not reopen Phase 1–3 green rows to re-check soft or public runs (D39.23).

## Runs

Artifact column: path under `.scaffold/e2e/artifacts/`.
Notes: ImageID Y, `RISC0_DEV_MODE`, `E2E_CLAIM_OPTIONAL`, `SKIP_BUILD`, shield path.

| Date | Commit | Profile | Artifact | Result | Notes |
| --- | --- | --- | --- | --- | --- |
| 2026-07-22 | c1f5b60 | local module full privacy | module-e2e-20260722T134520.log | pass | Phase 1; SKIP_BUILD=1; RISC0_DEV_MODE=1; claim_balance vault_drop=45 |
| 2026-07-22 | c1f5b60 | local Store full privacy | e2e-20260722T135206.log | pass | Phase 1; SKIP_BUILD=1; make verify-store-local-full-privacy |
| 2026-07-22 | c1f5b60 | local module public | module-e2e-20260722T140326.log | pass | Phase 1; SKIP_BUILD=1 |
| 2026-07-22 | c1f5b60 | local Store public | e2e-20260722T140901.log | pass | Phase 1; SKIP_BUILD=1 |
| 2026-07-22 | c1f5b60 | freeze build | (Docker ELF) | pass | ImageID Y=072a26cc…; ELF 362044 bytes; Y≠de17c0db → redeploy |
| 2026-07-22 | 61e8085 | deploy-testnet + fixture sync | (fixtures/scripts tip = Y) | pass | deploy-program exit 0; prefix checks 1–3 OK; operational de17c0db grep clean |
| 2026-07-22 | cf43886 | public module testnet | module-e2e-20260722T142615.log | pass | Phase 3; SKIP_BUILD=1; MODULE_E2E_SKIP_FUND=1; ImageID Y=072a26cc… |
| 2026-07-22 | cf43886 | public Store testnet (1st) | e2e-20260722T143205.log | fail | vault_ensure skipped (`chainaction_vault_ensure_local_only`); createStream account data missing |
| 2026-07-22 | cf43886+fix | public Store testnet | e2e-20260722T143847.log | pass | Phase 3; port-gap: enable chainAction vault ensure on testnet; SKIP_BUILD=1 |
| 2026-07-22 | de167d3 | module full privacy testnet (1st) | module-e2e-20260722T145028.log | fail | privacy accounts local-only; fixture public owner + privacy_tier=1 → resolve failed 7 |
| 2026-07-22 | de167d3+fix | module full privacy testnet (2nd, soft) | module-e2e-20260722T150845.log | fail | not DoD after D39.4 amend; soft stubs rejected by public sequencer |
| 2026-07-22 | — | D39.4 amend | (docs) | policy | Green for Phase 4 requires RISC0_DEV_MODE=0 |
| 2026-07-22 | — | D39.22–D39.24 | (docs) | policy | Wallet-CLI shield; no reopen Phase 1–3; local real-prove smoke before testnet Phase 4 |
| 2026-07-22 | 5674fcb | module full privacy testnet (3rd, real) | module-e2e-20260722T154921.log | fail | RISC0_DEV_MODE=0; logoscore transfer_shielded_owned RPC_FAILED (~20s IPC vs PPE wall clock) |
| 2026-07-22 | a443f66+ | local module full privacy (real smoke) | module-e2e-20260722T172701.log | partial | RISC0_DEV_MODE=0; wallet shield OK; vault_init+deposit real-prove OK; create_stream incl timeout; later FFI 99; core_service timeout fix in logos-logoscore-cli 66c4194 |
| 2026-07-22 | c905417+ | local module full privacy (real smoke 2) | module-e2e-20260722T183436.log | partial | RISC0_DEV_MODE=0; Clock50 module; create_stream Validated; close InvalidPrivacyPreservingProof (window crossed); accrual needs CLOCK_50 tick wait |

## Agent summary (after Phase 5)

Phases 1–3 green (do not reopen). Phase 3b wallet-CLI shield + logoscore IPC
timeout port-gaps landed. Root cause for create_stream failure on first real
smoke: PPE `public_pre_states` include CLOCK_01 (mislabeled clock_10); multi-minute
prove crossed a clock update → `InvalidPrivacyPreservingProof` (D39.25).

Second local real smoke (`module-e2e-20260722T183436.log`): vault/deposit/create
green under CLOCK_50; close still hit InvalidPrivacyPreservingProof when prove
crossed a CLOCK_50 tick. Accrual polls finished before CLOCK_50 advanced.

Next: CLOCK_50 window align before close/claim + accrual epoch wait; re-smoke
local module real-prove to full green (D39.24), then Phase 4 testnet.


## Human close (D39.15)

_pending — human only: review above, then move packet to completed or write off._
