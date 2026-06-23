# Step 20 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 20, Developer journey doc packet

Prerequisites by published runtime target:

| Journey runtime target | Required before Step 20 |
| --- | --- |
| Local LEZ (coordinator module from Step 25) | Steps 17 and 25 DoD satisfied |
| Testnet v0.2 (public LEZ, local dual-host Store via coordinator module) | Steps 17, 18, and 25 DoD satisfied |
| Testnet v0.2 + hosted public Store provider | Steps 17, 18, 25, and 23 DoD satisfied |
| Any | Step 19 complete on `feat/payment-streams-onchain-part` (`345c8eef`); cite that branch in the doc packet ([feature-branch-pins.md](../../feature-branch-pins.md)) |

Architectural context:
Logos documentation intake uses a doc packet issue in `logos-co/logos-docs` (template
[`resources/templates/doc-packet.md`](https://github.com/logos-co/logos-docs/blob/main/resources/templates/doc-packet.md),
label `type:journey`). Docs drafts the public page; R&D SME reviews; Red Team dogfoods the
published instructions ([`logos-docs/CONTRIBUTING.md`](https://github.com/logos-co/logos-docs/blob/main/CONTRIBUTING.md)).

This step is the primary documentation deliverable for integrators: reproduce paid Store +
LIP-155 eligibility via `logoscore` and module APIs, not Basecamp clicks. The single-command
entry is `payment_streams_demo_coordinator.runDemo` (Step 25); the doc packet must not document
the retired Python orchestrator (`scripts/e2e/run_local_e2e.py`). Pattern references:
[logos-docs#311](https://github.com/logos-co/logos-docs/issues/311),
[logos-docs#307](https://github.com/logos-co/logos-docs/issues/307).

Deliver:

- Filled doc packet: outcome, components (`payment_streams_module`, `logos_execution_zone`,
  forked `delivery_module`, `payment_streams_demo_coordinator`), pinned repo refs
  ([`feature-branch-pins.md`](../../feature-branch-pins.md)), runtime target (testnet v0.2 when
  Step 18 landed), copy-paste happy path lifted from the Step 25 `runDemo` invocation and Step
  17/18 fixture prepare, success command, expected JSON/log outcomes (phase row shape unchanged
  from Step 17), configuration (`FIXTURE_MANIFEST`, `registerProviderMapping`, eligibility
  registration, async `storeQuery` event), failure modes.
- SME validation: run the issue command block verbatim before handoff to Docs.
- Link LIP-155 on-chain section on branch `feat/payment-streams-onchain-part` (Step 19) and
  [`integration-contracts.md`](../../integration-contracts.md) for API shapes; do not duplicate
  full contract tables in the packet.

Definition of done:

- Doc packet issue filed and linked in the journeys workflow; SME sign-off on technical content.
- Red Team completes when org process requires `quality:verified` on the published doc (tracked on
  logos-docs project board, not via a script in this repo).

Not in scope: UI journey (Step 22); hosted provider ops (Step 23) unless the journey targets
that deployment model; implementing new backend features.
