# Step 18 — plan excerpt

Active-work packet for agents. Index: [integration-index.md](../../../integration-index.md).

### Step 18, Public testnet demo

Prerequisite: `scripts/demo-e2e-local.sh` satisfies all Step 17 definition-of-done criteria on a
local LEZ sequencer ([N12](../../reference/decisions-and-notes.md#n12-step-16-vs-step-17-verification-scope-2025-06-18)).

Architectural context:
Step 17 proves the stack on a disposable local LEZ sequencer (`127.0.0.1:3040`).
Step 18 reuses the same dual-`logoscore` Store + eligibility flow against public testnet LEZ
(testnet v0.2 target). Messaging remains the public Logos dev preset (`logos.dev`); only chain
fixture and wallet `sequencer_addr` change. Do not apply local reset-first policy from
[`demo-localnet-recovery.md`](../../demo-localnet-recovery.md); see testnet contrast there and
[`step12-user-eligibility.md`](../../step12-user-eligibility.md) (Persistence across runs).

Deliver:

- Committed template `fixtures/testnet.json.example` (no secrets); operators copy to gitignored
  `fixtures/testnet.json` (or set `FIXTURE_MANIFEST`) and fill real ids. Same JSON shape as
  [`fixtures/localnet.json.example`](../../../fixtures/localnet.json.example).
- Testnet bootstrap runbook section or script variant: deploy program if needed, fund accounts,
  `initialize_vault` / `deposit` / stream setup per Step 12 testnet table; persist vault and
  manifest across runs.
- `demo-e2e-testnet.sh` or documented `CHAIN=testnet` path parallel to Step 17 (separate from
  CI default so Step 17 stays hermetic on localnet).

Definition of done:

- Documented public sequencer URL and LEZ revision aligned with wallet pins
  ([`feature-branch-pins.md`](../../feature-branch-pins.md)) or an explicit pin bump for testnet.
- Dual-host paid Store success and inbound eligibility failure match Step 17 outcomes on testnet
  fixture state (same wire and module behavior; different manifest).
- Operator can reproduce without wiping chain state; persistence rules documented.

Not in scope: replacing Step 17 local DoD; automatic testnet faucet (unless testnet ships a
supported funding API).
