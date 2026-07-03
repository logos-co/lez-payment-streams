### What the user achieves

A developer builds a Logos Core module that attaches payment stream proofs to Store requests,
enabling paid historical message retrieval where providers verify active streams before serving queries.

### Why it matters

Logos networks should be self-sustaining:
users should pay providers for services rather than relying on external subsidies.
This example uses Store (a Logos Delivery protocol) to demonstrate paid querying of historical messages through payment streams.

### Key components

* `lez-payment-streams` (on-chain program): SPEL guest implementing LIP-155 payment streams — vaults, streams, deposits, claims. Runs on Logos Execution Zone (LEZ).
* `payment_streams_module`: Universal Logos Core module exposing LIP-155 via `chainAction` and eligibility proof methods.
* `delivery_module`: Logos Delivery module with Store protocol and eligibility hooks.
* `wallet_module` (`logos_execution_zone`): Chain interaction for the payment streams module.
* `scripts/e2e/run_local_e2e.py`: Dual-host orchestrator driving user and provider logoscore instances.

### Repository

https://github.com/logos-co/lez-payment-streams

### Runtime target

Verification is one dual-host Store run:
two `logoscore` processes (user and provider) coordinated by `./scripts/e2e.sh` and `scripts/e2e/run_local_e2e.py`.
The same flow runs on localnet and on public TestNet v0.2.
`CHAIN` selects the network (`make verify-store-local` vs `make verify-store-testnet`).

TestNet v0.2 is the default and primary target.
It exercises real sequencer inclusion, libp2p between hosts, and the shared LIP-155 program already deployed on that network.
Program id and sequencer URL come from repo fixtures; you do not deploy the guest as part of the run.
Localnet remains useful for faster iteration.

### Prerequisites

Verification setup (host, Nix, scaffold, Store delivery checkout, testnet bootstrap) is documented in the lez-payment-streams repository README (Prerequisites section at https://github.com/logos-co/lez-payment-streams#prerequisites).

### Commands and expected outputs

End-to-end flow: vault ensure and deposit, provider peer mapping, stream create, Store messages during accrual, eligibility proof, paid Store query, Store query without proof (expect rejection), close stream, claim when teardown accrual is non-zero.

Each Store run scans vault ids from 0 upward and uses the first unused id.
`VAULT_ID=<id>` pins a vault.
`E2E_REUSE_BASELINE_VAULT=1` selects the vault-0 reuse path for `make verify-store-local-lifecycle`.

#### Testnet verification

Before the first Store run on public TestNet, run a one-time bootstrap on a machine that can reach the sequencer RPC.
The script creates or reuses a testnet wallet layout, funds the owner account, creates a provider account if needed, and writes `fixtures/testnet.json`.
That manifest holds your owner and provider ids plus shared chain fields such as `sequencer_url` and `program_id_hex`.
Later `make verify-store-testnet` runs read this manifest and reuse the same accounts; each run still picks a fresh vault id.

```bash
make bootstrap-testnet
```

Full end-to-end verification:

```bash
make verify-store-testnet
```

Equivalent:

```bash
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
```

Expected: exit 0; artifact `.scaffold/e2e/artifacts/e2e-*.log` with these Store query lines:

```jsonl
{"phase":"store_query_success","ok":true}
{"phase":"store_query_missing_proof","ok":true}
```

Settlement (same run, after the Store query checks):

```jsonl
{"phase":"auth_init_owner","ok":true}
{"phase":"auth_init_provider","ok":true}
{"phase":"close_stream","ok":true}
{"phase":"close_state","ok":true}
{"phase":"claim","ok":true}
{"phase":"claim_balance","ok":true}
```

#### Localnet verification

Local Store verification uses a disposable LEZ chain on your machine.
There is no testnet-style bootstrap; owner, provider, program id, and demo policy come from the committed `fixtures/localnet.json`.

Full end-to-end verification:

```bash
make verify-store-local
```

Equivalent:

```bash
./scripts/e2e.sh local run
```

Corrupted or stale snapshot: run `make prepare-localnet` to restore or prefund again, or `make full-reset-localnet` to reseed the funded baseline and rewrite the snapshot.

### Expected result

Exit code 0.
JSON-lines artifact at `.scaffold/e2e/artifacts/e2e-*.log` with `store_query_success` and `store_query_missing_proof` reporting `"ok":true`, `auth_init_owner` / `auth_init_provider`, `close_state` before `claim`, and when teardown accrual is non-zero, `claim` with `"ok":true`.
If nothing accrued before close, teardown logs `claim` with `"ok":true` and `"reason":"zero_accrued"` instead of submitting a claim transaction; the run can still succeed.

### Configuration details

#### Demo assumptions

The script is a demo harness, not a production deployment pattern.
Provider libp2p peer id for `registerProviderMapping` comes from the fixture.
On testnet, `E2E_CLAIM_OPTIONAL` defaults to `1`; set `0` for strict claim confirmation.

#### Key environment variables

* `PAYMENT_STREAMS_GUEST_BIN`: Path to compiled guest ELF
* `MODE`: `store` (default) or `module` (single-host module E2E only)
* `CHAIN`: `local` or `testnet`
* `SKIP_BUILD=1`: Skip `.lgx` rebuilds on subsequent runs
* `E2E_CLAIM_OPTIONAL`: Testnet claim strictness (default `1`; use `0` for strict)
* `FIXTURE_MANIFEST`: Override fixture path
* `E2E_CLOSE_VIA`: `seed` (default) or `chainaction` for close/claim submit path
* `VAULT_ID`: Pin vault id (default: scan for first empty config)
* `E2E_REUSE_BASELINE_VAULT=1`: Vault-0 reuse path (lifecycle regression)
* `SEED_ALLOCATION`: CreateStream allocation in lo (testnet Store default: 400)
* `SEED_DEPOSIT_AMOUNT`: Vault deposit in lo (testnet Store default: 500)
* `E2E_CREATE_VIA`: `seed` or `chainaction` for stream create (testnet default: `chainaction`)

#### Module dependencies

At runtime the Store demo loads `logos_execution_zone`, `payment_streams_module`, and `delivery_module`.
Module-only verification (`MODE=module`) does not need delivery checkouts.

#### Verbosity

Console output level via `./scripts/e2e.sh --verbosity quiet|normal|verbose` or `E2E_VERBOSITY`:

* `quiet` — JSON-lines artifact only
* `normal` — phase headers, status markers, on-chain values
* `verbose` — adds concept explanations

### Failure modes and limits

| Failure | Cause | Resolution |
|---------|-------|------------|
| `NO_ELIGIBLE_VAULT` | Vault missing or insufficient deposit | Run vault ensure / deposit; check vault scan |
| `STREAM_DEPLETED` | Stream ran out of allocated funds | Create a new stream or top up |
| `PROOF_INVALID` | Eligibility proof verification failed | Confirm stream is active; check N8 payload |
| `STREAM_NOT_ACTIVE` | Stream closed or not yet active | Create a new stream on the vault |
| Claim fails on Store testnet teardown | AT or fixture provider | Re-run AT ensure; fix `provider_account_id` |
| Vault unallocated on testnet | Depleted holding for owner | Deposit or re-bootstrap with testnet wallet home |
| Store query dial failures | Provider unreachable on libp2p | Check multiaddr and peer id in manifest |

### GitHub handle

@s-tikhomirov

### Discord handle

sergei.tikhomirov

### Existing docs or specs

* LIP-155 (Payment Streams): https://lip.logos.co/anoncomms/raw/payment-streams.html
* RFC 73 (Store Eligibility): https://rfc.vac.dev/spec/73/
* integration-contracts.md: [docs/reference/integration-contracts.md](docs/reference/integration-contracts.md)
* Store integration: [docs/store-integration/README.md](docs/store-integration/)
* Verification matrix: [docs/reference/verification-matrix.md](docs/reference/verification-matrix.md)

## Additional context

### Sibling repositories

Store integration requires patched forks `logos-delivery` and `logos-delivery-module`.
Use the branch recorded in [docs/reference/feature-branch-pins.md](docs/reference/feature-branch-pins.md).

### Estimated time to complete

* Cold start (first time): 20–40 minutes
* Testnet Store runs (primary): often 10–20+ minutes
* Subsequent local Store runs: about 3–8 minutes

### Security notes

* Fixture manifests contain test keys; use on test networks only
* Private keys stay in `wallet_module`; proofs are signed attestations
* This journey uses transparent vault mode
