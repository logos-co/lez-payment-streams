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
  runtime target local LEZ first, **testnet supported** (see testnet commands below).
- SME + Red Team per [`logos-docs/CONTRIBUTING.md`](https://github.com/logos-co/logos-docs/blob/main/CONTRIBUTING.md).
- Cross-link Step 20 for "payment streams used with Logos Delivery Store" — do not duplicate
  Store integration steps.
- Cross-link Step 28 for "User Journey on TestNet" — testnet module verification
  commands and bootstrap.
- **Future enhancement:** if Step 21 (Basecamp UI) ships, update Step 22 doc with UI screenshots
  and Basecamp plugin paths. This is additive — the CLI-based journey remains valid.

#### Testnet commands (User Journey)

One-time bootstrap (per operator, per machine):

```bash
make bootstrap-testnet-module
```

This creates `fixtures/testnet-module.json` with a funded vault (id 1, separate from
Store flow's vault 0). Requires the testnet wallet to exist (`lgs setup` with testnet
sequencer URL in `scaffold.toml`).

Run the full module E2E on testnet:

```bash
MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run
```

Or via Make alias:

```bash
make verify-module-testnet
```

The testnet path reuses owner/provider accounts from the fixture; the localnet path
creates fresh isolated accounts and funds them via `lgs wallet topup`. Both paths
exercise the same phases: `vault_init`, `deposit`, `create_stream`, `pause_stream`,
`resume_stream`, `topUpStream`, `claim`.

#### Localnet commands (User Journey)

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

No fixture required; creates a fresh isolated wallet under
`.scaffold/module-e2e-wallet/` each run.

Definition of done:

- Published User Journey doc covering CLI workflows for both localnet and testnet;
  Red Team when required by release milestone.
- Step 28 cross-link present (testnet module verification commands).
- Verification matrix reflects Required on both chains for the User Journey.

Not in scope: dual-host Store demo; `delivery_module` procedures; replacing Step 20 Developer
Journey; new backend APIs; Basecamp UI (covered by optional Step 21).
