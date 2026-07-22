# Step 39 — testnet privacy gate execution log

Append-only record for native guest redeploy and required privacy gates on
public testnet.
SSOT:
[step-39-testnet-privacy-e2e.md](../upcoming/step-39-testnet-privacy-e2e.md).

Locked highlights: soft proving (D39.4); module full then Store full (D39.7);
one green (D39.8); agent tries deploy, flag+stop on failure (D39.9); Docker
ELF + ImageID from `make program-id` (D39.10); fixture sync not full bootstrap
(D39.11); strict claim on privacy gates — `E2E_CLAIM_OPTIONAL=0` (D39.13);
agent reports greens, human alone moves packet to completed or writes off
(D39.15); Y-equal no-op contingency (D39.16); funding defaults then bump
(D39.18).

## Deploy

| Field | Value |
| --- | --- |
| Freeze commit | _pending Phase 2_ |
| Deploy date | _pending_ |
| ImageID Y (hex) | _pending_ (replaces `de17c0db…`, or unchanged per D39.16) |
| ELF size (bytes) | _pending_ |
| ELF path | `methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin` |
| Operator | Agent (`make build` → `make deploy-testnet` + prefix checks) |

## Required commands

Public regression (default claim optional):

```bash
./scripts/fund-testnet-accounts.sh   # defaults OWNER_TARGET=550 PROVIDER_MIN=50
SKIP_BUILD=1 MODULE_E2E_SKIP_FUND=1 make verify-module-testnet
SKIP_BUILD=1 make verify-store-testnet
```

Privacy (soft proving; strict claim). Order matters; fund before each:

```bash
./scripts/fund-testnet-accounts.sh
SKIP_BUILD=1 MODULE_E2E_SKIP_FUND=1 RISC0_DEV_MODE=1 E2E_CLAIM_OPTIONAL=0 \
  MODE=module CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
  ./scripts/e2e.sh testnet run

./scripts/fund-testnet-accounts.sh
SKIP_BUILD=1 RISC0_DEV_MODE=1 E2E_CLAIM_OPTIONAL=0 \
  MODE=store CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
  ./scripts/e2e.sh testnet run
```

Funding-short once: `OWNER_TARGET=700 PROVIDER_MIN=100 ./scripts/fund-testnet-accounts.sh`.

## Optional (not DoD)

Skip by default (D39.19). If run and fails, append a row marked optional; does
not block close.

```bash
RISC0_DEV_MODE=0 MODE=store CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 \
  E2E_CLAIM_OPTIONAL=0 SKIP_BUILD=1 ./scripts/e2e.sh testnet run
```

## Runs

Artifact column: path under `.scaffold/e2e/artifacts/`.
Notes: ImageID Y, `RISC0_DEV_MODE`, `E2E_CLAIM_OPTIONAL`, `SKIP_BUILD`.

| Date | Commit | Profile | Artifact | Result | Notes |
| --- | --- | --- | --- | --- | --- |
| | | | | | _no runs yet_ |

## Agent summary (after Phase 5)

_pending — agent fills when required gates finish (green or incomplete)._

## Human close (D39.15)

_pending — human only: review above, then move packet to completed or write off._
