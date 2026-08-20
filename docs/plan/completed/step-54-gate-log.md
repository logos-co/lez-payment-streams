# Step 54 gate log

Packet: [step-54-withdraw-owner-and-recipient.md](step-54-withdraw-owner-and-recipient.md).
D54.1 coverage (`fast` + module local both withdraw paths + negatives +
Store-local + stub-privacy withdraw to owner + module testnet withdraw
to owner) is recorded below.
Program identity: README [Public testnet guest program](../../../README.md#public-testnet-guest-program)
and [verify/fixtures/testnet-module.json](../../../verify/fixtures/testnet-module.json).
Living runner: `./verify/e2e.sh`.

## Identity

| Field | Value |
| --- | --- |
| Product ImageID (hex bytes) | `9d88f2e7498ae25caa3a5d829a3d1ed39f08673ba90987333074b965a81b90f8` |
| Product ELF size | 378516 |
| Product freeze commit | `9937e06639008883db68ae6cbde34d5103900405` (pre-rebase; ELF built then) |
| Product ELF pin | `.scaffold/program-bins/lez_payment_streams-9d88f2e7.bin` (gitignored) |
| Deploy date | 2026-08-20 |
| Deploy tx | `0c04d5ebf3d74b438f4e98ee8e892cb32054919ef0bc13eece75ddfbd4202372` |
| Deploy block | 15855 |
| Branch | `feat/step-54-withdraw-owner-and-recipient` (rebased onto `origin/master` `f8f7a3d`) |
| Tip at gate | `3a80cd0` |
| AT program | `fe96c4228babbe8bc578e3e25b884cacb07f8c86541f27ed676789875eef875a` |
| logos-lips | `docs/payment-streams-dual-close` / `6d71addda422eeac7816faa37541c34af65594c9` (local; unpushed) |

## Spec pin (D54.7)

| Artifact | Value |
| --- | --- |
| `logos-lips` | `docs/payment-streams-dual-close` / `6d71addda422eeac7816faa37541c34af65594c9` |
| Contains | Step 40 `435a6f18`, Step 41 `f09f9e9e`, Step 44 `32d7da4e`, Step 54 withdraw split `92685c78`, credit-path MUST `6d71addd` |
| Path | `docs/anoncomms/raw/payment-streams.md` |

## Fast (2026-08-20)

`RISC0_DEV_MODE=1`. Pre-cut cargo on freeze `9937e066`. Qt nix after header fix `5f4b20c` (rebased `b50639d`).

| Cell | Command | Result | Notes |
| --- | --- | --- | --- |
| clippy | `RISC0_SKIP_BUILD=1 cargo clippy --workspace` | pass | pre-cut |
| tests | `RISC0_DEV_MODE=1 cargo test --workspace` | pass | 155 core + 37 FFI; PP withdraw to owner included |
| terminology + links | `make check` | pass | |
| module Qt unit tests | `nix build ./module#checks.x86_64-linux.unit-tests -L` | pass | 42 tests (uniqueness, classify, `Withdraw4`, amount-cap) |

## Local public (2026-08-20)

`SKIP_BUILD=1` after the first module run rebuilt `payment_streams_module`.
ImageID `9d88f2e7…`. Fresh genesis localnet (bloated snapshot replay would
not bring RPC up).

| Date (UTC) | Cell | Artifact | Result | ImageID | `withdraw_path` |
| --- | --- | --- | --- | --- | --- |
| 2026-08-20 | module withdraw to owner | `.scaffold/e2e/artifacts/module-e2e-20260820T173715.log` | pass | `9d88f2e7…` | `owner` (plus `withdraw_equal_owner`) |
| 2026-08-20 | module withdraw (explicit `withdraw_to`) | `.scaffold/e2e/artifacts/module-e2e-20260820T174514.log` | pass | `9d88f2e7…` | `withdraw` |
| 2026-08-20 | module withdraw negatives | `.scaffold/e2e/artifacts/module-e2e-20260820T175246.log` | pass | `9d88f2e7…` | `args_mismatch` / `withdraw_prestate_unavailable` |
| 2026-08-20 | Store eligibility | `.scaffold/e2e/artifacts/e2e-20260820T180150.log` | pass | `9d88f2e7…` | (Store; no withdraw phase) |

Store query status 200, missing-proof reject, close, claim.

## Local private stub (2026-08-20)

`RISC0_DEV_MODE=1 OWNER_PRIVACY=1`. Omit-only withdraw (no equal-owner second submit).

| Date (UTC) | Cell | Artifact | Result | ImageID | `withdraw_path` |
| --- | --- | --- | --- | --- | --- |
| 2026-08-20 | module withdraw to owner | `.scaffold/e2e/artifacts/module-e2e-20260820T181344.log` | pass | `9d88f2e7…` | `owner` |

## Testnet public (2026-08-20)

`SKIP_BUILD=1`. Product ImageID `9d88f2e7…`. Omit-only withdraw.

| Date (UTC) | Cell | Artifact | Result | Notes |
| --- | --- | --- | --- | --- |
| 2026-08-20 | module withdraw to owner | `.scaffold/e2e/artifacts/module-e2e-20260820T182325.log` | pass | `withdraw_path=owner`, tx `8f05e4f5…` |

## UI

QML Owner Withdraw omits `withdraw_to`; kit amount-cap tests passed in Fast.
Operator Basecamp loop (init → deposit → stream → close → claim → withdraw)
was not driven in this headless session. Walkthrough is in
[basecamp-ui.md](../../reproduce/basecamp-ui.md).
