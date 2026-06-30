# Step 22 — plan excerpt

Active-work packet for agents. Index: [index.md](../index.md).

**Active** — CLI-based User Journey documentation. Does **not** require Step 21 (optional UI).

### Step 22, User Journey — doc packet

Prerequisite: None (Step 21 UI is optional; if shipped, Step 22 may be updated with UI
screenshots and Basecamp-specific paths).

Architectural context:

End-user doc packet in `logos-co/logos-docs` (`type:journey`), **parallel to but separate from**
Step 20 (Developer Journey). Step 20 documents **integrators** (Delivery Store + eligibility,
script-orchestrated dual-host demo). Step 22 documents **end users** operating **payment streams
only** via command-line — vaults, streams, accrual, optional claim.

Pattern: [logos-docs#299](https://github.com/logos-co/logos-docs/issues/299) (chat UI journey).

Deliver:

- Doc packet: install `payment_streams_module` (`lgpm` / `nix build`), load wallet,
  payer path (create stream to payee account, list vaults/streams), optional payee path
  (**claim** after accrual). CLI commands only — no UI required.
- **Out-of-band assumption (required copy):** to demonstrate payee claim, the User Journey must state
  that the stream creator shares stream identity (vault id, stream id, relevant manifest or
  account context) with the payee outside the app so the payee knows where to claim ([N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)).
- Pins match Step 20 module refs where shared (`payment_streams_module`, wallet);
  runtime target local LEZ first (testnet when Step 18 unblocks).
- SME + Red Team per [`logos-docs/CONTRIBUTING.md`](https://github.com/logos-co/logos-docs/blob/main/CONTRIBUTING.md).
- Cross-link Step 20 for "payment streams used with Logos Delivery Store" — do not duplicate
  Store integration steps.
- **Future enhancement:** if Step 21 (Basecamp UI) ships, update Step 22 doc with UI screenshots
  and Basecamp plugin paths. This is additive — the CLI-based journey remains valid.

Definition of done:

- Published User Journey doc covering CLI workflows; Red Team when required by release milestone.

Not in scope: dual-host Store demo; `delivery_module` procedures; replacing Step 20 Developer
Journey; new backend APIs; Basecamp UI (covered by optional Step 21).
