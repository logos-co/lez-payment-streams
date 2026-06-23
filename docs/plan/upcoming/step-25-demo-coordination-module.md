# Step 25 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 25, Demo coordination Logos module

Prerequisite: Step 18 definition of done (testnet sequencer E2E, local dual-host Store).
Step 17 remains the local-LEZ regression gate and must keep passing throughout this step.

Architectural context:

Steps 17 and 18 coordinate the dual-host paid Store E2E run from outside the node via
`scripts/e2e/run_local_e2e.py` (Python; shells out to `logoscore call` and `logoscore
watch`). This step replaces that external orchestrator with an in-process Logos module so
the demo is driven by a single `logoscore` invocation (or, later, a Basecamp plugin) rather
than a host-side script.

The coordinator module drives the three production modules already on the host
(`logos_execution_zone`, `payment_streams_module`, `delivery_module`) via LogosAPI. It does
not re-wrap their APIs or duplicate their logic. It owns only sequencing, waits, retries,
and artifact emission — the same responsibilities the Python orchestrator has today.

What moves into the module:

- Fixture manifest loading (`FIXTURE_MANIFEST`) and wallet sync to sequencer height.
- Dual-host bring-up sequencing: `load-module` order, `logos_execution_zone open`,
  `delivery_module createNode` + `start`, eligibility registration
  (`setEligibilityVerifier` on provider, `setEligibilityProvider` on user,
  `registerProviderMapping`).
- Provider advertisement emission (`provider_peer_id`, `provider_store_multiaddr`,
  `content_topic`, `service_id`) for the user host to consume, replacing the off-band
  `E2E_PROVIDER_AD` file.
- Happy path: user `subscribe` + `send`, then `prepareEligibilityForStoreQuery`,
  `storeQuery` with attached proof, wait on `storeQueryCompleted`, assert non-empty
  messages and status 200.
- Missing-proof failure path: clear user `setEligibilityProvider`, re-issue `storeQuery`,
  assert 400 / empty messages / non-OK verdict.
- Provider claim (`chainAction claim`) and `tx_hash` capture.
- JSON-lines artifact emission, one object per phase, to a path under the host persistence
  dir (shape aligned with the Step 17 artifact so downstream tooling is unchanged).
- Late stream create (`ensure_fresh_demo_stream`) and top-up retries, ported from the
  Python orchestrator.

What stays out of the module:

- Chain fixture seeding (Step 17b snapshot restore, `demo-localnet-prepare.sh`,
  `demo-localnet-fresh.sh`). The module assumes a funded sequencer and a manifest with
  `vault_id` / `stream_id` / account ids; fixture preparation remains a host-side
  prerequisite. The module may call `createStream` / `topUpStream` via
  `payment_streams_module.chainAction` to ensure a fresh stream at proof time, mirroring
  the current late-create path.
- Building and installing `.lgx` artifacts. `make verify-step17` / `verify-step18` still
  run `nix build` and `lgpm install` before invoking the module.
- N8 canonical wire hex computation. Host `cargo run -p lez-payment-streams-core --bin
  n8_canonical_wire_hex` remains the source; the module accepts `N8_WIRE_HEX` (or an
  equivalent LogosAPI argument) rather than building the guest toolchain in-process.

#### Module shape

Universal module (`type: core`), sibling of or under `logos-payment-streams-module`. Suggested
runtime id: `payment_streams_demo_coordinator` (do not collide with `payment_streams_module`).
Public LogosAPI methods (single-line `Q_INVOKABLE`, codegen per
[D6](../../reference/decisions-and-notes.md#d6-universal-module-interface)):

| Method | Role |
| --- | --- |
| `runDemo` | Single entry: args are manifest path, wallet config/storage, modules dir, persist dir, phase (`core` / `claim` / `all`), optional chain selector (`local` / `testnet`). Emits phase artifacts as events; returns final status JSON. |
| `getArtifact` | Read the current run's JSON-lines artifact path (for Basecamp / external viewers). |
| `cancelDemo` | Stop the in-flight run and tear down daemons the module started. |

Events (LogosAPI event stream):

- `demoPhase` — one per phase (`seed`, `provider_ad`, `store_query_success`,
  `store_query_missing_proof`, `claim`, `late_create_stream`, `fatal`), payload matches the
  Step 17 artifact row shape so existing triage tooling keeps working.
- `demoCompleted` — final status, artifact path, exit reason.

Dual-host layout: the module runs inside a single `logoscore` process that loads the user
and provider module sets in separate namespaces (mirroring the Step 17 two-daemon layout).
If a single-process dual-host layout is not feasible without delivery changes, the module
may drive a second local `logoscore` subprocess for the provider role via LogosAPI; document
the chosen layout in the step completion note. Either way, the operator invokes one command
(`make verify-step17` / `verify-step18` or `logoscore call payment_streams_demo_coordinator
runDemo ...`), not a Python script.

#### Deliver

- New module under `logos-payment-streams-module` (or sibling repo); `nix build .#lgx`
  produces an installable `.lgx`.
- `make verify-step17` and `make verify-step18` retargeted to invoke the coordinator
  module. The Python orchestrator (`scripts/e2e/run_local_e2e.py`,
  `scripts/e2e/debug_happy_path.py`, `scripts/e2e/seed_provider_acceptance.py`) is retired
  or moved to `scripts/e2e/legacy/`; the artifact path and phase row shape stay stable so
  downstream tooling (CI, dashboards) is unchanged.
- Runbook section in [step17-e2e-local.md](../../step17-e2e-local.md) and the Step 18
  runbook documenting the new single-command entry and the module load order
  (`logos_execution_zone`, `payment_streams_module`, `delivery_module`,
  `payment_streams_demo_coordinator`).

#### Definition of done

- `make verify-step17` (local LEZ) and `make verify-step18` (testnet) emit the same
  artifact rows as today (`store_query_success`, `store_query_missing_proof`, `claim`)
  from inside the coordinator module, with no external Python orchestrator on the run
  path.
- Local-LEZ regression gate (Step 17) stays green; hermetic
  (`SKIP_LIBLOGOSDELIVERY_OVERLAY=1`) and overlay paths both supported.
- `payment_streams_demo_coordinator.runDemo` is the single entry operators and Step 20
  (developer journey) document; no host-side Python glue required for a clean run.
- Artifact path and phase row shape unchanged from Step 17 so CI and dashboards do not
  need updates.

Not in scope: Basecamp UI (Step 21); public hosted provider (Step 23); changing LIP-155,
Store tag `30`, or delivery hook ABIs; new privacy tiers or shielded execution paths.

Follow-on: Step 20 (developer journey) documents this module's `runDemo` entry as the
single-command demo; optional Step 21 (Basecamp) wraps `runDemo` in a `ui_qml` plugin.
