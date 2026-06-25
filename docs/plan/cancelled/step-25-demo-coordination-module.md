# Step 25 — demo coordination Logos module (won't fix)

Status: **Won't fix** (2026-06). Index: [integration-index.md](../../../integration-index.md).

Decision: [N17](../../reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06).
Active demo and developer journey use **external orchestration** (Step 17 scripts), not an
in-process `payment_streams_demo_coordinator` module.

## What was considered

An in-process Logos module (`runDemo`, dual-host sequencing via LogosAPI) to replace
`scripts/e2e/run_local_e2e.py` as the default CI entry and Basecamp hook.

## Why closed

- Dual-host E2E requires a driver **outside** each `logoscore` process (or subprocess CLI to
  both configs). A module inside one host does not simplify that topology.
- Integrators consume **`payment_streams_module`** and **`delivery_module`** APIs; demo
  coordination via host script matches that story (Step 20).
- A dedicated demo coordinator `.lgx` is out of scope for this integration milestone; third
  parties may still ship their own app modules that compose the production modules.

## What we ship instead

- Regression gate: `make verify-step17` → [`scripts/demo-e2e-local.sh`](../../../scripts/demo-e2e-local.sh)
  + [`scripts/e2e/run_local_e2e.py`](../../../scripts/e2e/run_local_e2e.py).
- Documentation: Step 20 developer journey (one-script path + step-by-step `logoscore` commands
  on user and provider hosts).

Do not implement Step 25 unless the program decision is explicitly reopened.
