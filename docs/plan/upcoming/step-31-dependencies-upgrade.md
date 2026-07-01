# Plan — rebase delivery forks + upgrade wallet module

## Status

Complete (2026-07-01). All five phases executed; hermetic E2E gate green.

| Phase | Result |
| --- | --- |
| A — `logos-delivery` rebase | Rebased onto `origin/master` (`cbb601ec`); tip `64593368` pushed to `logos-messaging/logos-delivery` `feat/payment-streams-store-eligibility`. Adapted `eligibility_api.nim` to api-shape phase2 (`FFIContext[LogosDelivery]`, `ctx.myLib[].waku.node.` access). All eligibility Nim tests pass. |
| B — `logos-delivery-module` rebase | Rebased onto `origin/master`; tip `f8a76ba` pushed to `s-tikhomirov/logos-delivery-module` `feat/payment-streams-store-eligibility`. Resolved `delivery_module_plugin.h` conflict (preserved both `collectOpenMetricsText()` and eligibility methods). `nix build .#lgx-portable` + `nix flake check` green. |
| C — wallet module | Bumped `logos-execution-zone-module` from `d70225c` to `b555cd5` (latest `main`). All three patches apply cleanly. `nix build .#lib` + `build-wallet-lgx.sh` green. |
| D — payment-streams consume | E2E gate `SKIP_LIBLOGOSDELIVERY_OVERLAY=1 make verify-store-local` PASSED: stream 0 created/closed/claimed, 99-message Store query returned status 200, missing-proof query correctly rejected. |
| E — doc updates | Pin tables and verification commands updated in `docs/reference/feature-branch-pins.md`. Path references (`kernel_api/protocols/store_api.nim`) confirmed unchanged by phase2 — no path edits needed. |

Pre-existing fix surfaced during Phase D: `logos-module-builder` `#lgx` and the default `nix-bundle-lgx` bundler now emit `linux-amd64-dev` variants, which `lgpm` 0.2.0 rejects for `logoscore`. Switched `e2e.sh` to `#lgx-portable` and `build-wallet-lgx.sh` to `nix-bundle-lgx#portable` (commit `153a83e`).

## Goal

Rebase `logos-delivery` and `logos-delivery-module` onto upstream `master`, refresh the wallet module to latest `main`, and update the payment-streams repo to consume the rebased artifacts. All while keeping the payment-streams demo (Steps 17, 26–30) green.

## Guiding constraints

- The eligibility bridge (Step 16) and Store hooks (Step 15) must remain functional end-to-end.
- The C ABI contract (`logosdelivery_store_query`, `logosdelivery_set_eligibility_verifier`, `logosdelivery_set_eligibility_provider`) is the load-bearing surface; it must survive the rebase.
- LEZ core stays pinned at `v0.2.0` (`a58fbce`) — already at `main` tip, no action.
- SPEL stays pinned at `v0.5.0` — LEE patch still needed, no action.
- LIP-155 spec branch is out of scope (spec edits, not a rebase).
- `program_id_hex` and on-chain PDAs must not change — guest stays at `v0.2.0` LEZ.

## Order of operations (with rationale)

The order is forced by a dependency: our `logos-delivery-module` flake input points at our `logos-delivery` branch. So `logos-delivery` must be rebased first (or at least its branch ref must remain valid throughout), then the module flake re-locked against the rebased delivery rev.

```
Phase A: logos-delivery rebase           (the hard one — api-shape phase2 merge)
   ↓
Phase B: logos-delivery-module rebase    (easy — CI/release/docs only)
   ↓ (re-locks logos-delivery flake input to Phase A result)
Phase C: wallet module patch verification (parallel to A/B — independent repo)
   ↓
Phase D: payment-streams repo consume    (refresh flake locks, rebuild, E2E)
   ↓
Phase E: doc updates                     (pin tables, path references)
```

Phase C (wallet module) can run in parallel with A/B since it is a separate repo with no cross-dependency on the delivery rebases. Listed after B only because it is lower priority and benefits from the same hermetic-build verification step in Phase D.

---

## Phase A — rebase `logos-delivery` fork onto upstream `master`

**Repo:** `logos-delivery`
**Branch:** `feat/payment-streams-store-eligibility` (tip `d214ac05`)
**Target:** `origin/master` (`cbb601ec`)

### What we are rebasing

8 fork-only commits:

```
d214ac05 Align Store eligibility N8 tests with demo e2e reference wire.
47ef62f3 Add Step 17 Store query N8 canonical parity tests.
39b467ec fix(store): retain eligibilityProof on outbound storeQuery
ed41c826 fix(store): parse eligibilityProofHex with hexToSeqByte try/except
743143e0 fix(store): import byteutils for eligibilityProofHex parsing
800107e3 feat(store): allow eligibilityProofHex in store query JSON
e59319d8 feat(store): add liblogosdelivery eligibility hooks and store query (Step 15)
d033a493 feat(store): add optional eligibility fields on Store query RPC (tag 30)
```

### ~25 upstream commits being pulled in

Key ones that overlap our files:

- `a7f89355` "Integrate api-shape phase2 (#3999)" — reshapes `library/` into a thin C-ABI shim delegating to per-layer `api/` folders. Touches `library/liblogosdelivery.h` (+30 lines, the reliable-channel ops from `89474e72` which is now in master) and `library/kernel_api/protocols/store_api.nim` (16 lines).
- `38d951a2` "Rename kernel_api dir to waku_node" — moves `waku/node/kernel_api/` → `waku/node/waku_node/` (Nim node internals, not the `library/kernel_api/` FFI shim).
- RLN phase 1–4 reshuffles (`ec36e09b`, `57ff2476`, `4ba5710a`, `cbb601ec`) — unrelated to Store/eligibility.
- `a45b7851` "messaging depend on Waku kernel, not raw WakuNode" — internal refactor.

### Conflict surface (verified)

Files both branches touch:

1. `library/liblogosdelivery.h` — upstream adds reliable-channel ops (`channel_create`/`channel_send`/`channel_close`); we add eligibility symbols (`EligibilityVerifierCb`, `logosdelivery_set_eligibility_*`, `logosdelivery_store_query`). Both additive to the same header — conflict is textual (insertion location), resolvable by keeping both blocks.
2. `library/kernel_api/protocols/store_api.nim` — upstream thins it to delegate to the new `api/` layer (16 lines changed); our fork added `eligibilityProofHex` parsing here. Conflict requires re-applying our JSON parsing changes against the thinned shim. The thinning changed how `waku_store_query` delegates; our `logosdelivery_store_query` adaptation must be re-checked against the new delegation pattern.

Files only we touch (no conflict, but path may need update if moved):

- `library/store_eligibility/eligibility_api.nim`, `library/store_eligibility/store_query_json.nim` — our new files; phase2 does not touch `store_eligibility/`. Safe.
- `logos_delivery/waku/waku_store/eligibility_canonical.nim`, `eligibility_hooks.nim` — our new files; phase2 does not touch `waku_store/` eligibility files. Safe.
- `logos_delivery/waku/waku_store/protocol.nim`, `rpc_codec.nim`, `common.nim` — our tag-30 additions; phase2 does not touch these. Safe.

### Steps

1. **Pre-flight:** tag current fork tip for rollback: `git tag pre-rebase-delivery-fork d214ac05`.
2. **Rebase:** `git rebase origin/master` on `feat/payment-streams-store-eligibility`.
3. **Resolve conflicts:**
   - `library/liblogosdelivery.h`: keep both upstream reliable-channel ops and our eligibility block.
   - `library/kernel_api/protocols/store_api.nim`: re-apply `eligibilityProofHex` parsing against the new thinned delegation. Verify our `logosdelivery_store_query` adaptation still delegates correctly (it was modeled on the old `waku_store_query`; the new shim may have changed the delegation target).
4. **Build smoke:** `make liblogosdelivery` — must produce `build/liblogosdelivery.so`.
5. **Eligibility smoke:** `make logosdelivery_eligibility_smoke` — exercises the C ABI smoke test (`library/tests/test_eligibility_hooks.c`).
6. **Nim parity tests:** `nimble buildTest tests/waku_store/test_store_eligibility_canonical.nim` and `test_store_eligibility_hooks.nim` — run the binaries. These verify N8 canonical-bytes parity with Rust.
7. **Push:** force-push the rebased branch to the fork remote (`s-tikhomirov/logos-delivery` or `logos-messaging/logos-delivery` fork). Record the new tip rev.
8. **Rollback:** if any of steps 4–6 fail, `git reset --hard pre-rebase-delivery-fork` and report.

### Risk

Medium-high. The api-shape phase2 reshuffle is the largest upstream change. Mitigations: the eligibility surface is additive (new C exports, new subdirectory, new Nim files), and phase2 does not touch `store_eligibility/` or `waku_store/eligibility_*`. The real risk is in `store_api.nim` where our JSON parsing changes meet the thinned delegation — that needs careful re-application, not a blind `git checkout --ours`.

---

## Phase B — rebase `logos-delivery-module` fork onto upstream `master`

**Repo:** `logos-delivery-module`
**Branch:** `feat/payment-streams-store-eligibility` (tip `9361e49`)
**Target:** `origin/master` (`c21ffb8`)

### What we are rebasing

8 fork-only commits (the Step 16 bridge):

```
9361e49 fix(eligibility): marshal module verify calls on LogosAPIClient thread
4e4d370 chore: pin logos-delivery ed41c826
be85dde chore: bump logos-delivery pin for byteutils fix (743143e0)
132768f chore: pin logos-delivery eligibilityProofHex store query support
1aa4083 fix(eligibility): introspect object getPluginMethods and flat prepare JSON
bf104a6 feat(step-16): wire Store eligibility bridge and async storeQuery
ef64fa0 test: add Approach A thread probe for Step 16 storeQuery
5e86d2b chore(flake): pin logos-delivery to payment-streams store eligibility branch
```

### 7 upstream commits being pulled in

All CI/release/docs, none touch `src/delivery_module_plugin.{h,cpp}` or `src/delivery_eligibility.{h,cpp}`:

```
c21ffb8 chore: bump logos-module-builder (#60)
620225e flake: add Logos Attic public cache as substituter (#63)
3a4234f ci: switch Nix binary cache from Cachix to Attic (#61)
ab904fd docs: add prebuilt-binary run path to run-node guide (#54)
794c21c chore: release v0.1.3 (#57)
9e6d2f6 feat: expose node metrics via collectOpenMetricsText() for openmetrics (#55)
b43a6e5 docs: point run-node guide at logos.test fleet (#53)
```

### Conflict surface

Only `flake.lock` (the `logos-delivery` input rev will have moved after Phase A). The bridge source files (`delivery_eligibility.*`, `delivery_module_plugin.*`) have zero upstream changes. Expected to rebase cleanly.

### Steps

1. **Pre-flight:** tag current tip: `git tag pre-rebase-delivery-module-fork 9361e49`.
2. **Rebase:** `git rebase origin/master`.
3. **Update flake input:** after Phase A, update `flake.nix` `logos-delivery` input to point at the rebased `feat/payment-streams-store-eligibility` branch (it already does — same branch name, new tip). Run `nix flake update logos-delivery` to refresh `flake.lock` to the Phase A tip rev.
4. **Build smoke:** `nix build .#lgx` — must produce the `delivery_module` `.lgx`.
5. **Unit tests:** `nix build .#tests` or `make test` — must pass `test_delivery_eligibility`, `test_delivery_eligibility_json`, `test_approach_a_thread_probe`.
6. **Push:** force-push rebased branch to `s-tikhomirov/logos-delivery-module`. Record new tip rev.
7. **Rollback:** `git reset --hard pre-rebase-delivery-module-fork` on failure.

### Risk

Low. Pure CI/docs rebase + flake lock refresh. The bridge code is untouched upstream.

---

## Phase C — verify wallet module patches against latest `main`

**Repo:** `logos-execution-zone-module` (upstream, not a fork)
**Patches:** in `lez-payment-streams/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`

### What upstream moved (10 commits past our implicit pin)

```
b555cd5 chore: bump to v0.2.0 (#40)
9bed110 bump to v0.2.0
d70225c Merge #38
38515bd feat: add identifiers, withdraw methods and vault claim functions
... (mnemonic updates, nix-bundle-lgx bumps)
```

### Surface we depend on (verified stable)

- `send_generic_public_transaction(account_ids, signing_requirements, instruction, program_id_hex)` — still 4-arg on `main`. Our `wallet-qt-send-generic-public-transaction-json.patch` should apply.
- `sign_public_payload` — still absent upstream. Our `wallet-qt-sign-public-payload.patch` still needed.
- CMake FFI include — our `wallet-qt-cmake-ffi-include.patch` should apply.

### Additive surface we pick up (no action needed, no risk)

- `bridge_withdraw`, `vault_claim`, `vault_claim_private` — new `Q_INVOKABLE` slots, additive.

### Steps

1. **Refresh upstream:** `git fetch origin` in the upstream checkout. (Already at `b555cd5`.)
2. **Verify patches apply:** run the wrapper flake build:
   ```bash
   cd lez-payment-streams/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched
   nix build .#lib
   ```
   If a patch fails to apply, inspect the conflict — most likely `wallet-qt-sign-public-payload.patch` hits a context drift in `logos_execution_zone_wallet_module.cpp` where PR #38 added new methods near our patch site.
3. **Fix patches if needed:** adjust the patch context lines (the `+++`/`---` hunk headers) to match the new file layout. The semantic changes we add (`sign_public_payload` method, JSON submit helper) are unchanged.
4. **Verify wallet `.lgx` builds:**
   ```bash
   cd lez-payment-streams
   ./scripts/archive/build-wallet-lgx.sh
   ```
5. **Smoke:** `logoscore` loads `logos_execution_zone` and `get_account_public` returns. (Covered in Phase D E2E.)

### Risk

Low. Upstream changes are additive methods + version bumps. The 4-arg signature we depend on is unchanged. Worst case is a patch-context drift, which is a 5-line fix.

---

## Phase D — refresh payment-streams repo + verify

**Repo:** `lez-payment-streams`

### Steps

1. **Re-lock the module flake:**
   ```bash
   cd logos-payment-streams-module
   nix flake update logos-delivery
   git diff flake.lock  # verify the rev moved to Phase A tip
   ```
   The `logos-delivery-module` is consumed as a sibling checkout (`DELIVERY_MODULE_ROOT`), not a flake input here — no lock refresh needed for it in this repo. But if `scripts/e2e.sh` references a locked rev, update it.
2. **Rebuild the module bundle:**
   ```bash
   nix build .#lgx
   ```
3. **Rebuild FFI + wallet:**
   ```bash
   nix build .#payment-streams-ffi
   ./scripts/archive/build-wallet-lgx.sh
   ```
4. **Hermetic E2E (no overlay):**
   ```bash
   SKIP_LIBLOGOSDELIVERY_OVERLAY=1 make verify-step17
   ```
   This is the critical gate. It forces the `delivery_module` to use the `liblogosdelivery.so` bundled from the rebased `logos-delivery` flake input (Phase A), not a sibling overlay. If this passes, the rebased delivery stack is wire-compatible with the payment-streams module.
5. **Overlay E2E (sanity):**
   ```bash
   make verify-step17
   ```
   Confirms the overlay path (which copies `build/liblogosdelivery.so` from a sibling `logos-delivery` checkout) still works against the rebased sibling.
6. **Back-to-back regression:**
   ```bash
   make verify-step17-back-to-back
   ```
   Monotonic stream ids, no state leakage across runs.

### Rollback

If Phase D step 4 fails (hermetic E2E red):
- First suspect: the `store_api.nim` re-application in Phase A produced a `logosdelivery_store_query` that does not delegate correctly.
- Second suspect: `liblogosdelivery.h` merge lost an eligibility symbol.
- Do not roll back LEZ/SPEL pins — those are unchanged and known-good.
- Roll back the delivery forks to pre-rebase tags if diagnosis points to a rebase regression.

---

## Phase E — doc updates

**Files:**
- `docs/reference/feature-branch-pins.md` — update the locked rev table (rows for `logos-delivery` and `logos-delivery-module`).
- `docs/reference/integration-contracts.md:37` — update path `logos-delivery/library/kernel_api/protocols/store_api.nim` → `logos-delivery/library/waku_node/protocols/store_api.nim` (if phase2 rename applies to the FFI shim path — verify after rebase; the rename commit `38d951a2` moved `waku/node/kernel_api/` not `library/kernel_api/`, so this path may be unchanged).
- `docs/plan/completed/step-15-normative.md:193` — same path reference.
- `docs/plan/index.md:106` — update the locked rev mention if cited.
- `docs/plan/completed/step-16.md`, `step-17.md` — update fork rev citations if they cite specific commits.

### Verification commands for the doc update

```bash
cd lez-payment-streams
# Confirm new revs
git -C ../logos-delivery log -1 --format="%H %s" feat/payment-streams-store-eligibility
git -C ../logos-delivery-module log -1 --format="%H %s" feat/payment-streams-store-eligibility
# Confirm path references after rebase
ls ../logos-delivery/library/kernel_api/protocols/store_api.nim 2>/dev/null || \
  ls ../logos-delivery/library/waku_node/protocols/store_api.nim
```

---

## Risk summary

| Phase | Risk | Mitigation |
|---|---|---|
| A (logos-delivery rebase) | Medium-high | Additive eligibility surface; phase2 does not touch `store_eligibility/` or `waku_store/eligibility_*`; rollback tag |
| B (logos-delivery-module rebase) | Low | CI/docs only upstream; bridge untouched; rollback tag |
| C (wallet module) | Low | Additive upstream; 4-arg signature stable; patch-context fix if needed |
| D (verify) | Gate | Hermetic E2E is the truth; overlay + back-to-back as follow-ups |
| E (docs) | None | Path/rev updates only |

## Decision points before I start

1. **Phase A push target:** push the rebased `logos-delivery` branch to `logos-messaging/logos-delivery` (org fork, if you have push rights) or to `s-tikhomirov/logos-delivery` (personal fork)? The `flake.nix` in `logos-delivery-module` currently points at `github.com/logos-messaging/logos-delivery?ref=feat/payment-streams-store-eligibility` — if we push to the personal fork, we must update that URL too.

2. **Phase B push target:** same question for `s-tikhomirov/logos-delivery-module` vs `logos-co/logos-delivery-module`.

3. **Should I run Phase A and Phase C in parallel?** Phase C is independent of A/B. Running it first would surface any wallet patch breakage early, before committing to the delivery rebase. I would recommend doing Phase C first (cheap, surfaces risk), then Phase A (hard), then Phase B (easy), then Phase D (verify).

Ready to start when you confirm the push targets and the ordering preference. I recommend Phase C → A → B → D → E.