# Store integration

Reference integration: paid Store queries carry a LIP-155 eligibility proof (RFC 73 wire pattern).
Dual-host demo — user host and provider host — orchestrated by
[scripts/e2e/run_local_e2e.py](../../scripts/e2e/run_local_e2e.py).

Protocol: [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html).
Wire and hooks: sibling repos `logos-delivery` / `logos-delivery-module` (pins in
[reference/feature-branch-pins.md](../reference/feature-branch-pins.md)).
API summary: [reference/integration-contracts.md](../reference/integration-contracts.md).

## Required — localnet

```bash
./scripts/e2e.sh local run
```

Prepare funded baseline without demo:

```bash
make prepare-localnet
# FULL_RESET=1 make full-reset-localnet  — reseed snapshot
```

Success: exit 0; artifact `e2e-*.log` under `.scaffold/e2e/artifacts/` with
`store_query_success`, `store_query_missing_proof`, `claim`; configs under
`.scaffold/e2e/user/` and `.scaffold/e2e/provider/`.

Module/wallet setup: [payment-streams-module](../payment-streams-module/).
First machine (scaffold, tooling shell, delivery siblings):
[cold start](../reference/verification-matrix.md#cold-start-first-time-on-a-machine).

## Advanced — testnet

One-time:

```bash
make bootstrap-testnet
```

Run:

```bash
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

Optional: `make deploy-testnet` (guest ELF change), `make verify-step18-testnet-read-smoke`.
Payee `claim` may be optional on testnet —
[archive/operator/testnet-claim-known-issue.md](../archive/operator/testnet-claim-known-issue.md).

## Developer journey (future work)

Published integrator journey targets logos-docs. In-repo draft SSOT for Store + payment streams
content is this README; checklist:
[plan/upcoming/step-20-developer-journey.md](../plan/upcoming/step-20-developer-journey.md).

## Recovery

[archive/operator/localnet-recovery.md](../archive/operator/localnet-recovery.md).

## Related

- [reference/verification-matrix.md](../reference/verification-matrix.md)
- [development-map/program-index.md](../development-map/program-index.md)
