# User Journey — payment streams on TestNet v0.2

Install `payment_streams_module` next to `logos_execution_zone`, then on public testnet run the
LIP-155 flow as payer (vault, deposit, stream, close) and payee (claim). One `logoscore` daemon
and one wallet file hold two public accounts on a single host.

Pause, resume, and top-up are out of scope ([chainAction catalogue](../payment-streams-module/README.md#chainaction-catalogue)).

## Glossary

| Term (prose)               | JSON / wire                                      | Meaning                                                                                     |
| -------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| Payer                      | `signer` on writes; `owner` on reads and `claim` | Vault owner; closes the stream in this flow                                                 |
| Payee                      | `provider` on `createStream` and `claim`         | Recipient; claims after close                                                               |
| Vault                      | `vault_id`                                       | Holds deposits and allocations                                                              |
| Stream                     | `stream_id`                                      | Pays payee at `rate` up to `allocation`                                                     |
| `*_lo` / `*_hi`            | writes and `getStreamStatus`                     | 128-bit amount as two uint64s: `lo + (hi << 64)`. Values here fit in `*_lo` with `*_hi` = 0 |
| `accrued_*`, `unaccrued_*` | `getStreamStatus`                                | Claimable vs not-yet-time-accrued allocation                                                |
| `stream_state`             | 0 Active, 1 Paused, 2 Closed                     |                                                                                             |
| `MIN_ACCRUED`              | (shell only)                                     | Minimum `accrued_lo` before close in this walkthrough; token units, not seconds             |
| Authenticated transfer (AT) | `register_public_account` on testnet            | Lets public accounts spend tokens; required before deposit and stream writes                  |

Default sizing: sequencer `https://testnet.lez.logos.co/`, program id
`de17c0db368abf9f6476f4d67a56ad24e89ddb23bc49b58f7effb566146c1677` (release guest ELF
361716 bytes), deposit 500 (must cover
`allocation` 80; leftover deposit stays in the vault after close), rate 1 token per second of chain
clock time, `MIN_ACCRUED` 1, vault/stream ids start at 0. Testnet program id is org-deployed;
your built guest must match `fixtures/testnet-module.json` (Step 3).

## Prerequisites

Host: Git, Nix (flakes), Docker, RISC Zero CLI (`cargo-risczero`, `rzup install`), Rust/cargo
(for `lgs setup` and for the shell to install `lgs` when it is missing), network access to the
public sequencer.

```bash
git clone https://github.com/logos-co/lez-payment-streams.git
cd lez-payment-streams
chmod +x scripts/user-journey-shell.sh
./scripts/user-journey-shell.sh
```

The script installs Logos Scaffold (`lgs`) from [logos-co/scaffold](https://github.com/logos-co/scaffold)
when it is not on `PATH`, then opens a shell with `logoscore` and `lgpm` 0.2.0. Run the journey
from the repo root inside that shell. Re-export Step 1 if you open a new terminal.

## Step 1 — Session variables

```bash
export REPO_ROOT="$(pwd)"
export LEZ_PIN="$(grep -A2 '^\[repos.lez\]' "$REPO_ROOT/scaffold.toml" | sed -n 's/^pin = "\(.*\)"/\1/p')"
export SCAFFOLD_LEZ_CACHE="${HOME}/.cache/logos-scaffold/repos/lez/${LEZ_PIN}"
export SCAFFOLD_WALLET="${SCAFFOLD_LEZ_CACHE}/target/release/wallet"
export MODULES="$REPO_ROOT/.scaffold/e2e/user/modules"
export WALLET_HOME="$REPO_ROOT/.scaffold/e2e/testnet-wallet"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
export WALLET_CONFIG="$WALLET_HOME/wallet_config.json"
export WALLET_STORAGE="$WALLET_HOME/storage.json"
export WALLET_PASSWORD="choose-a-local-password"
export PAYMENT_STREAMS_GUEST_BIN="$REPO_ROOT/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
export SEQUENCER_URL="https://testnet.lez.logos.co/"
export DEPOSIT=500
export ALLOCATION=80
export RATE=1
export MIN_ACCRUED=1
export VAULT_ID=0
export STREAM_ID=0
export PAYER=""
export PAYEE=""
```

## Step 2 — Sequencer up

```bash
curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}'
```

## Step 3 — Build guest ELF and check ImageID

`make build` uses Docker guest-builder `risczero/risc0-guest-builder:r0.1.88.0` and the pinned
`methods/guest/Cargo.lock`. Do not run `cargo update` under `methods/guest/` unless you intend to
change program identity.

```bash
cd "$REPO_ROOT"
make build
test -f "$PAYMENT_STREAMS_GUEST_BIN"
```

```bash
cd "$REPO_ROOT"
EXPECTED=$(grep -o '"program_id_hex": "[^"]*"' "$REPO_ROOT/fixtures/testnet-module.json" \
  | sed -n 's/.*"program_id_hex": "\([^"]*\)".*/\1/p')
BUILT=$(make program-id 2>/dev/null | sed -n 's/.*ImageID (hex bytes): //p' | tr -d '[:space:]')
if [[ -z "$BUILT" ]]; then
  echo "Could not read ImageID from make program-id (run make build first)." >&2
elif [[ "$BUILT" == "$EXPECTED" ]]; then
  echo "Program id matches testnet fixture."
else
  echo "Guest ImageID mismatch." >&2
  echo "  built:    $BUILT" >&2
  echo "  expected: $EXPECTED" >&2
fi
```

Stop if ImageID does not match the fixture.

## Step 4 — Scaffold and wallet CLI

```bash
cd "$REPO_ROOT"
lgs init
lgs setup
test -x "$SCAFFOLD_WALLET"
```

## Step 5 — Wallet config and module install

```bash
mkdir -p "$WALLET_HOME" "$MODULES"
cp "$REPO_ROOT/fixtures/testnet-wallet_config.example.json" "$WALLET_CONFIG"

cd "$REPO_ROOT"
PS_LGX_OUT=$(nix build ./logos-payment-streams-module#lgx-portable -L --no-link --print-out-paths | tail -1)
lgpm --modules-dir "$MODULES" install --file "$PS_LGX_OUT"/*.lgx --force

"$REPO_ROOT/scripts/archive/build-wallet-lgx.sh"
WALLET_LGX=$(readlink -f "$REPO_ROOT/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/wallet-lgx-out/"*.lgx)
lgpm --modules-dir "$MODULES" install --file "$WALLET_LGX" --force
```

## Step 6 — Start logoscore and open wallet

```bash
cd "$REPO_ROOT"
logoscore stop 2>/dev/null || true
logoscore -D -m "$MODULES" &
sleep 3
logoscore load-module logos_execution_zone
logoscore load-module payment_streams_module
if [[ ! -f "$WALLET_STORAGE" ]]; then
  logoscore call logos_execution_zone create_new "$WALLET_CONFIG" "$WALLET_STORAGE" "$WALLET_PASSWORD"
else
  logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"
fi
logoscore call logos_execution_zone save
```

Use the last line of each `logoscore call` for JSON `status` / `result`.

## Step 7 — Create payer and payee accounts

```bash
if [[ -z "$PAYER" ]]; then
  PAYER_HEX=$(logoscore call logos_execution_zone create_account_public | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  PAYER=$(logoscore call logos_execution_zone account_id_to_base58 "$PAYER_HEX" | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  export PAYER
fi
if [[ -z "$PAYEE" ]]; then
  PAYEE_HEX=$(logoscore call logos_execution_zone create_account_public | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  PAYEE=$(logoscore call logos_execution_zone account_id_to_base58 "$PAYEE_HEX" | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  export PAYEE
fi
logoscore call logos_execution_zone save
```

## Step 8 — Sync to chain

Define once; run after chain writes before trusting reads.

```bash
sync_to_chain() {
  local raw height
  raw=$(curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}')
  height=$(printf '%s' "$raw" | sed -n 's/.*"result":\([0-9][0-9]*\).*/\1/p')
  if [[ -z "$height" ]]; then
    height=$(printf '%s' "$raw" | sed -n 's/.*"result":"\([0-9][0-9]*\)".*/\1/p')
  fi
  if [[ -z "$height" ]]; then
    echo "sync_to_chain: could not parse getLastBlockId from sequencer" >&2
    return 1
  fi
  logoscore call logos_execution_zone sync_to_block "$height"
  sleep 3
}
```

When a step says sync to chain, run `sync_to_chain`.

## Step 9 — Authenticated transfer registration

Payer:

```bash
PAYER_HEX=$(logoscore call logos_execution_zone account_id_from_base58 "$PAYER" | tail -1 \
  | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
logoscore call logos_execution_zone register_public_account "$PAYER_HEX"
```

Sync to chain. Payee:

```bash
PAYEE_HEX=$(logoscore call logos_execution_zone account_id_from_base58 "$PAYEE" | tail -1 \
  | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
logoscore call logos_execution_zone register_public_account "$PAYEE_HEX"
```

Sync to chain.

## Step 10 — Fund accounts (pinata)

Close the in-process wallet before the standalone `wallet` binary claims pinata.

```bash
chain_balance() {
  curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$1\"]}" \
    | sed -n 's/.*"balance":\([0-9][0-9]*\).*/\1/p' | head -1
}

export PATH="$(dirname "$SCAFFOLD_WALLET"):$PATH"
PINATA_PER_CLAIM=150
PAYER_TARGET=$((DEPOSIT + 50))
PAYEE_TARGET=50
MAX_CLAIMS_PER_ACCOUNT=6

pb=$(chain_balance "$PAYER"); pb=${pb:-0}
pe=$(chain_balance "$PAYEE"); pe=${pe:-0}
payer_claims=0
payee_claims=0
if (( pb < PAYER_TARGET )); then
  payer_claims=$(( (PAYER_TARGET - pb + PINATA_PER_CLAIM - 1) / PINATA_PER_CLAIM ))
  (( payer_claims > MAX_CLAIMS_PER_ACCOUNT )) && payer_claims=$MAX_CLAIMS_PER_ACCOUNT
fi
if (( pe < PAYEE_TARGET )); then
  payee_claims=$(( (PAYEE_TARGET - pe + PINATA_PER_CLAIM - 1) / PINATA_PER_CLAIM ))
  (( payee_claims > MAX_CLAIMS_PER_ACCOUNT )) && payee_claims=$MAX_CLAIMS_PER_ACCOUNT
fi

logoscore call logos_execution_zone close
for (( i = 0; i < payer_claims; i++ )); do
  "$SCAFFOLD_WALLET" pinata claim --to "Public/$PAYER"
  sleep 2
done
for (( i = 0; i < payee_claims; i++ )); do
  "$SCAFFOLD_WALLET" pinata claim --to "Public/$PAYEE"
  sleep 2
done
logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"
logoscore call logos_execution_zone save

pb=$(chain_balance "$PAYER"); pb=${pb:-0}
pe=$(chain_balance "$PAYEE"); pe=${pe:-0}
echo "Payer balance $pb (target $PAYER_TARGET); payee balance $pe (target $PAYEE_TARGET)"
```

Sync to chain.

## Step 11 — Pick vault id

```bash
export VAULT_ID=0
while logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID}" | tail -1 | grep -q '"status":"ok"'; do
  VAULT_ID=$((VAULT_ID + 1))
done
export VAULT_ID
echo "Using vault_id=$VAULT_ID"
```

## Step 12 — Initialize vault

```bash
logoscore call payment_streams_module chainAction initializeVault \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID}"
```

Sync to chain, then:

```bash
logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID}"
```

## Step 13 — Deposit

```bash
logoscore call payment_streams_module chainAction deposit \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}"
```

Sync to chain, then:

```bash
logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID}"
```

## Step 14 — Create stream

```bash
logoscore call payment_streams_module chainAction createStream \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"provider\":\"$PAYEE\",\"rate\":$RATE,\"allocation_lo\":$ALLOCATION,\"allocation_hi\":0}"
```

Sync to chain, then:

```bash
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

## Step 15 — Wait for accrual

Wait at least `$MIN_ACCRUED` seconds, sync to chain, then:

```bash
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

## Step 16 — Close stream (payer)

Omit `authority` so the payer (`signer`) signs close.

```bash
logoscore call payment_streams_module chainAction closeStream \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

Sync to chain, then:

```bash
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

## Step 17 — Claim (payee)

```bash
logoscore call payment_streams_module chainAction claim \
  "{\"owner\":\"$PAYER\",\"provider\":\"$PAYEE\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

Sync to chain.

## Step 18 — Confirm

```bash
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
chain_balance "$PAYEE"
```

## If something fails

| Symptom | Try |
| --- | --- |
| `Source plugin file does not exist` on `lgpm install` | Step 5 uses `#lgx-portable` and `"$PS_LGX_OUT"/*.lgx`, not `#lgx` |
| `linux-x86_64-dev` / `linux-amd64-dev` variant mismatch | Run `./scripts/user-journey-shell.sh` (`logos-logoscore-cli/0.2.0` includes matching `lgpm`) |
| `missing wallet debug config in lez repo` after `lgs setup` | Update `lgs` from logos-co/scaffold, or copy `$SCAFFOLD_LEZ_CACHE/lez/wallet/configs/debug/wallet_config.json` to `$REPO_ROOT/.scaffold/wallet/wallet_config.json` and re-run Step 4 |
| Stale reads | `sync_to_chain`, poll again |
| Deposit rejected | Step 10 pinata for payer |
| Stream not Closed | Sync; Step 16 without `authority` |
| Empty claim | Step 15 until `accrued_lo` ≥ `MIN_ACCRUED` |
| AT errors | Step 9; sync |
| Pinata no effect | `LEE_WALLET_HOME_DIR` = `$WALLET_HOME`; close wallet before claims (Step 10) |

## Reference

- [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html)
- [Logos build-and-run](https://docs.logos.co/core/build-modules/build-and-run-a-logos-core-module)
- [payment-streams-module chainAction catalogue](../payment-streams-module/README.md#chainaction-catalogue)
