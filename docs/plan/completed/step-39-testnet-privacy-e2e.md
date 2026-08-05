# Step 39 — testnet privacy E2E after native guest deploy

Index: [index.md](../index.md). Status: **complete** (human close 2026-08-05;
D39.15).

Goal: freeze the current native-token guest, redeploy it to public testnet
(agent-run), and close privacy v1 with real proving on public testnet:
module full privacy warm-up, then Store full privacy on that ImageID,
without waiting for multi-token vaults.

This step is agent-run deploy + verification only (D39.9).
Product paths: [Step 36](step-36-payer-funder-unlinkability.md),
[Step 37](step-37-payee-receiver-privacy.md).
Local Store privacy: [Step 38](step-38-store-privacy-e2e.md) Phases A/B.
Promotes Phase C raw TODO
([testnet-privacy-e2e-after-guest-deploy.md](testnet-privacy-e2e-after-guest-deploy.md)).
Do not reopen Steps 36–38 product DoD.

Gate log (append runs here):
[step-39-testnet-gate-log.md](step-39-testnet-gate-log.md).

## Implementer plan of action

Do these in order. Stop on failure.
Local Phase 1 preflight may use soft proving (`RISC0_DEV_MODE=1`; localnet
accepts stubs). After Phase 3, harness must align shield with the LEZ wallet
CLI tutorial path (D39.22), then run a local real-prove module smoke
(D39.24) before testnet privacy. Required testnet privacy gates use real
proving (`RISC0_DEV_MODE=0`; D39.4). One green per required gate (D39.8).
Do not reopen Phase 1–3 green rows (D39.23).

| Step | Action | Required |
| --- | --- | --- |
| 1 | Local preflight (privacy first, then public): see Phase 1 | Yes (done) |
| 2 | `make build` (Docker guest path); capture freeze commit + ImageID Y + ELF size | Yes (done) |
| 3 | Credential / funding checklist (Phase 1); confirm Y ≠ fixture `de17c0db…` (or Y-equal path) | Yes (done) |
| 4 | `make deploy-testnet`; fixture sync (not full re-bootstrap); prefix checks | Yes (done) |
| 5 | Public testnet with `SKIP_BUILD=1` (module then Store) | Yes (done) |
| 5b | Harness: wallet-CLI shield/dust (D39.22); local module full privacy with `RISC0_DEV_MODE=0` (D39.24) | Yes (done) |
| 6 | Module full privacy testnet (`E2E_CLAIM_OPTIONAL=0`, `RISC0_DEV_MODE=0`) | Yes (done) |
| 7 | Store full privacy testnet (`E2E_CLAIM_OPTIONAL=0`, `RISC0_DEV_MODE=0`) | Yes (done) |
| 8 | Phase 5 docs minimum; gate-log summary that required gates are green | Yes (done) |
| 9 | Human review of gate log + artifacts; then move packet to completed | Yes (done 2026-08-05) |
| — | Isolation cells; new Make aliases | Optional / deferred |

Exact env and commands: [Verification](#verification).

## Problem

Local privacy profiles were green while testnet fixtures still pinned ImageID
`de17c0db…` (deploy 2026-07-15). Step 39 froze, redeployed Docker ELF ImageID
`072a26cc…`, synced fixtures, and closed required privacy gates on that tip.
Authoritative ImageID remains `make program-id` after `make build` at the
freeze commit (D39.10).

## Outcome (agent-reported)

| Gate | Artifact | Notes |
| --- | --- | --- |
| Module full privacy testnet | `module-e2e-20260722T215542.log` | `RISC0_DEV_MODE=0`; claim vault_drop=400 |
| Store full privacy testnet | `e2e-20260724T144726.log` | `RISC0_DEV_MODE=0`; `E2E_CLAIM_OPTIONAL=0`; claim_balance vault_drop=400; ~106 min CPU |
| Phase 5 docs | E2E.md + verification-matrix + gate-log summary | D39.14 |

Full run history:
[step-39-testnet-gate-log.md](step-39-testnet-gate-log.md).

## Why a new step (not reopen Step 38)

Step 38 DoD was local Store privacy (Phases A/B).
Step 39 owns freeze, deploy, fixture sync, and the required testnet privacy
gates. Do not reopen Step 38.

## Decision log (SSOT)

| Id | Topic | Outcome |
| --- | --- | --- |
| D39.1 | Packet ownership | New Step 39; do not reopen Step 38. |
| D39.2 | Guest freeze | Native-token only. Multi-token deferred. |
| D39.3 | Order | Amended 2026-07-22. Local soft preflight → build/freeze → deploy → public testnet → wallet-CLI shield harness + local real-prove module smoke → module full testnet → Store full. |
| D39.4 | Proving | Amended 2026-07-22. Public testnet privacy DoD requires real proofs (`RISC0_DEV_MODE=0`). Soft stubs (`RISC0_DEV_MODE=1`) are for Phase 1 local preflight only — the public sequencer rejects FakeReceipts. |
| D39.5 | Matrix tier | Privacy testnet stays optional in verification-matrix until boringly green. |
| D39.6 | Harness | Inherit Step 38 / D38.8. Port-gap rule: see [Port-gap vs protocol](#port-gap-vs-protocol). |
| D39.7 | Required cells | Module full privacy, then Store full privacy. Isolation cells optional. |
| D39.8 | Green passes | One green per required gate. |
| D39.9 | Operator | Agent tries deploy (`make deploy-testnet`), then fixture sync and prefix checks. On deploy/credential failure, stop, flag in the gate log, and wait for human — do not invent a workaround. |
| D39.10 | Freeze + ImageID rule | See [Freeze and deploy identity](#freeze-and-deploy-identity). |
| D39.11 | Fixture sync | See [Fixture sync vs re-bootstrap](#fixture-sync-vs-re-bootstrap). |
| D39.12 | Testnet env defaults | See [Testnet command defaults](#testnet-command-defaults). |
| D39.13 | Claim strictness | Strict for required privacy gates: `E2E_CLAIM_OPTIONAL=0`. Vault_holding drop / claim_balance must pass. Do not reopen optional claim to close DoD. (Amended 2026-08-03: Step 32 D3 closed; public Store default is also `0`.) |
| D39.14 | Phase 5 minimum | Two required privacy recipes in E2E.md + optional matrix rows + gate-log link. Do not mirror local isolation cells. Privacy gates keep explicit `E2E_CLAIM_OPTIONAL=0`. Make aliases deferred. |
| D39.15 | Close path | Agent never moves the packet to completed. Agent reports greens (or incomplete) in the gate log. Human alone decides completed or write-off. |
| D39.16 | Y-equal contingency | If freeze ImageID Y equals fixture `de17c0db…`, skip redeploy, log no-op, still verify fixtures + prefix checks, proceed. Do not invent a guest edit to force a new ImageID. (Post-`6772238b` guest commit `a59a66d` makes Y≠fixture the expected path.) |
| D39.17 | Phase 1 local purpose | Harness regression only (`SKIP_BUILD=1` against existing local guest). Not identity parity with freeze ImageID Y. |
| D39.18 | Funding targets | Fund before module full and again before Store full (defaults `OWNER_TARGET=550` / `PROVIDER_MIN=50`). Do not use a stacked target (for example 1200) for both runs — re-fund restores the public owner between runs. If Store funding-short: once `OWNER_TARGET=700 PROVIDER_MIN=100`, then escalate. No wallet wipe between runs. |
| D39.19 | Soft proving on testnet | Superseded by D39.4 amendment. Soft proving on public testnet is not DoD and must not greenwash a privacy gate. Local soft proving remains valid for Phase 1. |
| D39.20 | Store cold start | Orchestrator starts dual-host logoscore; no pre-running daemons required. Cold-start remediation from verification-matrix is in scope if local Store preflight fails; stop if it needs human auth/flake access. |
| D39.21 | Real-proof wall clock | Phase 4 real proves can take many minutes per PPE submit (shield, vault, stream, claim). Raise timeouts as a port-gap if needed (D39.6); do not fall back to soft mode to pass. |
| D39.22 | Shield path | Align pre_shield and provider dust shield with the LEZ tutorial operator path: `wallet auth-transfer send` (Public→Private), with the same logoscore↔wallet handoff family already used for AT-init. Do not rely on `logoscore call … transfer_shielded_owned` for real proving — LogosAPI default IPC timeout (~20s) is far below PPE prove wall clock (~30s–4 min per LEZ tutorial; ~2 min/CPU bench). Soft-mode logoscore shield remains acceptable only where Phase 1 soft proving is still in force. |
| D39.23 | Reopen Phase 1–3 greens | Do not. Phase 1 soft greens stand under D39.4/D39.19. Phase 2 has no proving. Phase 3 public module/Store are public txs (no client PPE); `RISC0_DEV_MODE` is irrelevant there. |
| D39.24 | Local real-prove smoke | After D39.22 harness lands, and before testnet Phase 4, run one local module full-privacy smoke with `RISC0_DEV_MODE=0` and `E2E_CLAIM_OPTIONAL=0`. Not a reopen of Phase 1 DoD — isolation only. If PP `chainAction` then hits logoscore outer IPC timeout, raise that timeout as a port-gap (D39.6/D39.21) before spending testnet wall clock. |
| D39.25 | PPE clock for real prove | Soft path keeps genesis CLOCK_01 (fixture `clock_10_account_id` / RWNU — historical misnomer). Real prove (`RISC0_DEV_MODE=0`) uses CLOCK_50 for stream/claim account lists so multi-minute PPE does not cross a clock `public_pre_state` update (sequencer would reject with `InvalidPrivacyPreservingProof`). Align to an early CLOCK_50 window (`block_id % 50 <= 2`) before every clock-using PP op (create/close/claim). Wait for a CLOCK_50 epoch advance before accrual polls. Default skip pause/top-up under real prove. Soft Phase 1–3 unchanged (D39.23). |
| D39.26 | GPU optional for real prove | Amended 2026-07-23. Missing NVIDIA / CUDA is not a Phase 4 stop. RISC Zero already proves on CPU when no GPU is usable (`r0vm` without CUDA toolkit or without `nvidia.ko`). Gate real-prove readiness on an includable PPE smoke (`wallet auth-transfer send` → `getTransaction` non-null under `RISC0_DEV_MODE=0`), not on `nvidia-smi`. Log CPU vs GPU as an operator note (wall clock only). Do not greenwash soft stubs (D39.4). |

## Shield and prove path (D39.22, D39.24, D39.26)

Canonical LEZ testnet shielded native transfer (tutorial
`token-transfer.md`):

```bash
wallet auth-transfer send \
  --from Public/<sender_b58> \
  --to Private/<recipient_b58> \
  --amount <n>
```

That CLI proves in-process and may take tens of seconds to several minutes
(CPU often ~2–4 min per submit on a laptop; GPU faster when CUDA actually
engages). `lez-programs` testnet runbooks use `wallet` / `spel` the same way.

Do not treat `nvidia-smi` failure as a prove blocker (D39.26).
If the driver is missing, continue on CPU and raise timeouts if needed
(D39.21). Soft mode remains forbidden on public privacy gates (D39.4).

Payment-streams privacy E2E historically called
`logoscore call logos_execution_zone transfer_shielded_owned`, which only
finishes under soft stubs inside the default LogosAPI timeout.
For Step 39 real proving, module and Store harnesses must use the wallet CLI
path for owner pre_shield and provider dust shield (D39.22), matching AT-init
handoff practice.

PP vault/stream/claim still go through `payment_streams_module` `chainAction`
and also prove. The local real-prove smoke (D39.24) is the place to discover
whether logoscore outer call timeouts must be raised before testnet.

## Freeze and deploy identity

### Freeze (D39.10)

Procedural, not a git tag or CI lock:

1. Finish Phase 1 local gates on the intended tree.
2. Record `git rev-parse HEAD` as freeze commit in the gate log.
3. Run `make build` at that commit (no further guest edits, no multi-token).
4. Record ImageID Y from `make program-id` and ELF byte size.

Redeploy is mandatory when Y ≠ current
`fixtures/testnet-module.json` / `fixtures/testnet.json` `program_id_hex`.
At Step 39 open that tip was `de17c0db…`; after freeze/deploy both fixtures
pin ImageID Y `072a26cc…` (guest `a59a66d` clock normalization after earlier
freeze `6772238b`).

If Y equals the fixture (D39.16): skip redeploy, log “ImageID unchanged;
redeploy no-op” in the gate log, still run fixture verify + prefix checks,
then continue to Phase 3. Do not edit the guest to force a new ImageID.

Never treat a localnet tip ImageID as the deploy target without running
`make program-id` at the freeze commit.

### Build path (D39.10)

Deploy ELF is the Docker guest output used by Make:

- `make build` → `cargo risczero build --manifest-path methods/guest/Cargo.toml`
- Binary: `methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin`
- Same path as `PROGRAM_BIN` in Makefile and `scripts/archive/testnet-common.sh`

Do not deploy a non-docker / host-built guest. Record ELF size next to ImageID
(same shape as
[testnet-guest-program-id-sync-2026-07-15.md](testnet-guest-program-id-sync-2026-07-15.md)).

### Deploy failure semantics (D39.10)

`deploy-testnet.sh` may print non-zero `deploy-program` when the program is
already present. That is not automatic success.

Phase 2 passes only when all hold:

1. Built ELF ImageID Y equals `make program-id`.
2. Both testnet fixtures’ `program_id_hex` equal Y.
3. Sequencer reachable.
4. At least one public testnet gate (Phase 3) can resolve the program for the
   fixture owner (module or Store public run).

If fixtures still point at `de17c0db…` after a new deploy, or Y cannot be used
by module/Store runs, stop and investigate wallet/sequencer (in scope for the
agent under D39.9). Do not proceed to privacy gates.

## Fixture sync vs re-bootstrap

### What Phase 2 updates (D39.11)

Fixture sync (required), not a full vault/stream re-bootstrap by default:

1. `fixtures/testnet-module.json` — `program_id_hex`, and when present
   `guest_deploy_date`, `guest_deploy_source_commit`, `guest_elf_bytes`.
2. `fixtures/testnet.json` — `program_id_hex` (and deploy metadata fields if used).
3. Operational defaults that hardcode the old ImageID (grep `de17c0db`):
   - `scripts/e2e/ensure-testnet-vault.sh`
   - `scripts/bootstrap-testnet-module.sh`
   - `scripts/archive/bootstrap-testnet.sh`
   - `scripts/archive/create-testnet-stream-fixture.sh`
   - `fixtures/testnet.json.example`
   - Current tip docs: `README.md`, `docs/journeys/USER_JOURNEY.md`
4. Leave historical completed notes that document the 2026-07-15 deploy as
   history (for example
   [testnet-guest-program-id-sync-2026-07-15.md](testnet-guest-program-id-sync-2026-07-15.md)).

Keep existing fixture `owner_account_id` / `provider_account_id` when the
testnet wallet already owns them.

Do not require `make bootstrap-testnet` or `make bootstrap-testnet-module`
solely because ImageID changed. Module and Store runs already choose a fresh
`vault_id` and refresh vault PDAs per run (Step 33 / module e2e). Stale
`vault_*` PDA fields in JSON are overwritten by the orchestrator.

Re-bootstrap only if fixtures are missing, wallet layout is broken, or
owner/provider ids are invalid for the agent wallet.

Phase 2 complete when: grep for live `de17c0db` in fixtures + operational
scripts/docs above is clean (or only historical completed docs remain), and
prefix checks pass.

## Testnet command defaults

### Credentials and funding (D39.12)

Phase 1 pass/fail checklist before deploy:

- Sequencer: `https://testnet.lez.logos.co/` (or `TESTNET_SEQUENCER`) reachable.
- Wallet dir exists and is usable:
  `.scaffold/e2e/testnet-wallet` (see `ps_e2e_testnet_wallet_dir` /
  `TESTNET_WALLET_DIR`), with `wallet_config.json` / storage as created by
  prior bootstrap or `ensure_testnet_wallet`.
- `make deploy-testnet` can resolve `lez_wallet_bin` (scaffold cache / pinned LEZ).
- Docker guest ELF present after `make build`.
- For module/Store runs: fixture owner funded enough for deposit sizing.
  Prefer `./scripts/fund-testnet-accounts.sh` before privacy/public runs;
  then module may use `MODULE_E2E_SKIP_FUND=1`.

Store cold start (delivery checkout / logoscore) follows
[verification-matrix cold start](../../reference/verification-matrix.md#cold-start-first-time-on-a-machine)
(D39.20). Prerequisite for Phase 3/4 Store, not only wallet+RPC.
`run_local_e2e.py` starts both logoscore daemons; they need not be pre-running.
If local Store preflight fails for missing delivery/module layout, remediate
from the matrix; if that needs human (Nix/flake/auth), stop and flag.

### Ordering (D39.12, D39.17)

Phase 1 order wins over any older quick-table sketch:

1. Local privacy + public gates — harness regression only
   (`SKIP_BUILD=1` against existing local guest; not parity with ImageID Y).
2. Then `make build` + freeze commit + ImageID Y.
3. Then deploy.

Post-deploy verification and privacy runs use `SKIP_BUILD=1` so they pin the
deployed ELF (same pattern as E2E.md testnet recipes and `e2e.sh testnet run`
defaulting `SKIP_BUILD=1` after prepare).

### Claim and confirmation (D39.12, D39.13)

| Gate | `E2E_CLAIM_OPTIONAL` | Fire-and-forget |
| --- | --- | --- |
| Phase 3 public module/Store | default `0` (Step 32 D3 closed) | unset (`E2E_ALLOW_FIRE_AND_FORGET` off) |
| Phase 4 required privacy | `0` (strict; set explicitly) | unset (off) |

Reason: Store/module privacy confirm private claim via `vault_holding` drop.
Required privacy runs set `E2E_CLAIM_OPTIONAL=0` so `claim_balance` /
vault_holding drop is enforced (do not soft-pass with `=1`).

If strict claim flakes, record the artifact and escalate in the gate log.
Do not set `E2E_CLAIM_OPTIONAL=1` to greenwash. Close only after a real pass
or an explicit D39.15 write-off that states the residual risk.

Do not set `E2E_ALLOW_FIRE_AND_FORGET=1` for Step 39. Downstream state polls
remain the gate (D38.9).

### Dual-host Store full privacy (D39.6)

`run_local_e2e.py` already implements D38.8 for privacy flags without a
localnet-only guard (private accounts, dust host, claim host selection,
vault_holding confirm). Treat testnet Store full privacy as already wired.

If it fails for env propagation, wrong-host dust, or missing guest bin on a
daemon, fix that harness port gap under Step 39. Do not treat “never tried on
testnet” as out of scope.

### Port-gap vs protocol (D39.6)

Allowed mid-step without a new decision (port-gap):

- Env propagation to both daemons
- Dust host / claim host selection
- Guest bin path / module path wiring
- Timeouts, logging, artifact paths
  (including logoscore outer `callModuleMethod` / CLI invoke budgets once
  wallet-CLI shield works and PP `chainAction` still times out under real prove)
- Routing pre_shield / dust shield through `wallet auth-transfer send` with
  existing AT-init handoff (D39.22)
- Making an already-locked explicit env (for example `E2E_CLAIM_OPTIONAL=0`)
  actually take effect on the command line

Stop and escalate (not port-gap):

- Guest / FFI / protocol edits
- Delivery wire changes
- Changing claim-optional policy or defaults away from the post–Step 32 D3
  strict default (`E2E_CLAIM_OPTIONAL=0`) without an explicit plan decision
- Relaxing D39.13 to greenwash (setting `E2E_CLAIM_OPTIONAL=1` on a
  required privacy command to pass a flake)
- Re-opening Phase 1–3 green rows to “re-verify” soft or public runs (D39.23)

On a strict-claim flake: record artifact, escalate in the gate log, wait for
human (D39.15). Do not “fix” the flake by changing defaults.

Rule of thumb: if the edit only makes the locked Step 39 command line work,
it is a port-gap. If it changes what green means, escalate.

### Module then Store interference (D39.12, D39.18)

Module uses `fixtures/testnet-module.json`; Store uses `fixtures/testnet.json`
(shared owner/provider ids today). After module full privacy:

- Rely on fresh `vault_id` resolution (do not pin a spent vault).
- Re-run `./scripts/fund-testnet-accounts.sh` before Store (defaults
  `OWNER_TARGET=550` / `PROVIDER_MIN=50`). That is per-run top-up, not a
  stacked budget for module+Store together.
- Private accounts are not faucet targets; dust and deposit buffer come from
  the public owner via pre-shield.
- If Store fails funding-short: once
  `OWNER_TARGET=700 PROVIDER_MIN=100 ./scripts/fund-testnet-accounts.sh`,
  then escalate (do not invent another funding path).
- Do not wipe the testnet wallet between the two runs.

### Claim optional and Step 32 D3 (D39.13)

Step 32 D3 closed 2026-08-03 using Step 39 Phase 4 Store/module full privacy
testnet (`E2E_CLAIM_OPTIONAL=0`) as evidence; public Store default is now `0`.
Phase 4 privacy gates still set `E2E_CLAIM_OPTIONAL=0` explicitly on the command
line.

## Closed clarifications (do not re-open)

Former implementer flags, all locked:

| Flag | Resolution |
| --- | --- |
| Redeploy needed? | Yes expected (`a59a66d` after `6772238b`); D39.16 if Y equal. |
| Agent deploy? | D39.9 — try; flag+stop on failure. |
| Phase 1 local = Y parity? | No — harness only (D39.17). |
| Port-gap vs policy on claim default? | Changing testnet claim default is policy; escalate (D39.6). |
| Delivery / dual-host setup? | Orchestrator starts daemons (D39.20); cold start if missing. |
| Funding target? | 550/50 before each privacy run; bump once to 700/100 (D39.18). |
| Phase 5 doc shape? | Two required recipes only; distinguish claim `0` vs public `1` (D39.14). |
| Testnet proving mode? | Real proofs required (`RISC0_DEV_MODE=0`); soft only for Phase 1 (D39.4 amended). |
| Who closes / write-off? | Human only (D39.15). Agent reports. |
| Shield via logoscore IPC? | No for real prove — wallet CLI `auth-transfer send` (D39.22). |
| Reopen Phase 1–3 greens? | No (D39.23). Local real-prove smoke is new isolation only (D39.24). |

## Open for deliberation

None. Locked through D39.24 (wallet-CLI shield + local real-prove smoke;
2026-07-22).

## Prerequisites

- Steps 36–38 complete.
- Verification-matrix cold start satisfied for Store testnet.
- Phase 1 credential checklist green (D39.12).
- Pinned `wallet` on PATH and wallet home handoff usable (same as AT-init).

## Scope

In scope: freeze; agent deploy + fixture sync + prefix checks; public
regression; wallet-CLI shield harness (D39.22); local real-prove module smoke
(D39.24); required privacy gates; port-gap harness fixes (D39.6); Phase 5
doc minimum (D39.14).

Out of scope: multi-token; reopening 36–38 product DoD; Delivery wire changes;
soft-proof DoD on public testnet; reopening Phase 1–3 green rows (D39.23);
two consecutive greens; promoting matrix tier to required; Make aliases
(deferred); full six-cell privacy matrix; rewriting historical deploy notes.

## Implementation plan

### Phase 0 — Packet

Done.

### Phase 1 — Preflight (no deploy)

1. Local module full privacy:
   `SKIP_BUILD=1 MODE=module CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run`
2. Local Store full privacy: `make verify-store-local-full-privacy`
   (or equivalent `SKIP_BUILD=1 MODE=store … OWNER_PRIVACY=1 PROVIDER_PRIVACY=1`).
3. Local public: `make verify-module-local` and `make verify-store-local`.
4. Credential / funding checklist (D39.12).
5. `make build` (Docker guest) at the freeze commit. A pre-existing ELF under
   `methods/guest/target/.../docker/` (for example ~362044 bytes after
   `a59a66d`) is a hint only — still rebuild and take `make program-id` as Y.
6. Freeze: record commit, ImageID Y, ELF size.
7. Compare Y to fixture `de17c0db…`. Expected: unequal → redeploy.
   If equal: D39.16 no-op path.

### Phase 2 — Deploy and fixture sync

8. `make deploy-testnet` with the frozen Docker ELF.
9. Fixture sync per [Fixture sync vs re-bootstrap](#fixture-sync-vs-re-bootstrap)
   (D39.11). No routine full bootstrap.
10. Prefix checks (D39.10 deploy failure semantics).
11. Gate log deploy row (commit, date, ImageID Y, ELF bytes, operator agent).

### Phase 3 — Public regression

12. `SKIP_BUILD=1 make verify-module-testnet`
    (E2E.md also allows `MODULE_E2E_SKIP_FUND=1` after
    `./scripts/fund-testnet-accounts.sh`).
13. `SKIP_BUILD=1 make verify-store-testnet`
    (default `E2E_CLAIM_OPTIONAL=0` after Step 32 D3; Phase 4 still sets `0`
    explicitly on privacy commands).

Stop if either fails.

Do not re-run Phase 1 soft privacy or Phase 3 public gates to “confirm”
real proving (D39.23).

### Phase 3b — Wallet-CLI shield harness + local real-prove smoke

Required before testnet Phase 4 (D39.22, D39.24).

13b. Port harness (module `module-e2e.sh` and Store `run_local_e2e.py`) so
     owner `pre_shield` and provider dust shield use
     `wallet auth-transfer send` (Public→Private) with logoscore↔wallet
     handoff, not `logoscore call … transfer_shielded_owned`, when real
     proving is required (`RISC0_DEV_MODE=0`). Soft-mode local Phase 1 path
     may keep logoscore shield if already green.
13c. Local module full privacy smoke (isolation only; not Phase 1 reopen):

    ```bash
    SKIP_BUILD=1 RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 \
      MODE=module CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
      ./scripts/e2e.sh local run
    ```

     Expect long wall clock. If shield succeeds but PP `chainAction` fails
     with logoscore IPC / RPC timeout, raise outer call timeouts as a
     port-gap (D39.6/D39.21), re-run 13c, then proceed. Do not skip 13c
     and burn testnet time.

### Phase 4 — Privacy E2E (testnet)

`RISC0_DEV_MODE=0` (real proofs; D39.4). `E2E_CLAIM_OPTIONAL=0`. No fire-and-forget.
Expect long wall clock per PPE submit (D39.21). Requires Phase 3b green.

14. Prefund, then module full:
    ```bash
    ./scripts/fund-testnet-accounts.sh
    SKIP_BUILD=1 MODULE_E2E_SKIP_FUND=1 RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 \
      MODE=module CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
      ./scripts/e2e.sh testnet run
    ```
15. Prefund again, then Store full:
    ```bash
    ./scripts/fund-testnet-accounts.sh
    SKIP_BUILD=1 RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 \
      MODE=store CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
      ./scripts/e2e.sh testnet run
    ```
    Funding-short once: `OWNER_TARGET=700 PROVIDER_MIN=100` then re-fund (D39.18).

Pass: exit 0; Store phases include paid query + settlement; private claim
shows vault_holding drop (`claim_balance` with `provider_private` / vault_drop).
Do not set `RISC0_DEV_MODE=1` to close Phase 4 on public testnet (D39.19).

### Phase 5 — Housekeeping

16. Make aliases: deferred (not DoD).
17. Doc minimum (D39.14): E2E.md sections for the two required testnet privacy
    commands above only (must state `RISC0_DEV_MODE=0` and wallet-CLI shield);
    verification-matrix optional rows that state `E2E_CLAIM_OPTIONAL=0`, real
    proving, + gate-log link. Leave the public Store default-`1` line
    untouched; add a one-line distinguisher that privacy gates are the
    exception. Do not mirror local isolation cells.
18. Gate log run rows: Artifact = path under `.scaffold/e2e/artifacts/`;
    Notes = `RISC0_DEV_MODE`, `E2E_CLAIM_OPTIONAL`, ImageID Y, `SKIP_BUILD=1`,
    shield path (`wallet` vs logoscore).
    Append a short summary that required gates are green (or incomplete).
19. Agent stops. Human reviews gate log + artifacts, then moves the packet to
    completed (or writes off incomplete gates per D39.15). Agent does not
    move the packet or close the raw TODO until human directs.

## Verification

| Gate | Command | Pass criteria |
| --- | --- | --- |
| Local module full (Phase 1 soft) | `SKIP_BUILD=1 MODE=module CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` | Exit 0 (soft OK) |
| Local Store full (Phase 1) | `make verify-store-local-full-privacy` | Exit 0 |
| Local public | `make verify-module-local`; `make verify-store-local` | Exit 0 |
| Build / freeze | `make build`; `make program-id` | Y recorded; Docker ELF path exists |
| Deploy + sync | `make deploy-testnet` + D39.11 | Fixtures/scripts tip = Y; prefix checks |
| Public module testnet | `SKIP_BUILD=1 make verify-module-testnet` | Exit 0 |
| Public Store testnet | `SKIP_BUILD=1 make verify-store-testnet` | Exit 0 |
| Local module real-prove smoke | Phase 3b command (`RISC0_DEV_MODE=0`, claim optional 0; wallet-CLI shield) | Exit 0; fix port-gaps before testnet |
| Module full testnet | Phase 4 command (`RISC0_DEV_MODE=0`, claim optional 0) | Exit 0; fix before Store |
| Store full testnet | Phase 4 command (`RISC0_DEV_MODE=0`, claim optional 0) | Exit 0; vault_holding drop on claim |

## Definition of done

Agent-reported (gate log + artifacts):

- [x] Freeze commit + ImageID Y + ELF size in gate log; Docker ELF deployed
  (or Y-equal no-op per D39.16); fixtures/operational defaults match Y
  (D39.10–D39.11).
- [x] Public module + Store testnet green (one pass each).
- [x] Wallet-CLI shield/dust harness (D39.22) + local module real-prove smoke
  green (D39.24).
- [x] Module full privacy testnet green with real proving (`RISC0_DEV_MODE=0`)
  and `E2E_CLAIM_OPTIONAL=0`.
- [x] Store full privacy testnet green with real proving (`RISC0_DEV_MODE=0`),
  `E2E_CLAIM_OPTIONAL=0`, and vault_holding drop confirmation.
- [x] E2E.md + verification-matrix updated to Phase 5 minimum (D39.14).
- [x] Gate-log summary that required gates are green (or incomplete with risk).

Human-only (D39.15):

- [x] Human reviews gate log + artifacts and moves packet to
  `docs/plan/completed/` (and closes raw TODO / refreshes index), or appends
  an explicit write-off for incomplete gates. Agent does not self-close.
  Closed 2026-08-05 on artifacts
  `module-e2e-20260722T215542.log` and `e2e-20260724T144726.log`.

## Deliverables

- [x] Packet, wiring, gate log, decisions through D39.21 (Phase 0 + proving amendment).
- [x] Phase 1 preflight + freeze build.
- [x] Phase 2 deploy + fixture sync + prefix checks.
- [x] Phase 3 public testnet.
- [x] Phase 3b wallet-CLI shield + local real-prove smoke (D39.22–D39.24).
- [x] Phase 4 module full then Store full (real proving).
- [x] Phase 5 docs + agent gate-log summary.
- [x] Human close (completed 2026-08-05).
- [x] Decisions D39.22–D39.24 locked in packet (2026-07-22).

## Risk

| Risk | Mitigation |
| --- | --- |
| ImageID surprise (host vs Docker) | Docker ELF only (D39.10). |
| Optional claim fakes privacy DoD | `E2E_CLAIM_OPTIONAL=0` on Phase 4 (D39.13). |
| Soft proofs on public testnet | DoD requires `RISC0_DEV_MODE=0` (D39.4); soft only for Phase 1. |
| Logoscore IPC vs PPE wall clock | Wallet-CLI shield (D39.22); local smoke then timeout port-gap for `chainAction` (D39.24). |
| Real-proof wall clock / timeouts | Raise as port-gap (D39.21); no soft fallback. |
| Missing NVIDIA / CUDA | Continue on CPU (D39.26); gate on includable PPE smoke, not `nvidia-smi`. |
| Dual-host / dust / guest env | Warm-up module first; port-gap fixes in scope (D39.6). |
| Shared fixture wallet after module | Fresh vault_id; re-fund before Store. |
| Recycled private ids after seed reclones | `prepare-testnet-privacy-seed.sh` + refuse list; public funder SSOT = `testnet-module.json`. |
| CLOCK_50 tick drains Store unaccrued | Do not advance CLOCK_50 before fundable/query; advance before close for claim. |
| Deploy / credential failure | Agent tries; flag in gate log and stop (D39.9). |
| Strict claim flake | Escalate; no optional-claim bypass (D39.13); human write-off only (D39.15). |
| Y equals fixture | Redeploy no-op path (D39.16); still run privacy gates. |
| Later multi-token redeploy | Accepted; this step is v1 network proof. |

## Related

- [step-36-payer-funder-unlinkability.md](step-36-payer-funder-unlinkability.md)
- [step-37-payee-receiver-privacy.md](step-37-payee-receiver-privacy.md)
- [step-38-store-privacy-e2e.md](step-38-store-privacy-e2e.md)
- [testnet-privacy-e2e-after-guest-deploy.md](testnet-privacy-e2e-after-guest-deploy.md)
- [testnet-guest-program-id-sync-2026-07-15.md](testnet-guest-program-id-sync-2026-07-15.md)
- [step-39-testnet-gate-log.md](step-39-testnet-gate-log.md)
- [E2E.md](../../journeys/E2E.md),
  [verification-matrix.md](../../reference/verification-matrix.md)
- LEZ testnet tutorial `token-transfer.md` (wallet `auth-transfer send`
  Public→Private; prove wall clock note)
