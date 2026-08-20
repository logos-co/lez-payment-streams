# Step 52 — wrap-up verification

Index: [index.md](../index.md). Status: complete (2026-08-16).
Gate log: [step-52-gate-log.md](step-52-gate-log.md).

Owns the post-50 protocol matrix that Step 46 called D46.11
and that briefly sat on Step 51 as D51.1.
Prerequisite: [Step 50](step-50-consistency-and-clarity.md)
(persist merge, `await_tx`, kit).
[Step 51](../upcoming/step-51-forum-post.md) cites this gate log for wrap-up claims.
Independent of [Step 21](step-21-basecamp-ui.md)
(Basecamp UI is a separate track; this packet certifies the CLI / Store stack).

## Goal

Record one green wrap-up on the current ImageID that covers unit tests,
localnet E2E, and testnet E2E, including private execution.
Fill rows from artifacts on the post-50 tree.
Cite README guest section and fixtures for ImageID.

## Coverage (what this run must prove)

Full privacy (`OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`) stands in for the
single-flag privacy cells.
Owner-close is the default module happy path;
`CLOSE_ROLE=provider` is the extra close role.

| Function | Stage | Command |
| --- | --- | --- |
| Clippy | Fast | `RISC0_SKIP_BUILD=1 cargo clippy --workspace` |
| Rust unit / integration (core + FFI) | Fast | `RISC0_DEV_MODE=1 cargo test --workspace` |
| Terminology | Fast | `make check-terminology` |
| Qt kit (`persist` merge, clock fold) | Fast | `nix build ./logos-payment-streams-module#tests -L` |
| Module lifecycle (init, deposit, create, owner-close, claim) | Local public | `MODE=module ./scripts/e2e.sh local run` |
| Provider-close | Local public | `MODE=module CLOSE_ROLE=provider ./scripts/e2e.sh local run` |
| Close / create reject tokens | Local public | `make verify-module-local-close-negatives` |
| Store eligibility (paid query, missing-proof reject, close, claim) | Local public | `MODE=store ./scripts/e2e.sh local run` |
| Module private accounts (stub receipts) | Local private | `MODE=module OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` (`RISC0_DEV_MODE=1`) |
| Store private accounts (stub receipts) | Local private | `MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` (`RISC0_DEV_MODE=1`) |
| Module PPE smoke before testnet | Local real-prove | `RISC0_DEV_MODE=0 MODE=module OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` |
| Module on public testnet | Testnet public | `MODE=module ./scripts/e2e.sh testnet run` |
| Store on public testnet | Testnet public | `MODE=store ./scripts/e2e.sh testnet run` |
| Module full privacy, real proving | Testnet private | `RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 MODE=module OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh testnet run` |
| Store full privacy, real proving | Testnet private | `RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh testnet run` |

`SKIP_BUILD=1` after the first local prepare that already built `.lgx`.
Testnet legs use existing fixtures (`make bootstrap-testnet` /
`make bootstrap-testnet-module` only if those files are missing).
Fund before each testnet privacy leg (`./verify/testnet/fund-testnet-accounts.sh`).
CPU prove is valid (D39.26).
On the Step 53 tree the living runner is `./verify/e2e.sh`
(coverage rows above still spell the pre-move `./scripts/e2e.sh`).

## This run

Complete. Branch `feat/step-53-repository-structure`.
Gate log: [step-52-gate-log.md](step-52-gate-log.md).

Product ImageID (README, funded snapshot, `verify/fixtures/testnet-module.json`)
is `c30781ea9d7cc7b3be36f459ce9094644b984224d3d3119a644bb1b21ba2982a`.
Local harness guest remains
`cdc9bfea4fdb6490a99929619bfb2c0eefd36a936668b1b3a22f684e66b44f0c`
for local real-prove only.
Product ELF recovered 2026-08-15 by isolated `make build` at
`8a0e374a7e7171cd5b60ad20d46b9510b057dfe3`
(373916 bytes, ImageID match).
Pin: `.scaffold/program-bins/lez_payment_streams-c30781ea.bin` (gitignored).
Live guest was not replaced.
Testnet cells used the product ImageID.

Operator logs: `/tmp/step52-verify/log.txt`,
`/tmp/step52-verify/local-realprove-45s.log`,
`/tmp/step52-verify/testnet-private-module.log`,
`/tmp/step52-verify/testnet-private-store.log`.

### Verified

| Function | Status | Artifact |
| --- | --- | --- |
| Clippy | pass | `/tmp/step52-verify/clippy.log` (`RISC0_SKIP_BUILD=1 cargo clippy --workspace`) |
| Rust unit / integration | pass | `/tmp/step52-verify/cargo-test.log` (`RISC0_DEV_MODE=1 cargo test --workspace`) |
| Terminology | pass | `make check-terminology` (also `make check`) |
| Qt kit | pass | `/tmp/step52-verify/nix-tests.log` (`nix build ./module#checks.x86_64-linux.unit-tests -L`) |
| Module lifecycle local public | pass | `.scaffold/e2e/artifacts/module-e2e-20260814T144631.log` |
| Provider-close local public | pass | `.scaffold/e2e/artifacts/module-e2e-20260814T145407.log` |
| Close / create reject tokens | pass | `.scaffold/e2e/artifacts/module-e2e-20260814T150159.log` |
| Store eligibility local public | pass | `.scaffold/e2e/artifacts/e2e-20260814T153211.log` |
| Module private stub | pass | `.scaffold/e2e/artifacts/module-e2e-20260814T154533.log` (`RISC0_DEV_MODE=1 OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`) |
| Store private stub | pass | `.scaffold/e2e/artifacts/e2e-20260814T174058.log` (`RISC0_DEV_MODE=1 OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`) |
| Module testnet public | pass | `.scaffold/e2e/artifacts/module-e2e-20260814T180419.log` |
| Store eligibility testnet public | pass | `.scaffold/e2e/artifacts/e2e-20260814T190338.log` |
| Module local real-prove | pass | `.scaffold/e2e/artifacts/module-e2e-local-realprove-45s-retry.log` (`RISC0_DEV_MODE=0 OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 E2E_CLAIM_OPTIONAL=0 LOCALNET_BLOCK_TIME=45s`, logoscore `pre-release-66c4194`, LEZ `47eba256`, guest `cdc9bfea…`). Claim `0e8a2566…`. |
| Module testnet private | pass | `.scaffold/e2e/artifacts/module-e2e-testnet-private-retry.log` (`RISC0_DEV_MODE=0 OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 E2E_CLAIM_OPTIONAL=0`, pinned ELF `c30781ea…`). Claim `34e48ed2…`, vault_drop 400. |
| Store testnet private | pass | `.scaffold/e2e/artifacts/e2e-testnet-store-private-retry.log` (`RISC0_DEV_MODE=0 OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 E2E_CLAIM_OPTIONAL=0`, pinned ELF `c30781ea…`). Query 200, missing-proof reject, close, claim `2547744f…`, vault_drop 400. |

Fail artifacts and retries are listed in the [gate log](step-52-gate-log.md).
Step 51 may fill wrap-up claims from the green rows only.

## Time estimates

Wall-clock on one Linux host, CPU prove, modules already built
(`SKIP_BUILD=1`).
Sources: Step 49 artifact start deltas (public),
Step 44 local close cells,
Step 39 passing Store privacy run (~106 min CPU) and PPE include smokes
(~3–4 min).
Retries are extra; historically they dominated testnet-private.

| Stage | Estimate | Notes |
| --- | --- | --- |
| Fast (clippy, cargo test, terminology) | 15–30 min | Step 50 recorded cargo test + terminology only |
| Qt kit tests | 5–15 min first Nix; 2–5 min later | Unrun at Step 50 close |
| Cold local prepare (first `.lgx` / snapshot) | 20–40 min | Once per machine; skip if `.scaffold/e2e` is populated |
| Local public (module, provider-close, close-negatives, store) | 40–70 min | Module cells ~5–10 min each; Store ~15–25 min |
| Local private stub (module + store) | 25–40 min | Same shape as public, stub receipts |
| Local real-prove module smoke | 20–40 min | Warm-up before burning testnet PPE |
| Testnet public (module + store) | 30–45 min | Step 49 ~12 min module, ~15 min store |
| Testnet fund / PPE include smoke | 10–20 min | Per privacy leg; amount=1 include ~3–5 min |
| Testnet private module | 45–90 min | Real prove + CLOCK_50 wait |
| Testnet private store | 90–120 min | Step 39 green was ~106 min CPU |

First green pass, already-built tree: about 5–7 hours operator time.
Cold Nix plus a failed testnet-private retry: budget a second day.
Run fast → local public → local private stub → local real-prove →
testnet public → testnet private so a cheap fail stops a dearer leg.

## Scope

- Run the coverage table on the post-50 tree (ImageID `c30781ea…` unless
  a later guest cut landed).
- Write [step-52-gate-log.md](step-52-gate-log.md) with
  artifact paths, ImageID citations, and wall-clock notes.
- Reuse a post-50 artifact only when the command and tree match a row.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D52.1 | Coverage | Absorbs D46.11 / D51.1. Function rows above, including module and Store testnet-private. Full-privacy flags cover single-flag privacy cells. |
| D52.2 | Timing | Independent of Step 21. Step 51 wrap-up claims wait on this gate log. |
| D52.3 | Prove | Local stub receipts for account-flow privacy; `RISC0_DEV_MODE=0` for local smoke and both testnet-private legs. |
| D52.4 | Identity | Cite README guest section and fixtures; keep hex out of reproduce and forum prose. |

## Done when

- Every coverage row has a pass (or a cited equivalent post-50 artifact).
- Gate log lists artifact paths and ImageID.
- Step 51 can fill wrap-up claims from those rows.

## Related

- [step-50-consistency-and-clarity.md](step-50-consistency-and-clarity.md)
- [step-51-forum-post.md](../upcoming/step-51-forum-post.md)
- [step-39-testnet-gate-log.md](step-39-testnet-gate-log.md)
- [README Testing](../../../README.md#testing)
- [matrix.md](../../reference/matrix.md)
