# Step 32 — testnet gate execution log

Append-only record for **D3** maintainer gate runs.
D3 closed 2026-08-03: Step 39 Store full privacy testnet with
`E2E_CLAIM_OPTIONAL=0` accepted as evidence; follow-up landed (strict default +
drop `demo_claim` alias).

## Primary gate (required before follow-up PR)

```bash
E2E_CLAIM_OPTIONAL=0 make verify-store-testnet
make verify-module-testnet
```

Pass criteria: [step-32-auth-transfer-unify-store-claim.md](step-32-auth-transfer-unify-store-claim.md) D3 section.

## Optional appendix (Option B evidence)

```bash
E2E_CLOSE_VIA=chainaction E2E_CLAIM_OPTIONAL=0 make verify-store-testnet
```

## Runs

| Date | Commit | Store artifact | Module artifact | Result | Notes |
| --- | --- | --- | --- | --- | --- |
| 2026-07-24 | e9703a2+ | `e2e-20260724T144726.log` | `module-e2e-20260722T215542.log` | pass | Credited from Step 39 Phase 4: Store + module full privacy testnet, `RISC0_DEV_MODE=0`, `E2E_CLAIM_OPTIONAL=0`, claim_balance vault_drop=400; AT ImageID verify via shared ensure. See [step-39-testnet-gate-log.md](step-39-testnet-gate-log.md). |
| 2026-08-03 | (this tip) | — | — | follow-up | Default `E2E_CLAIM_OPTIONAL=0`; `demo_claim` alias removed from `run_local_e2e.py`. |

## Optional appendix runs

| Date | Commit | Store artifact | Result | Notes |
| --- | --- | --- | --- | --- |
| | | | | Option B (`E2E_CLOSE_VIA=chainaction`) not required for D3 close |
