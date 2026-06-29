# Step 22 — plan excerpt

Active-work packet for agents. Index: [program-index.md](../../development-map/program-index.md).

**Optional stretch** — execute only after Step 21 ships. Track split:
[N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06).

### Step 22, Payment streams UI journey doc packet

Prerequisite: Step 21 payment streams Basecamp UI DoD satisfied.

Architectural context:

End-user doc packet in `logos-co/logos-docs` (`type:journey`), **parallel to but separate from**
Step 20. Step 20 documents **integrators** (Delivery Store + eligibility, script-orchestrated
dual-host demo). Step 22 documents **end users** operating **payment streams only** through the
`payment_streams_ui` plugin — vaults, streams, accrual, optional claim.

Pattern: [logos-docs#299](https://github.com/logos-co/logos-docs/issues/299) (chat UI journey).

Deliver:

- Doc packet: install plugin (`lgpm` / `nix build`), load wallet, payer path (create stream to
  payee account, list vaults/streams), optional payee path (**claim** after accrual).
- **Out-of-band assumption (required copy):** to demonstrate payee claim, the journey must state
  that the stream creator shares stream identity (vault id, stream id, relevant manifest or
  account context) with the payee outside the app so the payee knows where to claim ([N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)).
- Screenshots from Step 21 UI; pins match Step 20 module refs where shared (`payment_streams_module`,
  wallet); runtime target local LEZ first (testnet when Step 18 unblocks, if UI supports it).
- SME + Red Team per [`logos-docs/CONTRIBUTING.md`](https://github.com/logos-co/logos-docs/blob/main/CONTRIBUTING.md).
- Cross-link Step 20 for “payment streams used with Logos Delivery Store” — do not duplicate
  Store integration steps.

Definition of done:

- Published UI journey doc; Red Team when required by release milestone.

Not in scope: dual-host Store demo; `delivery_module` procedures; replacing Step 20 developer
journey; new backend APIs.
