# Step 17 — local E2E demo (operator)

Plan excerpt: [plan/completed/step-17.md](plan/completed/step-17.md). Contracts:
[integration-contracts.md](integration-contracts.md).

## Goal

Two `logoscore` hosts on one machine: provider archives relay traffic and serves paid Store;
user publishes, queries with LIP-155 proof, and reads results. Provider advertises libp2p
identity off-band (mimicked by a file). Missing-proof Store requests are rejected on the provider.

## Environment

| Variable | Default (under `REPO`) | Role |
| --- | --- | --- |
| `REPO` | repo root | |
| `MODULES_USER` | `.scaffold/e2e/user/modules` | User `lgpm` + `logoscore -m` |
| `MODULES_PROVIDER` | `.scaffold/e2e/provider/modules` | Provider install tree |
| `LOGOSCORE_CONFIG_USER` | `.scaffold/e2e/user/logoscore` | User `--config-dir` |
| `LOGOSCORE_CONFIG_PROVIDER` | `.scaffold/e2e/provider/logoscore` | Provider `--config-dir` |
| `PERSIST_USER` | `.scaffold/e2e/user/persist` | User `--persistence-path` |
| `PERSIST_PROVIDER` | `.scaffold/e2e/provider/persist` | Provider `--persistence-path` |
| `FIXTURE_MANIFEST` | `fixtures/localnet.json` | Chain ids |
| `PAYMENT_STREAMS_GUEST_BIN` | guest `.bin` path | Both daemons |
| `E2E_PROVIDER_AD` | `.scaffold/e2e/provider-advertisement.json` | Off-band mimic (written by script) |
| `DELIVERY_MODULE_ROOT` | `../logos-delivery-module` | Checkout for `nix build …#lgx` |
| `LOGOS_DELIVERY_ROOT` | `../logos-delivery` | Optional overlay source (see below) |
| `SKIP_LIBLOGOSDELIVERY_OVERLAY` | `0` | Set `1` for hermetic nix-only `liblogosdelivery` |
| `E2E_PHASE` | `all` | `core`, `claim`, or `all` |
| `SKIP_BUILD` | `0` | `1` to reuse installed modules |
|| `SKIP_SEED` | `1` with existing manifest | Skip prepare when `fixtures/localnet.json` exists (no validity check) |
|| `FULL_RESET` | `0` | `1` rebuilds funded snapshot (pinata + prefund) before create-stream |
| `N8_WIRE_HEX` | (computed) | Host `cargo run -p lez-payment-streams-core --bin n8_canonical_wire_hex` if unset |

Wallet (demo): both hosts may `open` the same Step 10a
`.scaffold/wallet/wallet_config.json` and `storage.json` (owner + provider keys from seed).
Use separate `--persistence-path` per host for `payment_streams_state.json`.

Tooling shell (each tab):

```bash
nix shell \
  github:logos-co/logos-package-manager \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-module#lm \
  --command bash
```

CLI targeting a daemon (repeat on every subcommand):

```bash
export LC_USER=(--config-dir "$LOGOSCORE_CONFIG_USER")
export LC_PROVIDER=(--config-dir "$LOGOSCORE_CONFIG_PROVIDER")
```

Two `-D` processes on one machine require **different** `--config-dir` values. There is no
integration rule against sharing a single `MODULES` tree; Step 17 uses two install roots so
user and provider can diverge later (pins, delivery builds, testnet wallets).

## Fixture — prepare (Step 17b)

Before dual-host messaging, `make verify-step17` runs the prepare phase
([`scripts/e2e.sh`](../scripts/e2e.sh) `local prepare`, also `make prepare-localnet`):

- **Default:** restore `.scaffold/snapshots/funded/` when `snapshot.json` matches LEZ pin and
  `make program-id`, then `fixture.sh vault ensure` + `vault manifest` (vault-only baseline). The
  per-run stream is created by the orchestrator at `next_stream_id`.
- **No snapshot / invalid snapshot:** prefund once (pinata + `prefund-onchain` + snapshot), then
  restore + ensure vault.
- **`FULL_RESET=1`:** rebuild snapshot from scratch (`make full-reset-localnet`).

Per-run cost: restore (directory copy + sequencer restart) + one `CreateStream`.

`SKIP_SEED=1` (with `RESTORE_LOCALNET=0`) skips restore/reseed and continues on the live ledger.

If Step 12 strict path would hit `STREAM_DEPLETED` on an old chain without restore, run
`make prepare-localnet` or `FULL_RESET=1 make verify-step17`.

## Build and install

Primary entrypoint:

```bash
make verify-step17
# or: MODE=store CHAIN=local ./scripts/e2e.sh local run
```

The script (`SKIP_BUILD=1` to reuse installed modules) builds/installs from `REPO` using pins in
[feature-branch-pins.md](feature-branch-pins.md). Implementation:
[`scripts/e2e.sh`](../scripts/e2e.sh).

1. `nix build ./logos-payment-streams-module#lgx` → `lgpm install --force` into
   `MODULES_USER` and `MODULES_PROVIDER`.
2. Patched wallet `.lgx` from `scripts/archive/build-wallet-lgx.sh` (built on first run if missing) →
   both module dirs via `lgpm`.
3. `nix build "$DELIVERY_MODULE_ROOT#lgx"` → `lgpm install` into both module dirs. The bundle
   includes `delivery_module_plugin.so`, `liblogosdelivery.so`, and runtime deps (`librln`,
   `libpq`) from the module flake `postInstall` ([logos-delivery-module](https://github.com/logos-co/logos-delivery-module)
   integration branch).
4. Optional overlay (default when sibling tree exists): if `SKIP_LIBLOGOSDELIVERY_OVERLAY` is not
   `1` and `LOGOS_DELIVERY_ROOT` contains a Makefile, run `make liblogosdelivery` there and copy
   `build/liblogosdelivery.so` over each `…/delivery_module/` install. Use this while iterating on
   `logos-delivery` without bumping `logos-delivery-module/flake.lock`. For reproducible nix-only
   libs, set `SKIP_LIBLOGOSDELIVERY_OVERLAY=1` ([N13](reference/decisions-and-notes.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18)).

Orchestration: [`scripts/e2e/run_local_e2e.py`](../scripts/e2e/run_local_e2e.py) via
[`scripts/e2e.sh`](../scripts/e2e.sh) (JSON-lines artifact under
`.scaffold/e2e/artifacts/`). This runbook is **Track A** — Store + eligibility integration
([N18](reference/decisions-and-notes.md#n18-integration-demo-vs-payment-streams-ui-tracks-2026-06)).
Payment-streams-only Basecamp UI is **Track B** (Steps 21–22), not described here.
Policy: [N17](reference/decisions-and-notes.md#n17-demo-orchestration-stays-external-script-2026-06).
Step 20 developer journey adds a step-by-step command path equivalent to the script; see
[step-20-developer-journey.md](plan/upcoming/step-20-developer-journey.md).
Optional probe: [`scripts/e2e/debug_happy_path.py`](../scripts/e2e/debug_happy_path.py).

## Hermetic run (hand-off)

A hermetic Step 17 run uses only nix-built artifacts and committed flake locks — no local
`make liblogosdelivery` overlay and no copying `.so` files by hand. Use this to verify pins
after pushing delivery repos or before marking Step 17 complete.

Prerequisites:

- This repo (`lez-payment-streams`) at the integration branch you intend to ship.
- Sibling checkout `logos-delivery-module` at `DELIVERY_MODULE_ROOT` with committed
  `flake.lock` pinning `logos-delivery` at rev `39b467ec` or newer (outbound
  `eligibilityProof` retained in `logosdelivery_store_query`). See
  [feature-branch-pins.md](feature-branch-pins.md).
- You do not need a sibling `logos-delivery` tree for the run itself when overlay is skipped.
- Host Rust toolchain (script computes `N8_WIRE_HEX` via `cargo run` before the nix tooling
  shell).
- Guest ELF: `make build` once if `PAYMENT_STREAMS_GUEST_BIN` is missing.
- Local LEZ sequencer on `127.0.0.1:3040` (script calls `demo-localnet-fresh.sh` when fixture
  checks fail).
- Patched wallet `.lgx` inputs resolve via nix (`scripts/archive/build-wallet-lgx.sh` on first run).

Command (from repo root):

```bash
export DELIVERY_MODULE_ROOT="${DELIVERY_MODULE_ROOT:-$PWD/../logos-delivery-module}"
export SKIP_LIBLOGOSDELIVERY_OVERLAY=1
make verify-step17
```

What “hermetic” does not mean here:

- The script still uses the host `cargo` binary for N8 and `python3` for the orchestrator.
- `lgpm` and `logoscore` come from a ephemeral `nix shell` (not a single locked devShell for
  the whole repo).
- Both hosts may share the Step 10a wallet files under `.scaffold/wallet/` (demo policy).

After `lgpm install`, each `…/modules/delivery_module/` tree must load at runtime. If
`delivery_module` fails with `MODULE_LOAD_FAILED`, inspect that directory for
`liblogosdelivery.so` and bundled deps next to the plugin; compare with the store path from
`nix build "$DELIVERY_MODULE_ROOT#lgx" --print-out-paths` and the unpacked `.lgx` layout.

Failure triage without overlay:

| Symptom | Likely cause |
| --- | --- |
| Provider `BAD_REQUEST`, empty inbound proof | Stale `liblogosdelivery.so` (lock below `39b467ec`) or wrong file in `delivery_module/` |
| Client `BAD_REQUEST` on a query *with* proof | Provider verifier rejected. The client only sees `BAD_REQUEST`; the orchestrator logs the real verdict in the `store_query_eligibility_verdict` artifact line (calls the provider verifier directly). Read `eligibility` + `message` (policy rejects include `reject_reason=N`). |
| `verify` / prepare → `STREAM_NOT_ACTIVE` | Stream depleted or wrong id; fresh prepare + per-run create, or `demo-localnet-fresh.sh`. Default allocation `200` at rate `1` (~200 s runway per stream). |
| `verify` → `PARAMS_REJECTED` (`reject_reason=4`) | `RateBelowAcceptedParams`: on-chain rate below the accepted/proposal rate. Fixed 2026-06-19 (`fillServiceId` no longer clobbers rate/allocation). If it recurs, the stream's on-chain rate genuinely differs from `kDemoRate` for the proposal arm. |
| `MODULE_LOAD_FAILED` for `delivery_module` | Incomplete `lgpm` install or missing bundled `.so` in module dir |

Default developer path (overlay on): omit `SKIP_LIBLOGOSDELIVERY_OVERLAY` when
`../logos-delivery` is present — full E2E gate green 2026-06-19 with `make verify-step17`
(`store_query_success` 200, missing-proof rejected, claim `tx_hash`) after the `fillServiceId`
rate/allocation fix. Earlier overlay/hermetic paths verified 2026-06-18 with
`SKIP_LIBLOGOSDELIVERY_OVERLAY=1` and `logos-delivery` `39b467ec` in the module lock.

After user `prepareEligibilityProofWithStreamProofForStoreQuery`, run `scripts/e2e/seed_provider_acceptance.py` to
copy `session_public_key_hex` into provider `provider_acceptances` (dual-host warm-up), then
reload `payment_streams_module` on the provider host. The script selects the negotiation matching
the current manifest provider (newest-first), so a stale negotiation from a prior session under a
different provider key cannot seed the wrong key when `PERSIST_USER` is not cleared. The documented
re-run gate still clears `PERSIST_USER` / `PERSIST_PROVIDER`.

## Provider service advertisement (off-band mimic)

Production: users learn `provider_peer_id` and dial multiaddr from out-of-band docs.

Demo: after provider `createNode` + `start`, script calls
`logoscore "${LC_PROVIDER[@]}" call delivery_module getNodeInfo MyPeerId` (and listens address
info if needed), then writes `E2E_PROVIDER_AD`:

```json
{
  "provider_peer_id": "<libp2p peer id string>",
  "provider_store_multiaddr": "/ip4/127.0.0.1/tcp/<tcpPort>/p2p/<peerId>",
  "content_topic": "/lez-payment-streams/1/e2e-eligibility/proto",
  "service_id": "/vac/waku/store-query/3.0.0"
}
```

User host reads this file (not hardcoded Step 12 placeholder peer strings) for
`registerProviderMapping` and `storeQuery(..., providerAddr)`.

## Delivery `createNode` defaults

Keys are flat JSON for `WakuNodeConf` ([logos-delivery-module README](https://github.com/logos-co/logos-delivery-module)).
Use camelCase or CLI `name:` aliases (e.g. `store-message-retention-policy`).

Provider (Store service + archive):

```json
{
  "mode": "Core",
  "preset": "logos.dev",
  "logLevel": "INFO",
  "portsShift": 100,
  "relay": true,
  "store": true,
  "storeMessageRetentionPolicy": "capacity:10000",
  "storeMessageDbUrl": "sqlite://store.sqlite3"
}
```

Place `store.sqlite3` under `PERSIST_PROVIDER` (absolute `file://` or path relative to cwd at
`createNode` — keep cwd stable in the script).

User (relay client; no local Store service):

```json
{
  "mode": "Core",
  "preset": "logos.dev",
  "logLevel": "INFO",
  "portsShift": 0,
  "relay": true,
  "store": false,
  "staticnode": ["<provider_store_multiaddr from E2E_PROVIDER_AD>"]
}
```

`portsShift` avoids TCP/discv5 collisions between daemons
([logos-delivery-module#18](https://github.com/logos-co/logos-delivery-module/issues/18)).

## Module load order (each host)

```bash
logoscore "${LC_*[@]}" -D -m "$MODULES_*" --persistence-path "$PERSIST_*" -q
logoscore "${LC_*[@]}" load-module logos_execution_zone
logoscore "${LC_*[@]}" load-module payment_streams_module
logoscore "${LC_*[@]}" load-module delivery_module
logoscore "${LC_*[@]}" call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"
```

Sync wallet to sequencer height before chain-dependent calls.

## Eligibility registration

| Host | Call | Purpose |
| --- | --- | --- |
| Provider | `setEligibilityVerifier` → `payment_streams_module` | Paid mode: every inbound Store verified |
| User (happy path) | `setEligibilityProvider` → `payment_streams_module` | Attach proof on outbound `storeQuery` |

After `createNode` on each host ([N3b](reference/decisions-and-notes.md#n3b-step-16-hook-registration-lifecycle-2025-06-18)).

User before Store:

```bash
PROVIDER_B58=$(python3 -c "import json; print(json.load(open('$FIXTURE_MANIFEST'))['provider_account_id'])")
PEER=$(python3 -c "import json; print(json.load(open('$E2E_PROVIDER_AD'))['provider_peer_id'])")
logoscore "${LC_USER[@]}" call payment_streams_module registerProviderMapping "$PEER" "$PROVIDER_B58"
```

## Happy path (smoke)

1. Both nodes `createNode` + `start`; write `E2E_PROVIDER_AD`.
2. User `subscribe` + `send` on demo `content_topic`; wait for provider `messageReceived` (or brief sleep).
3. Build `queryJson` (camelCase, unique `requestId`, `includeData`, `paginationForward`, filter
   on `contentTopics` / time window as needed). Omit `eligibilityProof`.
4. Async `storeQuery`:

```bash
logoscore "${LC_USER[@]}" call delivery_module storeQuery "$QUERY_JSON" "$PROVIDER_ADDR"
```

5. Wait for completion (see below). Assert response JSON: success, non-empty messages when
   archived traffic exists, eligibility OK on provider side in logs.

Canonical bytes for prepare are not needed when proof is attached via the provider hook; the
hook uses N8 bytes inside liblogosdelivery. Ensure stream `0` (or bound stream) is active on chain.

## Async `storeQuery` — recommended wait pattern

`storeQuery` dispatches FFI and completes on the `storeQueryCompleted` event ([N3a](reference/decisions-and-notes.md#n3a-step-16-threading--approach-a-experiment-2025-06-18)).

Reasonable approach for bash:

1. Start background watcher before or immediately after `call storeQuery`:

```bash
logoscore "${LC_USER[@]}" watch delivery_module > "$PERSIST_USER/store-query-events.log" 2>&1 &
WATCH_PID=$!
```

2. Issue `storeQuery` via `logoscore call`.
3. Poll the watch log (or use `timeout`) for a line containing `storeQueryCompleted` and parse
   `responseJson` (third argument in the event payload per module codegen).
4. Kill watcher; fail if timeout (e.g. 120s).

Alternative: run a short Python helper using LogosAPI if `watch` output format is awkward;
keep the same timeout and assertions.

## Failure path — missing proof (paid provider)

Provider keeps `setEligibilityVerifier` enabled.

User host: call `setEligibilityProvider` with an **empty** module name to clear outbound
proof attachment ([N3b](reference/decisions-and-notes.md#n3b-step-16-hook-registration-lifecycle-2025-06-18)),
then `storeQuery` with the same `providerAddr`. liblogosdelivery sends no tag-30 proof.

Assert provider-side outcome: HTTP/wire 400, `eligibility_status` with non-OK verdict, empty
messages ([N3c](reference/decisions-and-notes.md#n3c-inbound-missing-proof-null-proof_hex-2025-06-18)).

Re-enable user `setEligibilityProvider` before the happy path if order is reversed in the script.

## Phase B — teardown (close then claim)

Step 24c runs teardown in the **`core`** phase before exit (default `E2E_PHASE=all`).

Close (local default `E2E_CLOSE_VIA=seed`):

- Seed path: `close-stream-onchain` with **stream provider** as authority signer (provider key lives
  in `.scaffold/wallet` from prefund). Orchestrator calls `release_logoscore_wallet` on the user
  host while seed runs.
- Fallback: provider host `chainAction closeStream` with JSON
  `{"signer":"<owner>","vault_id":N,"stream_id":N,"authority":"<provider_account_id>"}`.
- Success: on-chain `stream_state == Closed` for the run’s stream PDA (see `demo_close_stream_verify`
  in the artifact). `total_allocated_lo` may stay high until claim when the stream accrued during
  the demo.

Claim (provider host, same manifest `vault_id` / `stream_id` as the run):

```bash
logoscore "${LC_PROVIDER[@]}" call payment_streams_module chainAction claim \
  '{"provider":"<provider_account_id>","vault_id":0,"stream_id":<run_stream_id>}'
```

If accrued is zero after close, the orchestrator logs `demo_claim` with
`skipped: true`, `reason: zero_accrued` (still exit 0 when Store DoD passed).

Legacy manual claim-only snippet (superseded by close-then-claim in the script):

```bash
# Prefer: make verify-step17 (orchestrator owns stream_id from manifest)
logoscore "${LC_PROVIDER[@]}" call payment_streams_module chainAction claim \
  '{"provider":"<provider_account_id>","vault_id":0,"stream_id":0}'
```

Script flag: `E2E_PHASE=core|claim|all` (default `all`). Claim-only phase is legacy; `core` includes
close and claim.

## Log artifact

Path: `.scaffold/e2e/artifacts/demo-e2e-local-<timestamp>.log`

JSON-lines, one object per phase, e.g.:

```json
{"phase":"seed","ok":true,"manifest":"fixtures/localnet.json"}
{"phase":"provider_ad","ok":true,"provider_peer_id":"…","provider_store_multiaddr":"…"}
{"phase":"store_query_success","ok":true,"message_count":1,"status":200}
{"phase":"store_query_missing_proof","ok":true,"status":null,"message_count":0}
{"phase":"demo_close_stream","ok":true,"stream_id":0,"via":"seed_close_stream_onchain"}
{"phase":"vault_liquidity_after_close","ok":true,"total_allocated_lo":0,"unallocated_lo":2000}
{"phase":"demo_claim","ok":true,"skipped":true,"reason":"zero_accrued","stream_id":0}
```

On a failed `store_query_success`, the orchestrator adds a diagnostic line with the provider's
real verdict (the client-visible error is only `BAD_REQUEST`):

```json
{"phase":"store_query_eligibility_verdict","ok":false,"eligibility":"PARAMS_REJECTED","message":"stream policy check failed (reject_reason=4)","verdict":{…}}
```

Script exit code: non-zero if any required phase has `"ok":false`.

## Run duration and fast iteration

Local block production is not the main cost of a healthy Step 17 run. The debug sequencer uses
Bedrock-driven block creation (`block_create_timeout` 15s in LEZ debug config), but when the
chain is healthy a `createStream` confirms in seconds. Multi-minute or 20+ minute runs usually
stack unrelated waits:

| Source | Typical cost | Notes |
|--------|--------------|--------|
| `prefund-localnet.sh` / `FULL_RESET` | ~10+ min | Pinata rounds + deploy; not per-rerun if snapshot is fresh |
| Stale snapshot restore | Extra prefund or `STREAM_DEPLETED` | Ledger clock vs wall time; see `fold_gap_seconds` in artifacts |
| Seed `create-localnet-stream-fixture.sh` | Up to **3 × `E2E_SUBPROC_TIMEOUT_S`** (default 600s) | Retries on wallet `TxPoller` miss |
| Wallet `TxPoller` (seed path) | ~**120 s** worst case per attempt (E2E overrides) | Exponential backoff from 250 ms, cap `E2E_WALLET_POLL_MAX_DELAY` (default 8s); rebuild `wallet` from LEZ with updated `poller.rs` |
| Orchestrator | ~1–4 min when green | `PUBLISH_WAIT_S=15`, dual daemons, storeQuery up to 120s, teardown |
| Nix module build | Minutes | Skip with `SKIP_BUILD=1` after first install |

Debug the sequencer before blaming “slow blocks”:

```bash
make debug-sequencer-latency
# or: REPO=$PWD SEQ_PROBE_WINDOW_S=8 python3 scripts/e2e/sequencer_latency_probe.py
```

If `block_delta_over_*` is zero, fix localnet (restart sequencer, check Bedrock) before re-running E2E.

E2E artifacts now include `run_config`, `timing_mark`, `create_stream_poll_budget`, per-seed
`create_stream_seed_attempt`, and `run_total` elapsed seconds.

Fast loop after one green run (same ledger, monotonic stream ids):

```bash
SKIP_BUILD=1 SKIP_SEED=1 RESTORE_LOCALNET=0 MODE=store CHAIN=local ./scripts/e2e.sh local run
```

Continuation defaults use seed create (not `chainAction`-only). The Makefile gate runs owner pinata
between legs: `make verify-step17-back-to-back`.

Fail faster while debugging create confirm (does not fix a stalled sequencer):

```bash
export E2E_SUBPROC_TIMEOUT_S=120
export E2E_STREAM_FUNDABLE_WAIT_S=15
```

Seed create polling (copied wallet only):

```bash
export E2E_WALLET_POLL_MAX_DELAY=8s
export E2E_WALLET_POLL_MAX_ATTEMPTS=22
```

Requires a `wallet` binary built from LEZ with exponential `TxPoller` (`logos-execution-zone/lez/wallet/src/poller.rs`).

## Verify entrypoint

```bash
# Restore funded snapshot, new stream at vault next_stream_id, full demo (close then claim)
make verify-step17

# Step 24c gate: second run on same ledger (no restore; next_stream_id increments)
make verify-step17-back-to-back

# Rebuild vault-only snapshot (after LEZ pin / guest ImageID change, or polluted ledger)
make full-reset-localnet
```

Local prepare policy (Step 17b + 24c):

- First run (or after reset): `scripts/e2e.sh local prepare` restores `.scaffold/snapshots/funded/` and writes a vault-only manifest (no stream fields).
- Repeat demos on the same chain: `SKIP_SEED=1` or `RESTORE_LOCALNET=0` so the orchestrator does not rewind rocksdb; each run creates at `next_stream_id`.
- E2E artifacts include `plan_demo_stream`, `baseline_before_create`, `checkpoint_after_create`, `demo_close_stream`, `vault_liquidity_after_close`, `demo_claim`.
- Env: `E2E_CLOSE_VIA=seed` (default local) or `chainaction`; `E2E_STRICT_SEQUENCER_TX_WAIT=1` to require `getTransaction` after wallet submit.

Equivalent: `MODE=store CHAIN=local ./scripts/e2e.sh local run` (phases `E2E_PHASE=core|claim|all`, default `all`).
Requires local LEZ on `127.0.0.1:3040` and a valid funded snapshot (or `make full-reset-localnet` once).

## Related

- Recovery: [demo-localnet-recovery.md](demo-localnet-recovery.md)
- Runtime install loop: [logos-runtime-guide.md](logos-runtime-guide.md) (Step 17 dual-host section)
- Step 12 prepare/mapping (single-host): [step12-user-eligibility.md](step12-user-eligibility.md)
