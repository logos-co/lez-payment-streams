# Step 32 — Authenticated transfer unify and Store claim phase

Index: [index.md](../index.md). Status: **signed off** — implementation PR may start.

Evidence logs: [step-32-step0-validation.md](../completed/step-32-step0-validation.md) (O1/O2),
[step-32-testnet-gate-log.md](../completed/step-32-testnet-gate-log.md) (D3 runs).

## Summary

1. **One shared AT ensure** for module E2E, `fixture.sh` prefund, and Store run hook —
   on-chain verify against **`authenticated_transfer`** ImageID (not “any non-zero
   `program_owner`”).
2. **Store:** split `demo_teardown` into narrated **Close** then **Claim** phases;
   same lifecycle order as today’s seed path.
3. **Module:** reorder to **close then claim** (today claim then close).
4. **Lifecycle norm (all journeys):** close stream → claim residual on closed stream.
   See [integration-contracts.md](../../reference/integration-contracts.md#chain-teardown-step-24c-local-e2e).

Prerequisites: [Step 27](../completed/step-27-claim-fix-verification.md),
[Step 28](../completed/step-28-user-journey-testnet.md),
[Step 24c](../completed/step-24c-simplify-demo-flow.md). Update [Step 29](../completed/step-29-e2e-script-ux.md)
phase tables in the same PR as orchestrator changes.

## Scope

| In | Out |
| --- | --- |
| `scripts/lib/auth_transfer.sh`, `scripts/lib/chain_poll.sh`, `scripts/auth-transfer-ensure.sh` | Guest / AT semantics changes |
| Wire ensure + phase split + artifacts | Step 21 UI, Step 25 |
| Module close→claim reorder; journey + matrix docs | Replace Python orchestrator with bash |
| `LEE_WALLET_HOME_DIR` only in new code; drop NSSA re-export from `module-e2e.sh` | Bootstrap archive refactor (**D6** deferred) |

## Decisions (D1–D8)

| Id | Outcome |
| --- | --- |
| **D1** | One testnet wallet home: `.scaffold/e2e/testnet-wallet`. Manifest owner + provider must be signable from that storage (bundle). Single ensure per run. |
| **D2** | Store: default `E2E_CLOSE_VIA=seed` for **both** close and claim (document name; no rename). Module: `chainAction` only. No `E2E_CLAIM_VIA`. Option B (`chainAction` default) only after D3 appendix. |
| **D3** | Step 32 PR keeps Store testnet `E2E_CLAIM_OPTIONAL=1`. Maintainer gate after merge; **follow-up micro-PR blocked** until a **pass** row exists in [step-32-testnet-gate-log.md](../completed/step-32-testnet-gate-log.md) (strict default + drop `demo_claim` alias). |
| **D4** | Post-close claim; module keeps balance deltas (provider up, vault holding down). Store mirrors `claim_balance` JSON when accrued at close > 0. Re-read accrued on chain before claim (not from `close_state` alone). |
| **D5** | One paragraph under integration-contracts Chain teardown: E2E norm vs optional integrator open-stream claim. |
| **D6** | Ensure via `fixture.sh` + Store run hook only; optional bootstrap one-liner deferred. |
| **D7** | Dual-emit `demo_claim` + `claim` one release; remove `demo_claim` in D3 follow-up PR. |
| **D8** | LEE only ([step-32-step0-validation.md](../completed/step-32-step0-validation.md)); remove `NSSA_WALLET_HOME_DIR` from `module-e2e.sh`. |

**D3 gate (record in gate log):**

```bash
E2E_CLAIM_OPTIONAL=0 make verify-store-testnet
make verify-module-testnet    # VAULT_ID defaults to 0
```

Pass: close on chain before claim; claim `ok` without `claim_optional_unconfirmed`;
residual accrued cleared or `claim_balance` ok; `auth_init_*` ok. Optional appendix:
`E2E_CLOSE_VIA=chainaction E2E_CLAIM_OPTIONAL=0 make verify-store-testnet`.

## Implementation sequence

1. **`common.sh`:** `ps_prepend_lez_wallet_path`; `ps_authenticated_transfer_program_id_hex`;
   tighten `ps_account_is_at_initialized` (AT ImageID compare — see step-0 log).
2. **`chain_poll.sh`:** extract inclusion helpers; defaults match `module-e2e.sh`
   (local 20×5s, testnet 45×2s).
3. **`auth_transfer.sh` + `auth-transfer-ensure.sh`:** contracts below; local RPC smoke.
4. **`fixture.sh`:** replace ad-hoc init with `ps_auth_transfer_ensure` (keep deploy +
   fund order); **`module-e2e.sh`:** source lib, LEE only, **close then claim**.
5. **`run_local_e2e.py`:** ensure hook placement; phase split; module-shaped JSONL +
   D7 alias; release/reopen wallets before standalone `wallet`.
6. **Verify locally:** `make verify-module-local`, `make verify-store-local`.
7. **Docs (step 5):** D5 paragraph, DEVELOPER_JOURNEY bundle + `E2E_CLOSE_VIA` note,
   USER_JOURNEY order, matrix footnote (optional claim until D3 gate), Step 29,
   trim `testnet-claim-known-issue.md` Symptom C diagnostic.
8. **Post-merge:** run D3 gate; append pass to gate log → **only then** follow-up PR:
   `E2E_CLAIM_OPTIONAL=0` default, drop `demo_claim` alias. No follow-up without
   a pass row in the gate log.

Do **not** block merge on D3 gate or D6 bootstrap migration.

## Shared AT ensure

### On-chain verify (replaces loose `program_owner` check)

- Normalize `getAccount.program_owner`: **8 × uint32 LE limbs** → 64-char lowercase hex
  (details and examples in [step-32-step0-validation.md](../completed/step-32-step0-validation.md)).
- Compare to AT **ImageID (hex bytes)** from `spel inspect` on pinned ELF. Resolve ELF:
  `$(ps_lez_cache)/artifacts/program_methods/authenticated_transfer.bin`, then
  `.../artifacts/lez/programs/authenticated_transfer.bin`, then git checkout fallback.
- Env override: `PS_AUTHENTICATED_TRANSFER_PROGRAM_ID_HEX`.
- If id unresolved: testnet must fail closed; local may emit `verify: program_owner_nonzero_only`.

### `ps_auth_transfer_init_one <base58>`

1. If AT-owned (strict check) → success.
2. Else if pinned `wallet` on PATH + `LEE_WALLET_HOME_DIR` → `wallet auth-transfer init`
   `--account-id Public/<id>` (**short-circuit on chain before wallet** to avoid extra txs).
3. Else logoscore `register_public_account` on **user** daemon (Store: shared storage;
   module: existing logoscore path) + inclusion poll.
4. Re-verify AT-owned after sync; else fail.

**RPC:** up to 3× `getAccount` retries (2s) on curl/RPC error → `error_class=rpc_error`.
After init tx included, re-read once; do not loop init.

### `ps_auth_transfer_ensure <owner_b58> <provider_b58>`

- Requires **`ARTIFACT`** path; appends module-style JSONL via `emit_phase`.
- Order: owner, then provider; exit **1** on first failure, **0** if both ok.
- Does **not** deploy guest program or fund accounts.

### Funding and fixture order (`fixture.sh`)

Target sequence after deploy:

1. `wallet deploy-program`
2. `fund_owner_account` — pinata funding; **today also runs `wallet auth-transfer init` for the owner** inside this function (required before pinata credits the account).
3. **`ps_auth_transfer_ensure "$owner" "$provider"`** — replaces standalone `init_provider_account`; owner short-circuits with `already_initialized: true` if step 2 already AT-init’d the owner.
4. `prefund-onchain`

Step 32 may later move owner AT-init entirely into ensure (single call before pinata); **not required for the first PR** if ensure runs after `fund_owner_account` as above.

**`ARTIFACT` in prefund:** `fixture.sh` has no JSONL log today. Default:

`FIXTURE_ARTIFACT="${FIXTURE_ARTIFACT:-$REPO_ROOT/.scaffold/e2e/artifacts/fixture-prefund-$(date +%Y%m%dT%H%M%S).log}"`

Export `ARTIFACT="$FIXTURE_ARTIFACT"` before calling `ps_auth_transfer_ensure`. Optional `FIXTURE_ARTIFACT=/dev/null` only if implementor confirms append-safe (prefer real path for debugging).


### Stale provider

No nonce heuristics. Ensure **before** first provider-signed tx in a run. On init
failure: `auth_init_provider` `ok:false`, `hint=rotate_provider_account_id`.

## CLI `auth-transfer-ensure.sh`

```
--owner <base58> --provider <base58> --artifact <file> --wallet-home <dir>
```

Sets `LEE_WALLET_HOME_DIR`, and when files exist:
`WALLET_CONFIG=<dir>/wallet_config.json`, `WALLET_STORAGE=<dir>/storage.json`.
Does not release logoscore wallets — **callers** do (Store: both hosts, same as seed).

## Call sites

| Consumer | Change |
| --- | --- |
| `module-e2e.sh` | Shared lib; LEE only; `auth_init_*` phases; close → claim |
| `fixture.sh prefund` | One `ps_auth_transfer_ensure` after `fund_owner_account`; see Funding section |
| `run_local_e2e.py` | Ensure immediately after wallet open, **before** stream create / provider txs; split close + claim phases |
| `e2e.sh cmd_prepare_testnet` | No ensure (fixture presence + read-smoke only) |
| `scripts/archive/testnet-common.sh` | Pointer comment only (**D6**) |

**Store ensure hook:** after both daemons open wallet + env set; before
`create_demo_stream_for_run`, precreated stream paths, and any provider-signed op.

**Wallet contention:** `release_logoscore_wallet` / `reopen_logoscore_wallet` on
**user and provider** before standalone `wallet` (same as seed teardown).

**Module single-host:** when `auth_transfer.sh` uses the pinned `wallet` CLI while
logoscore already has the same `storage.json` open (module local + testnet), call
`logoscore call logos_execution_zone close` before `wallet auth-transfer init` and
`open` + sync after (same pattern as Store; no second daemon). Prefer on-chain
short-circuit so wallet is not invoked when already AT-owned. Local module may
still use logoscore `register_public_account` when wallet is not used.

## Store ensure hook (`run_local_e2e.py`)

Invoke **`auth-transfer-ensure.sh` as a subprocess** (tests the CLI end-to-end).
Pass manifest `owner_account_id` and `provider_account_id` (base58), `--artifact`
(the run’s `$ARTIFACT` path), `--wallet-home` = directory containing the same
`wallet_config.json` / `storage.json` already used for the run.

Before subprocess: `release_logoscore_wallet` on **both** cfg_user and cfg_provider;
after success: reopen both and reload payment_streams wallet state (same as seed).

Child env: inherit `CHAIN`, sequencer; CLI sets `LEE_WALLET_HOME_DIR` and
`WALLET_CONFIG` / `WALLET_STORAGE` from `--wallet-home`. Do not duplicate ensure
logic inside Python.

Insert immediately after both daemons open the wallet and **before**
`create_demo_stream_for_run` / precreated-stream paths.


Module canonical shape: `{"phase","ok","extra"}` (`module-e2e.sh` `emit_phase`).

| Phase | `extra` (minimum) |
| --- | --- |
| `auth_init_owner`, `auth_init_provider` | `account_id`, `already_initialized`, `via`, optional `tx_hash`, `error`, `verify` |
| `close_state` | `vault_balance`, `total_allocated`, `stream_accrued`, `stream_unaccrued`, `stream_state` (module field names) |
| `claim_balance` | `received`, `provider_pre`, `provider_post`, `vault_pre`, `vault_post`, `attempts` |
| `claim` | same payload as today’s claim metadata; **D7:** duplicate line with phase `demo_claim` |

Store: append **one module-shaped line per phase** to `$ARTIFACT`; keep `log_artifact`
during transition if needed. Parsers: **`claim` canonical**; dedupe alias by phase or
`tx_hash`. Update Step 29 / matrix in the same PR.

**Claim skip:** accrued at close == 0 → `claim` ok + `skipped` + `reason: zero_accrued`;
no `claim_balance`.

**`claim_balance` after close:** sample `PRE_CLAIM_*` immediately before claim.
If provider balance up but vault drop ≠ received, use existing module soft path
(`ok:true` + hint), not a hard fail.

## Store phases (Developer Journey)

After `store_query_*`:

1. **Close** — `close_stream`, `close_state` (seed default via `E2E_CLOSE_VIA=seed`
   governs **close and claim** seed submit; `chainaction` fallback unchanged).
2. **Claim** — `claim`, optional `claim_balance`.

Testnet: default `E2E_CLAIM_OPTIONAL=1` in Step 32 PR. Matrix footnote until D3 gate.

## Module journey (User Journey)

Move Close + `close_state` before Claim + `claim_balance`. Module has **no** optional
claim: failed claim after close fails the run (fresh `stream_id` for retry).
Document in USER_JOURNEY.

**`MODULE_E2E_SKIP_CLOSE=1`:** after reorder, this skips **both** close and claim
(settlement block). Emit `close_stream` and `claim` skipped artifacts (or omit claim
phases with documented skip reason). Do not run claim on an active stream in the
default journey. Keep env name for testnet tx savings; behavior is “skip settlement”.

**Wallet home (`LEE_WALLET_HOME_DIR`):**

| Module chain | Home directory |
| --- | --- |
| Local | `.scaffold/module-e2e-wallet` (isolated; default `WALLET_E2E_DIR`) |
| Testnet | `.scaffold/e2e/testnet-wallet` (`ps_chain_wallet_home`) |

Store dual-host local uses `.scaffold/wallet` or e2e paths from `run_local_e2e.py`;
not the module isolated dir. Shared lib uses whatever `WALLET_HOME` / `--wallet-home`
the caller sets.

## Chain defaults (Store)

| | Localnet | Testnet |
| --- | --- | --- |
| Store wallet home | `.scaffold/wallet` or e2e user paths in orchestrator | `.scaffold/e2e/testnet-wallet` |
| Sequencer | local | manifest URL |
| Store dual-host | one storage for both daemons | D1 bundle |

(Module wallet homes: see Module journey table above.)

## Verification

| Gate | Command | Pass |
| --- | --- | --- |
| Module local | `make verify-module-local` | `auth_init_*`; `close_*` before `claim*`; all ok |
| Module testnet | `make verify-module-testnet` | Same |
| Store local | `make verify-store-local` | `store_query_*`; `close_state` before `claim*`; `auth_init_*` |
| Store testnet | `make verify-store-testnet` | Default optional claim ok; strict runs use `E2E_CLAIM_OPTIONAL=0` |

**DoD highlights:** shared ensure with AT id verify; run-time ensure on testnet at
least once with fresh init or new provider id; local gates green; D5 doc; D7 alias in
Step 32 PR.

## References

- [integration-contracts.md](../../reference/integration-contracts.md)
- [USER_JOURNEY.md](../../journeys/USER_JOURNEY.md), [DEVELOPER_JOURNEY.md](../../journeys/DEVELOPER_JOURNEY.md)
- [verification-matrix.md](../../reference/verification-matrix.md)
- [scripts/module-e2e.sh](../../../scripts/module-e2e.sh),
  [scripts/e2e/run_local_e2e.py](../../../scripts/e2e/run_local_e2e.py),
  [scripts/fixture.sh](../../../scripts/fixture.sh)
- [testnet-claim-known-issue.md](../../archive/operator/testnet-claim-known-issue.md)
