# Step 45 gate log

Complete. Index: [index.md](../index.md).
Packet: [step-45-dependencies-and-patches.md](step-45-dependencies-and-patches.md).

Agreed scope for this freeze: Phase 1 + Phase 2 soft-only; Phase 3 testnet
deferred to Step 48; no Tier C; force-push feature branches only after Phase 2.

## Pins (freeze)

| Layer | Value |
| --- | --- |
| Operator LEZ | `v0.2.4` / `47eba256479f6f785acbd138834340703cd03401` |
| Program-graph LEZ | `v0.2.0` / `a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a` (Cargo `tag = "v0.2.0"`) |
| Spel | `v0.6.0` / `0cb7e0980535af619482cf1c823f4d394b3ebd61` (C) |
| LEZ-module | `549cf1159f20fa0c3fe8e88a5ab71de68a5aa34b` |
| Helper LEZ rev | `47eba256479f6f785acbd138834340703cd03401` |
| Guest ImageID | `dea010d9cb75887e8350f3dbd45b0efb8517e822fa105bc3e7b9fa2c9a2908ba` |
| ELF size | 366868 |
| Delivery eligibility tip | `c101e31a` on `feat/payment-streams-store-eligibility` (D45.19; pre-step45 `155957ad`) |
| Delivery-module eligibility tip | `49eb6c0` on `feat/payment-streams-store-eligibility` (D45.20/D45.21; flake locked to `c101e31a`; pre-step45 `2296f87`) |
| payment-streams freeze branch | `feat/step-45-dependencies-and-patches` |

## O45.6 AT hexes

| Source | Hex |
| --- | --- |
| Program-graph FFI (`v0.2.0`) | `dcbbfebcd59399961ed9973b8307dc475fd4c5ca5779aacfe7588f7dbc3f4a71` |
| Operator `v0.2.4` | `fe96c4228babbe8bc578e3e25b884cacb07f8c86541f27ed676789875eef875a` |
| Live testnet | `fe96c4228babbe8bc578e3e25b884cacb07f8c86541f27ed676789875eef875a` |

## Phase 0 outcomes

| ID | Result | Notes |
| --- | --- | --- |
| O45.1 spel×graph | C | stock `v0.6.0` guest build green; `nssa_core` alias in guest+core; `tag=v0.2.0` unify; IDL via `spel generate-idl` |
| Wallet schema probe | dual-keys | `ps_ensure_wallet_config_split_compatible` keeps `sequencer_addr` + `sequencers[]`; seed uses graph WalletCore + direct getTransaction confirm |
| O45.3 rebase census | proceed → amended | Tip-of-master delivery rebase pulled nim-ffi / post–Jul-31 event ABI; Phase 2 blocked. Locked D45.19 (parent = upstream-module pin `f8b03659…`) + D45.20 (`interfaces/` + runtime module name). Q45.1–Q45.5 closed; D45.21/D45.22 locked in Phase 2. |
| O45.6 AT hexes | recorded | helper assert == operator/live `fe96c422…` |

## Verification rows

| Date (UTC) | Repo commit | Cell | Artifact | Result | ImageID | ELF size | RISC0_DEV_MODE | Pins | Clock | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-08-12 | `7d51ce3` | full-reset-localnet | `.scaffold/snapshots/funded` | pass | `dea010d9…` | 366868 | 1 | op `47eba256` / graph `v0.2.0` / spel `0cb7e098` / helper `47eba256` | n/a | AT hex exported `fe96c422…`; vault already funded skip; snapshot lez_pin matches operator |
| 2026-08-12 | `931a6a0` | module-local | `.scaffold/e2e/artifacts/module-e2e-20260812T172651.log` | pass | `dea010d9…` | 366868 | 1 | op/graph/spel/helper as Pins | n/a | soft; AT hex set |
| 2026-08-12 | `1ec1263` | module-local-provider-close | `.scaffold/e2e/artifacts/module-e2e-20260812T173450.log` | pass | `dea010d9…` | 366868 | 1 | as Pins | n/a | soft; CLOSE_ROLE=provider |
| 2026-08-12 | `1ec1263` | module-local-close-negatives | `.scaffold/e2e/artifacts/module-e2e-20260812T174317.log` | pass | `dea010d9…` | 366868 | 1 | as Pins | n/a | soft; MODULE_E2E_CLOSE_NEGATIVES=1 |
| 2026-08-12 | `d1b6698` | store-local-hermetic-pre-rebase | `.scaffold/e2e/artifacts/e2e-20260812T175924.log` | pass | `dea010d9…` | 366868 | 1 | old delivery pins; SKIP_LIBLOGOSDELIVERY_OVERLAY=1 | n/a | E2E_CLAIM_OPTIONAL=0; soft; missing-proof reject observed |
| 2026-08-12 | `ebc3d06`+local tips | store-local-hermetic-post-rebase | `.scaffold/e2e/artifacts/e2e-20260812T212907.log` | pass | `dea010d9…` | 366868 | 1 | delivery `c101e31a` / module `49eb6c0` (local roots); SKIP_LIBLOGOSDELIVERY_OVERLAY=1 | n/a | Phase 2; D45.21 Lp async; D45.22 no PS unload after seed; paid 200; missing-proof reject; claim green; unit-tests 53 pass |
