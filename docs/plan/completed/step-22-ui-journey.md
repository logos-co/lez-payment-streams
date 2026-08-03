# Step 22 — plan excerpt

Completed. Index: [index.md](../index.md).

User Journey logos-docs packet:
[logos-docs#370](https://github.com/logos-co/logos-docs/issues/370)
(Operate Payment Streams via Logos Core Module).
Does not require Step 21 (wontfix UI).

### Step 22, User Journey — doc packet

Prerequisite: None (Step 21 UI is wontfix; if resurrected, the journey may be updated with UI
screenshots and Basecamp-specific paths).

Architectural context:

End-user doc packet in `logos-co/logos-docs` (`type:journey`), **parallel to but separate from**
Step 20 (Developer Journey). Step 20 documents **integrators** (Delivery Store + eligibility,
script-orchestrated dual-host demo). Step 22 documents **end users** operating **payment streams
only** via command-line — vaults, streams, accrual, close, claim.

Pattern: [logos-docs#299](https://github.com/logos-co/logos-docs/issues/299) (chat UI journey).
Published packet: [logos-docs#370](https://github.com/logos-co/logos-docs/issues/370).

In-repo SSOT draft: [docs/journeys/USER_JOURNEY.md](../../journeys/USER_JOURNEY.md)
(testnet walkthrough rewrite owned by [Step 34](step-34-user-journey-manual-walkthrough.md)).

Delivered:

- Doc packet: install `payment_streams_module` (`lgpm` / `nix build`), load wallet,
  payer path (create vault, deposit, open stream to payee), payer **close**, payee **claim**
  residual accrued. CLI commands only — no UI required.
- Out-of-band assumption: in a real two-party setup the stream creator shares stream identity
  with the payee outside the app
  ([N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)).
- Runtime target: public TestNet v0.2 (see packet and in-repo USER_JOURNEY).
- Cross-link Step 20 for Store integration; do not duplicate Store steps.
- Cross-link Step 28 for automated module×testnet verification.
- Future enhancement: if Step 21 (Basecamp UI, wontfix) is resurrected, update the journey
  with UI screenshots. Additive — the CLI journey remains valid.

#### Canonical E2E phase list (`scripts/module-e2e.sh`)

Default scenario (no optional top-up). Pause and resume are **not** exercised.

| Phase | Role |
| --- | --- |
| `wallet_open` | Wallet + modules ready |
| `auth_init_owner`, `auth_init_provider` | `authenticated_transfer` registration (required on LEZ v0.2+) |
| `vault_init`, `deposit`, `deposit_balance` | Fund vault on chain |
| `create_stream` | Open stream to provider |
| `accrual` | Poll until `accrued_lo` ≥ `MIN_ACCRUED` |
| `close_stream`, `close_state` | Owner closes; verify closed stream + vault |
| `claim`, `claim_balance` | Provider claims residual accrued; verify balances |
| `module_e2e_complete` | Gate |

Optional when `MODULE_E2E_TOPUP=1`: `topup_stream`, `topup_allocation` after `create_stream`.

Optional when `MODULE_E2E_SKIP_CLOSE=1`: skip close writes (artifact records `close_stream` skipped); use a new `STREAM_ID` on the next run.

Console markers: `→` intent, `✓` success, `✗` failure, `!` clarification (see Step 29 UX packet).

#### Testnet commands (User Journey verification)

Automated gate (maintainer / matrix), distinct from the #370 walkthrough:

```bash
make bootstrap-testnet-module   # one-time
VAULT_ID=<unused> make verify-module-testnet
```

Localnet:

```bash
make verify-module-local
```

Recipes: [E2E.md](../../journeys/E2E.md). Hands-on walkthrough:
[USER_JOURNEY.md](../../journeys/USER_JOURNEY.md).

Definition of done:

- [x] User Journey doc packet filed:
  [logos-docs#370](https://github.com/logos-co/logos-docs/issues/370).
- [x] In-repo draft [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) covers the CLI lifecycle
  (Step 34 walkthrough rewrite).
- [x] Step 28 cross-link present (testnet module verification commands).
- [x] Verification matrix reflects Required on both chains for the User Journey.

Not in scope: dual-host Store demo; `delivery_module` procedures; replacing Step 20 Developer
Journey; new backend APIs; Basecamp UI (wontfix Step 21).
