# Step 18b — rc5 tooling unification then finish Step 18

Handoff packet for continuing public testnet work after recon on dual-pin tooling.
This document does not replace
[step-18-public-testnet-demo.md](step-18-public-testnet-demo.md) or
[step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md);
it records the recommended execution order and what to keep from in-flight WIP.

Status: Step 18b implementation on branch `feat/lez-unify-v0.2.0-rc5` (merge to `master` when
`make verify-step17` and manual testnet smoke pass). Operator defaults recorded below.

## Audience and goal

Operators and agents picking up Step 18 Part B on branch
`feat/step18-public-testnet` after a context reset.

End state:

- One LEZ pin everywhere: `logos-execution-zone` tag `v0.2.0-rc5`
  (git `27360cb7d6ccb2bfbcca7d171bab8a3938490264`).
- Local Step 17 and testnet Step 18 use the same wallet, module `.lgx`, and
  signing rules (LEE v0.3 public message hash on testnet).
- `make verify-step17` green on `master` after unification.
- `make verify-step18` green on the Step 18 feature branch after rebase.
- Docs and runbooks describe a single pin, not 510 local plus rc3 testnet writes.

## Program context (unchanged)

Step 18 Part B: dual-host Store E2E like Step 17, but chain reads and writes go to
public testnet while relay and Store stay on two local `logoscore` hosts.
`CHAIN=testnet`, `FIXTURE_MANIFEST=fixtures/testnet.json`.

Org payment-streams guest is already deployed on public testnet (operators do not
re-deploy unless guest ELF or ImageID changes).

| Field | Value |
| --- | --- |
| Sequencer | `https://testnet.lez.logos.co/` |
| Org `program_id_hex` | `79b1dd5c441caede8f9f82c30de637aba465f94cc43817b1105c8c48c77d0fc9` |
| Deploy tx hash | `1787368626484789a2976a2aa8631d2b5b39c415c0a74b5a345474d1415f79b1` |
| Deploy block | 3284 |
| Explorer | `https://explorer.testnet.lez.logos.co/transaction/1787368626484789a2976a2aa8631d2b5b39c415c0a74b5a345474d1415f79b1` |

Read order for agents:
[`docs/AGENT-BRIEF.md`](../../AGENT-BRIEF.md) → this file →
[step-18-public-testnet-demo.md](step-18-public-testnet-demo.md) →
[step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md) →
[integration-index.md](../../../integration-index.md).

## Why dual-pin is wrong for public testnet

Earlier Step 18 work assumed:

- Local and module pin `62d9ba10` (510-era LEZ in scaffold and nix).
- Testnet chain writes via rc3 (`cf3639d8`) plus `tools/lez-testnet-submit`.

Recon against live testnet:

- Builtin program ImageIDs on chain match the 510-era wallet view (e.g.
  authenticated transfer `d9a19237…`), not rc3 local builtins (`a96e0889…`).
- Public transactions on testnet use LEE v0.3 message hashing
  (`/LEE/v0.3/Message/Public/…` plus SHA256). Stock rc3 `wallet` signs raw
  borsh and returns `InvalidSignature` for auth-transfer init, Piñata claim, and
  submits unless compensated manually.
- Public testnet sequencer runs LEZ aligned with the 510 merge lineage, not rc3.

Unification target is therefore **`v0.2.0-rc5`**, not rc3:

- Ancestor of both rc3 and the 510 merge in `logos-execution-zone`.
- Includes `lez/wallet-ffi`, generic public tx flows, program deploy, and
  `message.hash()` signing in nssa (same signing model testnet expects).
- Same `authenticated_transfer.bin` blob lineage as `62d9ba10` for guest deploy
  compatibility.

After rc5 pin everywhere, testnet becomes configuration plus fixtures plus
bootstrap scripts, not a second wallet stack.

## Current git and WIP snapshot (2026-06)

Branch: `feat/step18-public-testnet` (tracks
`origin/feat/step18-public-testnet`; local WIP may be ahead).

Recent commits on the branch (reference):

- Part B testnet path and guest `lee_core` diet.
- Document deploy size blocker (superseded by successful org deploy on chain).
- WIP migrate to testnet sequencer (tx size / tooling notes).

Large uncommitted delta (do not blindly discard):

- `tools/lez-testnet-submit`: jsonrpsee submit path, delete legacy sequencer RPC,
  untracked `sequencer_rpc.rs`, manual testnet hash signing, instruction hex as LE
  u32 words, nonces only for signing accounts, testnet auth-transfer program id
  default; Cargo still pins **`cf3639d8`** (must move to rc5).
- Scripts: `bootstrap-testnet.sh`, `testnet-common.sh`, `demo-e2e-local.sh`,
  `run_local_e2e.py` testnet guards, read smoke, deploy-testnet simplification.
- Module: `payment_streams_module_writes.cpp` testnet submit and ELF path from file.
- Examples: `bootstrap_testnet_fixture.rs`.
- Docs: plan packet, runbook, `integration-index.md`, `AGENT-BRIEF.md` (partially
  staged).

Verification observed during WIP (re-run after rc5):

| Target | Result |
| --- | --- |
| `make verify-step18-testnet-read-smoke` | PASS (with WIP scripts) |
| `make bootstrap-testnet` | PASS for submits after hash-signing fix |
| `make verify-step18` | FAIL — eligibility / vault funding / store path (not signature) |
| rc3 `wallet check-health` vs testnet | Fails (builtin id mismatch) |
| rc3 `wallet auth-transfer init` / pinata | `InvalidSignature` |

Piñata and owner funding: intended in `scripts/testnet-common.sh`
(`ensure_testnet_owner_funded`). Broken with rc3 CLI on testnet; should work with
rc5 `wallet` once pins unify. `TESTNET_SKIP_PINATA=1` reuses manifest owner;
owner balance 0 on chain yields vault holding 0 and E2E `NO_ELIGIBLE_VAULT`.

## Recommended approach (Path C)

Do not unify tooling entirely on the Step 18 feature branch in one mega-PR.
Do not discard all uncommitted work.

Three layers:

1. Snapshot WIP on `feat/step18-public-testnet` (WIP commit or
   `backup/step18-wip-<date>` branch). Reference only until rebase.
2. Platform unification on **`feat/lez-unify-v0.2.0-rc5`** branched from **`master`**,
   merge to `master` when gated.
3. Rebase **`feat/step18-public-testnet`** onto updated `master`, re-apply kept
   behavioral fixes, drop rc3 compensations, finish Part B DoD.

Optional early doc PR to `master`: factual recon only (org deploy table above,
testnet URL, statement that sequencer expects rc5 lineage). Do not merge
dual-pin procedures as canonical until Step 18b lands.

### Order of operations

**Phase 0 — Snapshot**

- Commit or branch current tree on `feat/step18-public-testnet` so nothing is lost.
- User policy: no commits unless explicitly requested; operator may choose WIP
  commit or backup branch before starting Phase 1.

**Phase 1 — rc5 unification (`feat/lez-unify-v0.2.0-rc5` from `master`)**

- Bump LEZ pin to `v0.2.0-rc5` / `27360cb7…` in at least:
  - `scaffold.toml` (today `62d9ba10…`)
  - `logos-payment-streams-module/nix/flakes/…/lez-wallet-ffi-patched/flake.nix`
  - Any `LEZ_RC3_REV`, rc3 wallet fetch, or duplicate rev constants in
    `scripts/testnet-common.sh`, `scripts/deploy-testnet.sh`, Makefile helpers.
- Rebuild payment-streams `.lgx` and module artifacts; run **`make verify-step17`**.
- Retarget `tools/lez-testnet-submit/Cargo.toml` git deps to rc5; remove manual
  `PUBLIC_MESSAGE_HASH_PREFIX` hack if rc5 nssa already matches testnet.
- Prefer rc5 `wallet` CLI in testnet scripts for Piñata, auth-transfer init,
  and deploy-program; align `WALLET_CONFIG` / `WALLET_STORAGE` (single storage).
- Decide helper fate:
  - Short term: helper on rc5 until module `chainAction` proven on testnet.
  - End state (Phase 9 in step plan): delete helper and C++ `CHAIN=testnet`
    submit branch when stock module writes work with rc5 only.
- Update docs to single-pin narrative (replace dual-pin sections in
  `step18-public-sequencer-e2e.md` and the step plan).

**Phase 1 exit gates**

- `make verify-step17` on `master` after merge.
- Minimal testnet smoke on rc5 build:
  - RPC reachable.
  - `wallet check-health` with testnet `sequencer_addr` (should pass with rc5).
  - One of: Piñata or funded owner, auth-transfer init, or successful
    `bootstrap-testnet` deposit path with vault holding balance greater than zero.

**Phase 2 — Rebase Step 18 feature branch**

- `git rebase master` (or merge `master` if policy prefers).
- Cherry-pick or manually restore from snapshot:

  Keep (adapt to rc5):

  - Bootstrap fixture and `make bootstrap-testnet` idempotency
    (`TESTNET_SKIP_PINATA`, reuse manifest owner/provider, seed rate envs).
  - `scripts/e2e/run_local_e2e.py` testnet path (skip local stream fixture
    corrupting `fixtures/testnet.json`, accrual wait, stream listing checks).
  - Read smoke script structure and Makefile wiring.
  - ELF via file path (avoid huge env hex) if still needed.
  - `fixtures/testnet.json.example` shared chain fields.

  Drop or rewrite after rc5:

  - Dual-pin docs and rc3-as-testnet claims.
  - `LEZ_TESTNET_USE_RC3_BUILTIN_IDS`, rc3-only program id overrides.
  - Manual hash signing in helper if rc5 wallet and submit stack match testnet.
  - Hard dependency on `cf3639d8` anywhere.
  - Stale or corrupted gitignored `fixtures/testnet.json` (regenerate via bootstrap).

**Phase 3 — Finish Step 18 Part B**

- Funded owner (Piñata or reuse with balance check).
- `make bootstrap-testnet` → gitignored `fixtures/testnet.json`.
- `export CHAIN=testnet FIXTURE_MANIFEST=fixtures/testnet.json`.
- `make verify-step18` (dual-host E2E).
- Merge feature branch when DoD in step plan is met.
- Update [`docs/AGENT-BRIEF.md`](../../AGENT-BRIEF.md) and
  [`integration-index.md`](../../../integration-index.md) active-step table.

## Paths not recommended

**Path A only (docs to master, discard WIP, unify on master, restart Step 18)**

Acceptable for docs-if-factual-only, but discarding uncommitted work loses
validated submit and E2E guard behavior; restarting Step 18 from scratch is slower
than rebase plus selective restore.

**Path B (stash on Step 18 branch, unify there, one big merge)**

Avoids losing context but produces a mixed infra plus product PR and painful
conflicts on files rewritten during unification.

## Key files and tools

| Area | Path |
| --- | --- |
| Submit helper | `tools/lez-testnet-submit/` |
| Bootstrap | `scripts/bootstrap-testnet.sh`, `scripts/testnet-common.sh` |
| Deploy | `scripts/deploy-testnet.sh` |
| E2E | `scripts/demo-e2e-local.sh`, `scripts/e2e/run_local_e2e.py` |
| Verify | `scripts/verify-step18.sh`, `scripts/verify-step18-testnet-read-smoke.sh` |
| Fixture template | `fixtures/testnet.json.example`, gitignored `fixtures/testnet.json` |
| Module writes | `logos-payment-streams-module/src/payment_streams_module_writes.cpp` |
| Bootstrap binary | `examples/src/bin/bootstrap_testnet_fixture.rs` |
| LEZ pin (local) | `scaffold.toml`, module nix `lez-wallet-ffi-patched` |
| Operator runbook | `docs/step18-public-sequencer-e2e.md` |
| Step DoD | `docs/plan/upcoming/step-18-public-testnet-demo.md` |

External repo tag for unification:
`https://github.com/logos-blockchain/logos-execution-zone` → `v0.2.0-rc5`.

## Environment variables (testnet, post-unification intent)

| Variable | Role |
| --- | --- |
| `CHAIN=testnet` | Select testnet sequencer and manifest |
| `FIXTURE_MANIFEST=fixtures/testnet.json` | Per-operator ids after bootstrap |
| `WALLET_CONFIG` / `WALLET_STORAGE` | Single rc5 wallet storage for module and CLI |
| `TESTNET_SKIP_PINATA=1` | Reuse manifest owner; owner must have balance |
| `LEZ_TESTNET_SUBMIT` | Optional path to helper until Phase 9 retirement |
| `PAYMENT_STREAMS_GUEST_BIN` | Guest ELF for deploy or helper |
| `PAYMENT_STREAMS_ALLOW_DEPLETED_STREAM_PROOF` | Testnet E2E accrual timing (see WIP orchestrator) |

After unification, remove or narrow rc3-only vars documented in the old dual-pin
runbook (`LEZ_TESTNET_WALLET_CONFIG` split from 510 storage, rc3 deploy-only paths).

## Fresh chat checklist

1. Read this file and confirm `master` LEZ pin (expect rc5 after Phase 1).
2. Confirm branch: unification work on `feat/lez-unify-v0.2.0-rc5` or post-merge
   `master`; Step 18 work on rebased `feat/step18-public-testnet`.
3. If WIP snapshot exists, diff against current tree before re-applying hacks.
4. Run `make verify-step17` before any testnet work.
5. Run read smoke, then bootstrap, then full `make verify-step18`.
6. Do not commit unless the operator asks (repo user rule).

## Open work items (implementation)

- Execute Phase 1 pin migration across scaffold, nix, scripts, helper Cargo.
- Confirm rc5 `wallet check-health`, auth-transfer init, pinata claim against live testnet.
- Regenerate `fixtures/testnet.json` with vault holding balance greater than zero.
- Green `make verify-step18`; regression `make verify-step17`.
- Revise Step 18 docs for single pin rc5; document Phase 9 helper retirement criteria.
- Optional decision note in `docs/reference/decisions-and-notes.md` (N16) after merge.

## Related decisions

Dual-pin was a workaround for testnet ahead of local pin and rc3 behind testnet
signing and builtins. Step 18b supersedes that workaround with rc5 as the single
compatibility point. Step 24 (`lee` harness at 510) remains complete for local
`program_tests`; scaffold operational pin moves to rc5 for wallet and sequencer
compatibility with public testnet.

When Step 18b completes, update the dual-pin section in
[step18-public-sequencer-e2e.md](../../step18-public-sequencer-e2e.md) and trim
obsolete Phase 9 wording that references PR 491 and 510 as future testnet state.
