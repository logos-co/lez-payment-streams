# Step 50 — consistency and clarity polish

Upcoming. Index: [index.md](../index.md).

Prerequisite: [Step 46](step-46-docs-unify-and-forum-post.md) (living docs IA)
and [Step 49](../completed/step-49-native-token-spec-alignment.md) (token_id /
policy types and ImageID cut landed 2026-08).
[Step 48](../wontfix/step-48-program-graph-lez-unify.md) is not a prerequisite.

Workstream A (Makefile help) shares a surface with Step 46 README Testing
recipes. Persist, clock, and the C++ kit do not wait on 46 dogfood.

## Goal

Make the tree easier to enter and internally consistent without adding
structure for its own sake, and without changing demo outcomes.

Step 46 owns documentation IA and journey-name retirement.
Step 49 owns LIP multi-token types (complete; do not recut ImageID here).
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
  and omits `verify-step28-module-smoke`. `.PHONY` still lists `payee-close`
  aliases. `scripts/check-terminology.sh` special-cases `payee-close` until
  this step.
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

- Step 46 doc IA, forum post, journey redirects.
- Step 49 token types / PDA / non-native reject / guest ImageID.
- Token program custody path.
- Merging bash and Python orchestrators into one language.
- Sharing typed post-inclusion state polls (would need a translation layer).
- Splitting the guest binary into many files by default (only if a helper
  is already duplicated with core).
- Renaming `user-journey-*.sh` (D46.14).
- Dropping `extern crate nssa_core as lee_core` (blocked on spel / Step 48, wontfix).
- Step 21 UI.
- Reviving archived step DoD scripts as living gates.
- Sweeping `ift-ts` (already clean) or rewriting historical Step 3a cites in
  `integration-decisions.md`.

## Workstreams

### A. Maintainer surfaces

- Generate living `help` from `##` target comments so omitted living targets
  cannot drift. Keep one explicit line for historical DoD (`scripts/archive/`).
- Remove `verify-module-local-payee-close*` aliases (do not keep them as
  documented aliases). Same commit, drop the `payee-close` special case in
  `scripts/check-terminology.sh` and edit `.PHONY`.
- `Cargo.toml` comment: `completed/step-26-…`, not `upcoming/`.
- `handoff.md`: fold the duplicate pins link. Keep the retired redirect.
- Bootstrap / `ensure-testnet-vault` `PROGRAM_ID_HEX` defaults: copy the
  `deploy-testnet.sh` pattern (`make -s program-id` when `TESTNET_PROGRAM_ID_HEX`
  is unset). Keep the env override. Fail clearly if the guest binary is
  absent. Leave hardcoded hex in `scripts/archive/` frozen.

### B. Shared invariants

- One C++ clock helper with `>= 1_000_000_000_000`, comment citing
  `chain_timestamp_to_fold_seconds`. Replace both `foldClockForPolicy` and
  `chainTimestampToFoldSeconds`. Do not route wall-clock folds through FFI
  decode.
- Python and `seed_localnet_fixture` already match core. Add a one-line
  comment pointing at core. No behavior change.
- Account-id parse (hex-64 then base58) stays one helper per language
  (already D47.7); delete extra copies in the module .cpp files.

### C. Persist merge (first)

Land in its own commit, before mechanical helper moves (same spirit as D50.1).

- Choke point is `persistIfDirty` / `saveStateToDisk`, not `rediscoverStreams`
  alone. Before write, merge disk `provider_acceptances` and `peer_mappings`
  for keys memory lacks. Memory wins on conflict.
- Keep the verify-path `loadStateFromDisk` fallback on session miss.
- Regression test in the module Qt harness
  (`logos-payment-streams-module/tests/`): seed acceptances on disk, call
  rediscover (or any persist writer), assert the row survives.
- Keep E2E seed-after-rediscover until this merge lands. Do not revert to
  seed-then-rediscover. After the merge, the E2E order may stay as defense
  in depth.

### D. Module helper kit

- One `.h` / `.cpp` pair with a named namespace (true sharing, matches D50.3).
  An anonymous-namespace header would duplicate per translation unit.
- Kit list: `makeErrorJson` / `makeOkJson`, `walletAccountIdHexFromBase58`,
  `parseWalletAccountJson`, `hex32FromQString`, fixture path lookup,
  two-phase FFI buffer helpers, plus `makePlainError` /
  `makeEligibilityError` / `makeVerifyEligibilityError`.
- `parseWalletAccountJson`: decide one signature (superset with optional
  `errorOut` and `balanceHexOut`, or two functions). Do not pick one copy
  blindly.
- Drop unused `getVaultStatus` `streamId` from the C++ signature. Dispatch
  already passes `{}`. README chainAction table already matches
  (`owner`, `vault_id`). Expect zero living-doc lines.

### E. Orchestrators

Split confirmation into two layers.

- E1. Shared tx-inclusion wait. One Python `await_tx` entrypoint that both
  orchestrators invoke (bash `seq_tx_included` already shells to `python3`).
  Pick the wall-clock budget model (better under testnet latency variance).
  Module-e2e retry budgets will shift slightly. Keep artifact logging.
- E2. State polls stay per-orchestrator (logoscore JSON vs `call_ps` + jq),
  aligned only through the matrix on-chain confirmation principle.
- Do not rewrite `run_local_e2e.py` (5100 lines) in this step.
- Leave `user-journey-*.sh` filenames; comments may say “manual protocol
  path” now that Step 46 dropped living journey nav.
- Keep exporting live AT ImageID into logoscore before it starts. Do not
  restore an operator-override path.

### F. FFI

- Remove `payment_streams_ffi_ping`. Repoint `c_header_smoke.c` at a real
  export. Regenerate `lez_payment_streams_ffi.h` via cbindgen in the same
  commit.
- Rustdoc only: replace remaining “Step 3a” cites in
  `lez-payment-streams-core` and `lez-payment-streams-ffi` with a living
  pointer (policy module / LIP). Leave `integration-decisions.md`.
- Do not macro-generate per-instruction FFI planners (cbindgen constraint
  stands).

## Implementor order

1. Persist merge (C), own commit, own Qt test.
2. Clock helper plus kit (B, D).
3. Makefile / `.PHONY` / terminology special-case / `Cargo.toml` / `handoff.md`
   / program-id defaults (A). Remove aliases before rewriting help so help
   matches the final target list.
4. FFI ping + cbindgen + smoke (F), isolated because it touches a generated
   header.

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D50.1 | Guest / ImageID | No intentional ImageID cut. If a guest bug is found, split a tiny fix and record it; do not bundle with helper moves. Clock `>` vs `>=` is not a guest bug. |
| D50.2 | Orchestrators | E1 shares tx-inclusion (Python entrypoint, wall-clock budget). E2 keeps per-orchestrator state polls. Two entrypoints (`MODE=module` vs `MODE=store`). |
| D50.3 | C++ sharing | One named-namespace `.h` / `.cpp` pair, not a framework and not an anonymous-namespace header. |
| D50.4 | Clock | Core function is SSOT; C++ matches `>= 1_000_000_000_000`. Purity only. |
| D50.5 | Makefile | Living help from `##` comments; archive scripts stay in `scripts/archive/`. Remove `payee-close` aliases. |
| D50.6 | Docs | Touch living surfaces only if an API wart still disagrees (getVaultStatus likely zero lines). No new reproduce doc. |
| D50.7 | Verification | Commands below. No testnet unless D50.1 guest fix landed. Persist wipe is a module-state ordering bug, reproducible locally. |

## Done when

- Workstreams A–F landed or explicitly deferred in this packet with a reason.
- Clock conversion matches core in every living caller. Module test covers
  the `>= 1e12` boundary.
- Module JSON helpers exist once; `getVaultStatus` has no dummy `streamId`.
- Persist merge has a Qt regression test (disk seed survives rediscover).
- Makefile `help`, `.PHONY`, and the matrix alias list agree.
- cbindgen header regenerated; `c_header_smoke.c` links a real export.
- `make check-terminology`.
- `RISC0_DEV_MODE=1 cargo test --workspace`.
- `MODE=module ./scripts/e2e.sh local run` (no privacy flags).
- `MODE=store ./scripts/e2e.sh local run` (no privacy flags).
- Gate log: [step-50-gate-log.md](../completed/step-50-gate-log.md) (create
  when closing).

README Testing handles `fast` / `local-public` map to the commands above.
The verification matrix does not use those handle names; the gate is the
commands.

## Related

- [step-46-docs-unify-and-forum-post.md](step-46-docs-unify-and-forum-post.md)
- [step-47-unify-role-terminology.md](../completed/step-47-unify-role-terminology.md)
- [step-49-native-token-spec-alignment.md](../completed/step-49-native-token-spec-alignment.md)
- [naming-conventions.md](../../reference/naming-conventions.md)
- [verification-matrix.md](../../reference/verification-matrix.md)
- [payment-streams-module README](../../payment-streams-module/README.md)
  (chainAction table)
