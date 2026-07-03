# Step 33 — testnet gate execution log

Append-only record for the fresh-vault-per-run Store testnet gate.
Definition of done: two consecutive green passes (Store + Module) on the
public testnet sequencer with no manual deposit or vault bootstrap.

Pass criteria and scope: [step-33-store-e2e-fresh-vault.md](../upcoming/step-33-store-e2e-fresh-vault.md) Verification section.

## Commands

```bash
SKIP_BUILD=1 make verify-store-testnet
SKIP_BUILD=1 make verify-module-testnet
```

`SKIP_BUILD=1` reuses the already-built guest binary and modules; the gate
does not depend on a clean build because the program image id is stable on
the testnet owner account.

## Runs

| Date | Commit | Store artifact | Module artifact | Result | Notes |
| --- | --- | --- | --- | --- | --- |
| 2026-07-03 | cd6329a | e2e-20260703T190207.log | module-e2e-20260703T193644.log | PASS | Store vault 8, module vault 9 (VAULT_ID=9). Owner funded via pinata faucet (no manual deposit). Store: store_query_success + close + claim green. Module: vault_init through claim_balance green, provider received 20 lo. Three testnet-only fixes landed during the gate: resolve_owner ignores stale localnet SIGNER_ID on testnet (788ac58), refresh_manifest_pdas passes vault_id to write-manifest (0b12843), seed provider state file when module has not persisted yet (cd6329a). |
