# Step 47 gate log

Feature branch: `feat/step-47-unify-role-terminology`.

## D47.4 external spot-check

| Surface | Finding | Action |
| --- | --- | --- |
| logos-docs User Journey (#370) | External doc packet; may still show historical `PAYER`/`PAYEE` / `"signer"` examples | Record only (no logos-docs PR in this step) |
| Basecamp bindings | No Basecamp serialization of write JSON keys found in-scope for this step | Record only |
| `logos-delivery` `feat/step-47-user-peer-id` | Eligibility C ABI / Nim hooks: `requester_peer_id` → `user_peer_id` | Hard-cut landed (commit `155957ad`) |
| `logos-delivery-module` `feat/step-47-user-peer-id` | Trampoline + stub header: `requester_peer_id` → `user_peer_id` | Hard-cut landed (commit `2296f87`) |

## Terminology gate

`scripts/check-terminology.sh` — Required. Temporary path-allowlist for
`docs/plan/upcoming/step-44-payer-and-payee-close.md` until Step 44 moves to
`completed/` (D47.5).

## Required rename smoke (D47.9)

Flags (fresh build; `SKIP_BUILD` unset for the rebuild/install leg that
produced the `.lgx` under test; subsequent script-only re-runs used the same
install):

```bash
MODE=module CHAIN=local \
  MODULE_E2E_TOPUP=1 MODULE_E2E_PAUSE_RESUME=1 \
  MODULE_E2E_WITHDRAW=1 CLOSE_ROLE=provider \
  ./scripts/e2e.sh local run
# or: ./scripts/module-e2e.sh with the same env
```

Pre–Step 44: `CLOSE_ROLE=provider` exercises the six-slot close with the
renamed `provider` JSON key. Owner-only omit-`provider` waits on dual-close.

Result (2026-08-11 local): green — artifact
`.scaffold/e2e/artifacts/module-e2e-20260811T204348.log`.
All rename sites hit: `owner` writes (init/deposit/create/pause/resume/topUp),
`withdraw` (`owner` + distinct `withdraw_to` — same-id `withdraw_to` duplicates
LEZ account slots), `closeStream` with `provider`, claim unchanged.
`make check-terminology` / `./scripts/check-terminology.sh` pass.
