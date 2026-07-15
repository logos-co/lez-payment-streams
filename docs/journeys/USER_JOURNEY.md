# User Journey — payment streams on TestNet v0.2

CLI on public testnet: vault, deposit, stream to a payee, payer closes, payee claims.
Uses `logos_execution_zone` and `payment_streams_module`.

One `logoscore` daemon and one wallet file hold both payer and payee public accounts — you
simulate two parties on one machine. Pause, resume, and top-up are out of scope here (see module
catalogue).

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
`16b95d3701d256eecd41d5a55e4f570331994d787abf0cba44eec209e24f8a44`, deposit 500 (must cover
`allocation` 80; leftover deposit stays in the vault after close), rate 1 token per second of chain
clock time, `MIN_ACCRUED` 1, vault/stream ids start at 0.

## Step 1 — Session variables

Set `REPO_ROOT` to your lez-payment-streams checkout.

Logos Scaffold pins logos-execution-zone in `scaffold.toml` (`[repos.lez]` → `pin` = git
commit). `lgs setup` (Step 5) builds that commit under
`~/.cache/logos-scaffold/repos/lez/<pin>/`. Set `LEZ_PIN` to that pin so Step 6 and Step 13
can use `$SCAFFOLD_WALLET`. Wallet files for this walkthrough live under
`$REPO_ROOT/.scaffold/e2e/`. Tools read `LEE_WALLET_HOME_DIR` for wallet data; this journey sets
it equal to `WALLET_HOME`.

```bash
export REPO_ROOT="/path/to/lez-payment-streams"
```

Print the pinned commit (one line):

```bash
grep -A2 '^\[repos.lez\]' "$REPO_ROOT/scaffold.toml" | sed -n 's/^pin = "\(.*\)"/\1/p'
```

Export the rest of the session. Paste the commit into `LEZ_PIN`.

```bash
export LEZ_PIN="paste-the-commit-from-above"
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

## Step 3 — Guest ImageID matches deployed program

```bash
cd "$REPO_ROOT"
make program-id | grep -F '16b95d3701d256eecd41d5a55e4f570331994d787abf0cba44eec209e24f8a44' \
  || { echo "Guest ImageID does not match deployed program id; rebuild the guest ELF (Step 7)." >&2; exit 1; }
```

## Step 4 — Tooling on PATH

```bash
lgs --version
logoscore --version
lgpm --version
```

## Step 5 — LEZ scaffold and wallet CLI

Builds the LEZ commit pinned in `scaffold.toml` (same `LEZ_PIN` as Step 1). Journey
wallet data stays under `$REPO_ROOT/.scaffold/e2e/`.

```bash
cd "$REPO_ROOT"
lgs init
lgs setup
```

## Step 6 — LEZ wallet binary

```bash
test -x "$SCAFFOLD_WALLET"
```

## Step 7 — Guest ELF present

```bash
test -f "$PAYMENT_STREAMS_GUEST_BIN"
```

## Step 8 — Wallet config and module install

Copy the testnet wallet config template (edit `sequencer_addr` only if you use another endpoint).
Install two bundles: `payment_streams_module` (LIP-155 chain writes) and a patched
`logos_execution_zone` wallet module from this repo’s flake (testnet authenticated-transfer support).

```bash
mkdir -p "$WALLET_HOME" "$MODULES"
cp "$REPO_ROOT/fixtures/testnet-wallet_config.example.json" "$WALLET_CONFIG"

cd "$REPO_ROOT"
nix build ./logos-payment-streams-module#lgx -o "$MODULES/payment_streams.lgx"
lgpm --modules-dir "$MODULES" install --file "$(readlink -f "$MODULES/payment_streams.lgx")"

WALLET_FLAKE="$REPO_ROOT/logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched"
(cd "$WALLET_FLAKE" && nix bundle --impure --bundler github:logos-co/nix-bundle-lgx#portable .#lib -o wallet-lgx-out -L)
lgpm --modules-dir "$MODULES" install --file "$(readlink -f "$WALLET_FLAKE/wallet-lgx-out"/*.lgx)"
```

## Step 9 — Start logoscore and open wallet

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

`logoscore call` prints JSON on the last line (`status`, `result`). Use the `result` field in
the steps below. If `load-module` fails, ensure no other `logoscore` daemon is using
`$MODULES`.

## Step 10 — Create payer and payee accounts

Set payer (skipped when `PAYER` is already set):

```bash
if [[ -z "$PAYER" ]]; then
  PAYER_HEX=$(logoscore call logos_execution_zone create_account_public | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  PAYER=$(logoscore call logos_execution_zone account_id_to_base58 "$PAYER_HEX" | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  export PAYER
fi
```

Set payee (skip when `PAYEE` is already set):

```bash
if [[ -z "$PAYEE" ]]; then
  PAYEE_HEX=$(logoscore call logos_execution_zone create_account_public | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  PAYEE=$(logoscore call logos_execution_zone account_id_to_base58 "$PAYEE_HEX" | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  export PAYEE
fi
```

Save:

```bash
logoscore call logos_execution_zone save
```

## Step 11 — Sync to chain

After every write in Steps 12–21, run this once before trusting reads (repeat if values look
stale):

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

When a step says sync to chain — Step 11, run `sync_to_chain`.

## Step 12 — Authenticated transfer registration

Payer:

```bash
PAYER_HEX=$(logoscore call logos_execution_zone account_id_from_base58 "$PAYER" | tail -1 \
  | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
logoscore call logos_execution_zone register_public_account "$PAYER_HEX"
```

Sync to chain — Step 11. Payee:

```bash
PAYEE_HEX=$(logoscore call logos_execution_zone account_id_from_base58 "$PAYEE" | tail -1 \
  | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
logoscore call logos_execution_zone register_public_account "$PAYEE_HEX"
```

Sync to chain — Step 11.

## Step 13 — Fund accounts (pinata)

Pinata pays testnet tokens per claim (~150 each, rate-limited). Only the standalone scaffold
`wallet` binary can claim; release the in-process wallet lock first so both tools do not open
`storage.json` at once.

Sequencer balance for a base58 public account (no wallet sync required):

```bash
chain_balance() {
  curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$1\"]}" \
    | sed -n 's/.*"balance":\([0-9][0-9]*\).*/\1/p' | head -1
}
```

Fund with enough pinata claims for the targets (~150 tokens per claim), then check balances on
the sequencer:

```bash
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

If either balance is still below target (rate limit or a short claim), wait a minute and re-run
Step 13 — only the claim loops run again; balances already at target need no more claims.

Sync to chain — Step 11.

## Step 14 — Pick vault id

Use the lowest `vault_id` that is not initialized yet. On a fresh wallet that is usually `0`.
When the inner `result` contains `"status":"ok"`, that vault already exists — try the next id.

```bash
export VAULT_ID=0
while logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID}" | tail -1 | grep -q '"status":"ok"'; do
  VAULT_ID=$((VAULT_ID + 1))
done
export VAULT_ID
echo "Using vault_id=$VAULT_ID"
```

## Step 15 — Initialize vault

```bash
logoscore call payment_streams_module chainAction initializeVault \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID}"
```

Sync to chain — Step 11, then read vault status:

```bash
logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID}"
```

## Step 16 — Deposit

```bash
logoscore call payment_streams_module chainAction deposit \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}"
```

Sync to chain — Step 11, then:

```bash
logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID}"
```

## Step 17 — Create stream

```bash
logoscore call payment_streams_module chainAction createStream \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"provider\":\"$PAYEE\",\"rate\":$RATE,\"allocation_lo\":$ALLOCATION,\"allocation_hi\":0}"
```

Sync to chain — Step 11, then:

```bash
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

In production the payee runs `getStreamStatus` and `claim` from their own wallet using payer
account id (`owner`), `vault_id`, and `stream_id` (out of band).

## Step 18 — Wait for accrual

While the stream is active, `accrued_lo` grows on chain at `RATE` tokens per second of chain
clock time (default `RATE` 1 → one token per second). The payee’s wallet balance changes
only after Step 20 (`claim`); until then only `accrued_lo` increases.

Wait at least `MIN_ACCRUED` seconds (plus a sync — Step 11) so `accrued_lo` reaches
`MIN_ACCRUED`. Longer waits accrue more up to `allocation`. Then check status:

```bash
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

Confirm `accrued_lo` ≥ `MIN_ACCRUED` before Step 19. The last line’s inner `result` is JSON; look
for `"accrued_lo":1` (or higher), `"stream_state":0`, and `"unaccrued_lo"` (allocation not yet
accrued).

If the read looks stale or too low, wait longer, sync again, and re-run the command.

## Step 19 — Close stream (payer)

`authority` optionally delegates who signs close; omit it so the payer (`signer`) signs. Do not
set `authority` to the payee. Close moves `unaccrued_lo` back to the vault; `accrued_lo` stays
claimable in Step 20.

```bash
logoscore call payment_streams_module chainAction closeStream \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

Sync to chain — Step 11, then:

```bash
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

Expect `"stream_state":2` (Closed) in the inner `result`.

## Step 20 — Claim (payee)

Pays `accrued_lo` from the stream to the payee (`provider`). Unaccrued allocation already
returned to the vault at close.

```bash
logoscore call payment_streams_module chainAction claim \
  "{\"owner\":\"$PAYER\",\"provider\":\"$PAYEE\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

Sync to chain — Step 11.

## Step 21 — Confirm success

```bash
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
chain_balance "$PAYEE"
```

After sync, the stream read should still show `"stream_state":2` and `accrued_lo` settled (0 after
claim). Payee sequencer balance should reflect the claimed tokens (minus gas).

## If something fails


| Symptom           | Try                                                       |
| ----------------- | --------------------------------------------------------- |
| Stale reads       | Sync to chain — Step 11, poll again                       |
| Deposit rejected  | Pinata fund payer; redo step 13                           |
| Stream not Closed | Sync to chain — Step 11; redo step 19 without `authority` |
| Empty claim       | Wait longer in step 18 until `accrued_lo` ≥ `MIN_ACCRUED` |
| AT errors         | Redo authenticated transfer (step 12); sync — Step 11     |


## Reference

- [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html)
- [Logos build-and-run](https://docs.logos.co/core/build-modules/build-and-run-a-logos-core-module)
- [payment-streams-module chainAction catalogue](../payment-streams-module/README.md#chainaction-catalogue)

