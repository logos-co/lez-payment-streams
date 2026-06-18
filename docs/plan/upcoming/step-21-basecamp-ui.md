# Step 21 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

Optional track — see [program outcomes](../../../integration-index.md#program-outcomes). Execute
only when the milestone includes Basecamp UI + UI journey (Step 22). Otherwise stop after Step 20.

### Step 21, Basecamp UI demo

Prerequisite: Step 17 demo wiring (local); Step 18 if the UI demo targets testnet LEZ.

Canonical module id for the plugin: `payment_streams_ui` (`metadata.json` `name`);
main plugin id `payment_streams_ui_plugin` (follow `logos-module-builder` template conventions).

Architectural context:
the UI plugin is a Logos module (`type: ui_qml` with a C++ backend), not part of Basecamp.
Basecamp loads it like `logoscore` loads core modules for Steps 7–17.
The plugin calls the same backend modules through `LogosAPI`; no protocol or guest work.

Scaffold a `ui_qml` plugin (sibling repo or under `logos-basecamp`) from
`logos-module-builder` `ui-qml-backend`, modeled on `logos-delivery-demo`.
Dependencies: `payment_streams_module`, `delivery_module` (integration branch pins).
Construct `LogosModules` in `initLogos`; wire async `storeQuery` completion like chat module
event handlers.

Surface vault state, stream state, pending proposal slot, and the most recent Store query result.
No custom backend beyond thin C++ → module calls.

Components: everything from Step 17 (and Step 18 if testnet) plus `logos-basecamp` and `lgpm`
install into Basecamp plugins directory.

Definition of done:

- `nix build` produces a `.lgx` that loads in Basecamp without QML errors.
- User completes the full demo flow through the UI without CLI (dual-instance layout as needed for
  user vs provider roles).
