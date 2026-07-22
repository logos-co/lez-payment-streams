# Step 39 — testnet privacy E2E after native guest deploy

Index: [index.md](../index.md). Status: **active**.

Goal: freeze the current native-token guest, redeploy it to public testnet
(agent-run), and close privacy v1 with soft proving: module full privacy
warm-up, then Store full privacy on that ImageID, without waiting for
multi-token vaults.

This step is agent-run deploy + verification only (D39.9).
Product paths: [Step 36](../completed/step-36-payer-funder-unlinkability.md),
[Step 37](../completed/step-37-payee-receiver-privacy.md).
Local Store privacy: [Step 38](../completed/step-38-store-privacy-e2e.md) Phases A/B.
Promotes Phase C raw TODO
([testnet-privacy-e2e-after-guest-deploy.md](../raw-todos/testnet-privacy-e2e-after-guest-deploy.md)).
Do not reopen Steps 36–38 product DoD.

Gate log (append runs here):
[step-39-testnet-gate-log.md](../completed/step-39-testnet-gate-log.md).

## Implementer plan of action

Do these in order. Stop on failure.
Soft proving (`RISC0_DEV_MODE=1` default). One green per required gate (D39.8).

| Step | Action | Required |
| --- | --- | --- |
| 1 | Local preflight (privacy first, then public): see Phase 1 | Yes |
| 2 | `make build` (Docker guest path); capture freeze commit + ImageID Y + ELF size | Yes |
| 3 | Credential / funding checklist (Phase 1); confirm Y ≠ fixture `de17c0db…` (or Y-equal path) | Yes |
| 4 | `make deploy-testnet`; fixture sync (not full re-bootstrap); prefix checks | Yes |
| 5 | Public testnet with `SKIP_BUILD=1` (module then Store) | Yes |
| 6 | Module full privacy testnet (`E2E_CLAIM_OPTIONAL=0`, soft proving) | Yes (warm-up) |
| 7 | Store full privacy testnet (`E2E_CLAIM_OPTIONAL=0`, soft proving) | Yes (primary) |
| 8 | Phase 5 docs minimum; gate-log summary that required gates are green | Yes (agent) |
| 9 | Human review of gate log + artifacts; then move packet to completed | Yes (human) |
| — | Isolation cells; `RISC0_DEV_MODE=0`; new Make aliases | Optional / deferred |

Exact env and commands: [Verification](#verification).

## Problem

Local privacy profiles are green.
Testnet fixtures still pin ImageID `de17c0db…` (deploy 2026-07-15).
Local fixtures currently show a newer tip (`072a26cc…` in `fixtures/localnet.json`)
as an example of drift, not a permanent expected id.
Authoritative ImageID is always `make program-id` after `make build` at the
freeze commit (D39.10).

## Why a new step (not reopen Step 38)

Step 38 DoD was local Store privacy (Phases A/B).
Step 39 owns freeze, deploy, fixture sync, and the required testnet privacy
gates. Do not reopen Step 38.

## Decision log (SSOT)

| Id | Topic | Outcome |
| --- | --- | --- |
| D39.1 | Packet ownership | New Step 39; do not reopen Step 38. |
| D39.2 | Guest freeze | Native-token only. Multi-token deferred. |
| D39.3 | Order | Local preflight → build/freeze → deploy → public testnet → module full → Store full. |
| D39.4 | Proving | Soft. `RISC0_DEV_MODE=1` closes DoD. Real proofs optional. |
| D39.5 | Matrix tier | Privacy testnet stays optional in verification-matrix until boringly green. |
| D39.6 | Harness | Inherit Step 38 / D38.8. Port-gap rule: see [Port-gap vs protocol](#port-gap-vs-protocol). |
| D39.7 | Required cells | Module full privacy, then Store full privacy. Isolation cells optional. |
| D39.8 | Green passes | One green per required gate. |
| D39.9 | Operator | Agent tries deploy (`make deploy-testnet`), then fixture sync and prefix checks. On deploy/credential failure, stop, flag in the gate log, and wait for human — do not invent a workaround. |
| D39.10 | Freeze + ImageID rule | See [Freeze and deploy identity](#freeze-and-deploy-identity). |
| D39.11 | Fixture sync | See [Fixture sync vs re-bootstrap](#fixture-sync-vs-re-bootstrap). |
| D39.12 | Testnet env defaults | See [Testnet command defaults](#testnet-command-defaults). |
| D39.13 | Claim strictness | Strict for required privacy gates: `E2E_CLAIM_OPTIONAL=0`. Vault_holding drop / claim_balance must pass. Do not reopen optional claim to close DoD. Public Phase 3 keeps default `1` even while Step 32 D3 has no pass row yet — that is intentional, not a Step 39 blocker. |
| D39.14 | Phase 5 minimum | Two required privacy recipes in E2E.md + optional matrix rows + gate-log link. Do not mirror local isolation cells. Distinguish privacy `E2E_CLAIM_OPTIONAL=0` from public Store default `1`. Make aliases deferred. |
| D39.15 | Close path | Agent never moves the packet to completed. Agent reports greens (or incomplete) in the gate log. Human alone decides completed or write-off. |
| D39.16 | Y-equal contingency | If freeze ImageID Y equals fixture `de17c0db…`, skip redeploy, log no-op, still verify fixtures + prefix checks, proceed. Do not invent a guest edit to force a new ImageID. (Post-`6772238b` guest commit `a59a66d` makes Y≠fixture the expected path.) |
| D39.17 | Phase 1 local purpose | Harness regression only (`SKIP_BUILD=1` against existing local guest). Not identity parity with freeze ImageID Y. |
| D39.18 | Funding targets | Fund before module full and again before Store full (defaults `OWNER_TARGET=550` / `PROVIDER_MIN=50`). Do not use a stacked target (for example 1200) for both runs — re-fund restores the public owner between runs. If Store funding-short: once `OWNER_TARGET=700 PROVIDER_MIN=100`, then escalate. No wallet wipe between runs. |
| D39.19 | Optional real proving | Skip by default. If run and fails: gate-log row marked optional / not DoD. Does not block close. |
| D39.20 | Store cold start | Orchestrator starts dual-host logoscore; no pre-running daemons required. Cold-start remediation from verification-matrix is in scope if local Store preflight fails; stop if it needs human auth/flake access. |

## Freeze and deploy identity

### Freeze (D39.10)

Procedural, not a git tag or CI lock:

1. Finish Phase 1 local gates on the intended tree.
2. Record `git rev-parse HEAD` as freeze commit in the gate log.
3. Run `make build` at that commit (no further guest edits, no multi-token).
4. Record ImageID Y from `make program-id` and ELF byte size.

Redeploy is mandatory when Y ≠ current
`fixtures/testnet-module.json` / `fixtures/testnet.json` `program_id_hex`
(today `de17c0db…`). Expected: Y differs because guest commit `a59a66d`
(clock normalization) postdates freeze `6772238b`.

If Y equals the fixture (D39.16): skip redeploy, log “ImageID unchanged;
redeploy no-op” in the gate log, still run fixture verify + prefix checks,
then continue to Phase 3. Do not edit the guest to force a new ImageID.

`072a26cc…` in `fixtures/localnet.json` is an example of local tip drift only.
Never treat it as the deploy target without running `make program-id`.

### Build path (D39.10)

Deploy ELF is the Docker guest output used by Make:

- `make build` → `cargo risczero build --manifest-path methods/guest/Cargo.toml`
- Binary: `methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin`
- Same path as `PROGRAM_BIN` in Makefile and `scripts/archive/testnet-common.sh`

Do not deploy a non-docker / host-built guest. Record ELF size next to ImageID
(same shape as
[testnet-guest-program-id-sync-2026-07-15.md](../completed/testnet-guest-program-id-sync-2026-07-15.md)).

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
   [testnet-guest-program-id-sync-2026-07-15.md](../completed/testnet-guest-program-id-sync-2026-07-15.md)).

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
| Phase 3 public module/Store | default `1` (Step 32 D3) | unset (`E2E_ALLOW_FIRE_AND_FORGET` off) |
| Phase 4 required privacy | `0` (strict) | unset (off) |

Reason: Store/module privacy confirm private claim via `vault_holding` drop.
With testnet default `E2E_CLAIM_OPTIONAL=1`, an unconfirmed claim is treated as
pass — that would fake Step 39 DoD. Required privacy runs must set
`E2E_CLAIM_OPTIONAL=0` so `claim_balance` / vault_holding drop is enforced.

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
- Making an already-locked explicit env (for example `E2E_CLAIM_OPTIONAL=0`)
  actually take effect on the command line

Stop and escalate (not port-gap):

- Guest / FFI / protocol edits
- Delivery wire changes
- Changing claim-optional policy or defaults — including editing
  `run_local_e2e.py` so testnet defaults to strict when
  `PROVIDER_PRIVACY=1`, or any other change to the unconditional
  `is_testnet` → `E2E_CLAIM_OPTIONAL=1` default
- Relaxing D39.13 to greenwash (setting `E2E_CLAIM_OPTIONAL=1` on a
  required privacy command to pass a flake)

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

Step 32 D3 may still have an empty gate-log runs table. That does not block
Step 39. Phase 3 public Store keeps default `E2E_CLAIM_OPTIONAL=1`. Phase 4
privacy overrides to `0` on the command line only. No dependency conflict.

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
| Optional `RISC0_DEV_MODE=0` fail? | Record optional row; cannot block (D39.19). |
| Who closes / write-off? | Human only (D39.15). Agent reports. |

## Open for deliberation

None. Locked through D39.20.

## Prerequisites

- Steps 36–38 complete.
- Verification-matrix cold start satisfied for Store testnet.
- Phase 1 credential checklist green (D39.12).

## Scope

In scope: freeze; agent deploy + fixture sync + prefix checks; public
regression; required privacy gates; port-gap harness fixes (D39.6); Phase 5
doc minimum (D39.14).

Out of scope: multi-token; reopening 36–38 product DoD; Delivery wire changes;
real-proof DoD; two consecutive greens; promoting matrix tier to required;
Make aliases (deferred); full six-cell privacy matrix; rewriting historical
deploy notes.

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
    (default `E2E_CLAIM_OPTIONAL=1` — kept while Step 32 D3 is open; not a
    Step 39 conflict; privacy strictness is Phase 4 only).

Stop if either fails.

### Phase 4 — Privacy E2E

`RISC0_DEV_MODE=1`. `E2E_CLAIM_OPTIONAL=0`. No fire-and-forget.

14. Prefund, then module full:
    ```bash
    ./scripts/fund-testnet-accounts.sh
    SKIP_BUILD=1 MODULE_E2E_SKIP_FUND=1 RISC0_DEV_MODE=1 E2E_CLAIM_OPTIONAL=0 \
      MODE=module CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
      ./scripts/e2e.sh testnet run
    ```
15. Prefund again, then Store full:
    ```bash
    ./scripts/fund-testnet-accounts.sh
    SKIP_BUILD=1 RISC0_DEV_MODE=1 E2E_CLAIM_OPTIONAL=0 \
      MODE=store CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
      ./scripts/e2e.sh testnet run
    ```
    Funding-short once: `OWNER_TARGET=700 PROVIDER_MIN=100` then re-fund (D39.18).

Pass: exit 0; Store phases include paid query + settlement; private claim
shows vault_holding drop (`claim_balance` with `provider_private` / vault_drop).

Optional `RISC0_DEV_MODE=0` (D39.19): skip by default; if attempted and fails,
log as optional / not DoD.

### Phase 5 — Housekeeping

16. Make aliases: deferred (not DoD).
17. Doc minimum (D39.14): E2E.md sections for the two required testnet privacy
    commands above only; verification-matrix optional rows that state
    `E2E_CLAIM_OPTIONAL=0` + gate-log link. Leave the public Store default-`1`
    line untouched; add a one-line distinguisher that privacy gates are the
    exception. Do not mirror local isolation cells.
18. Gate log run rows: Artifact = path under `.scaffold/e2e/artifacts/`;
    Notes = `RISC0_DEV_MODE`, `E2E_CLAIM_OPTIONAL`, ImageID Y, `SKIP_BUILD=1`.
    Append a short summary that required gates are green (or incomplete).
19. Agent stops. Human reviews gate log + artifacts, then moves the packet to
    completed (or writes off incomplete gates per D39.15). Agent does not
    move the packet or close the raw TODO until human directs.

## Verification

| Gate | Command | Pass criteria |
| --- | --- | --- |
| Local module full | `SKIP_BUILD=1 MODE=module CHAIN=local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` | Exit 0 |
| Local Store full | `make verify-store-local-full-privacy` | Exit 0 |
| Local public | `make verify-module-local`; `make verify-store-local` | Exit 0 |
| Build / freeze | `make build`; `make program-id` | Y recorded; Docker ELF path exists |
| Deploy + sync | `make deploy-testnet` + D39.11 | Fixtures/scripts tip = Y; prefix checks |
| Public module testnet | `SKIP_BUILD=1 make verify-module-testnet` | Exit 0 |
| Public Store testnet | `SKIP_BUILD=1 make verify-store-testnet` | Exit 0 |
| Module full testnet | Phase 4 command (claim optional 0) | Exit 0; fix before Store |
| Store full testnet | Phase 4 command (claim optional 0) | Exit 0; vault_holding drop on claim |

## Definition of done

Agent-reported (gate log + artifacts):

- [ ] Freeze commit + ImageID Y + ELF size in gate log; Docker ELF deployed
  (or Y-equal no-op per D39.16); fixtures/operational defaults match Y
  (D39.10–D39.11).
- [ ] Public module + Store testnet green (one pass each).
- [ ] Module full privacy testnet green with soft proving and
  `E2E_CLAIM_OPTIONAL=0`.
- [ ] Store full privacy testnet green with soft proving,
  `E2E_CLAIM_OPTIONAL=0`, and vault_holding drop confirmation.
- [ ] E2E.md + verification-matrix updated to Phase 5 minimum (D39.14).
- [ ] Gate-log summary that required gates are green (or incomplete with risk).

Human-only (D39.15):

- [ ] Human reviews gate log + artifacts and moves packet to
  `docs/plan/completed/` (and closes raw TODO / refreshes index), or appends
  an explicit write-off for incomplete gates. Agent does not self-close.

## Deliverables

- [x] Packet, wiring, gate log, decisions through D39.20 (Phase 0).
- [ ] Phase 1 preflight + freeze build.
- [ ] Phase 2 deploy + fixture sync + prefix checks.
- [ ] Phase 3 public testnet.
- [ ] Phase 4 module full then Store full.
- [ ] Phase 5 docs + agent gate-log summary.
- [ ] Human close (completed or write-off).

## Risk

| Risk | Mitigation |
| --- | --- |
| ImageID surprise (host vs Docker) | Docker ELF only (D39.10). |
| Optional claim fakes privacy DoD | `E2E_CLAIM_OPTIONAL=0` on Phase 4 (D39.13). |
| Dual-host / dust / guest env | Warm-up module first; port-gap fixes in scope (D39.6). |
| Shared fixture wallet after module | Fresh vault_id; re-fund before Store. |
| Deploy / credential failure | Agent tries; flag in gate log and stop (D39.9). |
| Strict claim flake | Escalate; no optional-claim bypass (D39.13); human write-off only (D39.15). |
| Y equals fixture | Redeploy no-op path (D39.16); still run privacy gates. |
| Later multi-token redeploy | Accepted; this step is v1 network proof. |

## Related

- [step-36-payer-funder-unlinkability.md](../completed/step-36-payer-funder-unlinkability.md)
- [step-37-payee-receiver-privacy.md](../completed/step-37-payee-receiver-privacy.md)
- [step-38-store-privacy-e2e.md](../completed/step-38-store-privacy-e2e.md)
- [testnet-privacy-e2e-after-guest-deploy.md](../raw-todos/testnet-privacy-e2e-after-guest-deploy.md)
- [testnet-guest-program-id-sync-2026-07-15.md](../completed/testnet-guest-program-id-sync-2026-07-15.md)
- [step-39-testnet-gate-log.md](../completed/step-39-testnet-gate-log.md)
- [E2E.md](../../journeys/E2E.md),
  [verification-matrix.md](../../reference/verification-matrix.md)
