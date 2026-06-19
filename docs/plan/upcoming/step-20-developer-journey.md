# Step 20 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 20, Developer journey doc packet

Prerequisites by published runtime target:

| Journey runtime target | Required before Step 20 |
| --- | --- |
| Local LEZ (script from Step 17 only) | Step 17 DoD satisfied |
| Testnet v0.2 (public LEZ, local dual-host Store) | Steps 17 and 18 DoD satisfied |
| Testnet v0.2 + hosted public Store provider | Steps 17, 18, and 23 DoD satisfied |
| Any | Step 19 on `main`, or doc packet links explicitly to the spec PR/branch until merge |

Architectural context:
Logos documentation intake uses a doc packet issue in `logos-co/logos-docs` (template
[`resources/templates/doc-packet.md`](https://github.com/logos-co/logos-docs/blob/main/resources/templates/doc-packet.md),
label `type:journey`). Docs drafts the public page; R&D SME reviews; Red Team dogfoods the
published instructions ([`logos-docs/CONTRIBUTING.md`](https://github.com/logos-co/logos-docs/blob/main/CONTRIBUTING.md)).

This step is the primary documentation deliverable for integrators: reproduce paid Store +
LIP-155 eligibility via `logoscore` and module APIs, not Basecamp clicks. Pattern references:
[logos-docs#311](https://github.com/logos-co/logos-docs/issues/311),
[logos-docs#307](https://github.com/logos-co/logos-docs/issues/307).

Deliver:

- Filled doc packet: outcome, components (`payment_streams_module`, `logos_execution_zone`,
  forked `delivery_module`), pinned repo refs ([`feature-branch-pins.md`](../../feature-branch-pins.md)),
  runtime target (testnet v0.2 when Step 18 landed), copy-paste happy path lifted from Step 17/18
  scripts, success command, expected JSON/log outcomes, configuration (`FIXTURE_MANIFEST`,
  `registerProviderMapping`, eligibility registration, async `storeQuery` event), failure modes.
- SME validation: run the issue command block verbatim before handoff to Docs.
- Link merged LIP-155 on-chain section (Step 19) and [`integration-contracts.md`](../../integration-contracts.md)
  for API shapes; do not duplicate full contract tables in the packet.

Definition of done:

- Doc packet issue filed and linked in the journeys workflow; SME sign-off on technical content.
- Red Team completes when org process requires `quality:verified` on the published doc (tracked on
  logos-docs project board, not via a script in this repo).

Not in scope: UI journey (Step 22); hosted provider ops (Step 23) unless the journey targets
that deployment model; implementing new backend features.
