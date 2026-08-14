# Step 53 — repository structure

Upcoming. Index: [index.md](../index.md).

Independent of [Step 21](step-21-basecamp-ui.md) and
[Step 52](step-52-wrap-up-verification.md).
Forum draft stays at `docs/external/forum-post.md` (Step 51).
This step does not move, edit, or link it (D53.12 superseded).

## Goal

A cold clone shows four trees: on-chain program, Logos module,
product docs, operational scripts.
Process packets stay in git, not on the product hub.
After the move the repo builds and living Make/E2E entries run.

Work lands on one feature branch.
Commit after each range.
Each range leaves a consistent tree: living path strings that name
files this range moved are retargeted in the same commit.
Do not merge a range that still points living commands at paths
this range deleted.

## End state

```text
lez-payment-streams/
├── README.md                  product door; FFI lives in module/ffi
├── AGENTS.md                  process door → docs/plan/index.md
├── Makefile
├── Cargo.toml
├── Cargo.lock
├── flake.nix                  FFI package inlined here
├── flake.lock
├── .gitignore
├── LICENSE-APACHE-v2
├── LICENSE-MIT
├── scaffold.toml              lgs init/setup/localnet (root)
├── spel.toml                  guest path under program/
├── program/
│   ├── README.md              guest review order / code map
│   ├── core/
│   ├── methods/               host embed + guest (AMM/RLN shape)
│   └── lez-payment-streams-idl.json
├── module/
│   ├── README.md              chainAction catalogue first
│   ├── CMakeLists.txt
│   ├── src/
│   ├── ffi/
│   ├── tests/
│   ├── nix/                   patched wallet flake
│   ├── metadata.json
│   ├── flake.nix              path:.. unchanged (same depth)
│   └── flake.lock
├── docs/
│   ├── README.md              product hub
│   ├── integrate.md           add streams to another protocol
│   ├── reproduce/
│   │   ├── module.md          MODE=module manual walkthrough
│   │   └── store.md           MODE=store orchestrated path
│   ├── reference/
│   │   ├── wire.md
│   │   ├── decisions.md
│   │   ├── pins.md
│   │   ├── names.md
│   │   ├── matrix.md
│   │   └── localnet-recovery.md
│   ├── plan/                  signpost README; context-manifest.json
│   ├── archive/
│   └── external/              Step 51 draft only; not linked from product doors
└── verify/
    ├── README.md
    ├── e2e.sh
    ├── lifecycle.sh
    ├── fixture.sh
    ├── module-e2e.sh
    ├── module-close-negatives.sh
    ├── repro-reset.sh
    ├── repro-shell.sh
    ├── repro-lgs-setup.sh
    ├── repro-install.sh
    ├── repro-auth-transfer.sh
    ├── check-terminology.sh
    ├── lib/
    ├── store/
    ├── testnet/
    │   └── submit/
    ├── fixtures/
    ├── seed/
    └── archive/
```

`docs/external/forum-post.md` remains for Step 51.
It is absent from the hub and from product-door link lists.

Conventions:

- Instruction API is module `chainAction`. ImageID is
  `spel inspect` (Make target may stay named `program-id`).
- Guest compile is `make build` (`cargo risczero`). No top-level
  `methods/` for unused `lgs build` / `lgs deploy`.
  `program/methods/` is AMM/RLN convention, not a risc0 force.
- Cargo package names stay, including `lez_payment_streams-examples`
  on the seed-only crate under `verify/seed/`.
  Move directories with `git mv`.
- `lgs` stays for `init` / `setup` / `localnet` only.
  Keep `make deploy` (`wallet deploy-program`).
- `.scaffold/` holds gitignored local state, including
  `.lez_payment_streams-state` and
  `.lez_payment_streams-fixture-provider`.
- Root `README.md` is the product door.
  `docs/README.md` is the product hub: Choose a path is product
  only; one maintainer note outside that table names `plan/` and
  `archive/` as internal.
  Hub Verify stays as links (README Testing, `matrix.md`,
  `verify/README.md`).
  Hub Reference stays as the renamed living reference files.
  Hub and root links to `module/README.md` say "chainAction API
  reference" and keep the catalogue heading anchor.
  Do not link the forum draft from product doors.
- `verify/` is operational scripts (E2E, testnet bootstrap,
  wallet `.lgx`, hygiene). Line 1 of `verify/README.md` says so.
  After the MODE × network table, fold the env-var table from
  today's `scripts/README.md` (it is not in `matrix.md`).
  Point cold start at `matrix.md`; do not duplicate that checklist.
  Then every file with its caller.
  Root files are what a human types from README Testing,
  `docs/reproduce/module.md`, or `make help`.
  Subdirs are ownership, not one taxonomy:
  `testnet/` public-network setup + submit crate;
  `store/` MODE=store Python + Store lifecycle;
  `seed/` local fixture generation;
  `fixtures/` JSON (names carry the network);
  `lib/` sourced helpers and Make/E2E-only execs;
  `archive/` non-runnable historical source.
  No `verify/local/` or `verify/module/`.
- `verify/archive/` is not runnable. Do not retarget DoD scripts
  at new helpers. Do not `chmod +x` them from the Makefile.
- No redirect stubs (D46.16).
- Broken relative links after a move: drop the link unless the
  target still exists and the citation is load-bearing
  (contracts, pins, decisions, reproduce, matrix, catalogues),
  in which case repoint. Do not keep a broken link.

### Fixtures (fresh clone)

Tracked SSOT under `verify/fixtures/` (schema and public-network
identity a clone cannot derive):

| File | Why it is committed |
| --- | --- |
| `localnet.json.example` | Localnet schema template. `localnet.json` is written by seed / prepare. |
| `testnet.json.example` | Store testnet per-operator template (empty owner/provider). Copy to gitignored `testnet.json` after bootstrap. |
| `testnet-module.json` | Public testnet ImageID, deploy metadata, and module-testnet accounts. Root README guest section cites it. |
| `testnet-wallet_config.example.json` | Testnet wallet-config template. |

Derived, gitignored, not committed:

| File | Writer |
| --- | --- |
| `localnet.json` | `seed.sh` / Store prepare |
| `testnet.json` | `make bootstrap-testnet` (copy of the example plus operator ids) |

Delete tracked `fixtures/localnet-debug.json` (stale local snapshot;
a clone cannot use those accounts).
Do not ignore the whole `verify/fixtures/` directory.

Gitignore entries for generated fixtures:

```text
verify/fixtures/localnet.json
verify/fixtures/testnet.json
```

Do not rename fixture JSON *names* or keys `reserved_for_step_*`.

### `verify/` inventory

Write this table into `verify/README.md` (MODE × network table,
then env-var table, then every file with its caller).
Any living file with no named caller goes to `verify/archive/`
in the same pass.
Add the `check-relative-links.sh` row in range 3, when that file
is created.

Root:

| File | Caller |
| --- | --- |
| `e2e.sh` | README Testing, Make `verify-*` |
| `lifecycle.sh` | `e2e.sh` |
| `fixture.sh` | `e2e.sh` |
| `module-e2e.sh` | `e2e.sh` when `MODE=module` |
| `module-close-negatives.sh` | Make `verify-module-local-close-negatives` only (`e2e.sh` does not exec it) |
| `repro-reset.sh` | `docs/reproduce/module.md` |
| `repro-shell.sh` | same |
| `repro-lgs-setup.sh` | same |
| `repro-install.sh` | same |
| `repro-auth-transfer.sh` | same |
| `check-terminology.sh` | Make `check-terminology` |

`lib/`:

| File | Caller |
| --- | --- |
| `common.sh`, `auth_transfer.sh`, `chain_poll.sh`, `vault_scan.sh`, `repro-env.sh` | sourced by root scripts |
| `await_tx.py` | `chain_poll.sh` |
| `ensure-scaffold-lez-layout.sh` | `seed.sh` |
| `auth-transfer-ensure.sh` | `repro-auth-transfer.sh`, Store E2E |
| `build-wallet-lgx.sh` | `e2e.sh`, Make `wallet-lgx` |
| `check-relative-links.sh` | Make `check-links` (added range 3) |
| `test_fixture_vault.sh` | Make `test-fixture-vault` (add this target; tests `fixture.sh` / `vault_scan`, not Store) |

`store/`:

| File | Caller |
| --- | --- |
| `run_e2e.py` | `e2e.sh` when `MODE=store` |
| `test_run_e2e_pure.py` | colocated unit test (`import run_e2e`) |
| `seed_provider_acceptance.py` | `run_e2e.py` |
| `continuation-owner-topup.sh` | `run_e2e.py` |
| `store-lifecycle.sh` | Make `verify-store-local-lifecycle` |
| `.gitignore` | from `scripts/e2e/.gitignore` |

`testnet/`:

| File | Caller |
| --- | --- |
| `bootstrap-testnet.sh`, `bootstrap-testnet-module.sh`, `deploy-testnet.sh` | Make |
| `fund-testnet-accounts.sh` | `docs/reproduce/store.md`, Store E2E, module-e2e hints (no Make target; do not add one) |
| `prepare-testnet-privacy-seed.sh` | `docs/reproduce/store.md`, Store E2E (no Make target; do not add one) |
| `ensure-testnet-vault.sh` | Store testnet E2E |
| `testnet-common.sh`, `fund_testnet.sh` | sourced by testnet operators |
| `testnet_rpc.py` | `testnet-common.sh`, `run_e2e.py` |
| `sequencer_latency_probe.py` | Make `debug-sequencer-latency` |
| `submit/` (src, `Cargo.toml`, `Cargo.lock`, `flake.nix`, `flake.lock`) | `ensure-testnet-vault.sh`, bootstrap |

`seed/`: `seed.sh`, `Cargo.toml`, `Cargo.lock`, seed and bootstrap
bins. Make `seed-fixture`.

## Move map

`git mv` unless marked delete. Rename in the same move.

| Today | End |
| --- | --- |
| `lez-payment-streams-core/` | `program/core/` |
| `methods/` | `program/methods/` |
| `lez-payment-streams-idl.json` | `program/` |
| `logos-payment-streams-module/` | `module/` |
| `lez-payment-streams-ffi/` | `module/ffi/` |
| `nix/payment-streams-ffi.nix` | inline into root `flake.nix`; then delete empty `nix/` |
| `scripts/e2e.sh` | `verify/e2e.sh` |
| `scripts/lifecycle.sh` | `verify/lifecycle.sh` |
| `scripts/fixture.sh` | `verify/fixture.sh` |
| `scripts/module-e2e.sh` | `verify/module-e2e.sh` |
| `scripts/module-e2e-close-negatives.sh` | `verify/module-close-negatives.sh` |
| `scripts/user-journey-reset.sh` | `verify/repro-reset.sh` |
| `scripts/user-journey-shell.sh` | `verify/repro-shell.sh` |
| `scripts/user-journey-lgs-setup.sh` | `verify/repro-lgs-setup.sh` |
| `scripts/user-journey-install-modules.sh` | `verify/repro-install.sh` |
| `scripts/user-journey-auth-transfer.sh` | `verify/repro-auth-transfer.sh` |
| `scripts/lib/user-journey-env.sh` | `verify/lib/repro-env.sh` |
| `scripts/lib/{common,auth_transfer,chain_poll,vault_scan,ensure-scaffold-lez-layout}.sh` | `verify/lib/` |
| `scripts/e2e/await_tx.py` | `verify/lib/` |
| `scripts/auth-transfer-ensure.sh` | `verify/lib/` |
| `scripts/build-wallet-lgx.sh` | `verify/lib/` |
| `tests/scripts/test_fixture_vault.sh` | `verify/lib/` (depth stays 2) |
| `scripts/e2e/run_local_e2e.py` | `verify/store/run_e2e.py` |
| `scripts/e2e/seed_provider_acceptance.py` | `verify/store/` |
| `scripts/e2e/continuation-owner-topup.sh` | `verify/store/` |
| `tests/e2e/test_run_local_e2e_pure.py` | `verify/store/test_run_e2e_pure.py` |
| `scripts/e2e/.gitignore` | `verify/store/.gitignore` |
| `scripts/e2e/ensure-testnet-vault.sh` | `verify/testnet/` |
| `scripts/e2e/sequencer_latency_probe.py` | `verify/testnet/` |
| `scripts/lib/{testnet-common,fund_testnet}.sh` | `verify/testnet/` |
| `scripts/testnet_rpc.py` | `verify/testnet/` |
| `scripts/{bootstrap-testnet-module,deploy-testnet,fund-testnet-accounts,prepare-testnet-privacy-seed}.sh` | `verify/testnet/` |
| `scripts/archive/bootstrap-testnet.sh` | `verify/testnet/bootstrap-testnet.sh` |
| `scripts/archive/verify-store-local-lifecycle.sh` | `verify/store/store-lifecycle.sh` |
| `scripts/check-terminology.sh` | `verify/check-terminology.sh` |
| `scripts/seed-localnet-fixture.sh` | `verify/seed/seed.sh` |
| `fixtures/` (SSOT files above) | `verify/fixtures/` |
| `examples/` seed and bootstrap bins + `Cargo.lock` | `verify/seed/` (one `Cargo.toml`) |
| `tools/lez-testnet-submit/` (whole crate) | `verify/testnet/submit/` |
| `docs/on-chain/README.md` | `program/README.md` |
| `docs/payment-streams-module/README.md` | `module/README.md` |
| `docs/integrate/eligibility.md` | `docs/integrate.md` |
| `docs/reproduce/payment-streams.md` | `docs/reproduce/module.md` |
| `docs/reproduce/store-eligibility.md` | `docs/reproduce/store.md` |
| `docs/reference/integration-contracts.md` | `docs/reference/wire.md` |
| `docs/reference/integration-decisions.md` | `docs/reference/decisions.md` |
| `docs/reference/feature-branch-pins.md` | `docs/reference/pins.md` |
| `docs/reference/naming-conventions.md` | `docs/reference/names.md` |
| `docs/reference/verification-matrix.md` | `docs/reference/matrix.md` |
| `docs/archive/operator/localnet-recovery.md` | `docs/reference/localnet-recovery.md` |
| `docs/context-manifest.json` | `docs/plan/context-manifest.json` |
| `scripts/archive/{build-wallet-lgx,ensure-scaffold-lez-layout,testnet-common}.sh` | delete (trampolines / dupes; living copies move) |
| `handoff.md` | delete |
| `docs/external/README.md` | delete (it exists to index the forum draft) |
| `docs/plan/waiting/` | delete |
| `docs/reference/README.md` | delete (hub Reference table is the index; merge nothing else) |
| `docs/presentation.md` | delete (slides; substance is in README / hub) |
| `docs/handoff-deposit-zero-instruction.md` | delete (point-in-time bug note; journeys it cites are gone) |
| `docs/archive/operator/testnet-claim-known-issue.md` | delete (resolved; matrix already requires claim) |
| `fixtures/localnet-debug.json` | delete |
| `examples/` CLI wrapper and `generate_idl` stub | delete |
| `scripts/README.md` | delete after folding env-var table into `verify/README.md` |
| top-level `tests/`, `examples/`, `tools/`, `scripts/`, `nix/`, `docs/on-chain/`, `docs/integrate/`, `docs/payment-streams-module/`, `docs/plan/waiting/` | remove once empty |

Do not move `docs/external/forum-post.md`.
Do not wholesale-move `scripts/e2e/` into `verify/store/`.
Do not keep living `user-journey-*` names (`repro-*` replaces D46.14).
Do not rename Cargo packages, `README.md`, `AGENTS.md`, the
Makefile filename, `lez-payment-streams-idl.json`, fixture JSON
names, or the submit crate package name.

Archive (keep in git, non-runnable) into `verify/archive/`:

- `module-e2e-local.sh`, `module-e2e-privacy.sh`
- `validate-restructure.sh`, `sync-seed-wallet-after-logoscore.sh`,
  `deploy-program-logoscore.sh`
- `verify-dual-wallet-isolation.sh`, `verify-ppe-shield-include.sh`,
  `verify-store-host-restart.sh`, `verify-wallet-cli-handoff.sh`
- `scripts/e2e/debug_happy_path.py`
- remaining `scripts/archive/` DoD scripts except the two living
  files moved above and the three deleted stubs

## Work

Four commit ranges on the feature branch.
Path-string retarget for files this range moved happens in the
same range.
Citation cleanup (dropping plan-packet numbers from product-door
prose) stays in range 4.

### Range 1 — program and module

Move `program/` and `module/` per the map.
Retarget every living path that named the old crate dirs
(`methods/guest` without `program/`, `lez-payment-streams-core/`
as a directory, `lez-payment-streams-ffi/` as a directory,
`logos-payment-streams-module`) in Makefile, `spel.toml`,
scripts (still at `scripts/`), C++, flakes, gitignore guest
target, `module/` Nix output paths, and living docs that cite
those crate paths (`docs/on-chain/README.md`, module README,
root README guest/prereq lines, `matrix.md`).
Do not colocate catalogues yet (hub still points at
`docs/on-chain/` and `docs/payment-streams-module/`).
Do not drop plan-packet prose.

- Workspace `Cargo.toml`:
  `members = ["program/core", "module/ffi", "program/methods"]`
  `exclude = ["program/methods/guest", "tools/lez-testnet-submit"]`
- Path deps:
  `program/methods` → `../core`
  `program/methods/guest` → `../../core`
  `module/ffi` → `../../program/core`
- `program/methods/build.rs`: Docker `root_dir` must stay the
  repository root (two parents, or walk to workspace `Cargo.toml`).
  Today it is `CARGO_MANIFEST_DIR.parent()`.
- Inline `nix/payment-streams-ffi.nix` into root `flake.nix`:
  `src = ./.;`, `lockFile = ./Cargo.lock`, `postInstall` copies
  `$src/module/ffi/lez_payment_streams_ffi.h`.
  Delete empty `nix/`.
- Point `spel.toml` `binary` and `idl`, and Makefile
  `PROGRAMS_DIR` / `IDL_FILE`, at `program/…`.
- In `module/src/payment_streams_module_writes.cpp`, default guest
  path `methods/guest/target/…` → `program/methods/guest/target/…`.
  Env `PAYMENT_STREAMS_GUEST_BIN` is not enough.
- Keep patched wallet flake under `module/nix/`.
- `make program-id` still uses `examples/` CLI this range.
- Living `nix build ./module#lgx` (replace
  `./logos-payment-streams-module#lgx`).

After this commit, `make build` / `make idl` / `make program-id`
work, and `scripts/e2e.sh` would still find the guest under
`program/methods/guest/`.

### Range 2 — verify and state

Create `verify/` and move files per the map.
Crate tests stay in `program/core`, `module/ffi` (Rust), and
`module/tests` (Qt).
Retarget every living citation of `scripts/`, top-level
`fixtures/`, `examples/`, and `tools/lez-testnet-submit` in
Makefile, README Testing, hub Verify, reproduce, `matrix.md`,
`names.md`, `AGENTS.md`, `docs/plan/index.md` (load-bearing path
cells), program/module docs still under `docs/`, and error text.

- `verify/seed` path dep: `../../program/core`.
  Package name stays `lez_payment_streams-examples`.
  Seed and bootstrap bins only.
- `verify/testnet/submit` path dep: `../../../program/core`.
  Workspace exclude becomes `verify/testnet/submit`.
- `run_e2e.py` / `test_run_e2e_pure.py`: mechanical path and import
  updates only (`scripts/`, `examples/`, `fixtures/`, `methods/`,
  root state files, `import run_local_e2e`). Do not rewrite
  orchestrator behaviour.
- Move state files under `.scaffold/`. Retarget Make `STATE_FILE`,
  `fixture.sh`, `seed.sh`, `run_e2e.py`,
  `continuation-owner-topup.sh`, and the snapshot save/restore
  loops in `lifecycle.sh` (they copy both markers by basename).
  `make clean` removes the new `STATE_FILE` only, not `.scaffold/`.
- In `module/src/payment_streams_module_kit.cpp`, default
  `fixtures/localnet.json` → `verify/fixtures/localnet.json`
  in this range (same commit as the fixtures move).
- Seed bin `--manifest` defaults to `verify/fixtures/localnet.json`.
- Retarget gitignore:
  `program/methods/guest/target/`,
  `verify/testnet/submit/target/`,
  `verify/fixtures/localnet.json`,
  `verify/fixtures/testnet.json`,
  `module/…`.
  Root `storage.json`, `wallet_config.json`, `store.sqlite3*`,
  `rust_out`, `result`, `.lez_payment_streams-*` stay ignored
  and untracked. No `git rm --cached` for those.
  `git rm` tracked `fixtures/localnet-debug.json`.
- Retarget living Make recipes to `verify/` paths.
  `make wallet-lgx` calls `verify/lib/build-wallet-lgx.sh`
  (today's archive copy is a trampoline; delete it).
  Help awk: `scripts/archive/` → `verify/archive/`.
- Drop Makefile aliases now, so help and recipes do not chmod
  `verify/archive/`:
  `verify-step10a` … `verify-step13`,
  `verify-step17`, `verify-step17-back-to-back`, `verify-step18`,
  `verify-step18-testnet-read-smoke`, `verify-step28-module-smoke`.
  Drop `module-e2e-privacy.sh` from living `chmod +x` lists.
  Drop living prose that still advertises those aliases
  (`names.md` legacy-alias row, `matrix.md` "Legacy names still
  work").
- Drop `make cli`. Point `make program-id` at `spel inspect`.
  Delete `examples/` CLI wrapper and `generate_idl` stub with the
  seed move so `examples/` can go.
- Add `make test-fixture-vault`.
- Keep `make deploy`, `make setup`, `make status` (status still
  reads `STATE_FILE`).
- Write `verify/README.md` from the inventory above (env-var table
  folded from `scripts/README.md`; then delete `scripts/README.md`).
  Mark `verify/testnet/submit/` as the testnet write helper for
  vault-ensure, bootstrap, and Store E2E.
- Testing `fast` handle stays `make check-terminology`
  (do not switch it to `make check`; `check-links` does not exist
  yet).
- `check-terminology.sh` is the highest-risk retarget (~40
  hardcoded paths). `ROOT` stays one `..`. Preflight every scan
  path, including the direct module `rg` for quoted `"signer"` /
  `"authority"`, `scan_journey_names`, and `scan_retired_paths`.
  Fail if a path is missing or `rg` errors. Drop archived
  filenames from scans. Drop the `authority_account` carve-out.
  Drop living "Step 47" from the header/banner.

After this commit, README Testing commands, `make verify-*`, and
`make program-id` match the tree.

### Range 3 — docs and links

- Colocate catalogues. Put the chainAction catalogue near the top
  of `module/README.md` (before Required verification / Setup).
  Drop the plan-packet LIP-appendix link in `program/README.md`
  (the LIP URL at the top stays).
  Do not merge `program/README.md`, `module/README.md`, and
  `docs/integrate.md`. No `docs/guides/`.
- Hub Choose-a-path: product paths with one-line purpose
  (`program/README.md`, `module/README.md`, reproduce, integrate).
  No forum row. No `plan/` row.
  Maintainers: `docs/reference/wire.md` and `AGENTS.md`.
  One note outside Choose-a-path: `plan/` and `archive/` are
  internal.
  Keep Verify (links only) and Reference (renamed files).
- Replace `docs/plan/README.md` with a signpost to
  `docs/README.md` and `index.md`. Set index Waiting to "None"
  and drop the `waiting/` row. Retarget load-bearing index paths
  that this range renamed.
- Delete leftover docs listed in the move map (`handoff.md`,
  `docs/external/README.md`, `docs/reference/README.md`,
  `docs/presentation.md`, `docs/handoff-deposit-zero-instruction.md`,
  `docs/archive/operator/testnet-claim-known-issue.md`,
  `docs/plan/waiting/`).
  Remove empty directories.
- Do not edit `docs/external/forum-post.md` or
  [step-51-forum-post.md](step-51-forum-post.md).
- Add `verify/lib/check-relative-links.sh`: resolve relative
  links in tracked `.md` outside `docs/plan/`, `docs/archive/`,
  and `docs/external/`.
  Drop broken links unless the target still exists and the
  citation is load-bearing, then repoint
  (today: `pins.md` / former feature-branch-pins, `decisions.md`,
  `wire.md`; `docs/reference/README.md` is deleted instead).
  Append the inventory row for `check-relative-links.sh`.
- Make targets (no extra directory, no second root script):
  `check-terminology` — terminology only
  `check-links` — relative-link checker
  `check` — both
  Testing `fast` stays `make check-terminology`.
- New terminology scans on product-door paths (root README,
  `docs/README.md`, `verify/README.md`, `program/README.md`,
  `module/README.md`, Makefile, `docs/integrate.md`,
  `docs/reference/matrix.md`): leftover
  `verify-step[0-9]`, leftover `user-journey`, links into
  `docs/plan/` / `step-NN-*.md`.
  Do not scan `docs/reproduce/` for bare `Step N` (recipe
  headings). D-IDs stay.
  Drop plan-packet links from `matrix.md` (gate-log citations
  are process; not load-bearing for a clone).

### Range 4 — retire and citations

- Drop `make status`. Keep `make setup` and `make deploy`.
  `make program-id` remains `spel inspect` from range 2.
- Retitle the Makefile off "SPEL Program".
- Leave `scaffold.toml` `[framework.idl] path = "idl"` with a
  comment: living IDL is `program/lez-payment-streams-idl.json`
  via `spel.toml` / `make idl`; do not create `idl/`.
- Remove dead C++ `chainUsesTestnetSubmit` /
  `submitGenericPublicViaTestnetHelper` (already `return false`)
  and the call site that branches on them.
- Product-door prose (root README, hub, Makefile help,
  `verify/README.md`, `program/README.md`, `module/README.md`,
  `docs/integrate.md`, `docs/reference/matrix.md`):
  no plan-packet numbers.
  Stop rule: `rg` `Step [0-9]` and `step-[0-9]` on those files;
  fix hits that are packet citations, not recipe headings or D-IDs.
  Manual pass on reproduce *plan-packet* citations only; recipe
  headings (`## Step 17 — Shut down`) stay.
- Code comments: drop notes that name a gone packet or retired
  path and no longer explain current behaviour (policy predicates,
  module callers, N8 Nim–Rust parity, gone
  `step3-policy-and-implementor-notes.md`). Keep D-IDs and living
  provenance. Do not sweep "Step" in `docs/plan/` or
  `verify/archive/`.
- Pin comments in root `Cargo.toml`, inlined `flake.nix`, and
  `verify/seed/Cargo.toml` point at `docs/reference/pins.md`.
- Leave plan-step numbers in `docs/plan/`, `docs/archive/`,
  `verify/archive/`, `AGENTS.md`, `docs/reference/decisions.md`,
  `pins.md`, and fixture keys `reserved_for_step_*`.
- Drop "stub" wording from `AGENTS.md` and `docs/plan/index.md`.
  Close the step: `git mv` this packet to `docs/plan/completed/`,
  move `53` from upcoming to completed in `context-manifest.json`.
  Gate log: `docs/plan/completed/step-53-gate-log.md`.

## Do not

- Relocate `AGENTS.md`.
- Merge `lifecycle.sh` / `fixture.sh` into `e2e.sh`.
- Split `e2e.sh` / `lifecycle.sh` / `module-e2e.sh` into
  localnet and testnet copies.
- Put typed `repro-*.sh` in `lib/`.
- Put testnet bootstrap/deploy bash into `seed/` or `fixtures/`.
- Add `verify/lint/`, `verify/check/`, `verify/local/`, or
  `verify/module/`.
- Fold `check-links` into the `check-terminology` target.
- Delete `docs/plan/` or `docs/archive/`.
- Split `docs/plan/completed/` into `docs/archive/`.
- Delete `lez-testnet-submit` while bootstrap and Store E2E exec it.
- Fold the submit crate into the guest workspace.
- Advertise a SPEL CLI next to `chainAction`.
- Strip D-IDs, reproduce recipe numbering, or step-number prose
  in `decisions.md` / `pins.md`.
- Rename fixture keys `reserved_for_step_*`.
- Create an `idl/` directory.
- Ignore the whole `verify/fixtures/` directory.
- Move, edit, or product-door-link the forum draft.
- Require Step 52 wrap-up, Store E2E, testnet, or privacy cells to close this brief.

## Verification

Prove the layout still builds and one living script graph still runs.
Step 52 owns the full matrix (Store, privacy, testnet, Qt kit).

Per range, on the feature branch, before the next range:

- Range 1: `make build`, `make idl`, `make program-id`,
  `nix build` of the root FFI package,
  `RISC0_DEV_MODE=1 cargo test --workspace`.
  Completeness subset: living trees have no crate-dir path forms
  (`methods/guest` without `program/`, old crate directories,
  `logos-payment-streams-module` as a path).
- Range 2: `bash -n` on moved living shells;
  `make program-id` (`spel inspect`);
  `test_run_e2e_pure.py` (imports `run_e2e`);
  `test -f` the retargeted guest default and
  `verify/fixtures/localnet.json.example`;
  `make help` has no `verify-step*`.
  Completeness subset: no top-level `scripts/`, `examples/`,
  `tools/lez-testnet-submit`, `(^|[^/])fixtures/` in living trees
  outside `docs/plan/`, `docs/archive/`, `docs/external/`,
  `verify/archive/`.
- Range 3: `make check-terminology` and `make check-links`.
- Range 4: full completeness `rg`; product-door `Step [0-9]`
  stop-rule `rg`.

Close gate (after all four ranges). Cheap first, then one chain smoke:

```bash
make build
make idl
make program-id
RISC0_DEV_MODE=1 cargo clippy --workspace
RISC0_DEV_MODE=1 cargo test --workspace
make check
MODE=module ./verify/e2e.sh local run
```

`local run` (not `prepare` only) is the one chain cell: it execs
`lifecycle.sh`, `fixture.sh`, `module-e2e.sh`, the module flake,
and guest/state paths. Do not `SKIP_BUILD=1` on this run.
Do not add Store, testnet, or `OWNER_PRIVACY` here.

Also confirm `.scaffold/.lez_payment_streams-state` exists after
that run (snapshot/state relocation).

## Completeness `rg`

After the move, living trees outside `docs/plan/`, `docs/archive/`,
`docs/external/`, and `verify/archive/` contain no legacy *path
forms* (not stems that match `program/methods/` or Cargo package
names):

| Search | Forbidden | Allowed |
| --- | --- | --- |
| `scripts/` as a path | old top-level `scripts/` | none |
| `methods/guest` without `program/` prefix | root `methods/` | `program/methods/` |
| `examples/` | old examples tree | none |
| `tools/lez-testnet-submit` | old tools path | `verify/testnet/submit` |
| `lez-payment-streams-core/` (directory) | old crate dir | package name in Cargo.toml |
| `lez-payment-streams-ffi/` (directory) | old crate dir | package name |
| `logos-payment-streams-module` | old module dir | `nix build ./module#lgx` |
| `docs/integrate/` (directory), `docs/on-chain/`, `docs/payment-streams-module` | old doc dirs | `docs/integrate.md` |
| `(^|[^/])fixtures/` | top-level `fixtures/` | `verify/fixtures/` |

`docs/external/` is excluded so the untouched forum draft does not
fail the gate.

Every path cited from the Makefile, root README, hub, `AGENTS.md`,
`verify/README.md`, reproduce, and reference exists on disk.

## Done when

- Tree matches End state. Front doors describe it.
  `verify/README.md` inventories every file with a caller.
- This packet is under `docs/plan/completed/` and is not a stub.
  Step 51 still points at `docs/external/forum-post.md`.
- Completeness `rg` is clean. `make help` has no `verify-step*`.
- Close-gate commands in Verification all pass.
- Gate log `docs/plan/completed/step-53-gate-log.md` records the
  four ranges and those commands.

## Decisions

| ID | One line |
| --- | --- |
| D53.1 | Four commit ranges on one feature branch; each range a consistent tree. |
| D53.3–D53.4 | `scaffold.toml` and `spel.toml` stay at repo root. |
| D53.5 | Delete `handoff.md`. |
| D53.6 | `plan/` stays, unlinked except a hub maintainer note. Signpost README. Delete empty `waiting/`. |
| D53.7 | IDL in `program/`. |
| D53.8–D53.9 | Catalogues next to trees. `docs/integrate.md` is the other-protocol guide. |
| D53.10 | State files under `.scaffold/`. Keep `make setup`. Drop `make status` in range 4. |
| D53.11 | Rename `decisions.md`; do not split or rewrite step prose. |
| D53.12 | Superseded. Forum draft stays at `docs/external/forum-post.md`; Step 51 owns it; no product-door link. |
| D53.13 | Hub Maintainers: `wire.md` and `AGENTS.md`. Choose-a-path is product only (no forum). |
| D53.14 | `verify/` operational; inventory; `repro-*` at root; archive non-runnable. |
| D53.15 | Keep submit crate under `verify/testnet/submit/`. |
| D53.16 | Inline FFI Nix package; delete empty `nix/`. |
| D53.17 | Narrow citation sweep on product doors plus dead code phase comments. Stop rule in range 4. |
| D53.18 | Seed bins in `verify/seed/` as one Cargo package named `lez_payment_streams-examples`. |
| D53.19 | Archive CLI in range 2. Drop `make cli`. `program-id` is `spel inspect` from range 2. Keep `make deploy`. |
| D53.20 | Top-level dirs: `program/`, `module/`, `docs/`, `verify/`. |
| D53.21 | `program/methods/`; Docker root stays repo root. |
| D53.22 | `test_fixture_vault.sh` in `verify/lib/`; add Make `test-fixture-vault`. |
| D53.23 | Root README states FFI under `module/ffi/`. |
| D53.24 | Un-archive living bootstrap and Store lifecycle; delete three stubs. |
| D53.25 | Root artifacts gitignored, not tracked. Generated fixture JSON gitignored by name, not the directory. |
| D53.26 | Unused `[framework.idl] path = "idl"` with a comment. |
| D53.27 | Separate `check-terminology` and `check-links`. Testing `fast` stays terminology-only. |
| D53.28 | Terminology scans fail on missing paths. |
| D53.29 | Cell is `MODE` × `local\|testnet` on `e2e.sh`. |
| D53.30 | `run_local_e2e.py` → `run_e2e.py` with mechanical edits. |
| D53.31 | Completeness uses legacy path forms. Exclude `docs/external/`. |
| D53.32 | `context-manifest.json` under `docs/plan/`. |
| D53.33 | `localnet-recovery.md` under `docs/reference/`. |
| D53.34 | Explicit Cargo path-dep table (range 1–2). |
| D53.35 | C++ defaults retarget in the same range as the files they name. |
| D53.36 | Short user-facing names. `repro-*` replaces `user-journey-*`. |
| D53.37 | Close gate is fast checks plus `MODE=module ./verify/e2e.sh local run`. Step 52 owns Store, privacy, testnet, Qt kit. |
| D53.38 | Living path strings retarget in the same range as the move. |
| D53.39 | Fixture SSOT: examples + `testnet-module.json` + wallet-config example. Delete `localnet-debug.json`. |
| D53.40 | Leftover markdown: fold unique living substance, else delete. Empty dirs go. |
| D53.41 | Broken links: drop; repoint only if load-bearing and the target exists. |
| D53.42 | Drop `verify-step*` aliases and `make cli` in range 2 (consistent Make after `verify/` exists). |

D53.2 (stub packet) is superseded.

## Related

- [index.md](../index.md)
- [AGENTS.md](../../../AGENTS.md)
- [step-46-docs-unify-and-forum-post.md](../completed/step-46-docs-unify-and-forum-post.md)
- [step-51-forum-post.md](step-51-forum-post.md)
- [step-52-wrap-up-verification.md](step-52-wrap-up-verification.md)
- [docs/README.md](../../README.md)
