# Step 26 — plan excerpt

Completed packet. Index: [index.md](../index.md).
Prerequisites: Step 18 (rc5 testnet integration, superseded); Step 24b/24c
(rc5 guest + tooling, demo flow).
Related: [step-27-claim-fix-verification.md](../completed/step-27-claim-fix-verification.md),
[step-30-static-dependency-migration.md](../completed/step-30-static-dependency-migration.md),
[feature-branch-pins.md](../../reference/feature-branch-pins.md),
[integration-decisions.md](../../reference/integration-decisions.md) (N10, N16).

## Status (2026-06-30)

Step 26 is complete. Payment streams integration runs on LEZ `v0.2.0`
(git rev `a58fbce2…`). The operational pin, wallet wrapper ref, Qt patch
rewrite, testnet-submit dispatch removal, and localnet baseline rebuild
are all landed on `master`. The Store-mode E2E blocker (owner prefund)
was resolved by [Step 27](../completed/step-27-claim-fix-verification.md);
all three localnet verification gates are green.

| Area | State |
| --- | --- |
| LEZ pin `a58fbce2…` across repo | Done; `grep -rl 27360cb7` returns only archived DoD scripts + historical packets |
| Wallet wrapper on `main`; 4-arg Qt patch | Done; `nix build ./logos-payment-streams-module#lgx` succeeds |
| `chainUsesTestnetSubmit()` always `false` | Done; all writes via `submitGenericPublicViaFfi` |
| `tools/lez-testnet-submit/` compiles on v0.2.0 | Done (fallback, not dispatched) |
| `nix build .#payment-streams-ffi` | Green (33/33 FFI tests) |
| `make program-id` | Produces `program_id_hex` `16b95d37…` |
| `make full-reset-localnet` | Green (owner prefunded via pinata; snapshot rebuilt) |
| `MODE=module CHAIN=local` E2E (incl. `claim`) | Green |
| `MODE=store CHAIN=local` E2E (incl. `demo_claim`) | Green |
| `make verify-step17-back-to-back` (two legs) | Green |
| Step 18 packets superseded banner | Done |
| N10 / N16 / D6 amendments | Done (D6 static-dep migration deferred to Step 30) |

Live testnet deployment (guest deploy, `program_id_hex` recording, read
smoke) remains deferred as noted in the step scope; the Developer Journey
testnet claim is owned by [Step 27](../completed/step-27-claim-fix-verification.md).

---

## Original packet

Index: [index.md](../index.md).

### Step 26, TestNet v0.2 Migration

Migrate payment streams integration to Logos Execution Zone TestNet v0.2.
Bump the operational LEZ pin, move the wallet module wrapper off the closed
PR 19 ref onto upstream `main` (now Universal + the v0.2.0 FFI surface),
rewrite the wallet Qt patch against the new `send_generic_public_transaction`
signature, stop dispatching testnet writes through the `lez-testnet-submit`
helper, and re-establish the localnet baseline against the new pin.

Live testnet deployment (guest deploy, `program_id_hex` recording, read smoke
against `https://testnet.lez.logos.co/`) is deferred to a follow-up phase after
the localnet path is green on v0.2.0; this step covers everything reachable
without live testnet access.

Prerequisite: Upstream LEZ release tag
[`v0.2.0`](https://github.com/logos-blockchain/logos-execution-zone/tree/v0.2.0)
(git rev `a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a`). This step replaces Step 18
testnet integration (rc5-era chain). Pin **`v0.2.0` directly** — do not stop at
`v0.2.0-rc6` or other release candidates.

#### Migration scope

| Component | From | To |
|-----------|------|-----|
| LEZ version | `v0.2.0-rc5` (`27360cb7…`) | [`v0.2.0`](https://github.com/logos-blockchain/logos-execution-zone/tree/v0.2.0) (`a58fbce2…`) |
| Wallet module upstream ref | `refs/pull/19/head` (closed draft, Legacy Qt, 5-arg `ProgramWithDependencies` shape) | `main` (open, Universal, 4-arg `program_id_hex` shape) |
| TestNet endpoint | `https://testnet.lez.logos.co/` | Same URL, new chain version (deferred — no live calls in this step) |
| Guest program | Prior `program_id_hex` | New deployment post-migration (deferred to follow-up phase) |
| Wallet Qt patch | `wallet-qt-send-generic-public-transaction-json.patch` (5-arg `ProgramWithDependencies` delegation) | Rewritten against `main`'s 4-arg `send_generic_public_transaction(account_ids, signing_requirements, instruction, program_id_hex)` |
| Module testnet dispatch | `chainUsesTestnetSubmit()` → `lez-testnet-submit` helper | `chainUsesTestnetSubmit()` always returns `false`; all writes route through `submitGenericPublicViaFfi` |

Operational pin: one git revision for scaffold, Nix LEZ fetch, wallet flakes, Rust
`lee` / `lee_core`, module `.lgx`, and testnet helpers — same policy as Step 24b,
but at `v0.2.0` instead of rc5. Record the full rev in
[feature-branch-pins.md](../../reference/feature-branch-pins.md) when complete.

#### Work branch and commit policy

Branch name: `feat/testnet-v0.2-migration`. Commit but do not push until the
localnet gates pass; push and PR target decided in the live-testnet follow-up.

Commit split (4 commits, by concern):

1. `pins` — all LEZ rev bumps + lockfile refreshes (atomic; repo does not build
   mid-bump, so a single commit is correct).
2. `wallet-qt-patch` — wrapper flake ref move (`refs/pull/19/head` → `main`),
   all four wallet patches rebased, `buildGenericPublicPayloadJson` C++ change.
3. `module-dispatch` — `chainUsesTestnetSubmit()` no-op + comment.
4. `docs` — `feature-branch-pins.md`, N16 amendment, Step 18 banners, archived
   DoD script comments, `feature-branch-pins.md:205` verification-commands line.

#### Prerequisites (host)

The verification gates assume a host with:

- `/tmp/lbc-pol-v0.5.0/logos-blockchain-circuits-v0.5.0-linux-x86_64/` present
  (`lez-wallet-ffi-patched/flake.nix` hardcodes this `lbcBase` path for the
  circuit libs; unchanged by v0.2.0).
- `lgs setup` run against the new `a58fbce2…` pin so
  `~/.cache/logos-scaffold/repos/lez/a58fbce2…/` is populated before
  `make full-reset-localnet`.
- Nix sandbox able to fetch `fetchFromGitHub` rev `a58fbce2…`.

#### Deliver

##### Pin bump (rc5 → v0.2.0)

Exhaustive file list (verified via `grep -rl 27360cb7`, excluding `Cargo.lock`,
`docs/`, `target/`, `.scaffold/`, and the two archived DoD scripts that stay on
rc5 per "Archived rc5-asserting scripts" below):

- `scaffold.toml` (`[repos.lez].pin`)
- `Cargo.toml` (root, comment only — "LEZ git pin ... v0.2.0-rc5" → v0.2.0)
- `nix/payment-streams-ffi.nix` (`fetchFromGitHub.rev` + `sha256`; the `sha256`
  is computed via `nix-prefetch`, not hardcoded)
- `lez-payment-streams-ffi/Cargo.toml` (`lee`, `lee_core` rev)
- `lez-payment-streams-core/Cargo.toml` (`lee`, `lee_core`, `clock_core` rev)
- `methods/guest/Cargo.toml` (`lee_core`, `authenticated_transfer_core` rev)
- `examples/Cargo.toml` (`lee`, `lee_core`, `common`, `sequencer_service_rpc`,
  `wallet` rev)
- `tools/lez-testnet-submit/Cargo.toml` + `flake.nix` (LEZ rev; helper kept in
  tree, see "Helper retirement" below)
- `vendor/spel-framework-core/Cargo.toml` (`lee`, `lee_core` rev)
- `vendor/spel-framework/Cargo.toml` (`lee` rev)
- `logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/flake.nix`
  (LEZ input rev — note: the `upstream` input ref change is covered separately
  under "Wallet wrapper ref move" below)
- `logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/lez-wallet-ffi-patched/flake.nix`
  (`inputs.lez.rev`)
- `scripts/archive/testnet-common.sh` (`LEZ_OP_REV` default)

Lockfile refreshes (run `cargo update -p` for the LEZ git deps, or
`cargo build` to regenerate):

- `Cargo.lock`, `examples/Cargo.lock`, `methods/guest/Cargo.lock`,
  `tools/lez-testnet-submit/Cargo.lock`
- `logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/flake.lock`
- `logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/lez-wallet-ffi-patched/flake.lock`

Note: `logos-payment-streams-module/` has no `Cargo.toml` (CMake/Qt module,
not Rust) — no Rust pin bump there.

Post-bump verification: re-run `grep -rl 27360cb7` across the repo (excluding
`docs/`, `target/`, `.scaffold/`); the only remaining hits should be the two
archived DoD scripts left on rc5 intentionally.

##### Wallet wrapper ref move (PR 19 closed → `main`)

PR 19 (`refs/pull/19/head`) was closed on 2026-06-19 (not merged). The
generic-transactions work landed on upstream `main`, which is now a Universal
module (`metadata.json` declares `"interface": "universal"` with codegen) and
carries the new 4-arg `send_generic_public_transaction(account_ids,
signing_requirements, instruction, program_id_hex)` signature (v0.2.0 dropped
`ProgramWithDependencies` in favour of `ProgramId` — commit `c74b035`
"refactor!(wallet): pass program_id instead of program in send_pub_tx()").

Change the wrapper flake's `upstream` input:

```nix
# before
upstream.url = "github:logos-blockchain/logos-execution-zone-module?ref=refs/pull/19/head";
# after
upstream.url = "github:logos-blockchain/logos-execution-zone-module";
```

Refresh `flake.lock` so `upstream` resolves to the current `main` HEAD;
record the resolved SHA in `feature-branch-pins.md`.

Flagged risk (requires build-time confirmation): the wrapper flake's build
scaffolding (`patchWalletInclude` + `addSdkApiHeaders` manual
`logos-cpp-generator` call) was written for PR 19's Legacy Qt-plugin shape.
Upstream `main` is Universal and may build via `mkLogosModule` with its own
codegen, in which case `addSdkApiHeaders` becomes redundant or its
`logos-cpp-generator` invocation needs adjustment. The `nix build
./logos-payment-streams-module#lgx` gate will surface this; treat the wrapper
flake's `patchWalletInclude` / `addSdkApiHeaders` logic as possibly needing
rework, not just a ref bump.

##### Wallet Qt patch rewrite (Path 1) + patch-set audit

Rewrite `wallet-qt-send-generic-public-transaction-json.patch` against the
`main` wallet module. The patched
`send_generic_public_transaction_json(const QString& payload_json)` will:

- Parse the same JSON payload, but read `program_id_hex` instead of
  `program_elf_hex` / `program_dependencies_hex`.
- Call `wallet_ffi_serialization_helper` on `instruction_hex` to produce u32
  words (unchanged — RISC0 serde stays inside the wallet module per N10).
- Delegate to upstream `send_generic_public_transaction(account_ids,
  signing_requirements, instruction, program_id_hex)` (new 4-arg signature).

Update `payment_streams_module_writes.cpp` `buildGenericPublicPayloadJson` to
emit `program_id_hex` (from the fixture manifest) and drop
`program_elf_hex` / `program_dependencies_hex` from the payload. Keep the
`PAYMENT_STREAMS_GUEST_BIN` env knob for the deploy path
(`wallet_ffi_program_deployment`), but not for generic-public-tx submission.

Patch-set audit (all four wallet patches must apply against `main`):

- `wallet-qt-guest-elf-from-env.patch` (38 lines) — rebase if context shifted.
- `wallet-qt-sign-public-payload.patch` (71 lines) — rebase if context shifted;
  N1's `sign_public_payload` is still not upstream, so this patch stays.
- `wallet-qt-send-generic-public-transaction-json.patch` (98 lines) — full
  rewrite per above.
- `lez-wallet-ffi-patched/lez-rust-sign-public-payload.patch` (103 lines, FFI
  crate) — rebase against v0.2.0 `lez/wallet-ffi/src/` (rc5→rc6 compare showed
  `wallet.rs` +116/-12, which may shift patch context).

Rationale (Path 1 vs Path 2): dropping the patch entirely and calling upstream
`send_generic_public_transaction` directly via LogosAPI is blocked because the
RISC0 word serialization (`wallet_ffi_serialization_helper`) is not exposed as
a Q_INVOKABLE on the upstream wallet module. Reimplementing RISC0 serde in the
payment-streams module C++ would duplicate `risc0_zkvm::serde::to_vec` and is a
silent-failure hazard. The JSON wrapper patch stays as the chosen shape per
N10; v0.2.0 lets it shrink.

N10 amendment (this step): the original N10 rationale said the JSON wrapper
exists "because QList-shaped cross-module IPC to the Legacy wallet is
unreliable." With the wallet module now Universal on upstream `main`, that
specific rationale is Legacy-specific and no longer applies — Universal→Universal
QList IPC through codegen is the supported path. The JSON wrapper now persists
purely for the RISC0-serde-stays-inside-the-wallet reason (the wallet runs
`wallet_ffi_serialization_helper` on the instruction bytes; the PS module
cannot do this without either a second patch exposing that FFI as a Q_INVOKABLE
or a C++ reimplementation of RISC0 serde). Amend N10 to reflect this.

##### Helper retirement — stop using, do not delete

Unify the module write path on the in-proc wallet FFI regardless of `CHAIN`:

- In `payment_streams_module_writes.cpp`, make `chainUsesTestnetSubmit()`
  always return `false`. Keep the function and
  `submitGenericPublicViaTestnetHelper` as dead code with a comment:
  "Retirement pending live-testnet verification; dispatched unconditionally to
  FFI in Step 26. Remove `chainUsesTestnetSubmit`,
  `submitGenericPublicViaTestnetHelper`, `tools/lez-testnet-submit/`, and
  `LEZ_TESTNET_SUBMIT` plumbing once `MODE=store CHAIN=testnet` passes on the
  FFI path (live-testnet follow-up)." All writes route through
  `submitGenericPublicViaFfi`, the same path localnet already uses.
- Leave `tools/lez-testnet-submit/` in tree and compiling against v0.2.0
  (pins bumped, `Cargo.lock` refreshed). It is no longer invoked from the
  module, but stays available as a manual operator fallback.
- Leave `scripts/archive/{bootstrap,verify-step18,testnet-preflight-topup,
  create-testnet-stream-fixture,demo-e2e-local}.sh` and
  `examples/src/bin/bootstrap_testnet_fixture.rs` referencing
  `LEZ_TESTNET_SUBMIT` as-is for now; they are not on the localnet critical
  path and will be cleaned up in the live-testnet follow-up phase.

The full deletion of `tools/lez-testnet-submit/`, `lez_testnet_submit_bin()`
plumbing in `testnet-common.sh`, the `LEZ_TESTNET_SUBMIT` env knob, the
`chainUsesTestnetSubmit` function, and `submitGenericPublicViaTestnetHelper`
is deferred to a follow-up phase gated on `MODE=store CHAIN=testnet
./scripts/e2e.sh testnet run` passing against live testnet (see "Live testnet
follow-up" below). N16 Phase 9 retirement criterion becomes "live testnet E2E
green on FFI path".

Rationale: the in-proc wallet FFI (`wallet_ffi_send_generic_public_transaction`
→ `WalletCore::send_pub_tx` → `sequencer_client.send_transaction`) submits to
whatever URL is in `wallet_config.json`'s `sequencer_addr` — it is
testnet/localnet agnostic. Localnet already proves this path works. The helper
was an rc3/rc5 workaround for in-module FFI dispatch against the remote
sequencer; v0.2.0 is the right point to stop using it, but deleting it before
live verification would burn a fallback bridge.

##### Localnet reset

- Run `lgs setup` against the new `a58fbce2…` pin to populate the LEZ cache.
- Run `make full-reset-localnet` to reseed the funded baseline and redeploy
  the guest against the v0.2.0 LEZ pin (the guest ELF ImageID changes with
  the toolchain bump, so the snapshot must be rebuilt).

Pin-agnosticism confirmed: `scripts/seed-localnet-fixture.sh` derives
`LEZ_PIN` dynamically from `scaffold.toml` (`grep -A2 '[repos.lez]'`), and
`scripts/lib/common.sh::ps_program_id_hex` computes the ImageID at runtime via
`make program-id` (it does not read `fixtures/localnet.json`'s
`program_id_hex`). So `make full-reset-localnet` works against v0.2.0 unchanged
once `lgs setup` has populated the cache.

##### `program_id_hex` recording

The localnet `program_id_hex` is computed at runtime by
`ps_program_id_hex()`; `fixtures/localnet.json.example`'s
`program_id_hex: "0000…"` placeholder stays as-is. Recording a concrete
`program_id_hex` is deferred to the live-testnet follow-up (where it goes
into `fixtures/testnet.json.example` after org deploy). The DoD
"make program-id produces a program_id_hex for the rebuilt guest" is a
build-output check, not a fixture-write.

##### Archived rc5-asserting scripts

Leave `scripts/archive/verify-step11a-dod.sh` and
`scripts/archive/verify-step11d-dod.sh` pinned to rc5
(`27360cb7d6ccb2bfbcca7d171bab8a3938490264`). Add a header comment to each
stating they are historical Step 11 DoD checks pinned to rc5 and are expected
to fail on newer LEZ pins; they are no longer run as gates.

##### Documentation

- Add a "Superseded by Step 26" banner to the top of
  `docs/plan/completed/step-18-public-testnet-demo.md` and
  `docs/plan/completed/step-18b-rc5-unify-handoff.md`. Leave both files in
  `completed/` for step-map history.
- Amend [N16](../../reference/integration-decisions.md#n16-step-18b-rc5-operational-pin-2026-06):
  record the v0.2.0 operational pin, note that `lez-testnet-submit` is no
  longer dispatched from the module as of Step 26, and reframe Phase 9
  retirement as pending live-testnet verification on the FFI path.
- Amend [N10](../../reference/integration-decisions.md#n10-step-11b-module-writes-decisions):
  the JSON wrapper's "QList IPC to Legacy wallet is unreliable" rationale is
  Legacy-specific and no longer applies (wallet is now Universal on `main`);
  the wrapper now persists purely for the RISC0-serde-stays-inside-the-wallet
  reason.
- Update `feature-branch-pins.md` "Wallet — primary path" section: rc5
  references become v0.2.0; note the Qt patch rewrite (Path 1), the wrapper
  ref move (PR 19 → `main`), and the helper-retirement deferral.
- Remove the `./scripts/archive/verify-step11d-dod.sh` line from
  `feature-branch-pins.md` "Verification commands" (line 205) and replace
  with a note: "Step 11 DoD scripts under `scripts/archive/verify-step11*-dod.sh`
  are pinned to rc5 and retained as historical checks; they fail on v0.2.0 and
  are not run as gates."

##### Deferred to live-testnet follow-up (not in this step)

- Deploy guest program to TestNet v0.2 (org deployment)
- Record new `program_id_hex` in `fixtures/testnet.json.example`
- Update `make program-id` output documentation
- Verify read operations against live testnet: `sync_to_block`,
  `get_account_public`, `check-health`
- Full deletion of `tools/lez-testnet-submit/`, `LEZ_TESTNET_SUBMIT` plumbing,
  `chainUsesTestnetSubmit`, `submitGenericPublicViaTestnetHelper`
- Update / retire `scripts/archive/testnet-*` shell scripts that invoke the
  helper

##### Deferred to Step 30 (separate step, gated on this step)

D6 static-dependency migration. D6 line 224's revisit condition is now met:
the wallet module (`logos_execution_zone`) is Universal on upstream `main`, so
`payment_streams_module`'s `metadata.json` could list it in `"dependencies"`
and codegen would emit typed `modules().logos_execution_zone()` wrappers,
replacing the current dynamic
`api->getClient("logos_execution_zone")->invokeRemoteMethod(...)` dispatch
(~20+ call sites across `payment_streams_module_impl.cpp` and
`payment_streams_module_eligibility.cpp`).

This is a separable code-quality refactor, not gated on v0.2.0 or testnet,
and is intentionally excluded from Step 26 to avoid coupling two independent
risks (v0.2.0 pin compatibility + codegen-typed-wrapper migration) in one
step. Tracked as [Step 30](../completed/step-30-static-dependency-migration.md); can start
once Step 26's wrapper builds against Universal `main` and parallelizes with
Steps 27-29. Step 30 is now complete.

#### Verification gates (this step)

| Gate | Command | Pass Criteria |
|------|---------|---------------|
| Localnet E2E (module) | `MODE=module CHAIN=local ./scripts/e2e.sh local run` | All phases succeed, including `claim` |
| Localnet E2E (store) | `MODE=store CHAIN=local ./scripts/e2e.sh local run` | Store integration dual-host happy path |
| Localnet lifecycle | `make verify-step17-back-to-back` | Two Store runs on one ledger |
| Program ID | `make program-id` | Computes a `program_id_hex` for the rebuilt guest ELF (new ImageID) |
| FFI build | `nix build .#payment-streams-ffi` | FFI builds against v0.2.0 `artifacts/` |
| Module bundle | `nix build ./logos-payment-streams-module#lgx` | `.lgx` bundles against v0.2.0 wallet + all four patches apply |
| Helper build | `cd tools/lez-testnet-submit && cargo build --release` | Compiles against v0.2.0 (kept in tree, unused by module) |
| Pin sweep | `grep -rl 27360cb7 . --include='*.toml' --include='*.nix' --include='*.sh' --include='*.rs' --include='*.cpp' --include='*.h' --include='*.json' \| grep -v -E 'Cargo.lock\|docs/\|target/\|\.scaffold/\|verify-step11[ad]-dod'` | No output (only the two archived DoD scripts remain on rc5) |

`make verify-step26-testnet-read-smoke` is intentionally **not** created in
this step. The canonical localnet flow (`scripts/e2e.sh local run`) already
covers module `open`, `sync_to_block`, `get_account_public`, `chainAction`,
and `claim`. A dedicated testnet read-smoke script will be revisited in the
live-testnet follow-up phase once `MODE=store CHAIN=testnet` is exercisable.

#### Definition of done

- [x] LEZ operational pin updated to upstream `v0.2.0` (`a58fbce2…`) across
  the exhaustive file list in "Pin bump" above; `grep -rl 27360cb7` sweep
  returns only the two archived DoD scripts
- [x] Wallet wrapper `upstream` input moved from `refs/pull/19/head` (closed)
  to `main`; resolved SHA recorded in `feature-branch-pins.md`
- [x] All four wallet patches (`wallet-qt-guest-elf-from-env`,
  `wallet-qt-sign-public-payload`, `wallet-qt-send-generic-public-transaction-json`,
  `lez-rust-sign-public-payload`) rebase cleanly against `main` + v0.2.0 FFI;
  `nix build ./logos-payment-streams-module#lgx` succeeds
- [x] `wallet-qt-send-generic-public-transaction-json.patch` rewritten against
  the 4-arg `send_generic_public_transaction(account_ids,
  signing_requirements, instruction, program_id_hex)` signature; payload
  carries `program_id_hex` instead of `program_elf_hex` /
  `program_dependencies_hex`
- [x] `buildGenericPublicPayloadJson` in `payment_streams_module_writes.cpp`
  emits `program_id_hex` (no `program_elf_hex` / `program_dependencies_hex`)
- [x] `chainUsesTestnetSubmit()` always returns `false` with the deferred-
  removal comment; all writes route through `submitGenericPublicViaFfi`
- [x] `tools/lez-testnet-submit/` compiles against v0.2.0 (kept in tree as
  fallback; not invoked from the module)
- [x] `lgs setup` run against `a58fbce2…`; localnet snapshot rebuilt
  (`make full-reset-localnet`)
- [x] `MODE=module CHAIN=local ./scripts/e2e.sh local run` passes (all phases,
  including `claim`)
- [x] `MODE=store CHAIN=local ./scripts/e2e.sh local run` passes
- [x] `make verify-step17-back-to-back` passes
- [x] `nix build .#payment-streams-ffi` succeeds
- [x] `make program-id` produces a `program_id_hex` for the rebuilt guest
- [x] `feature-branch-pins.md` updated (v0.2.0 pin, wrapper ref move, Qt patch
  rewrite, helper retirement deferred, `verify-step11d-dod.sh` line removed
  from Verification commands)
- [x] N16 amended (v0.2.0 operational pin, helper no longer dispatched,
  Phase 9 reframed)
- [x] N10 amended (QList-unreliability rationale marked Legacy-specific;
  JSON wrapper now persists for RISC0-serde reason)
- [x] D6 revisit condition noted as met; static-dependency migration
  deferred to [Step 30](../completed/step-30-static-dependency-migration.md)
  (now complete)
- [x] Step 18 completed packets get "Superseded by Step 26" banner
- [x] Archived rc5-asserting DoD scripts (`verify-step11a-dod.sh`,
  `verify-step11d-dod.sh`) carry a comment noting they are expected to fail
  on newer pins

#### Non-regression guard

Localnet verification must keep passing throughout:

```bash
make verify-step17
make verify-step17-back-to-back
```

Local-LEZ paths (`CHAIN=local`) must work identically before and after
the pin bump and the Qt patch rewrite. The `program_id_hex` differs between
chains (deployed separately), but client code must remain compatible.

#### Related

- [step-18-public-testnet-demo.md](../completed/step-18-public-testnet-demo.md) — prior testnet integration (historical; superseded banner added this step)
- [step-18b-rc5-unify-handoff.md](../completed/step-18b-rc5-unify-handoff.md) — rc5 unification handoff (historical; superseded banner added this step)
- [step-27-claim-fix-verification.md](../completed/step-27-claim-fix-verification.md) — depends on this step for claim verification
- [step-28-user-journey-testnet.md](../upcoming/step-28-user-journey-testnet.md) — enables module mode on testnet
- [N10](../../reference/integration-decisions.md#n10-step-11b-module-writes-decisions) — JSON-over-LogosAPI rationale for the wallet Qt patch (Path 1 justification)
- [N16](../../reference/integration-decisions.md#n16-step-18b-rc5-operational-pin-2026-06) — operational pin policy (amended this step)
- [feature-branch-pins.md](../../reference/feature-branch-pins.md) — dependency versions
