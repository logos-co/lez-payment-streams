# Step 21 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

**Optional stretch** after Step 20 — time-boxed; not required for integration milestone closure.
Track split: [N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).

### Step 21, Payment streams Basecamp UI

Prerequisite: Steps 10–11 and `payment_streams_module` operational on local LEZ (same fixture
patterns as runbooks). Does **not** require Step 17 dual-host demo or `delivery_module` in the
UI host unless an advanced integrator panel is added deliberately.

#### Scope boundary

This step is **payment streams protocol only** (LIP-155 / LEZ vaults and streams). It is **not**
the Store eligibility integration demo (that is Step 20 + Step 17 script). Store, `storeQuery`,
eligibility hook registration, and dual-`logoscore` provider/user layout are **out of scope**.

Canonical module id: `payment_streams_ui` (`metadata.json` `name`); plugin id
`payment_streams_ui_plugin` (`logos-module-builder` conventions).

Architectural context:

- `type: ui_qml` plugin with C++ backend calling **`payment_streams_module`** and
  **`logos_execution_zone`** via LogosAPI only — no `delivery_module`, no script orchestrator,
  no demo coordinator (Step 25 won't fix).
- Thin wrapper: buttons/forms map to existing LogosAPI (`chainAction`, reads, `listMyStreams`,
  `rediscoverStreams`) per [integration-contracts.md](../../reference/integration-contracts.md).
- **Single Basecamp / single `logoscore`** for typical flows; counterparty service is **out of
  band** (not modeled in UI).

#### Payer flows (in scope)

Examples: open wallet, initialize vault, deposit, create stream to a **payee account id**
(base58), list vaults/streams, pause/resume/top-up/close stream, read vault/stream status.

#### Payee flows (optional but documented)

To show the **receiver** side, include **claim** (`chainAction claim`) after accrual.

The payee cannot claim without knowing **which stream** to claim from. **User Journey assumption
(N18):** the stream creator sends the stream identity out of band (vault id, stream id, and any
manifest fields the payee wallet needs) to notify the payee where accrued funds can be claimed.
The plugin may offer a form to paste those ids; it does **not** implement discovery, Store, or
messaging.

#### Deliver

- `ui_qml` `.lgx` buildable with `nix build`; loads in Basecamp without QML errors.
- Dependency pins aligned with [feature-branch-pins.md](../../reference/feature-branch-pins.md) for
  `payment_streams_module` and patched wallet `.lgx`.

Definition of done:

- Operator can run payer flows (minimum: create stream, list streams) on local LEZ through the UI.
- If payee claim is implemented: doc packet / in-app hint states the out-of-band stream-id
  assumption ([N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)).

Not in scope: Step 17 E2E replay in UI; `delivery_module`; eligibility prepare/verify for Store;
Step 20 Developer Journey content (cross-link only).

Follow-on: Step 22 (User Journey doc packet) — Step 22 is active and CLI-based;
if Step 21 ships, Step 22 may be updated with UI content.
