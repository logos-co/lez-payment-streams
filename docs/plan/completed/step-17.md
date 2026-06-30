# Step 17 — plan excerpt (complete)

Normative packet for agents. Index: [index.md](../index.md).
Operator runbook: [store-integration/README.md](../../store-integration/README.md),
[archive/steps/local-store-dual-host-runbook.md](../../archive/steps/local-store-dual-host-runbook.md).

Status: Complete (2026-06-19). The full local dual-host gate is green via `make verify-step17`
(`scripts/e2e.sh` local run + `scripts/e2e/run_local_e2e.py`) on a clean tree plus a prepared
localnet fixture: paid `storeQuery` returns `statusCode:200` with messages, missing-proof is
rejected, and provider claim writes a `tx_hash`. Fixture prepare is
[Step 17b](step-17b-localnet-snapshot-restore.md) (`make prepare-localnet` / `scripts/e2e.sh local prepare`;
default restore + vault-only manifest; per-run stream in E2E — [Step 24c](step-24c-simplify-demo-flow.md);
`FULL_RESET=1` rebuilds snapshot via `e2e.sh local prepare`).
After a **guest rebuild** (`make build`) or LEZ pin change, run `FULL_RESET=1` prepare before E2E
(snapshot `program_id_hex` must match the deployed ImageID; restore-only prepare fails otherwise).
JSON-lines artifacts land under `.scaffold/e2e/artifacts/`. The 2026-06-18 checkpoint reported the
wiring green but the happy-path `storeQuery` still failed `BAD_REQUEST`; root cause was provider
verify returning `PARAMS_REJECTED` / `RateBelowAcceptedParams` because `fillServiceId` clobbered
the on-chain rate/allocation in `acceptedParams` (fixed 2026-06-19, with `reject_reason` now
surfaced in the verdict and an E2E `store_query_eligibility_verdict` diagnostic line).
Delivery install uses `nix build …#lgx` + `lgpm install` for all three modules; optional
`liblogosdelivery` overlay from sibling `logos-delivery` when not using hermetic mode
([N13](../../../reference/decisions-historical.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18),
runbook [Hermetic run](../../archive/steps/local-store-dual-host-runbook.md#hermetic-run-hand-off)).
Module bridge invokes eligibility on the `LogosAPIClient` thread ([N3a](../../reference/integration-decisions.md#n3a-step-16-threading--approach-a-experiment-2025-06-18)).
Seed economics (full rebuild, superseded values in parentheses): deposit `1000` / allocation
`200` / rate `1` via `prefund-localnet.sh` and env defaults (was `2000` / `1800`). Repeat runs
restore the funded vault baseline; each E2E run creates at `next_stream_id` and teardown closes
([Step 24c](step-24c-simplify-demo-flow.md), [N15](../../../reference/decisions-historical.md#n15-step-17b-localnet-snapshot-restore-2026-06-19)).

Post-Complete hardening (not gating): re-run the hermetic path (`SKIP_LIBLOGOSDELIVERY_OVERLAY=1`)
after the fix and drop the default overlay once hermetic installs are routine; align delivery
`.lgx` layout with `logoscore` if `MODULE_LOAD_FAILED` appears on clean machines only.
Back-to-back monotonic stream ids: `make verify-step17-back-to-back` (Step 24c).

### Step 17, End-to-end demo wiring

Prerequisite: Step 16 bridge complete ([N12](../../../reference/decisions-historical.md#n12-step-16-vs-step-17-verification-scope-2025-06-18);
`storeQuery` and eligibility routing on our delivery forks; see [N6](../../reference/integration-decisions.md#n6-delivery-module-store-query-exposure)).

Architectural context:
this step exercises every layer at once (two `logoscore` hosts, three backend modules each,
local LEZ, cross-host Store). Scope boundary: [N12](../../../reference/decisions-historical.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)
(Step 16 proved the bridge; Step 17 proves full-stack paid Store and eligibility on the wire).

Paid Store demo policy:
the provider operates in paid mode only (eligibility verifier registered for the whole demo).
Real users learn the provider libp2p identity from off-band service advertisement; the script
mimics that with a small advertisement artifact (see runbook). Inbound requests with no
LIP-155 proof must fail per [N3c](../../reference/integration-decisions.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18).

The user host must call `registerProviderMapping` on `payment_streams_module` before legitimate
outbound Store queries (PeerId from the advertisement file → manifest payee base58).

#### Localized chain fixture (DoD helper)

Do not require a full proposal → `createStream` arc on every run. The script should:

- detect whether the Step 10a fixture is usable (`fixtures/localnet.json`, sequencer up, stream
  `0` eligible for `stream_proof` or recover via top-up),
- if not, run the same blank-slate / seed path as `make full-reset-localnet` (deploy program, seed manifest; formerly [`scripts/archive/demo-localnet-fresh.sh`](../../../scripts/archive/demo-localnet-fresh.sh)).

Reuse seeded stream `0` for the happy-path Store query when chain state allows.

#### Dual hosts on one machine

Use **separate install and runtime trees** per role (future-proof for testnet and different
wallet homes later):

| Role | Env (convention) | Purpose |
| --- | --- | --- |
| User | `MODULES_USER`, `LOGOSCORE_CONFIG_USER`, `PERSIST_USER` | Client logoscore + modules |
| Provider | `MODULES_PROVIDER`, `LOGOSCORE_CONFIG_PROVIDER`, `PERSIST_PROVIDER` | Store provider logoscore + modules |

Two daemons on one machine are supported: give each `-D` process its own `--config-dir`
(and `--persistence-path`, `-m`). Point CLI `call` / `watch` / `stop` at the same
`--config-dir` as the daemon you target. Delivery P2P ports must not collide — use disjoint
`portsShift` on each `createNode` (example: user `0`, provider `100`).

Default layout under repo `.scaffold/e2e/` is documented in the runbook.

Build and install `.lgx` packages for `logos_execution_zone` (patched wrapper),
`payment_streams_module`, and `delivery_module` from the Step 16 integration branch into
**both** module directories (same artifacts, two install roots).

Create `scripts/demo-e2e-local.sh` that (implemented post–Step 24c as
[`scripts/e2e.sh`](../../../scripts/e2e.sh) `local run` +
[`scripts/e2e/run_local_e2e.py`](../../../scripts/e2e/run_local_e2e.py); archived wrapper
[`scripts/archive/demo-e2e-local.sh`](../../../scripts/archive/demo-e2e-local.sh)):


- ensures local LEZ + program + fixture (seed-if-needed above),
- builds/installs modules into `MODULES_USER` and `MODULES_PROVIDER`,
- starts provider then user `logoscore` with disjoint config and delivery `portsShift`,
- writes provider service advertisement (peer id + dial multiaddr) for the user host to consume,
- configures provider `delivery_module` with relay + Store service, SQLite archive, retention
  `capacity:10000` (defaults in runbook),
- configures user `delivery_module` as relay-capable Store **client** with static dial to provider,
- registers eligibility hooks (`setEligibilityVerifier` on provider, `setEligibilityProvider`
  on user for happy path),
- runs publish → archive → paid `storeQuery` success path,
- runs missing-proof failure (user issues Store without outbound proof while provider verifier
  stays enabled — see runbook),
- captures a structured log artifact (format in runbook).

Optional later: third publisher node for messages the user did not originate (not DoD).

#### Phase B — provider claim (required)

After accrual from served Store traffic (or existing stream balance), the provider host submits
`chainAction` `claim` for the demo vault/stream. This is required for Step 17 completion.

If implementation scope is too large for one change set, split delivery:

| Phase | Artifact | Scope |
| --- | --- | --- |
| 17A | `make verify-step17` / `scripts/e2e.sh local run` (or `E2E_PHASE=core`) | Dual host, hooks, Store success + missing-proof failure, logs |
| 17B | same script `--phase claim` or follow-up commit | Provider `claim` + log lines for chain txs |

Do not renumber Steps 18–23; 17B is a sub-phase of Step 17 only.

Components required to run:
LEZ sequencer on `127.0.0.1:3040`,
deployed `lez_payment_streams`,
two `logoscore` daemons (user + provider),
each with `logos_execution_zone`, `payment_streams_module`, `delivery_module`,
provider Store + SQLite archive,
user Store client targeting provider multiaddr,
shared or role-specific wallet files (demo may reuse seeded `.scaffold/wallet` on both hosts).

Definition of done:

1. Bridge and hooks: Step 16 branch installed on both hosts; provider registers
   `payment_streams_module` as **verifier**; user registers it as **provider** for outbound
   paid queries.
2. Success path: user `delivery_module.storeQuery` against provider multiaddr returns success
   with valid chain state and proofs; provider inbound eligibility OK.
3. Failure path: with provider verifier enabled, an inbound Store request **without** proof yields
   `BAD_REQUEST` (400), populated `eligibility_status`, empty messages ([N3c](../../reference/integration-decisions.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18)).
4. Claim (phase B): provider `chainAction` `claim` succeeds for manifest vault/stream when
   preconditions are met (document in artifact).
5. Artifact: script exits 0 on a clean workspace and writes
   `.scaffold/e2e/artifacts/e2e-<timestamp>.log` (JSON-lines phases; see runbook; legacy name
   `demo-e2e-local-*.log` from archived wrapper).

Follow-on: Step 18 (public sequencer, local P2P), Step 19 (LIP on-chain), Step 20 (developer
journey); Step 22 (User Journey doc, active); optional Step 21 (UI), Step 23 (hosted Store provider). See
[index.md](../index.md#program-outcomes).
