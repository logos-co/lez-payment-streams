# Payment streams module — runbook (Flow A)

Module-only verification on localnet. Terminology: [naming conventions](../reference/naming-conventions.md).

## Tier 1 — automated gate

```bash
make verify-module-local
```

Equivalent:

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
```

Prepare only (localnet up + build wallet and `payment_streams_module`, no delivery):

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local prepare
```

### What you should get

- Exit code 0 from Make or `e2e.sh`.
- JSON-lines artifact under `.scaffold/e2e/artifacts/` (default prefix `module-e2e-*.log`) with
  phases including `vault_init`, `deposit`, `create_stream`, `claim`, and `module_e2e_complete`.
- Flow A uses an isolated wallet under `.scaffold/module-e2e-wallet/` and vault 0 / stream 0 on
  the live local sequencer (independent of the Store demo snapshot).

Script reference: [scripts/module-e2e-local.sh](../../scripts/module-e2e-local.sh).

## Tier 2 — manual sequence

Run the same story by hand on one `logoscore` daemon after [setup.md](setup.md) and
`MODE=module CHAIN=local ./scripts/e2e.sh local prepare`.

Pattern (exact JSON shapes in the script):

1. Start localnet if needed: `./scripts/lifecycle.sh localnet start`
2. `logoscore -D -m "$MODULES" …` — load `logos_execution_zone`, `payment_streams_module`
3. Wallet `open` (or `create_new`) on an isolated wallet directory
4. `sync_to_block` against the sequencer
5. `chainAction` sequence: `initializeVault`, `deposit`, `createStream`, `pauseStream`,
   `resumeStream`, `topUpStream`, `claim`
6. Optional status reads: `getVaultStatus`, `getStreamStatus`

Expect `"status":"ok"` in nested results for each successful `chainAction`.

## Recovery

Local ledger issues during Store or module work:
[demo-localnet-recovery.md](../demo-localnet-recovery.md).

## Out of scope

- Flow A on testnet (unsupported; see [verification matrix](../verification-matrix.md))
- Steps 12–13 eligibility runbooks (development map only)
- Store integration — [store-integration/runbook-localnet.md](../store-integration/runbook-localnet.md)
