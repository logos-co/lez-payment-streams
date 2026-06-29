# Store integration — testnet runbook (Flow B, advanced)

Public LEZ sequencer with local dual-host Store and relay (chain I/O on testnet; P2P still local).
Support tier: Advanced in [verification-matrix.md](../verification-matrix.md).

## Tier 1 — automated gate

One-time bootstrap (writes `fixtures/testnet.json`):

```bash
make bootstrap-testnet
```

Full testnet E2E:

```bash
make verify-step18
```

Equivalent:

```bash
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

Read smoke (optional):

```bash
make verify-step18-testnet-read-smoke
```

Guest deploy on testnet (when ELF changes):

```bash
make deploy-testnet
```

### What you should get

- Exit code 0 from `verify-step18` when RPC, fixture, and demo phases succeed.
- Per-operator `fixtures/testnet.json` (gitignored) after bootstrap.
- Payee `claim` may be optional or flaky on testnet:
  [testnet-claim-known-issue.md](../testnet-claim-known-issue.md).

## Tier 2 — manual operator detail

[step18-public-sequencer-e2e.md](../step18-public-sequencer-e2e.md) — pins, wallet home, Part A/B,
and operator checks. Align commands with `e2e.sh testnet` and the verification matrix.

Plan excerpt: [step-18-public-testnet-demo.md](../plan/completed/step-18-public-testnet-demo.md).

## Not supported

Flow A (module-only) on testnet — see [verification matrix](../verification-matrix.md).
