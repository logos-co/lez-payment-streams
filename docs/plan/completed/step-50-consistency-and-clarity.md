# Step 50 — consistency and clarity polish

Index: [index.md](../index.md). Status: complete (2026-08).
Gate log: [step-50-gate-log.md](step-50-gate-log.md).

Workstreams A–F landed on `feat/step-50-consistency-and-clarity`.
Fast verification recorded 2026-08-13. No ImageID cut (D50.1).
Wrap-up matrix remains [Step 52](step-52-wrap-up-verification.md).

Prerequisite: [Step 46](step-46-docs-unify-and-forum-post.md)
(living docs IA, complete) and
[Step 49](step-49-native-token-spec-alignment.md)
(token_id / policy types and ImageID cut landed 2026-08).
[Step 48](../wontfix/step-48-program-graph-lez-unify.md) is not a prerequisite.
[Step 51](step-51-forum-post.md) (forum publish) and
[Step 52](step-52-wrap-up-verification.md) (wrap-up matrix) follow this step.

## Goal

Make the tree easier to enter and internally consistent without adding
structure for its own sake, and without changing demo outcomes.

Step 46 owns documentation IA and journey-name retirement (complete).
Step 49 owns LIP multi-token types (complete; do not recut ImageID here).
Step 51 owns forum publish.
Step 52 owns the full Testing-handle matrix.
This step owns remaining naming drift, duplicated glue, API warts, and
maintainer-surface clutter identified in the post-47 clarity review,
plus persist/E2E glue exposed by the Step 49 ImageID-cut dogfood.

## Problem

After 46 and 49, product docs and token types are aligned, but:

- Clock conversion is copied. Core, FFI, Python, and the seed fixture use
  `>= 1e12`. Two C++ helpers (`foldClockForPolicy` in eligibility.cpp,
  `chainTimestampToFoldSeconds` in impl.cpp) use `>`. The `>` vs `>=` split
  only matters at exactly `10^12`, which is unreachable in either unit here.
  This is purity, not a latent bug, and must not motivate a guest recut.
  FFI decode already folds values inside decoded structs.
  `foldClockForPolicy` folds a wall-clock value before policy construction,
  so C++ still needs one shared helper with `>=` and a comment citing core.
- Module `writes.cpp` / `eligibility.cpp` / `impl.cpp` each reimplement JSON
  helpers, account-id parse, fixture lookup, and two-phase FFI sizing.
  Copies have already diverged (`parseWalletAccountJson` in impl.cpp takes
  `errorOut`; writes/eligibility take `balanceHexOut`). Eligibility also has
  `makePlainError` / `makeEligibilityError` / `makeVerifyEligibilityError`.
- `getVaultStatus` still takes an unused `streamId` on the C++ signature
  (`impl.h`, `writes.cpp` with `Q_UNUSED`). Dispatch already passes `{}`.
  The README chainAction table already lists `owner`, `vault_id` only.
- Two orchestrators (`module-e2e.sh`, `run_local_e2e.py`) reimplement
  sequencer inclusion wait. Bash is fixed attempt-count × fixed sleep
  (`scripts/lib/chain_poll.sh`). Python uses a wall-clock budget, exponential
  backoff, hash normalization, and artifact logging. Typed state polls after
  inclusion are op-specific and must stay per-orchestrator.
- Makefile `help` is hand-written `@echo` and has drifted. It leads with
  archived `verify-step10*`…`step13` lines, omits living
  `verify-module-local-close-negatives` (listed in the verification matrix),
  and omits `verify-step28-module-smoke`. `.PHONY` is a hand-maintained list
  and still lists the retired-role close aliases.
  `scripts/check-terminology.sh` special-cases those aliases until this step.
- `Cargo.toml` still comments `upcoming/step-26-…`.
- `handoff.md` is already retired (line 3). The remaining wart is a duplicated
  pins link (the same `feature-branch-pins.md` href twice).
- FFI still exports `payment_streams_ffi_ping`. The C header smoke test
  (`lez-payment-streams-ffi/tests/c_header_smoke.c`) calls it, so a
  conditional “remove if unlinked” would no-op.
- `persistIfDirty` writes in-memory state and can wipe disk-seeded
  `provider_acceptances` / `peer_mappings`. Ten-plus call sites, not only
  `rediscoverStreams`. Step 49 testnet Store saw `PROOF_INVALID` / session
  public key unknown. E2E now seeds after rediscover (`12f93bd`). That order
  is a workaround. The verify path already `loadStateFromDisk` on session
  miss; that does not help if persist already overwrote the file.
- Bootstrap and `ensure-testnet-vault` hardcode `PROGRAM_ID_HEX`
  (`scripts/bootstrap-testnet-module.sh`, `scripts/e2e/ensure-testnet-vault.sh`).
  `scripts/deploy-testnet.sh` already derives it via `make -s program-id`.

Living `ift-ts` citations are gone outside plan/archive. Not a workstream.

## Scope

In scope (workstreams below).
Prefer extract-and-reuse over new layers.
No ImageID change unless a defect forces a guest edit (D50.1).
Timebox the C++ kit to the listed helpers plus the error variants.

Out of scope:

- Step 46 doc IA, journey redirects.
- Step 51 forum post.
- Step 52 wrap-up matrix.
- Step 49 token types / PDA / non-native reject / guest ImageID.
- Token program custody path.
- Merging bash and Python orchestrators into one language.
- Sharing typed post-inclusion state polls (would need a translation layer).
- Splitting the guest binary into many files by default (only if a helper
  is already duplicated with core).
- Renaming `user-journey-*.sh` (D46.14).
- Dropping `extern crate nssa_core as lee_core` (blocked on spel / Step 48, wontfix).
- Step 21 UI.
- Editing files under `scripts/archive/`.
- Re-homing living Makefile targets that still invoke archive scripts
  (flagged below; do not add new callers).
- Reviving archived step DoD scripts as living gates.
- Sweeping `ift-ts` (already clean) or rewriting historical Step 3a cites in
  `integration-decisions.md`.
- Full protocol matrix (privacy E2E, testnet public/private, extra close
  cells). That is Step 52 (D52.1).

## Workstreams

### A. Maintainer surfaces

- Generate living `help` from `##` target comments. Omit targets whose recipe
  invokes `scripts/archive/` (do not list each archived DoD). Keep one explicit
  line pointing at `scripts/archive/` for historical DoD.
- `.PHONY` is generated from every recipe target name in this Makefile
  (awk / `$(shell …)`), including aliases and archive wrappers. Do not
  hand-maintain a second list.
- Remove the retired-role Makefile close aliases (do not keep them as
  documented aliases). Same commit, drop the matching special case in
  `scripts/check-terminology.sh`.
- `Cargo.toml` comment: `completed/step-26-…`, not `upcoming/`.
- `handoff.md`: fold the duplicate pins link. Keep the retired redirect.
- Living bootstrap / `ensure-testnet-vault` `PROGRAM_ID_HEX` defaults: derive
  via `make -s program-id` when `TESTNET_PROGRAM_ID_HEX` is unset. Keep the
  env override. Fail clearly if the guest binary is absent (do not auto-build).
  Align `ensure-testnet-vault.sh` on `TESTNET_PROGRAM_ID_HEX` (it currently
  uses `PROGRAM_ID_HEX`). Do not edit `scripts/archive/`.

Living Makefile recipes that still invoke `scripts/archive/` (do not touch
those scripts in this step):

| Make target | Archive script | Role today |
| --- | --- | --- |
| `wallet-lgx` | `build-wallet-lgx.sh` | Documented in feature-branch-pins |
| `verify-store-local-lifecycle` | `verify-store-local-lifecycle.sh` | Maintainer lifecycle cell |
| `bootstrap-testnet` | `bootstrap-testnet.sh` | Store testnet fixture (still hardcoded ImageID) |
| `verify-step18-testnet-read-smoke` | `verify-step18-testnet-read-smoke.sh` | Historical smoke |
| `verify-step10a` … `verify-step13` | matching `verify-step*-dod.sh` | Historical DoD |

`make bootstrap-testnet` therefore keeps the archive hardcoded ImageID.
Only the living `bootstrap-testnet-module.sh` and `ensure-testnet-vault.sh`
paths get derive-and-fail.

### B. Shared invariants

- One C++ clock helper in the kit (workstream D) with
  `>= 1_000_000_000_000`, comment citing `chain_timestamp_to_fold_seconds`.
  Replace both `foldClockForPolicy` and `chainTimestampToFoldSeconds`.
  Do not route wall-clock folds through FFI decode.
- Python and `seed_localnet_fixture` already match core. Add a one-line
  comment pointing at core. No behavior change.
- Account-id parse (hex-64 then base58) stays one helper per language
  (already D47.7); delete extra copies in the module .cpp files.

### C. Persist merge (first)

Land in its own commit, before mechanical helper moves (same spirit as D50.1).
Create the kit `.h` / `.cpp` in this commit with the merge helper (and
optionally the clock helper). Remaining kit symbols land in commit 2.

- Choke point is `persistIfDirty` / `saveStateToDisk`, not `rediscoverStreams`
  alone. Before write, merge disk `provider_acceptances` and `peer_mappings`
  for keys memory lacks. Memory wins on conflict.
- Acceptance identity is `(vault_id, provider_id_hex)` (same as
  `findProviderAcceptanceIndex` and `seed_provider_acceptance.py`).
  `peer_mappings` merge by peer-id object key.
  Do not merge `negotiations` / `inventory` unless a second wipe shows up.
- Keep the verify-path `loadStateFromDisk` fallback on session miss.
- Regression test in the module Qt harness
  (`logos-payment-streams-module/tests/`): compile the kit `.cpp` the same
  way `test_privacy_submit_policy.cpp` compiles `payment_streams_privacy_policy.cpp`.
  Feed memory JSON and disk JSON into the merge helper. Assert a disk-only
  acceptance row survives and that a memory row wins on the same key.
  Do not instantiate `PaymentStreamsModuleImpl` or call `rediscoverStreams`
  (the current harness does not link wallet / FFI).
- Keep E2E seed-after-rediscover until this merge lands. Do not revert to
  seed-then-rediscover. After the merge, the E2E order may stay as defense
  in depth.

### D. Module helper kit

- One `.h` / `.cpp` pair with namespace `payment_streams_kit` (true sharing,
  matches D50.3). An anonymous-namespace header would duplicate per
  translation unit. Add the files to module `CMakeLists.txt` `SOURCES` and to
  tests `MODULE_SOURCES`.
- Kit list: clock fold, persist merge, `makeErrorJson` / `makeOkJson`,
  `walletAccountIdHexFromBase58`, `parseWalletAccountJson`, `hex32FromQString`,
  `variantToU64`, fixture path lookup, two-phase FFI buffer helper, plus
  `makePlainError` / `makeEligibilityError` / `makeVerifyEligibilityError`.
- `parseWalletAccountJson`: one superset. Optional `errorOut` and
  `balanceHexOut`. Require a non-empty `data` field only when `dataOut` is
  non-null (holding-balance callers pass `dataOut == nullptr`).
- Drop unused `getVaultStatus` `streamId` from the private C++ signature.
  Dispatch already passes `{}`. LogosAPI `chainAction` is unchanged.
  README chainAction table already matches (`owner`, `vault_id`).
  Expect zero living-doc lines.

### E. Orchestrators

Split confirmation into two layers.

- E1. Extract `wait_for_sequencer_tx` from `run_local_e2e.py` into a small
  shared module (for example `scripts/e2e/await_tx.py`) with a CLI entry
  `await_tx`. Wall-clock budget (`E2E_TX_ONCHAIN_WAIT_S`), exponential
  backoff, `0x`/case hash normalize. Optional artifact path; omit from bash
  if there is no JSONL handle. `chain_poll.sh` `await_inclusion` shells to
  that CLI. `run_local_e2e.py` imports the function (do not rewrite the
  5100-line orchestrator).
- Retire `INCLUSION_ATTEMPTS` / `INCLUSION_SLEEP` as the inclusion budget.
  Document `E2E_TX_ONCHAIN_WAIT_S` as the override. Module-e2e retry budgets
  will shift slightly toward the Python default (~110s).
- E2. State polls stay per-orchestrator (logoscore JSON vs `call_ps` + jq),
  aligned only through the matrix on-chain confirmation principle.
- Leave `user-journey-*.sh` filenames; comments may say “manual protocol
  path” now that Step 46 dropped living journey nav.
- Keep exporting live AT ImageID into logoscore before it starts. Do not
  restore an operator-override path.

### F. FFI

- Remove `payment_streams_ffi_ping`. Repoint `c_header_smoke.c` at an
  existing export (empty-input `payment_streams_ffi_decode_vault_config_bytes`
  is enough). Regenerate `lez_payment_streams_ffi.h` via cbindgen in the
  same commit (`lez-payment-streams-ffi/build.rs` already runs it).
- Rustdoc only: replace remaining “Step 3a” cites in
  `lez-payment-streams-core` and `lez-payment-streams-ffi` with a living
  pointer (policy module / LIP). Leave `integration-decisions.md`.
- Do not macro-generate per-instruction FFI planners (cbindgen constraint
  stands).

## Implementor order

1. Persist merge (C) in the new kit files, own commit, own Qt unit test.
2. Clock helper plus remaining kit symbols (B, D), including `getVaultStatus`.
3. Shared `await_tx` (E).
4. Makefile help / generated `.PHONY` / terminology special-case /
   `Cargo.toml` / `handoff.md` / program-id defaults (A). Remove aliases
   before rewriting help so help matches the final target list.
5. FFI ping + cbindgen + smoke (F), isolated because it touches a generated
   header.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D50.1 | Guest / ImageID | No intentional ImageID cut. If a guest bug is found, split a tiny fix and record it; do not bundle with helper moves. Clock `>` vs `>=` is not a guest bug. |
| D50.2 | Orchestrators | E1 shares tx-inclusion (extracted Python `await_tx`, wall-clock budget). E2 keeps per-orchestrator state polls. Two entrypoints (`MODE=module` vs `MODE=store`). |
| D50.3 | C++ sharing | One named-namespace `.h` / `.cpp` pair (`payment_streams_kit`), not a framework and not an anonymous-namespace header. |
| D50.4 | Clock | Core function is SSOT; C++ matches `>= 1_000_000_000_000`. Purity only. Kit unit test covers the `>= 1e12` boundary. |
| D50.5 | Makefile | Living help from `##` comments, skipping archive-invoking recipes. `.PHONY` generated from all recipe targets. Remove the retired-role close aliases. Do not edit `scripts/archive/`. |
| D50.6 | Docs | Touch living surfaces only if an API wart still disagrees (getVaultStatus likely zero lines). No new reproduce doc. |
| D50.7 | Verification | `fast` only (`make check-terminology`, `RISC0_DEV_MODE=1 cargo test --workspace`, plus the new Qt kit tests via the module test binary if that is how the harness is invoked). No E2E in this step’s gate. Wrap-up matrix is Step 52. |
| D50.8 | Persist test | Unit-test the merge helper with JSON fixtures. Do not call `rediscoverStreams` in Qt tests. |
| D50.9 | `parseWalletAccountJson` | One superset (optional `errorOut` and `balanceHexOut`; data required only when `dataOut` is non-null). |
| D50.10 | Program id | Derive-and-fail via `make -s program-id` when `TESTNET_PROGRAM_ID_HEX` is unset. No auto-build. |

## Done when

- Workstreams A–F landed or explicitly deferred in this packet with a reason.
- Clock conversion matches core in every living caller. Kit test covers
  the `>= 1e12` boundary.
- Module JSON helpers exist once; `getVaultStatus` has no dummy `streamId`.
- Persist merge has a Qt unit test (disk-only acceptance row survives merge).
- Makefile `help` lists living targets; `.PHONY` is generated; matrix alias
  list has no retired-role close names.
- cbindgen header regenerated; `c_header_smoke.c` links a real export.
- `make check-terminology`.
- `RISC0_DEV_MODE=1 cargo test --workspace`.
- Gate log: [step-50-gate-log.md](step-50-gate-log.md).

No `MODE=module` / `MODE=store` E2E in this gate. README Testing handle
`fast` maps to the commands above.

## After this step

[Step 52](step-52-wrap-up-verification.md) runs the wrap-up protocol matrix on the
current ImageID (D52.1). Step 51 publishes the forum post from that gate log. Step 49 proved public
local and public testnet only. Privacy never ran on this cut.

## Related

- [step-46-docs-unify-and-forum-post.md](step-46-docs-unify-and-forum-post.md)
- [step-51-forum-post.md](step-51-forum-post.md)
- [step-52-wrap-up-verification.md](step-52-wrap-up-verification.md)
- [step-47-unify-role-terminology.md](step-47-unify-role-terminology.md)
- [step-49-native-token-spec-alignment.md](step-49-native-token-spec-alignment.md)
- [naming-conventions.md](../../reference/naming-conventions.md)
- [verification-matrix.md](../../reference/verification-matrix.md)
- [payment-streams-module README](../../payment-streams-module/README.md)
  (chainAction table)
