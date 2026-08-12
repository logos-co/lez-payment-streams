# Step 45 — revisit dependencies and patches

Upcoming. Index: [index.md](../index.md).
Pins SSOT after freeze: [feature-branch-pins.md](../../reference/feature-branch-pins.md).
Follow-up (program-graph unify + drop AT hex): [Step 48](step-48-program-graph-lez-unify.md).

Absorbed raw TODOs (now this packet): PATH wallet vs LEZ pin; wallet Nix patch
inventory; spel vendor / IDL path; eligibility fork rebase onto `master`.
Parked (unchanged): private-account identifier hygiene; wrapped-native token.

## Goal

Freeze a reproducible wrap-up state: published pins, explicit patch inventory,
fetchable eligibility forks, and no tribal knowledge about split LEZ versions or
unpushed branches — **without** forcing host/guest LEZ unification (that is Step 48).

## Pin policy (locked)

Two LEZ pins on purpose. Do not “fix” the divergence.

| Layer | LEZ | Spel | What |
| --- | --- | --- | --- |
| Operator stack | `v0.2.4` (`47eba256479f6f785acbd138834340703cd03401`) | `v0.6.0` (`0cb7e0980535af619482cf1c823f4d394b3ebd61`) in `scaffold.toml` | `scaffold.toml [repos.lez]`, Nix wallet / `lez-wallet-ffi-patched`, PATH `wallet`, `tools/lez-testnet-submit`, `scripts/archive/testnet-common.sh` `LEZ_OP_REV` |
| Program graph | `v0.2.0` (`a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a`) | same `v0.6.0` tag-only (or C-fails: stay vendored `v0.5.0`) | `lez-payment-streams-core`, `-ffi`, `methods/guest`, `examples`, `nix/payment-streams-ffi.nix` |

Also lock: LEZ-module SHA `549cf1159f20fa0c3fe8e88a5ab71de68a5aa34b`.
Do not pin LEZ `dev` tip. Testnet marketing name `v0.2.1` is not a software pin.
Retire the old pins rule that `scaffold.toml [repos.lez].pin` must equal
`nix/payment-streams-ffi.nix` rev.

Other SHAs (reference only): LEZ `v0.2.1` `15144ddb…`, `v0.2.2` `d6e4ae69…`,
`v0.2.3` `43b66b15…`; current spel `v0.5.0` in scaffold `73fc462e…`.

## Locked decisions

| ID | Decision |
| --- | --- |
| D45.1 | Wrap-up maintenance, not new product. |
| D45.2 | Prefer published tags / locked SHAs. No spel vendor tree and no LEZ Cargo `[patch]` for this freeze. |
| D45.3 | Spel `v0.6.0` likely changes ImageID even with program-graph LEZ still `v0.2.0`. Record ImageID + ELF, redeploy, sweep fixtures. |
| D45.4 | Step 47 ABI already on eligibility forks; freeze those tips (no `requester_*`). |
| D45.5 | Pin policy table above. |
| D45.6 | LEZ-module at SHA above; refresh in-repo Nix patches; do not push local module `main` experiments upstream. |
| D45.7 | Spel `v0.6.0` tag-only in scaffold, guest, **core**, and **examples**; drop `vendor/spel-*` and path `[patch]` at root **and** `examples/`. Add `nssa_core = { package = "lee_core", … }` wherever stock macros expand (guest; check core in Phase 0). Gate: guest builds; ImageID + ELF; IDL byte-identical across two runs. **C-fails:** if stock `v0.6.0` will not build against program-graph `v0.2.0`, keep vendored `v0.5.0`, skip spel bump, keep current ImageID, finish the rest of Step 45, defer spel+graph unify to Step 48. |
| D45.8–D45.9 | Rebase delivery then delivery-module eligibility onto recorded `origin/master` SHAs (not delivery tag v0.38.1). Push **pre-rebase refs** before force-push. Apply D45.13 on the module rebase. |
| D45.10 | Delivery eligibility on `logos-messaging`; module eligibility stays on personal fork `s-tikhomirov` (no org move). Contingency: re-fork + URL; re-lock flake by recorded rev after force-push. |
| D45.11 | Patch-only delivery: rejected. |
| D45.12 | Tier A local required (Phase 1–2). Soft-only for this freeze (`RISC0_DEV_MODE=1`); module real-prove and Store soft full privacy = Tier C / later. Public testnet Tier A (Phase 3) deferred to Step 48. |
| D45.13 | On module rebase: keep unpaid sync `storeQuery(…)` → kernel. Rename paid async path to `storeQueryWithEligibility` / `storeQueryWithEligibilityCompleted` (payload unchanged). Keep delivery C `logosdelivery_store_query`. Do not make paid path sync. Anti-bypass: `store_query_missing_proof` still fails closed. Update orchestrator checks for both old and new event names. |
| D45.14 | Keep `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` as **documented testnet config** (required under the split: FFI AT id is program-graph `v0.2.0`, live sequencer is `v0.2.2+` / `fe96c422…`). Drop = Step 48. Keep `ps_ensure_wallet_statistics` / Python twin as supported ≥0.2.2 `open` args. Drop `wallet-v021.sh` and wallet-shim. Keep dual `NSSA_WALLET_HOME_DIR` + `LEE_WALLET_HOME_DIR` exports. |
| D45.15 | Invert `ps_prepend_lez_wallet_path`: prefer scaffold `v0.2.4` `target/release` over `~/.cargo/bin/wallet`. Delete shim-dir + `wallet-v021.sh`. Phase 1: `command -v wallet` under scaffold cache (unless Phase 0 schema failure-branch recorded). |
| D45.16 | **Keep** `lez-testnet-submit` (Required testnet bootstrap). Bump to operator LEZ `v0.2.4`. It is the AT **id and ELF** source for `bootstrap_testnet_fixture` / `ensure-testnet-vault` (no ELF override flag today). Rebuild release binary; assert `auth-transfer-program-id-hex` == O45.6 operator/live hex (stale `target/release` silently keeps old AT). Record helper LEZ rev in gate-log Pins. Rollback = revert helper crate/lockfile only. Removing the helper (WalletCore port) is **Tier C / later**, and must add AT hex + ELF inject flags first. |
| D45.17 | Freeze pin sites: scaffold; program-graph Cargo + `nix/payment-streams-ffi.nix`; spel in core/guest/examples; lockfiles root + `examples/` + `methods/guest/` + `tools/lez-testnet-submit/`; Nix wallet/module/delivery locks. Delete `vendor/spel-*` unless C-fails. Rewrite pins doc for the split. |
| D45.18 | Required: fix broken live `create-*-stream-fixture` fallbacks without calling `scripts/archive/` (live code must not depend on archive); drop `wallet-v021.sh`; **move** `testnet-common.sh` out of `archive/` into live `scripts/lib/` and set `LEZ_OP_REV` to operator `47eba256…`. Optional later: demos, old `verify-step10*`–`18*`, helper removal. |

Closed outcome IDs (same content as above; for gate-log cross-ref):

| ID | Outcome |
| --- | --- |
| O45.1 | Program graph stays on LEZ `v0.2.0` with stock spel `v0.6.0` (C). C-fails → Step 48. Reject forcing guest onto `v0.2.4` via vendor or LEZ `[patch]`. |
| O45.2 | Keep submit helper; operator pin. Removal = optional later. |
| O45.3 | Execution gate: rebase census; stop and defer rebase if API re-adaptation needed. |
| O45.4 | Module eligibility stays on personal fork. |
| O45.5 | Keep AT hex as documented config; drop in Step 48. Out of Step 45: script-harden all entrypoints to auto-fill AT; module runtime `getProgramIds`. |
| O45.6 | Execution gate: record program-graph FFI AT id, operator `v0.2.4` AT id, live testnet AT id. |

D45.13 names locked: `storeQueryWithEligibility` / `storeQueryWithEligibilityCompleted`.

## Implementor order

Do this sequence. Verification details are under [Verification](#verification-phased-dod).

0. **Phase 0 (no freeze commits)**
   - Stock spel `v0.6.0` + program-graph LEZ `v0.2.0` guest (+ core `nssa_core` alias) build. Red → C-fails (skip spel bump; keep ImageID).
   - Wallet schema probe: write home with operator `v0.2.4` CLI; read with program-graph `v0.2.0` `WalletCore`. Exposure is mainly **localnet seed** (`seed_localnet_fixture`); testnet bootstrap under O45.2 forwards files to the helper. **If red:** refresh localnet wallet home with
     `~/.cache/logos-scaffold/repos/lez/a58fbce…/target/release/wallet` as a narrow D45.15 exception for that seed path; gate-log it; dies in Step 48. Do not stall later steps.
   - O45.3 rebase census / throwaway rebase; apply stop rule.
   - O45.6 record three AT hexes.
   - Scratch `lgs setup` on LEZ `v0.2.4`.
0b. Skipped for this freeze (old ImageID streams not treated as valued).
1. Inventory + rewrite `feature-branch-pins.md` (split, patch inventory, shims, helper AT id/ELF note, live `testnet-common`).
2. Operator stack to `v0.2.4` + D45.6 module SHA; PATH invert (D45.15); bump + **rebuild** submit helper; AT id assert vs O45.6; move `testnet-common` live and set `LEZ_OP_REV` to operator SHA. Do **not** `full-reset-localnet` yet.
   Expected-red until step 3: snapshot `lez_pin` ≠ new scaffold pin — restores abort until `make full-reset-localnet`.
3. Spel `v0.6.0` (unless C-fails); drop vendor; guest build; ImageID + ELF; IDL two-run identical; **`make full-reset-localnet`**; local fixture sweep (manifest below). Public `deploy-testnet` / testnet fixture sync deferred to Step 48.
4. Phase 1 verification (soft-only).
5. Pre-rebase Store hermetic on new ImageID + **old** delivery pins (`SKIP_LIBLOGOSDELIVERY_OVERLAY=1`).
6. Pre-rebase refs; rebase delivery → org feature branch; rebase delivery-module (D45.13) → personal fork feature branch (or stop per O45.3); re-lock; flake.lock = pins tip. Force-push feature branches only after Phase 2 green. Never push `master`/`main`.
7. Phase 2. Phase 3 deferred to Step 48.
8. D45.18 required cleanups; optional Tier C archive / helper removal.
9. Pins freeze; old-ImageID grep-clean; move packet to `completed/`; update index / AGENTS (fix stale `072a26cc` in AGENTS).

## Out of scope

- Program-graph LEZ → `v0.2.4` / drop AT hex ([Step 48](step-48-program-graph-lez-unify.md)).
- Forcing that unify via vendor, spel-fork, or LEZ `[patch]`.
- Script-harden every testnet entrypoint to auto-fill AT hex; module runtime `getProgramIds`.
- Required removal of `lez-testnet-submit`.
- Hosting module eligibility on `logos-messaging`.
- Replacing patched wallet in Store before upstream signing APIs; LIP-155 wire renames.
- Steps 20 / 46; parked raw TODOs; Tier C unless claimed.
- Hard-delete of still-documented maintainer archives.
- Delivery rebase that fails O45.3 (separate packet).
- Public testnet Tier A cells (Phase 3) — deferred to [Step 48](step-48-program-graph-lez-unify.md).
- Module real-prove / CLOCK_50 (soft-only freeze for Step 45).

## Verification (phased DoD)

Matrix: [verification-matrix.md](../../reference/verification-matrix.md).
Gate log: `docs/plan/completed/step-45-gate-log.md` (create when executing).

| Column | Content |
| --- | --- |
| Date | UTC |
| Repo commit | payment-streams tip |
| Cell | e.g. `module-local`, `store-local-hermetic-pre-rebase` |
| Artifact | `.scaffold/e2e/artifacts/…` |
| Result | pass / fail |
| ImageID | guest hex |
| ELF size | every row after ImageID cut |
| RISC0_DEV_MODE | 0 / 1 |
| Pins | operator LEZ, program-graph LEZ, spel SHA, helper LEZ rev, module / delivery / delivery-module SHAs, flake.lock revs |
| Clock | real-prove: CLOCK_50 |
| Notes | AT hex set (O45.5), hermetic flag, schema exception, fail cause, grep-gate cmd |

### ImageID sweep manifest

Update when ImageID cuts: `fixtures/localnet.json`, `testnet-module.json`,
`testnet.json`, `testnet.json.example`; `scripts/e2e/ensure-testnet-vault.sh`;
`scripts/bootstrap-testnet-module.sh`; `README.md`; `AGENTS.md` (stale `072a26cc`);
archive copies under `scripts/archive/{bootstrap-testnet,create-testnet-stream-fixture}.sh`
(update or leave clearly historical).

### Phase 1 — after operator + guest/spel cut + snapshot

1. Build sanity — guest; ImageID + ELF; IDL two-run identical; no `vendor/spel-*`
   (unless C-fails); spel `v0.6.0` in core/guest/examples (unless C-fails);
   operator scaffold/Nix = `v0.2.4`; program-graph Cargo + `payment-streams-ffi.nix`
   = `v0.2.0`; PATH wallet under scaffold cache (or recorded schema exception);
   helper rebuilt + AT id assert; lockfiles committed.
2. Root workspace: `cargo clippy --workspace` and `RISC0_DEV_MODE=1 cargo test --workspace`
   (members: core, ffi, methods only).
3. Also build/clippy: `examples/Cargo.toml`; `tools/lez-testnet-submit/Cargo.toml`
   (`--release` for helper assert).
4. `make check-terminology`.
5–7. `verify-module-local`, `verify-module-local-payee-close`,
   `verify-module-local-close-negatives`.
8–9. Soft privacy / real-prove: Tier C for this freeze (not required).

Guardrails: no unrebased Store freeze claim; no force-push without pre-rebase
refs; no force-push until Phase 2 green; never push `master`/`main`; no LEZ
`[patch]` / re-vendor unless C-fails; live scripts must not call `scripts/archive/`.

### Phase 2 — after delivery rebase + re-lock

Pre-rebase: Store hermetic on new ImageID + old delivery pins
(`SKIP_LIBLOGOSDELIVERY_OVERLAY=1`, `E2E_CLAIM_OPTIONAL=0`).

After rebase:

10. Same Store hermetic on new delivery pins.
11. Module nix `tests` / `result-tests` (incl. thread probe, `storeQuery_*`).
12. Delivery Nim store/eligibility/codec tests via `all_tests_waku.nim`.
13. flake.lock revs = pins tips (delivery-module + payment-streams-module/wrappers).
14. `store_query_missing_proof` still fails closed.

### Phase 3 — public testnet (deferred to Step 48)

15–16. `verify-module-testnet` and `verify-store-testnet` (`E2E_CLAIM_OPTIONAL=0`)
with `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX` **set** to live/operator AT id
(O45.5). Two-consecutive-green only for these Tier A cells. Not required to close
Step 45.

### Tier C — optional

- Soft module privacy / real-prove (`RISC0_DEV_MODE=0` + CLOCK_50).
- Store soft full privacy; testnet real-prove privacy; `verify-store-local-lifecycle`;
  full USER_JOURNEY re-walk.
- Remove `lez-testnet-submit` after WalletCore port + AT hex/ELF inject flags;
  re-run module + store testnet.
- Broad archive of demos / old `verify-step10*`–`18*` aliases.

## Done when

- O45.1–O45.6 outcomes recorded (C or C-fails; helper kept; AT config kept;
  rebase proceed/defer; three AT hexes).
- Pins rewritten for the split; D45.17 freeze set landed; helper AT assert green;
  `LEZ_OP_REV` = operator SHA; D45.18 required cleanups done (`testnet-common`
  live under `scripts/lib/`; no live→archive script calls).
- Phase 1 + Phase 2 Tier A (soft-only) in the gate log. Phase 3 deferred to Step 48.
- Packet in `docs/plan/completed/`; index / AGENTS updated.
