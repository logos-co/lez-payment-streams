# Step 39 — testnet privacy gate execution log

Append-only record for native guest redeploy and required privacy gates on
public testnet.
SSOT:
[step-39-testnet-privacy-e2e.md](step-39-testnet-privacy-e2e.md).

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
smoke before testnet Phase 4 (D39.24). Missing NVIDIA/CUDA is not a Phase 4
stop — CPU prove is valid; gate on includable PPE smoke (D39.26).

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
| 2026-07-22 | 21333c2 | local module full privacy (real smoke 3) | module-e2e-20260722T191735.log | partial | create+close Validated under CLOCK_50; claim InvalidPrivacyPreservingProof (stale clock window after close); accrual ok |
| 2026-07-22 | d86146a | local module full privacy (real smoke 4) | module-e2e-20260722T200257.log | pass | D39.24; RISC0_DEV_MODE=0; CLOCK_50; claim vault_drop=400; all PPE Validated |
| 2026-07-22 | 8787afb | module full privacy testnet (4th, real) | module-e2e-20260722T205804.log | fail | RISC0_DEV_MODE=0; wallet shield+vault+create+close Validated; accrual fail — CLOCK_50 stuck at 32000 for 10min wait (need ~50 testnet blocks); claim skipped zero_accrued |
| 2026-07-22 | 9491316 | module full privacy testnet (5th, real) | module-e2e-20260722T215542.log | pass | D39.4/Phase 4 warm-up; RISC0_DEV_MODE=0; CLOCK_50; claim vault_drop=400; wallet shield |
| 2026-07-22 | 2d45928 | store full privacy testnet (1st, real) | e2e-20260722T234458.log | fail | AT-init fresh funder 5pLi93wd…; wallet init did not yield AT-owned; dual-daemon key handoff |
| 2026-07-22 | 867f64e | store full privacy testnet (2nd, real) | e2e-20260722T235004.log | fail | initializeVault FFI 7; provider stale save wiped private keys from shared storage |
| 2026-07-23 | b364840 | store full privacy testnet (2nd) | e2e-20260722T235004.log | fail | initializeVault FFI 7; shared storage wipe |
| 2026-07-23 | — | dual-wallet isolation smoke | verify-dual-wallet-isolation.sh | pass | split storage keeps A private; shared wipe reproduced |
| 2026-07-23 | 6cb8507 | store full privacy testnet (3rd, real) | e2e-20260723T001708.log | fail | split wallets OK; shields green; initializeVault FFI 7 — LEZ close no-op left stale wallet after CLI |
| 2026-07-23 | — | wallet-CLI handoff smoke | verify-wallet-cli-handoff.sh | pass | stop/restart keeps Private; close METHOD_FAILED |
| 2026-07-23 | — | Store host restart smoke | verify-store-host-restart.sh | pass | user stop/restart remounts delivery; provider peer unchanged |
| 2026-07-23 | (uncommitted) | store full privacy testnet (4th, real) | e2e-20260723T003716.log | fail | exclusive stop/restart wired; wallet shield timed out 600s |
| 2026-07-23 | — | PPE shield isolate | (manual wallet auth-transfer send) | fail | TX hash returned; `getTransaction` null; public pinata claim includes |
| 2026-07-23 | — | host GPU | `nvidia-smi` / `/dev/nvidia*` | blocked | kernel `7.0.0-28-generic` has no nvidia.ko (modules only under `6.17.0-40-generic`) |
| 2026-07-23 | — | D39.26 | (docs) | policy | GPU optional; gate on includable PPE smoke; CPU prove valid |
| 2026-07-23 | — | PPE shield isolate (CPU) | Ax7RuWwx… amount=1 | pass | RISC0_DEV_MODE unset; ~194s; getTransaction FOUND; GPU util 0 (CPU path) |
| 2026-07-23 | (uncommitted) | store full privacy testnet (5th, real) | e2e-20260723T103857.log | fail | owner shield to 8vSpcf… amount=550 timed out 1200s; then TimeoutExpired handler TypeError (bytes/str); fixed in run_local_e2e.py |
| 2026-07-23 | (uncommitted) | store full privacy testnet (6th, real) | e2e-20260723T110411.log | partial | fresh owner DaV7bT45…; shields+vault+create+Store query+close green (~41 min CPU); claim skipped zero_accrued after clock50_advance_before_accrual fail — not D39.13 DoD |
| 2026-07-23 | 3abcc0a | store full privacy testnet (7th, real) | e2e-20260723T121329.log | fail | CLOCK_50 Store wait raise landed; owner shield amount=550 to reused DaV7bT45… timed out 1200s — pin wallet returned TX `8d1b947d…` then hung on inclusion; `getTransaction` still null after fail (orphan PPE). No r0vm CPU after submit. |
| 2026-07-23 | — | prepare-testnet-privacy-seed | (seed storage) | pass | Sync tip + burn recycled private slots (DaV7/FqxTy/…); next create `T7qmBdn6…` |
| 2026-07-23 | — | PPE include smoke amount=1 | verify-ppe-shield-include.sh | pass | pin wallet; `Fhvzd6Cs…`; TX `deacd1b4…` included ~3 min; then re-burned that id on seed |
| 2026-07-23 | e9703a2 | harness | (scripts) | pass | Seed sync/burn; orphan fail-fast; recycled-id refuse; amount=1 smoke script |
| 2026-07-24 | e9703a2 | PPE include smoke amount=1 | verify-ppe-shield-include.sh | pass | TX `0f8d8704…` included ~4 min; recipient `3Z9fny…` (then burned) |
| 2026-07-24 | e9703a2+ | store full privacy testnet (8th) | e2e-20260724T131520.log | fail | AT-init on polluted fixture owner `DaV7…`; funder SSOT → testnet-module.json |
| 2026-07-24 | e9703a2+ | store full privacy testnet (9th) | e2e-20260724T132137.log | fail | CLOCK_50 advance before fundable drained allocation 400; removed advance from wait_stream_fundable |
| 2026-07-24 | e9703a2+ | store full privacy testnet (10th) | e2e-20260724T140412.log | fail | Store query green; same CLOCK_50 epoch at close → claim zero_accrued; tick before close |
| 2026-07-24 | e9703a2+ | store full privacy testnet (11th, real) | e2e-20260724T144726.log | pass | DoD; RISC0_DEV_MODE=0; E2E_CLAIM_OPTIONAL=0; SKIP_BUILD=1; ImageID Y=072a26cc…; wallet shield; owner `4AVejeor…` / provider `AJWwStrw…` / funder `DkT97…`; claim_balance vault_drop=400; ~106 min CPU |

## Agent summary (2026-07-24)

Required Phase 4 gates are green under real proving (`RISC0_DEV_MODE=0`) and
strict claim (`E2E_CLAIM_OPTIONAL=0`):

| Gate | Artifact |
| --- | --- |
| Phases 1–3 | green (do not reopen; D39.23) |
| Module full privacy testnet | `module-e2e-20260722T215542.log` |
| Store full privacy testnet | `e2e-20260724T144726.log` (vault_drop=400) |
| Phase 5 docs | E2E.md + verification-matrix + this summary |

Store harness notes (working tree on `e9703a2` tip, for human commit review):

- Default `CHAIN=testnet` before wallet-home resolve in seed/PPE scripts.
- Public funder from `fixtures/testnet-module.json` (privacy runs must not
  reuse mutated `testnet.json` private owner as funder).
- No CLOCK_50 advance before Store fundable/query; one tick before close so
  claim sees accrued.
- Seed prepare + recycled-id refuse for wiped-seed replays.

## Human close (D39.15)

Closed 2026-08-05. Reviewed gate-log greens and artifacts
`module-e2e-20260722T215542.log` (module full privacy) and
`e2e-20260724T144726.log` (Store full privacy; `vault_drop=400`).
Packet moved to `docs/plan/completed/`; raw TODO closed.
