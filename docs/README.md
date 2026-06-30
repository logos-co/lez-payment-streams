# Documentation

Entry for integrators and maintainers. Protocol text lives in
[LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html); cite the LIP instead of
duplicating semantics here.

## Choose a path

| Pillar | When |
| --- | --- |
| [on-chain](on-chain/) | Guest program, Rust tests, review order |
| [payment-streams-module](payment-streams-module/) | Logos module, module verification |
| [store-integration](store-integration/) | Store eligibility, dual-host demo |
| [plan](plan/) | Program index, plan packets |

## Verify (canonical)

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run   # module — Required
./scripts/e2e.sh local run                           # Store — Required
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run  # Store — Advanced
```

Details: [reference/verification-matrix.md](reference/verification-matrix.md) (includes
[cold start](reference/verification-matrix.md#cold-start-first-time-on-a-machine)),
[scripts/README.md](../scripts/README.md).

## Reference

| Doc | Role |
| --- | --- |
| [integration-contracts.md](reference/integration-contracts.md) | Cross-repo APIs, tag 30 |
| [integration-decisions.md](reference/integration-decisions.md) | Integration decisions (trimmed) |
| [feature-branch-pins.md](reference/feature-branch-pins.md) | Fork branches and flakes |
| [naming-conventions.md](reference/naming-conventions.md) | `MODE` values, Makefile names |
| [verification-matrix.md](reference/verification-matrix.md) | Mode × chain matrix |

## Archive

Historical step runbooks and operator notes: [archive/](archive/).

## Maintainers

[AGENTS.md](../AGENTS.md), [plan/](plan/), lifecycle regression:
`make verify-store-local-lifecycle`.
