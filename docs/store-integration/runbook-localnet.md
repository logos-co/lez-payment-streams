# Store integration — localnet runbook (Flow B)

Dual-host Store demo on local LEZ. Terminology: [naming conventions](../reference/naming-conventions.md).

## Tier 1 — automated gates

Primary lifecycle gate (deterministic stream ids on one ledger):

```bash
make verify-step17-back-to-back
```

Single full run (restore snapshot + demo):

```bash
make verify-step17
```

Equivalent:

```bash
MODE=store CHAIN=local ./scripts/e2e.sh local run
```

Prepare funded baseline without running the demo:

```bash
make prepare-localnet
# or FULL_RESET=1 make full-reset-localnet
```

### What you should get

- Exit code 0.
- JSON-lines artifact under `.scaffold/e2e/artifacts/` (`e2e-*.log`) with phases such as
  `store_query_success`, `store_query_missing_proof`, and `claim` (see
  [verification matrix](../verification-matrix.md)).
- Two logoscore config trees under `.scaffold/e2e/user/` and `.scaffold/e2e/provider/`.

Orchestrator: [scripts/e2e/run_local_e2e.py](../../scripts/e2e/run_local_e2e.py).

Deterministic lifecycle policy:
[step-24c-simplify-demo-flow.md](../plan/completed/step-24c-simplify-demo-flow.md).

## Tier 2 — manual dual-host

Detailed environment variables, module load order, eligibility registration, publish, and
`storeQuery` JSON are documented in:

[step17-e2e-local.md](../step17-e2e-local.md)

When following that runbook, use `scripts/e2e.sh` and `make verify-step17` as the normative
automation entrypoints (not legacy demo shell scripts).

## Recovery

[demo-localnet-recovery.md](../demo-localnet-recovery.md)

## Related

- [setup](../payment-streams-module/setup.md) — wallet, guest ELF, module build
- [integration contracts](../reference/integration-contracts.md) — prepare/verify methods, tag 30
- [program index](../development-map/program-index.md) — Steps 14–17
