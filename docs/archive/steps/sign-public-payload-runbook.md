# Step 11c — `sign_public_payload` on the patched wallet

Adds `wallet_ffi_sign_public_payload` to the Rust FFI layer
and exposes it as `Q_INVOKABLE sign_public_payload` on the Qt wallet module.

Prerequisites: Step 10b green (`./scripts/archive/verify-step10b-dod.sh` exits 0).

## What is added

### Rust FFI (`lez-rust-sign-public-payload.patch`)

`wallet_ffi_sign_public_payload(handle, account_id, canonical_payload_digest, out_signature)`

- `account_id`: `const FfiBytes32 *` (32 bytes, same hex format as `wallet_ffi_get_public_account_key`)
- `canonical_payload_digest`: `const FfiBytes32 *` (32-byte pre-hashed digest)
- `out_signature`: `FfiSignature64 *` (64-byte output)
- Returns `WalletFfiError::Success` or an error code.
- Signs using BIP-340 Schnorr over the account's signing key.

`FfiSignature64` is a new `#[repr(C)]` struct with `pub data: [u8; 64]`.

### Qt invokable (`wallet-qt-sign-public-payload.patch`)

```
Q_INVOKABLE QString sign_public_payload(
    const QString& account_id_hex,
    const QString& digest_hex
)
```

Both inputs are lowercase hex strings (64 chars = 32 bytes).

Return value is a compact JSON string:

```json
{"status":"ok","result":"<128-hex-char Schnorr signature>"}
```

or on error:

```json
{"status":"error","error":"<message>"}
```

## Patch delivery

Both patches live inside the `lez-wallet-ffi-patched` and outer
`logos-execution-zone-module-patched` flakes:

```
logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/
  wallet-qt-guest-elf-from-env.patch
  wallet-qt-sign-public-payload.patch         # Step 11c Qt patch
  lez-wallet-ffi-patched/
    lez-rust-sign-public-payload.patch        # Step 11c Rust patch
    flake.nix                                 # LEZ pin — see feature-branch-pins.md
```

The `lez-wallet-ffi-patched/flake.nix` LEZ input matches `scaffold.toml` and
`nix/payment-streams-ffi.nix` (currently
`62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60`, PR 510 merge).

## Build and install

Same patched wallet `.lgx` as Step 10b:

```bash
chmod +x scripts/archive/build-wallet-lgx.sh
./scripts/archive/build-wallet-lgx.sh
```

Install and `MODULES` layout: [`archive/steps/wallet-runtime-runbook.md`](archive/steps/wallet-runtime-runbook.md#install-into-modules).

## Verification

### Static check (no running chain needed)

```bash
lm methods "$MODULES/logos_execution_zone/logos_execution_zone_plugin.so" \
  | rg sign_public_payload
```

### Sign-then-verify smoke test

Requires Step 10a localnet running and `smoke_verify` built:

```bash
./scripts/archive/verify-step11c-dod.sh
```

The script:

1. Checks `lm methods` lists `sign_public_payload`.
2. Picks the first public account from `list_accounts` (any account the wallet holds
   a signing key for works; vault PDAs do not have signing keys).
3. Calls `sign_public_payload` with that account and sha256("test") as a known digest.
4. Calls `get_public_account_key` for the same account.
5. Runs `./target/debug/smoke_verify <pubkey_hex> <digest_hex> <sig_hex>` and asserts exit 0.

Manual smoke (once logoscore is running with wallet opened):

```bash
# sha256("test") — avoids an all-zeros Qt hex-decode edge case
DIGEST="9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"

# Pick first public wallet account (has a signing key by construction)
HEX=$(logoscore call logos_execution_zone list_accounts \
  | tail -1 | python3 -c "
import sys,json
outer=json.load(sys.stdin)
accounts=outer.get('result')
if isinstance(accounts,str): accounts=json.loads(accounts)
print([a for a in accounts if a.get('is_public')][0]['account_id'])
")

# Sign (result is a double-encoded JSON string)
SIG=$(logoscore call logos_execution_zone sign_public_payload "$HEX" "$DIGEST" \
  | tail -1 | python3 -c "
import sys,json
outer=json.load(sys.stdin)
print(json.loads(outer['result'])['result'])
")

# get_public_account_key returns the hex key directly in result (not double-encoded)
PUB=$(logoscore call logos_execution_zone get_public_account_key "$HEX" \
  | tail -1 | python3 -c "import sys,json; print(json.load(sys.stdin)['result'])")

./target/debug/smoke_verify "$PUB" "$DIGEST" "$SIG" && echo "PASS"
```
