# Raw TODO — testnet privacy E2E after next guest deploy

Ideas not yet scheduled as a plan step. Index: [../index.md](../index.md).

Extracted from [Step 38 Phase C](../completed/step-38-store-privacy-e2e.md)
(optional matrix completion). Local Store and module privacy profiles are green;
testnet privacy verification is postponed until further guest-touching work
lands, so one redeploy covers the accumulated ImageID drift.

## Why postponed

Privacy product paths (Steps 36–37) and Store harness wiring (Step 38 Phases A/B)
are implemented and verified on localnet only. Testnet still pins program id
`de17c0db…` (deploy 2026-07-15). Local privacy E2E already depends on a newer
guest (fold-seconds / ImageID `072a26cc…`). More guest changes may still land
before the next operator deploy. Redeploying and running privacy testnet gates
now would likely need a second redeploy soon after.

## Reminder when ready

1. Finish pending guest-related implementation (or decide guest is freeze-ready).
2. Build release guest, deploy to testnet, update fixtures (`program_id_hex`,
   deploy metadata, `PAYMENT_STREAMS_GUEST_BIN` / ImageID consumers).
3. Run privacy E2E against that deploy without relying on `RISC0_DEV_MODE` as
   the production path (dev-mode may still be used only if deliberately
   accepted for a smoke).

## Flows to verify on testnet

Same two-pass discipline as other testnet gates when promoting beyond optional.

Module (optional; local already closed under 36/37):

```bash
MODE=module CHAIN=testnet OWNER_PRIVACY=1 ./scripts/e2e.sh testnet run
MODE=module CHAIN=testnet PROVIDER_PRIVACY=1 ./scripts/e2e.sh testnet run
MODE=module CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh testnet run
```

Store (Step 38 Phase C primary ask):

```bash
MODE=store CHAIN=testnet OWNER_PRIVACY=1 ./scripts/e2e.sh testnet run
MODE=store CHAIN=testnet PROVIDER_PRIVACY=1 ./scripts/e2e.sh testnet run
MODE=store CHAIN=testnet OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh testnet run
```

Also keep public regression green:

```bash
MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run
MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run
```

Prefer Make aliases or a small matrix runner over a third top-level `MODE`.
Record results in the verification matrix (optional → required only when
boringly green).

## Pass criteria

- Deployed ImageID matches the guest ELF used for private submit / prove.
- Paid Store query + settlement under each privacy profile (or documented
  skip with reason if a profile stays local-only).
- Private-provider claim confirms via `vault_holding` drop.
- Public Store and module testnet regressions still green.

## Promotion

When scoped, promote to a small maintenance or matrix step (or reopen a
Phase C checklist under a new packet). Close this raw TODO once testnet
privacy gates are recorded or explicitly dropped with rationale.
