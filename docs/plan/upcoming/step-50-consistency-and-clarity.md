# Step 50 — consistency and clarity polish

Upcoming. Index: [index.md](../index.md).

Prerequisite: [Step 46](step-46-docs-unify-and-forum-post.md) (living docs IA)
and [Step 49](step-49-native-token-spec-alignment.md) (token_id / policy types
and ImageID cut already landed).
[Step 48](../wontfix/step-48-program-graph-lez-unify.md) is not a prerequisite.

## Goal

Make the tree easier to enter and internally consistent without adding
structure for its own sake, and without changing demo outcomes.

Step 46 owns documentation IA and journey-name retirement.
Step 49 owns LIP multi-token types.
This step owns remaining naming drift, duplicated glue, API warts, and
maintainer-surface clutter identified in the post-47 clarity review.

## Problem

After 46 and 49, product docs and token types are aligned, but:

- Clock conversion (`>= 1e12` vs `> 1e12`) is copied in core, FFI, C++ (twice),
  Python, and the seed fixture.
- Module `writes.cpp` / `eligibility.cpp` / `impl.cpp` each reimplement JSON
  helpers, account-id parse, fixture lookup, and two-phase FFI sizing.
- `getVaultStatus` still takes an unused `streamId`.
- Two orchestrators (`module-e2e.sh`, `run_local_e2e.py`) reimplement
  sequencer wait + state poll.
- Makefile still leads with archived `verify-step10a`… targets and
  `payee-close` aliases.
- Plan index / Cargo.toml comments still point at moved packets; `handoff.md`
  is a leftover front door.
- FFI still exports `payment_streams_ffi_ping`.

## Scope

In scope (workstreams below).
Prefer extract-and-reuse over new layers.
No ImageID change unless a defect forces a guest edit (D50.1).

Out of scope:

- Step 46 doc IA, forum post, journey redirects.
- Step 49 token types / PDA / non-native reject.
- Token program custody path.
- Merging bash and Python orchestrators into one language.
- Splitting the guest binary into many files by default (only if a helper
  is already duplicated with core).
- Renaming `user-journey-*.sh` (D46.14).
- Dropping `extern crate nssa_core as lee_core` (blocked on spel / Step 48, wontfix).
- Step 21 UI.
- Reviving archived step DoD scripts as living gates.

## Workstreams

### A. Maintainer surfaces

- Makefile `help`: canonical `verify-module-*` / `verify-store-*` first;
  archived `verify-step10*`… as a single “historical DoD (archive/)” line
  or drop from help (keep targets if still invoked).
- Remove or document `verify-module-local-payee-close*` as aliases of
  `provider-close` (terminology gate already special-cases them).
- Fix `plan/index.md` outcomes row that still marks Step 45 upcoming.
- `Cargo.toml` comment: `completed/step-26-…`, not `upcoming/`.
- `handoff.md`: keep as redirect or fold the duplicate pins link; it must
  not read as a third product front door.

### B. Shared invariants

- One clock rule: core `chain_timestamp_to_fold_seconds` (`>=` threshold).
  C++ must not reimplement with `>`. Call FFI decode (already normalized)
  or a single shared helper that matches core.
- Python and `seed_localnet_fixture` keep `>=` or call the same documented
  rule; add a one-line comment pointing at core.
- Account-id parse (hex-64 then base58) stays one helper per language
  (already D47.7); delete extra copies in the module .cpp files.

### C. Module structure

- Shared anonymous-namespace kit used by impl / writes / eligibility:
  `makeErrorJson` / `makeOkJson`, `walletAccountIdHexFromBase58`,
  `parseWalletAccountJson`, `hex32FromQString`, fixture path lookup,
  two-phase FFI buffer helpers.
- One extra `.cpp` / `.h` pair is enough. Do not introduce a folder of
  one-function files.
- Drop unused `getVaultStatus` `streamId`; `chainAction` already passes `{}`.
  Catalogue and C++ signature must match.

### D. Orchestrators

- Extract shared confirmation (wait for sequencer inclusion, then poll
  affected account) so bash and Python cannot drift from the matrix
  “on-chain confirmation principle”.
- Do not rewrite `run_local_e2e.py` (5100 lines) in this step.
- Leave `user-journey-*.sh` filenames; comments may say “manual protocol
  path” now that Step 46 dropped living journey nav.

### E. FFI / comments

- Remove `payment_streams_ffi_ping` if nothing in-tree links it.
- Off-chain / policy comments that still say Step 3a / `ift-ts` after 49:
  sweep leftovers.
- Do not macro-generate per-instruction FFI planners (cbindgen constraint
  stands).

## Decisions

| ID | Topic | Decision |
| --- | --- | --- |
| D50.1 | Guest / ImageID | No intentional ImageID cut. If a guest bug is found, split a tiny fix and record it; do not bundle with helper moves. |
| D50.2 | Orchestrators | Share confirmation helpers; keep two entrypoints (`MODE=module` vs `MODE=store`). |
| D50.3 | C++ sharing | One shared compilation unit, not a framework. |
| D50.4 | Clock | Core function is SSOT; other languages match `>= 1_000_000_000_000`. |
| D50.5 | Makefile | Living help matches verification-matrix; archive scripts stay in `scripts/archive/`. |
| D50.6 | Docs | Touch living surfaces only as needed for API warts (getVaultStatus). No new reproduce doc. |
| D50.7 | Verification | `fast` + `local-public` (module + store). Testnet only if D50.1 guest fix landed. |

## Done when

- Workstreams A–E landed or explicitly deferred in this packet with a reason.
- Clock conversion matches core in every living caller.
- Module JSON helpers exist once; `getVaultStatus` has no dummy `streamId`.
- Makefile help and plan index match completed/upcoming reality.
- `make check-terminology` clean; `RISC0_DEV_MODE=1 cargo test --workspace`
  and `local-public` module + store green.
- Gate log: [step-50-gate-log.md](../completed/step-50-gate-log.md) (create
  when closing).

## Related

- [step-46-docs-unify-and-forum-post.md](step-46-docs-unify-and-forum-post.md)
- [step-47-unify-role-terminology.md](../completed/step-47-unify-role-terminology.md)
- [step-49-native-token-spec-alignment.md](step-49-native-token-spec-alignment.md)
- [naming-conventions.md](../../reference/naming-conventions.md)
- [verification-matrix.md](../../reference/verification-matrix.md)
