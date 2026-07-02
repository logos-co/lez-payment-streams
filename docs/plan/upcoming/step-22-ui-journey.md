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
only** via command-line — vaults, streams, accrual, claim, close.

Pattern: [logos-docs#299](https://github.com/logos-co/logos-docs/issues/299) (chat UI journey).

In-repo SSOT draft: [docs/journeys/USER_JOURNEY.md](../../journeys/USER_JOURNEY.md).

Deliver:

- Doc packet: install `payment_streams_module` (`lgpm` / `nix build`), load wallet,
  payer path (create vault, deposit, open stream to payee), payee **claim** after accrual,
  owner **close** and reclaim unspent allocation. CLI commands only — no UI required.
- **Out-of-band assumption (required copy):** to demonstrate payee claim in a real two-party
  setup, the stream creator shares stream identity (vault id, stream id, relevant manifest or
  account context) with the payee outside the app so the payee knows where to claim
  ([N18](../../reference/integration-decisions.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)).
  The repo E2E demo collapses owner and provider into one wallet for a single-host verification
  run; claim still uses an explicit `owner` field on `chainAction` claim.
- Pins match Step 20 module refs where shared (`payment_streams_module`, wallet);
  runtime target local LEZ first, **testnet supported** (see testnet commands below).
- SME + Red Team per [`logos-docs/CONTRIBUTING.md`](https://github.com/logos-co/logos-docs/blob/main/CONTRIBUTING.md).
- Cross-link Step 20 for "payment streams used with Logos Delivery Store" — do not duplicate
  Store integration steps.
- Cross-link Step 28 for "User Journey on TestNet" — testnet module verification
  commands and bootstrap.
- **Future enhancement:** if Step 21 (Basecamp UI) ships, update Step 22 doc with UI screenshots
  and Basecamp plugin paths. This is additive — the CLI-based journey remains valid.

#### Canonical E2E phase list (`scripts/module-e2e.sh`)

Default scenario (no optional top-up). Pause and resume are **not** exercised.

| Phase | Role |
| --- | --- |
| `wallet_open` | Wallet + modules ready |
| `auth_init_owner`, `auth_init_provider` | `authenticated_transfer` registration (required on LEZ v0.2+) |
| `vault_init`, `deposit`, `deposit_balance` | Fund vault on chain |
| `create_stream` | Open stream to provider |
| `accrual` | Poll until `accrued_lo` ≥ `MIN_ACCRUED` |
| `claim`, `claim_balance` | Provider claims; verify balances |
| `close_stream`, `close_state` | Owner closes; verify closed stream + vault |
| `module_e2e_complete` | Gate |

Optional when `MODULE_E2E_TOPUP=1`: `topup_stream`, `topup_allocation` after `create_stream`.

Optional when `MODULE_E2E_SKIP_CLOSE=1`: skip close writes (artifact records `close_stream` skipped); use a new `STREAM_ID` on the next run.

Console markers: `→` intent, `✓` success, `✗` failure, `!` clarification (see Step 29 UX packet).

#### Testnet commands (User Journey)

One-time bootstrap (per operator, per machine):

```bash
make bootstrap-testnet-module
```

Creates `fixtures/testnet-module.json` (or reuse fields from `fixtures/testnet.json`).
Requires the testnet wallet under `.scaffold/e2e/testnet-wallet/` (`lgs setup` with testnet
sequencer URL). After a public sequencer relaunch:

```bash
make deploy-testnet
```

Run the full module E2E on testnet (defaults: `DEPOSIT=30`, `ALLOCATION=20`, `MIN_ACCRUED=1`):

```bash
VAULT_ID=<unused> make verify-module-testnet
```

The testnet path reuses fixture **owner** and **provider**; AT-init and pinata funding run in
the script. Pick an unused `VAULT_ID` per run. Localnet creates fresh accounts each run under
`.scaffold/module-e2e-wallet/` with larger defaults (`DEPOSIT=500`, `ALLOCATION=400`,
`MIN_ACCRUED=RATE*3`).

Verified on this repo (2026-07-02): `make verify-module-local` and
`VAULT_ID=5 make verify-module-testnet` — all artifact phases `ok:true`.

#### Localnet commands (User Journey)

```bash
make verify-module-local
```

Equivalent:

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

No fixture required; fresh isolated wallet each run.

Definition of done:

- Published User Journey doc covering CLI workflows for both localnet and testnet;
  Red Team when required by release milestone.
- In-repo draft [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md) matches `module-e2e.sh` phase list and env defaults.
- Step 28 cross-link present (testnet module verification commands).
- Verification matrix reflects Required on both chains for the User Journey.

Not in scope: dual-host Store demo; `delivery_module` procedures; replacing Step 20 Developer
Journey; new backend APIs; Basecamp UI (covered by optional Step 21).
