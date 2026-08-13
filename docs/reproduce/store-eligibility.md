# Reproduce Store eligibility

Orchestrated happy path: paid Store queries carry a LIP-155 eligibility proof.
Localnet is primary.
Testnet is a real-network follow-up after bootstrap.

Protocol-only module walkthrough: [payment-streams.md](payment-streams.md).
Wire and hooks: [integration-contracts.md](../reference/integration-contracts.md).
Pins: [feature-branch-pins.md](../reference/feature-branch-pins.md).
How to add eligibility to another protocol: [integrate/eligibility.md](../integrate/eligibility.md).

Entry: `scripts/e2e.sh` with `MODE=store` (default).
Each `local run` / `testnet run` does prepare, run, and teardown unless `SKIP_TEARDOWN=1`.

Dual-host debugging without the orchestrator: [archive/steps/local-store-dual-host-runbook.md](../archive/steps/local-store-dual-host-runbook.md).

## Success

Exit 0.
Artifact `.scaffold/e2e/artifacts/e2e-*.log` includes `store_query_success`, `store_query_missing_proof`, and `claim`.
Configs under `.scaffold/e2e/user/` and `.scaffold/e2e/provider/`.

Claim is required (`E2E_CLAIM_OPTIONAL=0`).
Settlement is close then claim on the run’s `stream_id`.
Omit `token_id` on `initializeVault`.
The native identity is 32 zero octets, stored as one `accepted_tokens` policy row.

## Cold start

[Verification matrix](../reference/verification-matrix.md#cold-start-first-time-on-a-machine).
Store runs need `../logos-delivery-module` (or `DELIVERY_MODULE_ROOT`) on the branch in feature-branch-pins.
`MODE=module` skips delivery.

## Localnet (primary)

```bash
./scripts/e2e.sh local run
```

Make alias: `make verify-store-local`.

Prepare funded baseline without the demo:

```bash
make prepare-localnet
```

`FULL_RESET=1 make full-reset-localnet` reseeds the snapshot.

Maintainer lifecycle (two runs, one ledger): `make verify-store-local-lifecycle`.

## Testnet (follow-up)

One-time:

```bash
make bootstrap-testnet
```

```bash
MODE=store ./scripts/e2e.sh testnet run
```

Make alias: `make verify-store-testnet`.

Prefund: `./scripts/fund-testnet-accounts.sh`.
Guest ELF change: `make deploy-testnet`.
Program identity: root README [Public testnet guest program](../../README.md#public-testnet-guest-program) and [fixtures/testnet.json](../../fixtures/testnet.json).

On LEZ v0.2.0, bootstrap runs `auth-transfer init` for owner and provider.
Claim in teardown needs a healthy AT-initialized provider.

## Privacy overlays

Flags are independent.
`PRIVACY=1` aliases `OWNER_PRIVACY=1`.
Local private submits default `RISC0_DEV_MODE=1` (stub receipts).
Testnet proving uses `RISC0_DEV_MODE=0` and `E2E_CLAIM_OPTIONAL=0`.

| Profile | Command |
| --- | --- |
| Owner privacy | `MODE=store OWNER_PRIVACY=1 ./scripts/e2e.sh local run` |
| Provider privacy | `MODE=store PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` |
| Full privacy | `MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run` |

Make aliases: `verify-store-local-owner-privacy`, `verify-store-local-provider-privacy`, `verify-store-local-full-privacy`.

Owner privacy: `PseudonymousFunding` vault, public provider.
Phases include `owner_privacy_accounts`, `pre_shield`, `vault_init` with `privacy_tier=1`.
Close and claim run on the user host (private owner NSK).

Provider privacy: public vault, private provider.
Phases include `provider_privacy_accounts`, `provider_dust_pre_shield`, `register_provider_mapping`.
Claim confirms via `vault_holding` drop.

Full privacy: both.
Private owner and provider share one user-host wallet session (shared seed).

Testnet full privacy (after prefund).
When recloning the testnet seed wallet, run `./scripts/prepare-testnet-privacy-seed.sh` so private ids are not recycled.

```bash
./scripts/fund-testnet-accounts.sh
SKIP_BUILD=1 RISC0_DEV_MODE=0 E2E_CLAIM_OPTIONAL=0 \
  MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
  ./scripts/e2e.sh testnet run
```

Account and submit deltas: [Private execution notes](payment-streams.md#private-execution-notes) in the protocol reproduce doc.

## Module verification

Single-host `chainAction` path.
Recipes here so the matrix has one operator page for orchestrated runs.

Local:

```bash
MODE=module ./scripts/e2e.sh local run
```

Optional top-up: `MODULE_E2E_TOPUP=1`.
Owner privacy: `MODE=module OWNER_PRIVACY=1 ./scripts/e2e.sh local run`.
Provider privacy: `MODE=module PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run`.
`CLOSE_ROLE=provider` for six-slot close.
Make aliases: `verify-module-local`, `verify-module-local-privacy`, `verify-module-local-provider-privacy`, `verify-module-local-provider-close`, `verify-module-local-close-negatives`.

Expected: exit 0.
Artifact `module-e2e-*.log` with `vault_init`, `deposit`, `create_stream`, `accrual`, `close_stream`, `claim`, `module_e2e_complete`.

Testnet, one-time `make bootstrap-testnet-module`, then:

```bash
SKIP_BUILD=1 MODULE_E2E_SKIP_FUND=1 MODE=module ./scripts/e2e.sh testnet run
```

Manual teaching path: [payment-streams.md](payment-streams.md).

## Configuration

| Variable | Role |
| --- | --- |
| `MODE` | `store` (default) or `module` |
| `OWNER_PRIVACY` / `PROVIDER_PRIVACY` | `0` or `1` |
| `SKIP_BUILD=1` | Skip `.lgx` rebuilds |
| `E2E_CLAIM_OPTIONAL` | Default `0` (required). `1` soft-passes an unconfirmed claim. |
| `VAULT_ID` | Pin vault id. Default: first unused config. |
| `E2E_REUSE_BASELINE_VAULT=1` | Vault-0 reuse (lifecycle regression). |
| `E2E_LIFECYCLE_VIA` | `chainaction` (default) or `seed` |
| `E2E_ALLOW_FIRE_AND_FORGET=1` | Non-local only: skip sequencer `getTransaction` wait. State poll remains the gate. |
| `SKIP_TEARDOWN=1` | Skip teardown |
| `SEED_ALLOCATION` / `SEED_DEPOSIT_AMOUNT` | Testnet Store defaults 400 / 500 |

Verbosity: `./scripts/e2e.sh --verbosity quiet|normal|verbose` or `E2E_VERBOSITY`.

Scaffold paths: [naming-conventions.md](../reference/naming-conventions.md#scaffold-layout).
Flag detail: [verification-matrix.md](../reference/verification-matrix.md).

## Failure modes

| Failure | Resolution |
| --- | --- |
| `NO_ELIGIBLE_VAULT` | Vault ensure / deposit. Check vault scan. |
| `STREAM_DEPLETED` | New stream or top-up. |
| `PROOF_INVALID` | Stream active. Check N8 payload. |
| `STREAM_NOT_ACTIVE` | Create a new stream. |
| Claim fails on Store testnet teardown | Re-run AT ensure. Check `provider_account_id`. |
| Store query dial failures | Check multiaddr and peer id in the manifest. |

Recovery: [archive/operator/localnet-recovery.md](../archive/operator/localnet-recovery.md).

## Private execution notes

Owner privacy (`OWNER_PRIVACY=1`) and provider privacy (`PROVIDER_PRIVACY=1`) are independent.
`PRIVACY=1` aliases `OWNER_PRIVACY=1`.
Account creation, pre-shield, `s:` prefix, `amount_le16_hex`, and `vault_holding` confirmation:
[payment-streams.md Private execution notes](payment-streams.md#private-execution-notes).

Store overlays:

- Owner privacy: `PseudonymousFunding` vault on the user host. Close and claim run there (private owner NSK). Public provider.
- Provider privacy: public vault. `registerProviderMapping` uses the private provider id. Claim confirms via `vault_holding` drop.
- Full privacy: both. Private owner and provider share one user-host wallet session (shared seed).
- Paid query uses `storeQueryWithEligibility` / `storeQueryWithEligibilityCompleted` with opaque proof bytes.

Local private submits default `RISC0_DEV_MODE=1` (stub receipts).
Testnet proving uses `RISC0_DEV_MODE=0` and `E2E_CLAIM_OPTIONAL=0`.

