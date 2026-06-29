# Step 11a — module chain reads

Wire `payment_streams_module` to `logos_execution_zone` for on-chain account reads (Step 11a
in [`program-index.md`](development-map/program-index.md)).

Prerequisites: Step 10a fixture, Step 10b wallet `.lgx`, load order wallet then payment streams,
wallet `open` with `.scaffold/wallet`.

Runtime loop: [`logos-runtime-guide.md`](logos-runtime-guide.md) Part 3.

## Module methods

All return compact JSON with `"status":"ok"` or `"status":"error"`.

| Method | Purpose |
| --- | --- |
| `readVaultConfigDecoded` | Vault config PDA (base58 id) → FFI decode |
| `readVaultHoldingDecoded` | Vault holding PDA |
| `readStreamConfigDecoded` | Stream config PDA |
| `readClockDecoded` | Clock account PDA (base58 id) |
| `readClock10Decoded` | Default `CLOCK_10` (`fixtures/localnet.json.example`) |
| `chainAction` | Step 11b writes and status (see [`archive/steps/module-chain-writes-runbook.md`](archive/steps/module-chain-writes-runbook.md)) |

For base58 → hex conversion, call `logos_execution_zone.account_id_from_base58` directly
(`accountIdHexFromBase58` is not on the public Universal surface).

Implementation uses `invokeRemoteMethod` into `logos_execution_zone` (D6), then
`payment_streams_ffi_decode_*` on account `data` bytes.

## logoscore smoke (fixture manifest)

```bash
export REPO="$PWD"
export MODULES="$HOME/Downloads/software/waku/lez-related/logos-cli/modules"
export WALLET_CONFIG="$REPO/.scaffold/wallet/wallet_config.json"
export WALLET_STORAGE="$REPO/.scaffold/wallet/storage.json"

# tooling shell + localnet up (Step 10a)
logoscore -D -m "$MODULES" -q &
sleep 3
logoscore load-module logos_execution_zone
logoscore load-module payment_streams_module
logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"

VC=$(python3 -c "import json; print(json.load(open('fixtures/localnet.json'))['vault_config_account_id'])")
logoscore call payment_streams_module readVaultConfigDecoded "$VC"
logoscore call payment_streams_module readClock10Decoded

logoscore stop
```

## Definition of done (automated)

```bash
./scripts/archive/verify-step11a-dod.sh
```

Skip live chain checks when localnet is down:

```bash
VERIFY_LOGOSCORE=0 ./scripts/archive/verify-step11a-dod.sh
```
