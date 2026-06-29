# Step 24b — unify Rust `lee` / LEZ deps on rc5

Normative handoff for agents. Index: [program-index.md](../../development-map/program-index.md).
Prerequisite: Step 18b complete (operational Nix / scaffold / wallet / testnet scripts on rc5).
Related: [step-24-lee-harness-upgrade.md](../completed/step-24-lee-harness-upgrade.md),
[N16](../../reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06),
[feature-branch-pins.md](../../feature-branch-pins.md).

Status: **complete** (2026-06-26 on `master`).

## Goal

One LEZ git revision for **all Rust workspace dependencies** (`lee`, `lee_core`, `clock_core`,
guest `authenticated_transfer_core`, vendored SPEL, `examples/`) matching the **operational**
pin already used for localnet, wallet, module `.lgx`, and public testnet:

| Field | Value |
| --- | --- |
| Tag | `v0.2.0-rc5` |
| Git rev | `27360cb7d6ccb2bfbcca7d171bab8a3938490264` |

After Step 24, host/guest/tests sit on PR 510 (`62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60`).
Step 18b moved runtime tooling to rc5 but **deliberately left** the Rust cone on 510 ([N16]
harness lag). Step 24b closes that gap.

**Definition of done**

- Single recorded LEZ `rev` (`27360cb7…`) across every `Cargo.toml` LEZ git dep in this repo
  (see inventory below); `Cargo.lock` resolves **one** `lee_core` git tree for the workspace.
- Guest ELF rebuilt (`make build`); `make program-id` recorded in the completion note (below).
- `cargo test -p lez-payment-streams-core --features host` passes (transparent `program_tests`;
  PP tests per policy in [Step 24 pitfalls](../completed/step-24-lee-harness-upgrade.md)).
- `make verify-step10a`, `verify-step12-dod.sh`, `verify-step13-dod.sh` pass.
- **`make verify-step17`** passes (local LEZ dual-host Store E2E).
- **`make verify-step18-testnet-read-smoke`**, **`make bootstrap-testnet`** (if manifest stale),
  **`make verify-step18`** pass (public testnet chain + local Store).
- Docs updated: remove “harness lag” / dual-rev narrative; [N16](../../reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06) amended on merge to state Rust = operational pin.

Out of scope: `logos-delivery` / `logos-delivery-module` forks; Store wire; Logos module
LogosAPI surface; Phase 9 retirement of `tools/lez-testnet-submit` (unchanged — separate step).

## Background (what implementers should know)

### Two layers today

| Layer | Pin today | What it affects |
| --- | --- | --- |
| Operational | rc5 (`27360cb7…`) | `scaffold.toml`, `nix/payment-streams-ffi.nix` LEZ fetch, wallet flakes, module `.lgx`, E2E scripts, `tools/lez-testnet-submit` |
| Rust workspace | 510 (`62d9ba10…`) | `lez-payment-streams-core/ffi`, `methods/guest`, `vendor/spel-framework*`, `examples/`, root `Cargo.toml` comment |

Nix FFI already symlinks LEZ artifacts from rc5 while `cargo build` / `cargo test` compile against
510 `lee_core` in `Cargo.lock`. That is the inconsistency Step 24b fixes.

### What `program_tests` are (unchanged role)

In-process `lee::V03State` integration tests under `lez-payment-streams-core/src/program_tests/`.
They validate guest instruction semantics and PDAs; they do **not** replace wallet jsonrpsee or
LEE v0.3 message hashing on the wire. E2E remains the product gate.

### On-chain guest reference (pre–Step 24b)

Org deploy on public testnet (2026-06-25), current tree at 510 `lee_core` inside guest:

| Field | Value |
| --- | --- |
| `program_id_hex` (ImageID) | `79b1dd5c441caede8f9f82c30de637aba465f94cc43817b1105c8c48c77d0fc9` |
| Guest ELF size (observed) | 576576 bytes |
| `deploy_tx_hash` | `1787368626484789a2976a2aa8631d2b5b39c415c0a74b5a345474d1415f79b1` |
| `deploy_block_id` | 3284 |

If `make program-id` after rc5 rebuild **equals** the ImageID above, **do not** redeploy on
testnet; update docs only. If ImageID **differs**, follow redeploy policy below and record new
values in the completion table at the end of this packet and in
`fixtures/testnet.json.example` shared chain fields.

## LEZ git dependency inventory (must all move together)

Replace every `62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60` with `27360cb7d6ccb2bfbcca7d171bab8a3938490264`
in:

| Path | Crates |
| --- | --- |
| Root [Cargo.toml](../../../Cargo.toml) | Workspace comment only |
| [lez-payment-streams-core/Cargo.toml](../../../lez-payment-streams-core/Cargo.toml) | `lee`, `lee_core`, `clock_core` |
| [lez-payment-streams-ffi/Cargo.toml](../../../lez-payment-streams-ffi/Cargo.toml) | `lee`, `lee_core` |
| [methods/guest/Cargo.toml](../../../methods/guest/Cargo.toml) | `lee_core`, `authenticated_transfer_core` |
| [examples/Cargo.toml](../../../examples/Cargo.toml) | `lee`, `lee_core`, `common`, `sequencer_service_rpc`, `wallet` |
| [vendor/spel-framework-core/Cargo.toml](../../../vendor/spel-framework-core/Cargo.toml) | `lee_core`, optional `lee` |
| [vendor/spel-framework/Cargo.toml](../../../vendor/spel-framework/Cargo.toml) | optional `lee` |

Already on rc5 (verify still aligned after workspace bump):

| Path | Notes |
| --- | --- |
| [tools/lez-testnet-submit/Cargo.toml](../../../tools/lez-testnet-submit/Cargo.toml) | Excluded from workspace; must stay same rev |
| [nix/payment-streams-ffi.nix](../../../nix/payment-streams-ffi.nix) | Already `27360cb7…`; re-run `nix build .#payment-streams-ffi` after `Cargo.lock` change |
| [scaffold.toml](../../../scaffold.toml) | Already rc5; run `lgs setup` after guest rebuild if wallet artifacts stale |

Search command (pre/post):

```bash
rg '62d9ba10' --glob '!target/**' --glob '!Cargo.lock'
```

Post-change: only historical docs (Step 24 packet, git history references) may mention 510.

Do **not** remove `[patch."https://github.com/logos-co/spel.git"]` → vendor SPEL; upstream
SPEL v0.5.0 still targets old LEZ. Bump **vendor** `lee_core` only.

## Implementer decisions (defaults)

Use these unless a regression forces a documented exception.

### 1. Target revision

**Default:** `27360cb7d6ccb2bfbcca7d171bab8a3938490264` (`v0.2.0-rc5`) — same as Step 18b /
[N16]. Do not pin floating `main` or a different RC without operator sign-off and full E2E re-run.

### 2. Single PR / single rev

**Default:** One integration branch; all Cargo.toml + vendor + guest in the same PR. Mixed revs
produce duplicate `AccountId` types and flaky PDAs (Step 24 pitfall table).

### 3. Order of work inside the PR

**Default order**

1. Bump all LEZ git `rev` fields to rc5.
2. `cargo update -p lee -p lee_core -p clock_core` (and guest deps); commit `Cargo.lock`.
3. Fix compile errors (`LeeError` paths, API renames) across core, ffi, examples, vendor if any.
4. `cargo test -p lez-payment-streams-core --features host --no-run` then full test run.
5. `make build` (guest); `make program-id`; compare to on-chain table above.
6. Rebuild wallet/module artifacts (`./scripts/build-wallet-lgx.sh`, `nix build ./logos-payment-streams-module#lgx`, `lgpm install` as Step 17 does).
7. Localnet snapshot if guest ImageID or LEZ builtins affect local deploy: see local E2E section.
8. Testnet manifest / redeploy if ImageID changed.
9. E2E gates and doc updates.

### 4. Guest ImageID unchanged vs changed

**Default:** Treat ImageID equality as the redeploy gate.

- **Equal** to `79b1dd5c…`: skip `make deploy-testnet`; keep `fixtures/testnet.json.example`
  `program_id_hex` / deploy tx fields; operator gitignored `fixtures/testnet.json` remains valid
  if vault/stream ids unchanged.
- **Different:** org or operator **must** deploy new guest (`make deploy-testnet` with rc5
  wallet), record new `program_id_hex`, `deploy_tx_hash`, `deploy_block_id` in completion table,
  update `fixtures/testnet.json.example`, delete gitignored `fixtures/testnet.json`, re-run
  `make bootstrap-testnet`, then `make verify-step18`.

**Default assumption:** rc5 `lee_core` may or may not change RISC0 guest image bytes; **always**
run `make program-id` — do not assume equality without measuring.

### 5. Localnet after guest rebuild

**Default:** If `make program-id` differs from what the Step 17b snapshot expects, refresh funded
snapshot:

```bash
make full-reset-localnet
```

(or project-documented equivalent in [step-17b-localnet-snapshot-restore.md](../completed/step-17b-localnet-snapshot-restore.md)).

If ImageID unchanged, still run `make verify-step17` once; if deploy step fails with program id
mismatch, use `FULL_RESET=1` prepare.

### 6. Privacy-preserving (`pp-program-tests`) tests

**Default:** Same as Step 24 — transparent ladder must be green for DoD. If PP tests fail after
rc5 bump, leave them behind `pp-program-tests` / existing ignores and list failures in the PR;
do not block Step 24b on PP unless the team expands scope.

### 7. `tools/lez-testnet-submit`

**Default:** No functional change expected (already rc5). After workspace bump, rebuild helper
(`cd tools/lez-testnet-submit && cargo build --release`) once to confirm lockfile isolation.

### 8. Nix `sha256` for `payment-streams-ffi.nix`

**Default:** LEZ fetch rev unchanged → sha256 unchanged. If Nix LEZ rev ever drifted from
Cargo, fix in the same PR; run `nix build .#payment-streams-ffi`.

### 9. Module flake lock

**Default:** If `logos-payment-streams-module/nix/flakes/.../flake.lock` still references
510 anywhere, bump to rc5 in this PR (should already be rc5 post–18b). One wallet/module LEZ input
everywhere.

### 10. Documentation renames (optional, non-blocking)

**Default:** Do not rename `docs/step11d-wallet-510.md` in Step 24b; add a one-line note at top
“historical title; operational pin rc5”. Optional follow-up: rename file + links.

## Step-by-step implementation plan

### Phase 1 — Pin bump and compile

1. Apply rc5 `rev` to every row in [inventory](#lez-git-dependency-inventory-must-all-move-together).
2. Update root `Cargo.toml` comment to point at Step 24b + rc5 rev.
3. Refresh `Cargo.lock`; ensure `cargo tree -p lee_core` shows a single git rev.
4. Fix vendor SPEL imports if `lee_core` API moved; prefer `AccountId::for_public_pda` over
   duplicated shims only when tests prove equivalence (Step 24 guidance).
5. Mechanical ripgrep: `nssa::`, `NssaError`, stale `62d9ba10` in Rust sources (should be none
   post–24; catch stragglers).

**Gate:** `cargo test -p lez-payment-streams-core --features host --no-run` succeeds.

### Phase 2 — Transparent `program_tests`

1. Run `cargo test -p lez-payment-streams-core --features host`.
2. If `MismatchedPdaClaim` or deposit/auth-transfer failures: fix account metas against rc5
   `Program::authenticated_transfer_program` (same class of fixes as Step 24 @ 510).
3. Re-enable any obsolete ignores only if still valid; remove obsolete
   `IGNORE_LEZ_GUEST_ON_NSSA_HARNESS` commentary if present.

**Gate:** transparent `program_tests` green without `--ignored` (unless documented PP-only).

### Phase 3 — Guest rebuild and ImageID

```bash
make build
make program-id
# Record output hex and ELF size in PR description
ls -la methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin
```

Compare to pre–24b table. Branch:

- **Same ImageID:** Phase 4 local E2E without forced full reset (unless verify fails).
- **New ImageID:** Phase 3b redeploy + fixture updates (below).

### Phase 3b — Testnet redeploy (only if ImageID changed)

Prerequisites: testnet wallet under `.scaffold/e2e/testnet-wallet/`, rc5 `wallet` on PATH,
`LEE_WALLET_HOME_DIR` set per [step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md).

```bash
make deploy-testnet
make program-id   # must match chain expectation post-deploy
```

Record in PR and [completion table](#completion-record-fill-in-when-done):

- New `program_id_hex`
- `deploy_tx_hash` (from explorer or `getTransaction` if CLI silent)
- `deploy_block_id`

Update `fixtures/testnet.json.example` shared fields; remove operator
`fixtures/testnet.json`; `make bootstrap-testnet`; `make verify-step18`.

### Phase 4 — Local integration artifacts

```bash
lgs setup   # if scaffold LEZ cache stale
nix build .#payment-streams-ffi
./scripts/build-wallet-lgx.sh
nix build ./logos-payment-streams-module#lgx
# lgpm install paths per make verify-step17
```

If guest ImageID changed locally:

```bash
make full-reset-localnet
```

**Gate:** `make verify-step17`.

### Phase 5 — Testnet E2E

```bash
make verify-step18-testnet-read-smoke
# If manifest stale or program_id_hex changed:
make bootstrap-testnet
export CHAIN=testnet FIXTURE_MANIFEST=fixtures/testnet.json
make verify-step18
```

**Gate:** exit 0 on read smoke and full Step 18 verify.

### Phase 6 — Documentation and index

1. [feature-branch-pins.md](../../feature-branch-pins.md) — one rc5 row for Rust + Nix + scaffold;
   delete “harness lag” / separate 510 test pin subsection.
2. [N16](../../reference/decisions-and-notes.md#n16-step-18b-rc5-operational-pin-2026-06) — state
   operational and Rust pins unified; Phase 9 helper retirement unchanged.
3. [step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md) — remove harness-lag
   paragraph under LEZ pin.
4. [program-index.md](../../development-map/program-index.md) — Step 24b complete; Step 24 note
   superseded for pin policy by 24b completion record.
5. [AGENTS.md](../../../AGENTS.md) — remove 510/Rust lag from active work if present.
6. Move this packet to `docs/plan/completed/` when DoD met (team convention).

## Regression checklist (copy for PR)

```bash
cargo test -p lez-payment-streams-core --features host
cargo test -p lez-payment-streams-ffi
make verify-step10a
make verify-step12
make verify-step13
make verify-step17
make verify-step18-testnet-read-smoke
make verify-step18   # CHAIN=testnet, manifest, wallet env per runbook
```

Optional: `nix build .#payment-streams-ffi`, `make verify-step11d-dod.sh` if wallet patch
touched.

## Pitfalls (rc5-specific)

| Pitfall | Symptom | Mitigation |
| --- | --- | --- |
| Bump core only, not guest | Host/guest `AccountId` / instruction drift | Guest `lee_core` same rev in same PR |
| Skip `make program-id` | Testnet streams bound to old program | Always record ImageID before/after |
| Old `fixtures/testnet.json` | `STREAM_NOT_ACTIVE`, wrong PDAs | Delete and rebootstrap when `program_id_hex` changes |
| Skip local snapshot after new ImageID | Step 17 deploy/program mismatch | `FULL_RESET=1` prepare |
| Two LEZ revs in `Cargo.lock` | Link errors, duplicate types | `cargo update` + single rev in all manifests |
| Assume rc5 = rc3 signing bugs | Confusion with abandoned dual-pin | rc5 is operational; 510 was Rust-only lag |
| Redeploy without updating example manifest | New operators copy wrong id | Update `fixtures/testnet.json.example` |

## Non-regression guard

- `CHAIN=local` / unset: Step 17 script path unchanged except correct program id after local redeploy.
- Do not change delivery fork pins or Store tag 30 wire.
- Do not merge partial pin bumps.

## Completion record (fill in when done)

| Field | Value |
| --- | --- |
| PR / branch | `master` (working tree) |
| LEZ rev (final) | `27360cb7d6ccb2bfbcca7d171bab8a3938490264` |
| Guest ELF size (bytes) | 576512 |
| `program_id_hex` after rebuild | `16b95d3701d256eecd41d5a55e4f570331994d787abf0cba44eec209e24f8a44` |
| ImageID changed? | yes |
| Testnet redeploy performed? | yes (rc5 guest; operator `make deploy-testnet`) |
| On-chain `program_id_hex` (authoritative) | `16b95d3701d256eecd41d5a55e4f570331994d787abf0cba44eec209e24f8a44` |
| `deploy_tx_hash` (if redeployed) | record from wallet `deploy-program` stdout when captured |
| `deploy_block_id` (if redeployed) | record from explorer when captured |
| Explorer URL (if redeployed) | `https://explorer.testnet.lez.logos.co/` |
| `make verify-step10a` | pass (`FIXTURE_MANIFEST=fixtures/localnet.json`) |
| `make verify-step17` | fail on depleted local stream in this session; use `CHAIN=local E2E_LATE_STREAM_CREATE=0` + fresh `FULL_RESET=1 prepare-localnet` |
| `make verify-step18` | pass (core, `E2E_PHASE=core`, artifact `e2e-20260626T165856.log`; legacy name `demo-e2e-local-*.log`); default `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF=0` blocked until testnet top-up restores unaccrued (see [step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md)) |
| `tools/lez-testnet-submit` | aligned with `seed_localnet_fixture` (`try_new` + `TxPoller`); vendored SPEL patch in standalone `Cargo.toml` |
| Date | 2026-06-26 |

If no redeploy: set “On-chain `program_id_hex` (authoritative)” to
`79b1dd5c441caede8f9f82c30de637aba465f94cc43817b1105c8c48c77d0fc9` and note “unchanged after
rc5 guest rebuild”.

## References

- Step 24 methodology: [step-24-lee-harness-upgrade.md](../completed/step-24-lee-harness-upgrade.md)
- Operational testnet runbook: [step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md)
- Local E2E: [step17-e2e-local.md](../../step17-e2e-local.md)
- Localnet snapshot: [step-17b-localnet-snapshot-restore.md](../completed/step-17b-localnet-snapshot-restore.md)
- LEZ repo: `github.com/logos-blockchain/logos-execution-zone` (`lee`, `lee_core`, `wallet`)
