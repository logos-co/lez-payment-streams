# Step 20 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 20, Developer journey doc packet

**Next active integration deliverable** (local LEZ). Step 18 testnet remains paused; defer
testnet and hosted-provider journey rows until Step 18 (and Step 23 where applicable) unblock.

Prerequisites:

| Journey runtime target | Required before Step 20 |
| --- | --- |
| Local LEZ (script-orchestrated dual-host demo) | Steps 17 and 19 DoD satisfied; **Step 24c local gate** complete (`make verify-step17-back-to-back`) |
| Testnet v0.2 (public LEZ, local dual-host Store) | Steps 17, 18 DoD satisfied (Step 18 Part B on rc5 tooling) |
| Testnet v0.2 + hosted public Store provider | Steps 17, 18, and 23 DoD satisfied |
| Any | Step 19 on `feat/payment-streams-onchain-part` (`345c8eef`); cite in packet ([feature-branch-pins.md](../../feature-branch-pins.md)) |

Orchestration policy: [N17](../../reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06).
Track split: [N18](../../reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)
(Step 20 = **Track A** integration demo only; Steps 21–22 = optional **Track B** payment
streams UI — not part of this step).
No in-process demo coordinator module (Step 25 won't fix).

Architectural context:
Logos documentation intake uses a doc packet issue in `logos-co/logos-docs` (template
[`resources/templates/doc-packet.md`](https://github.com/logos-co/logos-docs/blob/main/resources/templates/doc-packet.md),
label `type:journey`). Docs drafts the public page; R&D SME reviews; Red Team dogfoods the
published instructions ([`logos-docs/CONTRIBUTING.md`](https://github.com/logos-co/logos-docs/blob/main/CONTRIBUTING.md)).

This step is **Track A** ([N18](../../reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)):
the documentation deliverable for **integrators** showing payment streams **composed with
Logos Delivery Store** (LIP-155 eligibility on Store requests — one protocol use case). It is not
the payment-streams-only Basecamp UI journey (Track B, Steps 21–22).

Paid Store + eligibility uses **`payment_streams_module`**, **`delivery_module`**, and
**`logos_execution_zone`**, not Basecamp clicks. Demo coordination is a **host-side script**
that drives two local `logoscore` instances — the same model as Step 17
([step17-e2e-local.md](../../step17-e2e-local.md)). Integrators may later wrap the same module
calls in their own Logos app module; that pattern is mentioned briefly but not implemented here.

#### Journey structure (local LEZ)

Publish two tiers in one doc packet (or two linked sections):

1. **One-command path** — reproduce the full demo with fixture prepare + verify entrypoint
   (`make verify-step17` or `./scripts/demo-e2e-local.sh`), pointing at
   [`scripts/e2e/run_local_e2e.py`](../../../scripts/e2e/run_local_e2e.py) as the dual-host
   orchestrator. Success criteria: JSON-lines artifact phases
   (`store_query_success`, `store_query_missing_proof`, `claim`) under
   `.scaffold/e2e/artifacts/`.
2. **Step-by-step path** — same outcome without the Python orchestrator: explicit commands for
   **user** and **provider** `logoscore` configs (module load, wallet `open`, delivery
   `createNode` / `start`, eligibility registration, publish, paid `storeQuery`, missing-proof
   check, claim). Lift command order and JSON shapes from the runbook and from the script
   (script is normative for ordering until the journey is validated).

Both tiers must cite [integration-contracts.md](../../integration-contracts.md) for method names
and encodings; do not duplicate full contract tables in the packet.

Deliver:

- Filled doc packet: outcome, components (three production modules + script orchestrator),
  pinned repo refs ([feature-branch-pins.md](../../feature-branch-pins.md)), runtime target
  (local LEZ first), tier-1 and tier-2 command blocks, expected logs/JSON, configuration
  (`FIXTURE_MANIFEST`, `registerProviderMapping`, eligibility hooks, async `storeQuery`
  completion), failure modes + [demo-localnet-recovery.md](../../demo-localnet-recovery.md).
- SME validation: run tier-1 verbatim; spot-check tier-2 against script behavior before handoff.
- Link LIP-155 on-chain section (Step 19) and integration contracts.

Definition of done:

- Doc packet issue filed and linked in the journeys workflow; SME sign-off on technical content.
- Red Team completes when org process requires `quality:verified` on the published doc (tracked on
  logos-docs project board, not via a script in this repo).

Not in scope: Step 25 demo coordinator module; Track B payment streams UI (Steps 21–22);
hosted provider ops (Step 23) unless the journey targets that deployment model; new backend
features.

Pattern references:
[logos-docs#311](https://github.com/logos-co/logos-docs/issues/311),
[logos-docs#307](https://github.com/logos-co/logos-docs/issues/307).
