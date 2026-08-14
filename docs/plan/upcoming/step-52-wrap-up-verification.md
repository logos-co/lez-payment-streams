# Step 52 — wrap-up verification

Upcoming. Index: [index.md](../index.md).

Owns the post-50 protocol matrix that Step 46 called D46.11
and that briefly sat on Step 51 as D51.1.
Prerequisite: [Step 50](../completed/step-50-consistency-and-clarity.md)
(persist merge, `await_tx`, kit).
[Step 51](step-51-forum-post.md) cites this gate log for wrap-up claims.
Independent of [Step 21](step-21-basecamp-ui.md)
(Basecamp UI is a separate track; this packet certifies the CLI / Store stack).

Gate log: [step-52-gate-log.md](../completed/step-52-gate-log.md)
(create when closing).

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
Fund before each testnet privacy leg (`./scripts/fund-testnet-accounts.sh`).
CPU prove is valid (D39.26).

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
- Write [step-52-gate-log.md](../completed/step-52-gate-log.md) with
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

- [step-50-consistency-and-clarity.md](../completed/step-50-consistency-and-clarity.md)
- [step-51-forum-post.md](step-51-forum-post.md)
- [step-39-testnet-gate-log.md](../completed/step-39-testnet-gate-log.md)
- [README Testing](../../../README.md#testing)
- [verification-matrix.md](../../reference/verification-matrix.md)
