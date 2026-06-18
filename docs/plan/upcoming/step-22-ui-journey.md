# Step 22 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

Optional track — same decision as Step 21 ([program outcomes](../../../integration-index.md#program-outcomes)).

### Step 22, UI journey doc packet

Prerequisite: Step 21 Basecamp UI demo complete.

Architectural context:
Second doc packet for end-user reproduction via Basecamp (or standalone `nix run`), parallel to
Step 20 developer journey. Pattern: [logos-docs#299](https://github.com/logos-co/logos-docs/issues/299)
(chat app UI journey).

Deliver:

- `logos-docs` doc packet (`type:journey`): UI clicks, two instances if required, install path
  (package manager vs local `nix build`), screenshots, success criteria visible in the plugin.
- Pin the same module refs as Step 20; runtime target matches Step 18 when using testnet LEZ.
- SME + Red Team flow per [`logos-docs/CONTRIBUTING.md`](https://github.com/logos-co/logos-docs/blob/main/CONTRIBUTING.md).

Definition of done:

- Published UI journey doc; Red Team verification when required by release milestone.

Not in scope: new backend APIs; replacing Step 20 developer journey.
