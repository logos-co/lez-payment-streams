# Step 17 — local E2E demo (operator)

Plan excerpt: [plan/upcoming/step-17.md](plan/upcoming/step-17.md). Contracts:
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

## Fixture — seed if needed

Before dual-host messaging:

1. If `fixtures/localnet.json` is missing or `./scripts/verify-step10a-dod.sh` fails → run
   `./scripts/demo-localnet-fresh.sh`.
2. If Step 12 strict path would hit `STREAM_DEPLETED` → same fresh seed or
   `TRY_TOPUP=1 ./scripts/step12-topup-and-prepare.sh` logic inside the E2E script.

Localized DoD: prefer seeded stream `0`; only run full vault/stream lifecycle when seed detection fails.

## Build and install

Primary entrypoint:

```bash
make verify-step17
# or: ./scripts/demo-e2e-local.sh
```

The script (`SKIP_BUILD=1` to reuse installed modules) builds/installs from `REPO` using pins in
[feature-branch-pins.md](feature-branch-pins.md):

1. `nix build ./logos-payment-streams-module#lgx` → `lgpm install` to both module dirs.
2. Patched wallet `.lgx` → both dirs.
3. `nix build "$DELIVERY_MODULE_ROOT#packages.x86_64-linux.default"` (add `--impure` while the
   module checkout has uncommitted C++ changes) → copy `delivery_module_plugin.so` and bundled
   `liblogosdelivery.so` into each `…/delivery_module/` tree (not `.lgx` for the demo layout).
4. Overlay: `make liblogosdelivery` in `LOGOS_DELIVERY_ROOT` (default `../logos-delivery`) and
   copy `build/liblogosdelivery.so` over both delivery module dirs until nix-only installs include
   the outbound proof fix ([N13](reference/decisions-and-notes.md#n13-step-17-liblogosdelivery-bundle-vs-local-overlay-2026-06-18)).

Orchestration: `scripts/e2e/run_local_e2e.py` (dual hosts, JSON-lines artifact under
`.scaffold/e2e/artifacts/`). Optional probe: `scripts/e2e/debug_happy_path.py`.

After user `prepareEligibilityForStoreQuery`, run `scripts/e2e/seed_provider_acceptance.py` to
copy `session_public_key_hex` into provider `provider_acceptances` (dual-host warm-up), then
reload `payment_streams_module` on the provider host.

## Provider service advertisement (off-band mimic)

Production: users learn `provider_peer_id` and dial multiaddr from out-of-band docs.

Demo: after provider `createNode` + `start`, script calls
`logoscore "${LC_PROVIDER[@]}" call delivery_module getNodeInfo MyPeerId` (and listens address
info if needed), then writes `E2E_PROVIDER_AD`:

```json
{
  "provider_peer_id": "<libp2p peer id string>",
  "provider_store_multiaddr": "/ip4/127.0.0.1/tcp/<tcpPort>/p2p/<peerId>",
  "content_topic": "/lez-payment-streams/e2e/1/demo/proto",
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

## Phase B — provider claim

On provider host (manifest `vault_id` / `stream_id` for stream `0`):

```bash
logoscore "${LC_PROVIDER[@]}" call payment_streams_module chainAction claim \
  '{"provider":"<provider_account_id>","vault_id":0,"stream_id":0}'
```

Requires provider signer in wallet storage. Log `tx_hash` in the artifact. If accrual is zero,
document SKIP with reason (still exit 0 only if core Store DoD already passed and claim failure
is explained — prefer top-up or query before claim so claim is non-vacuous).

Script flag: `E2E_PHASE=core|claim|all` (default `all`).

## Log artifact

Path: `.scaffold/e2e/artifacts/demo-e2e-local-<timestamp>.log`

JSON-lines, one object per phase, e.g.:

```json
{"phase":"seed","ok":true,"manifest":"fixtures/localnet.json"}
{"phase":"provider_ad","peer_id":"…","multiaddr":"…"}
{"phase":"store_query_success","ok":true,"message_count":1}
{"phase":"store_query_missing_proof","ok":true,"status":400,"eligibility":"PROOF_INVALID"}
{"phase":"claim","ok":true,"tx_hash":"…"}
```

Script exit code: non-zero if any required phase has `"ok":false`.

## Verify entrypoint

```bash
make verify-step17
```

Equivalent: `./scripts/demo-e2e-local.sh` (phases `E2E_PHASE=core|claim|all`, default `all`).
Requires local LEZ on `127.0.0.1:3040` and Step 10a fixture (script seeds via
`demo-localnet-fresh.sh` when needed).

## Related

- Recovery: [demo-localnet-recovery.md](demo-localnet-recovery.md)
- Runtime install loop: [logos-runtime-guide.md](logos-runtime-guide.md) (Step 17 dual-host section)
- Step 12 prepare/mapping (single-host): [step12-user-eligibility.md](step12-user-eligibility.md)
