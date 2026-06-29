# Verification matrix (flow x chain)

Two verification narratives run on the unified script stack
(`scripts/e2e.sh`, `scripts/lifecycle.sh`, `scripts/fixture.sh`, `scripts/lib/common.sh`).
Each narrative is one flow; each flow can target one chain.
The operator model is `prepare(chain)` then `run(mode, chain)`,
selected with the `CHAIN` and `MODE` environment variables on `scripts/e2e.sh`.

- Flow A — module only (`MODE=module`).
  Single-host payment-streams happy path through `payment_streams_module`
  `chainAction`: vault init, deposit, stream create, pause, resume, top-up,
  accrual, payee claim. No `delivery_module`, no Store, no eligibility.
- Flow B — Store integration (`MODE=store`, default).
  Dual-host demo: a user host and a provider host exchange a paid Store query
  carrying an LIP-155 eligibility proof, driven by
  `scripts/e2e/run_local_e2e.py`.

## The matrix

|  | Localnet (`CHAIN=local`) | Testnet (`CHAIN=testnet`) |
| --- | --- | --- |
| Flow A — module only | `make verify-module-local` | future work (unsupported) |
| Flow B — Store integration | `make verify-step17` (+ `verify-step17-back-to-back`) | `make verify-step18` (advanced) |

## Support tiers

- Required (clone and verify).
  The localnet column for both flows.
  `make verify-module-local` (Flow A) and `make verify-step17` (Flow B)
  are the supported "does it work on my machine" gates.
- Advanced (integrators).
  Flow B on testnet (`make verify-step18`) runs against the public sequencer.
  It needs a per-operator `fixtures/testnet.json` (one-time
  `make bootstrap-testnet`) and tolerates slow blocks and clock skew.
  The payee `claim` is optional on testnet
  (see [testnet-claim-known-issue.md](testnet-claim-known-issue.md)).
- Future work (unsupported).
  Flow A on testnet is deliberately not provided.
  The module happy path ends in a payee `claim`, and `claim` does not reliably
  confirm on the public sequencer while every other instruction lands
  (see [testnet-claim-known-issue.md](testnet-claim-known-issue.md)).
  `scripts/e2e.sh` rejects `MODE=module CHAIN=testnet`.
  Revisit once module-local is stable and the testnet claim reject reason is
  diagnosed.

## Commands

```bash
# Flow A — module only, localnet
make verify-module-local
# equivalent: MODE=module CHAIN=local ./scripts/e2e.sh local run

# Flow B — Store integration, localnet
make verify-step17
make verify-step17-back-to-back   # restore run, then continue on the same ledger
# equivalent: MODE=store CHAIN=local ./scripts/e2e.sh local run

# Flow B — Store integration, testnet (advanced)
make bootstrap-testnet            # one-time, writes fixtures/testnet.json
make verify-step18
# equivalent: MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

## Notes

- `prepare` semantics differ by chain and must not be hidden.
  Localnet prepare restores or seeds a funded snapshot and resets the ledger;
  testnet prepare ensures the wallet and a read smoke without resetting chain
  state.
- Flow A drives its own vault lifecycle through the module, so localnet prepare
  in `MODE=module` only ensures localnet is up; it skips the Store-flow vault
  snapshot baseline and skips building `delivery_module`.
- Artifacts land under `.scaffold/e2e/artifacts/` as JSON-lines phase logs:
  Flow A writes `vault_init`, `deposit`, `create_stream`, `claim`, and
  `module_e2e_complete`; Flow B writes `store_query_success`,
  `store_query_missing_proof`, and `claim`.
- Flow A is revived and modernized from
  [`scripts/archive/step11b-logoscore-e2e.sh`](../scripts/archive/step11b-logoscore-e2e.sh)
  into [`scripts/module-e2e-local.sh`](../scripts/module-e2e-local.sh) on the
  unified stack.
