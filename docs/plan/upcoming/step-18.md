# Step 18 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 18, Optional Basecamp UI

Blocked on Step 17 demo wiring.

Architectural context:
the UI plugin added here is itself a Logos module
(`type: ui_qml` with a C++ backend),
not a piece of Basecamp.
Basecamp is the host that loads it,
in the same sense that `logoscore` is the host for Steps 7–17.
The plugin calls the unchanged backend modules from earlier steps
through the same `LogosAPI`;
no backend work is repeated here.

Scaffold a `ui_qml` plugin under `logos-basecamp` (or a sibling repo)
from the `logos-module-builder` `ui-qml-backend` template,
modeled on `logos-delivery-demo`.
It depends on `payment_streams_module` and `delivery_module`,
constructs `LogosModules` in `initLogos`,
and calls both modules through the generated typed wrappers.

Note: `LogosModules` is used here because UI modules run in-process with the host,
not in a `logos_host` sidecar. The crash documented in Issue #31 affects core modules only.

The plugin surfaces vault state, stream state,
the current pending-proposal slot,
and the result of the most recent Store query.
No custom backend is required for the MVP.

Components required to run:
everything from Step 17
plus `logos-basecamp` as the host
(new prerequisite — first step that uses a GUI host
instead of `logoscore`).
The new `ui_qml` module is installed via `lgpm`
into Basecamp's plugins directory.

Definition of done:
`nix build` produces a `.lgx` that loads in Basecamp without QML errors,
and a user can complete the full demo flow through the UI
without using the CLI.
