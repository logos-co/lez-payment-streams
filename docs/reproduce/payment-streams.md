# Reproduce payment streams

Manual teaching path for LIP-155 through `payment_streams_module` on a single host.
Testnet is the primary network.
Phases match `scripts/module-e2e.sh`: vault init, deposit, create stream, accrual, close, claim.

Paid Store queries live in [store-eligibility.md](store-eligibility.md).
`chainAction` catalogue: [payment-streams-module README](../payment-streams-module/README.md#chainaction-catalogue).
Program identity: root README [Public testnet guest program](../../README.md#public-testnet-guest-program) and [fixtures/testnet-module.json](../../fixtures/testnet-module.json).

Pause, resume, and top-up are listed in the catalogue.
This walkthrough covers the public happy path through close and claim.

## What you run

Fund a vault, open a stream to a provider at a fixed rate, wait for accrual, close the stream, then claim.
Both roles share one `logoscore` daemon and one wallet with two public accounts.

Toolchain helpers keep historical filenames: `scripts/user-journey-reset.sh`, `scripts/user-journey-shell.sh`, and `scripts/lib/user-journey-env.sh`.
Those remain the supported entry for this manual path (pinned logoscore and lgpm flake SHAs).

## Runtime

Public TestNet v0.2 at `https://testnet.lez.logos.co/`.

## Prerequisites

Host: Git, Nix (flakes), Docker, RISC Zero CLI (`cargo-risczero`, `rzup install`), Rust/cargo, network access to the sequencer.

```bash
git clone https://github.com/logos-co/lez-payment-streams.git
cd lez-payment-streams
chmod +x scripts/user-journey-*.sh
./scripts/user-journey-reset.sh
./scripts/user-journey-shell.sh
```

Inside the shell, run the steps below in order.
A new terminal needs Step 1 again.

`user-journey-shell.sh` installs `lgs` when missing, then opens a Nix shell with pinned `logoscore` and `lgpm` that load `linux-amd64-dev` modules.
Steps 4, 5, and 8 call `./scripts/user-journey-lgs-setup.sh`, `./scripts/user-journey-install-modules.sh`, and `./scripts/user-journey-auth-transfer.sh`.

Machine and Nix shell setup for automated verification: [README prerequisites](../../README.md#prerequisites).

## On-chain confirmation

A `chainAction` write returning `success` and a `tx_hash` means the wallet accepted the submit.
Confirm inclusion before the next step that reads that state.
Capture sequencer height `h0` before submit.
Wait until `last_block` is greater than `$h0`.
Then `sync_to_chain` and read `getVaultStatus` or `getStreamStatus`.
Proceed when the read shows the expected account.
If the change is missing after about a minute, re-run the write.

The same rule is stated for both orchestrators in the [verification matrix](../reference/verification-matrix.md#on-chain-confirmation-principle).

`sync_to_chain` pulls the wallet mirror to the sequencer tip.
Call it after each write, before a status read.
Skip it for `chain_balance` (sequencer `getAccount`).

## Step 1 — Session variables

```bash
export REPO_ROOT="$(pwd)"
export REPO="$REPO_ROOT"
export FIXTURE_MANIFEST="$REPO_ROOT/fixtures/testnet-module.json"
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
export OWNER=""
export PROVIDER=""
export LOGOSCORE_DAEMON_LOG="$REPO_ROOT/.scaffold/e2e/user-journey-logoscore-$(date -u +%Y-%m-%dT%H-%M-%SZ).log"
```

```bash
logoscore() { command logoscore -q "$@"; }
step_ok() { echo "Success: $*"; }
step_fail() { echo "Failed: $*" >&2; return 1; }
write_ok() {
  local label="$1" line="$2"
  if [[ -z "$line" ]] || echo "$line" | grep -q '"status":"error"'; then
    step_fail "$label"
    [[ -n "$line" ]] && echo "$line" >&2
    return 1
  fi
  step_ok "$label"
}
sync_to_chain() {
  local raw height
  raw=$(curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}')
  height=$(printf '%s' "$raw" | python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get("result"); print(r if isinstance(r,int) else (r or ""))' 2>/dev/null || true)
  if [[ -z "$height" ]]; then
    echo "sync_to_chain: could not parse getLastBlockId from sequencer" >&2
    return 1
  fi
  logoscore call logos_execution_zone sync_to_block "$height" >/dev/null
  sleep 3
}
chain_balance() {
  curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$1\"]}" \
    | sed -n 's/.*"balance":\([0-9][0-9]*\).*/\1/p' | head -1
}
last_block() {
  curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}' \
  | python3 -c 'import json,sys;print(json.load(sys.stdin)["result"])'
}
step_ok "Session variables and shell helpers ready"
```

`write_ok` checks the last line of a chain write for `"status":"error"`.
That is mempool acceptance.
The gate for the next step is the state read after `h0`.

Private execution: see [Private execution notes](#private-execution-notes).

## Step 2 — Sequencer up

```bash
curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}'
step_ok "Sequencer reachable"
```

## Step 3 — Build guest ELF and check ImageID

`make build` uses Docker guest-builder `risczero/risc0-guest-builder:r0.1.88.0` and the pinned `methods/guest/Cargo.lock`.

```bash
cd "$REPO_ROOT"
make build
test -f "$PAYMENT_STREAMS_GUEST_BIN"
step_ok "Guest ELF built"
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
  step_ok "Guest ELF built; program id matches testnet fixture"
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
./scripts/user-journey-lgs-setup.sh
export SCAFFOLD_WALLET="${SCAFFOLD_LEZ_CACHE}/target/release/wallet"
test -x "$SCAFFOLD_WALLET"
step_ok "Scaffold and standalone wallet CLI ready"
```

## Step 5 — Wallet config and module install

```bash
cd "$REPO_ROOT"
./scripts/user-journey-install-modules.sh
export WALLET_CONFIG="$WALLET_HOME/wallet_config.json"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
step_ok "Testnet wallet config and Logos modules installed"
```

## Step 6 — Start logoscore and open wallet

`payment_streams_module` reads `FIXTURE_MANIFEST` (Step 1) when the daemon starts.
If you change it later, run this step again.

```bash
cd "$REPO_ROOT"
mkdir -p "$(dirname "$LOGOSCORE_DAEMON_LOG")"
logoscore stop 2>/dev/null || true
logoscore -D -m "$MODULES" >>"$LOGOSCORE_DAEMON_LOG" 2>&1 &
ready=0
for (( i = 0; i < 20; i++ )); do
  if logoscore list-modules --loaded >/dev/null 2>&1; then ready=1; break; fi
  sleep 0.5
done
if (( ready )); then
  step_ok "logoscore daemon started (log: $LOGOSCORE_DAEMON_LOG)"
else
  step_fail "logoscore daemon not ready; check $LOGOSCORE_DAEMON_LOG"
fi
```

Only `capability_module` auto-loads.
Load wallet and payment streams, then open the wallet:

```bash
logoscore load-module logos_execution_zone
logoscore load-module payment_streams_module
if [[ ! -f "$WALLET_STORAGE" ]]; then
  logoscore call logos_execution_zone create_new "$WALLET_CONFIG" "$WALLET_STORAGE" "$WALLET_PASSWORD"
else
  logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"
fi
logoscore call logos_execution_zone save
step_ok "Modules loaded; wallet open (log: $LOGOSCORE_DAEMON_LOG)"
```

Use the last line of each `logoscore call` for JSON `status` / `result`.
Daemon log: `tail -f "$LOGOSCORE_DAEMON_LOG"`.

## Step 7 — Create owner and provider accounts

```bash
if ! logoscore list-modules --loaded 2>/dev/null | grep -q logos_execution_zone; then
  step_fail "logos_execution_zone not loaded; run the second block of Step 6 before Step 7"
else
if [[ -z "$OWNER" ]]; then
  OWNER_HEX=$(logoscore call logos_execution_zone create_account_public | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  OWNER=$(logoscore call logos_execution_zone account_id_to_base58 "$OWNER_HEX" | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  export OWNER
fi
if [[ -z "$PROVIDER" ]]; then
  PROVIDER_HEX=$(logoscore call logos_execution_zone create_account_public | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  PROVIDER=$(logoscore call logos_execution_zone account_id_to_base58 "$PROVIDER_HEX" | tail -1 \
    | sed -n 's/.*"result":"\([^"]*\)".*/\1/p')
  export PROVIDER
fi
logoscore call logos_execution_zone save
step_ok "Owner and provider public accounts ready (owner=$OWNER provider=$PROVIDER)"
fi
```

Private owner or provider: [Private execution notes](#private-execution-notes).

## Step 8 — Authenticated transfer registration

Same path as module E2E (`wallet auth-transfer init` with logoscore wallet handoff, then `register_public_account` if needed).
Needs `$OWNER` and `$PROVIDER` from Step 7.

```bash
cd "$REPO_ROOT"
./scripts/user-journey-auth-transfer.sh
sync_to_chain
step_ok "Authenticated transfer registered for owner and provider"
```

On success the script exits 0 and appends phases to `.scaffold/e2e/user-journey-at.jsonl`.

## Step 9 — Fund accounts (pinata)

Close the in-process wallet before the standalone `wallet` binary claims pinata.

```bash
export PATH="$(dirname "$SCAFFOLD_WALLET"):$PATH"
PINATA_PER_CLAIM=150
OWNER_TARGET=$((DEPOSIT + 50))
PROVIDER_TARGET=50
MAX_CLAIMS_PER_ACCOUNT=6

pb=$(chain_balance "$OWNER"); pb=${pb:-0}
pe=$(chain_balance "$PROVIDER"); pe=${pe:-0}
owner_claims=0
provider_claims=0
if (( pb < OWNER_TARGET )); then
  owner_claims=$(( (OWNER_TARGET - pb + PINATA_PER_CLAIM - 1) / PINATA_PER_CLAIM ))
  (( owner_claims > MAX_CLAIMS_PER_ACCOUNT )) && owner_claims=$MAX_CLAIMS_PER_ACCOUNT
fi
if (( pe < PROVIDER_TARGET )); then
  provider_claims=$(( (PROVIDER_TARGET - pe + PINATA_PER_CLAIM - 1) / PINATA_PER_CLAIM ))
  (( provider_claims > MAX_CLAIMS_PER_ACCOUNT )) && provider_claims=$MAX_CLAIMS_PER_ACCOUNT
fi

logoscore call logos_execution_zone close
for (( i = 0; i < owner_claims; i++ )); do
  "$SCAFFOLD_WALLET" pinata claim --to "Public/$OWNER"
  sleep 2
done
for (( i = 0; i < provider_claims; i++ )); do
  "$SCAFFOLD_WALLET" pinata claim --to "Public/$PROVIDER"
  sleep 2
done
logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"
logoscore call logos_execution_zone save

pb=$(chain_balance "$OWNER"); pb=${pb:-0}
pe=$(chain_balance "$PROVIDER"); pe=${pe:-0}
echo "Owner balance $pb (target $OWNER_TARGET); provider balance $pe (target $PROVIDER_TARGET)"
sync_to_chain
step_ok "Owner and provider funded on testnet (pinata)"
```

Private owner: pre-shield from the public funder into the private owner before vault init.
See [Private execution notes](#private-execution-notes).

## Step 10 — Initialize vault

Step 1 sets `VAULT_ID=0`.
After Step 7 you have a new owner, so vault 0 is usually free.
If `initializeVault` fails because that vault already exists for `$OWNER`, set `VAULT_ID` to the next free id and repeat.

```bash
h0=$(last_block)
line=$(logoscore call payment_streams_module chainAction initializeVault \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID}" | tail -1)
echo "$line"
write_ok "Vault created (vault_id=$VAULT_ID)" "$line"
echo "Submitted at chain height $h0"
```

Wait until `last_block` is greater than `$h0`, then:

```bash
sync_to_chain
logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID}"
```

Proceed to Step 11 when the read returns `vault_config`.
If the change is missing after about a minute, re-run this step.

Private vault: pass `"privacy_tier":1` and the private owner id.
See [Private execution notes](#private-execution-notes).

## Step 11 — Deposit

```bash
h0=$(last_block)
line=$(logoscore call payment_streams_module chainAction deposit \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}" | tail -1)
echo "$line"
write_ok "Vault funded ($DEPOSIT tokens, vault_id=$VAULT_ID)" "$line"
echo "Submitted at chain height $h0"
```

Wait until `last_block` is greater than `$h0`, then:

```bash
sync_to_chain
logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID}"
```

Proceed to Step 12 when the read shows the funded `vault_holding_balance`.

## Step 12 — Create stream

```bash
h0=$(last_block)
line=$(logoscore call payment_streams_module chainAction createStream \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"provider\":\"$PROVIDER\",\"rate\":$RATE,\"allocation_lo\":$ALLOCATION,\"allocation_hi\":0}" | tail -1)
echo "$line"
write_ok "Payment stream created (stream_id=$STREAM_ID, provider=$PROVIDER)" "$line"
echo "Submitted at chain height $h0"
```

Wait until `last_block` is greater than `$h0`, then:

```bash
sync_to_chain
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

Proceed to Step 13 when the read shows the stream.

## Step 13 — Wait for accrual

Wait about 30 seconds, then:

```bash
sync_to_chain
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
step_ok "Accrual window elapsed; check accrued_lo in JSON above (need ≥ $MIN_ACCRUED before close)"
```

## Step 14 — Close stream (owner)

Omit `provider`, or set it equal to `owner`, for owner-close.

```bash
h0=$(last_block)
line=$(logoscore call payment_streams_module chainAction closeStream \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}" | tail -1)
echo "$line"
write_ok "Stream closed by owner (vault_id=$VAULT_ID stream_id=$STREAM_ID)" "$line"
echo "Submitted at chain height $h0"
```

Provider-close uses the same `closeStream` action with a distinct `provider`.
Pass `"provider":"$PROVIDER"` so the module selects the six-slot provider instruction.

Wait until `last_block` is greater than `$h0`, then:

```bash
sync_to_chain
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

Proceed to Step 15 when `stream_state` is `2` (Closed).

## Step 15 — Claim (provider)

```bash
h0=$(last_block)
line=$(logoscore call payment_streams_module chainAction claim \
  "{\"owner\":\"$OWNER\",\"provider\":\"$PROVIDER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}" | tail -1)
echo "$line"
write_ok "Provider claimed accrued tokens" "$line"
echo "Submitted at chain height $h0"
```

`claim` is signed by the provider.
Wait until `last_block` is greater than `$h0`, then:

```bash
sync_to_chain
provider_bal=$(chain_balance "$PROVIDER"); provider_bal=${provider_bal:-0}
echo "Provider on-chain balance: $provider_bal"
```

Proceed to Step 16 when the provider balance reflects the payout.
Private provider: confirm via `vault_holding` drop.
See [Private execution notes](#private-execution-notes).

## Step 16 — Confirm

```bash
sync_to_chain
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$OWNER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
provider_bal=$(chain_balance "$PROVIDER"); provider_bal=${provider_bal:-0}
echo "Provider on-chain balance: $provider_bal"
step_ok "Payment stream walkthrough complete (provider balance $provider_bal)"
```

## Step 17 — Shut down

```bash
logoscore call logos_execution_zone close 2>/dev/null || true
logoscore stop
step_ok "logoscore stopped; exit the toolchain shell when ready"
exit
```

Wallet files remain under `$WALLET_HOME` unless you run `./scripts/user-journey-reset.sh` before the next walkthrough.

## Expected result

Each step prints a `Success: ...` line.
After close, `stream_state` is `2` (Closed).
After claim, accrued tokens move to the provider.
Step 16 prints the provider on-chain balance.

Sizing SSOT: `demo_deposit_amount` 500, `allocation` 80, `stream_rate` 1 in [fixtures/testnet-module.json](../../fixtures/testnet-module.json).
Deposit must cover allocation.
Leftover deposit stays in the vault after close.

Automated equivalent: `MODE=module ./scripts/e2e.sh testnet run` after `make bootstrap-testnet-module`.
See [store-eligibility.md](store-eligibility.md) for Store recipes and the [verification matrix](../reference/verification-matrix.md) for flags.

## Glossary

| Term | JSON / wire | Meaning |
| --- | --- | --- |
| Owner | `owner` on writes, reads, and `claim` | Vault owner. Closes the stream in this flow. |
| Provider | `provider` on `createStream` and `claim` | Recipient. Claims after close. |
| Vault | `vault_id` | Holds deposits and allocations. |
| Stream | `stream_id` | Pays provider at `rate` up to `allocation`. |
| `*_lo` / `*_hi` | writes and `getStreamStatus` | 128-bit amount as two uint64s: `lo + (hi << 64)`. Values here fit in `*_lo` with `*_hi` = 0. |
| `accrued_*`, `unaccrued_*` | `getStreamStatus` | Claimable vs remaining allocation. |
| `stream_state` | 0 Active, 1 Paused, 2 Closed | |
| `MIN_ACCRUED` | shell only | Minimum `accrued_lo` before close. Token units. |
| Authenticated transfer (AT) | `wallet auth-transfer init` / `register_public_account` | Lets public accounts spend tokens. Required before deposit and stream writes. |

## Failure modes

| Symptom | Try |
| --- | --- |
| Verbose `[logos_execution_zone]` lines | Re-run Step 1 (`logoscore()` wrapper) and Step 6 |
| `cannot open fixture manifest: fixtures/localnet.json` | `export FIXTURE_MANIFEST="$REPO_ROOT/fixtures/testnet-module.json"` and re-run Step 6 |
| Module variant / `load-module` failed | `./scripts/user-journey-reset.sh`, re-enter `./scripts/user-journey-shell.sh`, Step 5 |
| `Run this from the toolchain shell` | `./scripts/user-journey-shell.sh` before Step 5 |
| `missing wallet debug config in lez repo` | `./scripts/user-journey-lgs-setup.sh` |
| Empty `OWNER` / `PROVIDER` | Run the second block of Step 6, then Step 7 |
| `account data missing` after a write | Wait until `last_block` is past `$h0`, then re-read. After about a minute, re-run the write. |
| `initializeVault` fails for vault 0 | `export VAULT_ID=1` and retry Step 10, or reset and new accounts in Step 7 |
| Deposit rejected | Step 9 pinata for owner |
| Stream still Active | `sync_to_chain` and redo Step 14 |
| Empty claim | Step 13 until `accrued_lo` ≥ `MIN_ACCRUED` |
| AT errors | Step 8. Check `.scaffold/e2e/user-journey-at.jsonl` |
| Pinata no effect | `LEE_WALLET_HOME_DIR` = `$WALLET_HOME`. Close wallet before claims (Step 9). |

Localnet recovery: [archive/operator/localnet-recovery.md](../archive/operator/localnet-recovery.md).

## Private execution notes

Owner privacy (`OWNER_PRIVACY=1`) and provider privacy (`PROVIDER_PRIVACY=1`) are independent.
`PRIVACY=1` is an alias for `OWNER_PRIVACY=1`.
Automated module check: `MODE=module OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./scripts/e2e.sh local run`.
Local private submits default `RISC0_DEV_MODE=1` (stub receipts).
Public testnet proving uses `RISC0_DEV_MODE=0`.

Amounts on `vault_holding` stay public.
Identities can be shielded.

### Owner (`PseudonymousFunding`)

1. Create a private account: `logoscore call logos_execution_zone create_account_private`.
   Keys: `get_private_account_keys <private_account_id>`.
   NSK stays in the wallet.
2. Pre-shield from a public funder into that private owner.
   `amount_le16_hex` is 32 lowercase hex chars (u128 little-endian).
   Prefix with `s:` so logoscore keeps the value as a string.
   Quote the amount.

```bash
logoscore call logos_execution_zone transfer_shielded_owned \
  <public_account_id_hex> <vault_owner_private_account_id_hex> "s:<amount_le16_hex>"
```

PP `deposit` debits the vault owner private account.
Funds must already be there.
3. Initialize with `"privacy_tier":1` and `"owner"` set to the private account id.
4. Deposit, create, pause, resume, top-up, and owner-close submit privately when the vault is `PseudonymousFunding`.
5. Eligibility `VaultProof.owner_signature` is signed with the vault owner NSK (`sign_private_payload`).

### Provider (shielded claim)

1. Provider creates a private account and shares the NPK-derived id out of band.
   Reuse one `(npk, identifier)` for create and claim.
2. `createStream` JSON `"provider"` is that private id.
   On a public vault, create stays a public submit (`provider` is instruction data).
3. `claim` with that `"provider"` submits privately.
   Confirm via `vault_holding` drop.
   E2E dust-shields a small amount into the private provider first so a committed note exists.

### Limits

Deposit and claim amounts are visible on the public holding PDA.
The stream creator knows `provider_id`.
Amount and timing across the shielding boundary are side channels.

## Specs

- [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html)
- [chainAction catalogue](../payment-streams-module/README.md#chainaction-catalogue)
