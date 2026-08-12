# Step 45 gate log

Executing. Index: [index.md](../index.md).
Packet: [step-45-dependencies-and-patches.md](../upcoming/step-45-dependencies-and-patches.md)
(moves to `completed/` when Done-when closes).

Agreed scope for this freeze: Phase 1 + Phase 2 soft-only; Phase 3 testnet
deferred to Step 48; no Tier C; force-push feature branches only after Phase 2.

## Pins (fill as recorded)

| Layer | Value |
| --- | --- |
| Operator LEZ | `v0.2.4` / `47eba256479f6f785acbd138834340703cd03401` |
| Program-graph LEZ | `v0.2.0` / `a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a` (Cargo `tag = "v0.2.0"`) |
| Spel | `v0.6.0` / `0cb7e0980535af619482cf1c823f4d394b3ebd61` (C) |
| LEZ-module | `549cf1159f20fa0c3fe8e88a5ab71de68a5aa34b` |
| Helper LEZ rev | `47eba256479f6f785acbd138834340703cd03401` |
| Guest ImageID | `dea010d9cb75887e8350f3dbd45b0efb8517e822fa105bc3e7b9fa2c9a2908ba` |
| ELF size | 366868 |
| Delivery eligibility tip | TBD |
| Delivery-module eligibility tip | TBD |

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
| O45.3 rebase census | proceed | delivery conflicts in store_api/node_api; master has nim-ffi 0.3 — treat as conflict resolution + D45.13; stop only if unpaid sync cannot be kept |
| O45.6 AT hexes | recorded | helper assert == operator/live `fe96c422…` |

## Verification rows

| Date (UTC) | Repo commit | Cell | Artifact | Result | ImageID | ELF size | RISC0_DEV_MODE | Pins | Clock | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-08-12 | `7d51ce3` | full-reset-localnet | `.scaffold/snapshots/funded` | pass | `dea010d9…` | 366868 | 1 | op `47eba256` / graph `v0.2.0` / spel `0cb7e098` / helper `47eba256` | n/a | AT hex exported `fe96c422…`; vault already funded skip; snapshot lez_pin matches operator |
| 2026-08-12 | \`931a6a0\` | module-local | \`.scaffold/e2e/artifacts/module-e2e-20260812T172651.log\` | pass | \`dea010d9…\` | 366868 | 1 | op/graph/spel/helper as Pins | n/a | soft; AT hex set |
