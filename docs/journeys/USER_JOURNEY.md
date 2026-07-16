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
| Authenticated transfer (AT) | `wallet auth-transfer init` / `register_public_account` | Lets public accounts spend tokens; required before deposit and stream writes                  |

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
chmod +x scripts/user-journey-*.sh
./scripts/user-journey-reset.sh
./scripts/user-journey-shell.sh
```

Inside the shell, run Steps 1–18 in [USER_JOURNEY.md](USER_JOURNEY.md). If you open a new terminal, re-run Step 1 (exports and shell helper functions) before continuing.

`user-journey-shell.sh` installs `lgs` when missing (with a LEZ v0.2 wallet-config patch if upstream scaffold needs it), then opens a Nix shell with pinned `logoscore` and `lgpm` that load `linux-amd64-dev` modules. Steps 4, 5, and 9 call `./scripts/user-journey-lgs-setup.sh`, `./scripts/user-journey-install-modules.sh`, and `./scripts/user-journey-auth-transfer.sh`.

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
export PAYER=""
export PAYEE=""
export LOGOSCORE_DAEMON_LOG="$REPO_ROOT/.scaffold/e2e/user-journey-logoscore-$(date -u +%Y-%m-%dT%H-%M-%SZ).log"
```

Shell helpers used by later steps, one block per function:

```bash
logoscore() { command logoscore -q "$@"; }
```

```bash
journey_ok() { echo "Success: $*"; }
```

```bash
journey_fail() { echo "Failed: $*" >&2; return 1; }
```

```bash
journey_write_ok() {
  local label="$1" line="$2"
  if [[ -z "$line" ]] || echo "$line" | grep -q '"status":"error"'; then
    journey_fail "$label"
    [[ -n "$line" ]] && echo "$line" >&2
    return 1
  fi
  journey_ok "$label"
}
```

```bash
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
```

```bash
chain_balance() {
  curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccount\",\"params\":[\"$1\"]}" \
    | sed -n 's/.*"balance":\([0-9][0-9]*\).*/\1/p' | head -1
}
```

```bash
journey_ok "Session variables and shell helpers ready"
```

`logoscore()` wraps CLI calls with `-q`. `journey_ok` / `journey_fail` print step status;
`journey_write_ok` checks the last line of a chain write for `"status":"error"`. `sync_to_chain`
pulls the wallet mirror up to the sequencer tip (`getLastBlockId` → `sync_to_block`); see Step 8
for when to call it. `chain_balance` reads a sequencer `getAccount` balance (no sync needed). Step 6
redirects the background daemon to `$LOGOSCORE_DAEMON_LOG` (new filename each time you run Step 1,
UTC ISO timestamp in the name).

## Step 2 — Sequencer up

```bash
curl -sf -X POST "$SEQUENCER_URL" -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLastBlockId","params":[]}'
journey_ok "Sequencer reachable"
```

## Step 3 — Build guest ELF and check ImageID

`make build` uses Docker guest-builder `risczero/risc0-guest-builder:r0.1.88.0` and the pinned
`methods/guest/Cargo.lock`. Do not run `cargo update` under `methods/guest/` unless you intend to
change program identity.

```bash
cd "$REPO_ROOT"
make build
test -f "$PAYMENT_STREAMS_GUEST_BIN"
journey_ok "Guest ELF built"
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
  journey_ok "Guest ELF built; program id matches testnet fixture"
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
journey_ok "Scaffold and standalone wallet CLI ready"
```

## Step 5 — Wallet config and module install

```bash
cd "$REPO_ROOT"
./scripts/user-journey-install-modules.sh
export WALLET_CONFIG="$WALLET_HOME/wallet_config.json"
export LEE_WALLET_HOME_DIR="$WALLET_HOME"
journey_ok "Testnet wallet config and Logos modules installed"
```

## Step 6 — Start logoscore and open wallet

`payment_streams_module` needs `FIXTURE_MANIFEST` (Step 1) in the environment when the daemon
starts. If you add or change it later, run this step again.

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
  journey_ok "logoscore daemon started (log: $LOGOSCORE_DAEMON_LOG)"
else
  journey_fail "logoscore daemon not ready; check $LOGOSCORE_DAEMON_LOG"
fi
```

If the line above says Failed, inspect `$LOGOSCORE_DAEMON_LOG` before continuing. When the daemon is
ready, load modules and open the wallet:

```bash
logoscore load-module logos_execution_zone
logoscore load-module payment_streams_module
if [[ ! -f "$WALLET_STORAGE" ]]; then
  logoscore call logos_execution_zone create_new "$WALLET_CONFIG" "$WALLET_STORAGE" "$WALLET_PASSWORD"
else
  logoscore call logos_execution_zone open "$WALLET_CONFIG" "$WALLET_STORAGE"
fi
logoscore call logos_execution_zone save
journey_ok "Modules loaded; wallet open (log: $LOGOSCORE_DAEMON_LOG)"
```

Use the last line of each `logoscore call` for JSON `status` / `result`. For daemon debug logs:
`tail -f "$LOGOSCORE_DAEMON_LOG"`.

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
journey_ok "Payer and payee public accounts ready (payer=$PAYER payee=$PAYEE)"
```

## Step 8 — Sync to chain

`sync_to_chain` (defined in Step 1) pulls the wallet mirror up to the sequencer tip
(`getLastBlockId` → `sync_to_block`). Call it when the doc shows it on its own line.

When to call it:

- After each `chainAction` write, before `getVaultStatus` / `getStreamStatus` in the same step.
- Before a module status read that opens a step (Step 14).
- Not on a timer while idle; not for `chain_balance` (sequencer `getAccount`).

Pausing between steps is fine. The chain moving ahead does not break the walkthrough; the next
`sync_to_chain` always catches up to the current tip.

Status reads use the wallet mirror (`logos_execution_zone.get_account_public` on derived vault or
stream accounts). If that account is not in the mirror yet, the module returns
`account data missing` even when the write already returned a `tx_hash`. Sync again and retry the
read, or continue when the write succeeded.

`journey_write_ok` checks the last line of a chain write for `"status":"error"` and prints
Success or Failed. `sync_to_block` stdout is discarded so you do not see a bare `0`.

## Step 9 — Authenticated transfer registration

Uses the same path as module E2E (`wallet auth-transfer init` with logoscore wallet handoff, then
`register_public_account` if needed). Requires `$PAYER` and `$PAYEE` from Step 7.

```bash
cd "$REPO_ROOT"
./scripts/user-journey-auth-transfer.sh
sync_to_chain
journey_ok "Authenticated transfer registered for payer and payee"
```

On success the script exits 0 and appends phases to `.scaffold/e2e/user-journey-at.jsonl`. If it
fails, inspect that file and confirm `$SCAFFOLD_WALLET` exists (Step 4).

## Step 10 — Fund accounts (pinata)

Close the in-process wallet before the standalone `wallet` binary claims pinata.

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
sync_to_chain
journey_ok "Payer and payee funded on testnet (pinata)"
```

## Step 11 — Initialize vault

Step 1 sets `VAULT_ID=0`. After Step 7 you have a new payer, so vault 0 is usually free. If
`initializeVault` fails because that vault already exists for `$PAYER`, run
`export VAULT_ID=1` (or the next free id) and repeat this step.

Initialize the vault, then optionally read it back. The read may show `account data missing`
until the mirror catches up; continue if `initializeVault` returned success:

```bash
line=$(logoscore call payment_streams_module chainAction initializeVault \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID}" | tail -1)
echo "$line"
journey_write_ok "Vault created (vault_id=$VAULT_ID)" "$line"
sync_to_chain
logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID}"
```

## Step 12 — Deposit

Deposit into the vault, then optionally read it back:

```bash
line=$(logoscore call payment_streams_module chainAction deposit \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"amount_lo\":$DEPOSIT,\"amount_hi\":0}" | tail -1)
echo "$line"
journey_write_ok "Vault funded ($DEPOSIT tokens, vault_id=$VAULT_ID)" "$line"
sync_to_chain
logoscore call payment_streams_module chainAction getVaultStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID}"
```

## Step 13 — Create stream

Create the stream, then optionally read it back:

```bash
line=$(logoscore call payment_streams_module chainAction createStream \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID,\"provider\":\"$PAYEE\",\"rate\":$RATE,\"allocation_lo\":$ALLOCATION,\"allocation_hi\":0}" | tail -1)
echo "$line"
journey_write_ok "Payment stream created (stream_id=$STREAM_ID, payee=$PAYEE)" "$line"
sync_to_chain
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

## Step 14 — Wait for accrual

Wait approximately 30 seconds for funds to accrue, then:

```bash
sync_to_chain
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
journey_ok "Accrual window elapsed; check accrued_lo in JSON above (need ≥ $MIN_ACCRUED before close)"
```

## Step 15 — Close stream (payer)

```bash
line=$(logoscore call payment_streams_module chainAction closeStream \
  "{\"signer\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}" | tail -1)
echo "$line"
journey_write_ok "Stream closed by payer (vault_id=$VAULT_ID stream_id=$STREAM_ID)" "$line"
sync_to_chain
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
```

## Step 16 — Claim (payee)

```bash
line=$(logoscore call payment_streams_module chainAction claim \
  "{\"owner\":\"$PAYER\",\"provider\":\"$PAYEE\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}" | tail -1)
echo "$line"
journey_write_ok "Payee claimed accrued tokens" "$line"
sync_to_chain
```

## Step 17 — Confirm

Uses `chain_balance` from Step 1 (sequencer `getAccount`, no `sync_to_chain` needed).

```bash
sync_to_chain
logoscore call payment_streams_module chainAction getStreamStatus \
  "{\"owner\":\"$PAYER\",\"vault_id\":$VAULT_ID,\"stream_id\":$STREAM_ID}"
payee_bal=$(chain_balance "$PAYEE"); payee_bal=${payee_bal:-0}
echo "Payee on-chain balance: $payee_bal"
journey_ok "Payment stream walkthrough complete (payee balance $payee_bal)"
```

## Step 18 — Shut down

```bash
logoscore call logos_execution_zone close 2>/dev/null || true
logoscore stop
journey_ok "logoscore stopped; exit the journey shell when ready"
exit
```

`logoscore stop` stops the daemon started in Step 6. `exit` leaves the Nix journey shell. Wallet files remain under `$WALLET_HOME` unless you run `./scripts/user-journey-reset.sh` before the next walkthrough.

## If something fails

| Symptom | Try |
| --- | --- |
| Verbose `[logos_execution_zone]` lines | Re-run Step 1 (`logoscore()` wrapper) and Step 6; or `command logoscore -v …` to debug |
| `cannot open fixture manifest: fixtures/localnet.json` | `export FIXTURE_MANIFEST="$REPO_ROOT/fixtures/testnet-module.json"` and re-run Step 6; journey shell sets this automatically |
| Module variant / `load-module` failed | `./scripts/user-journey-reset.sh`, re-enter `./scripts/user-journey-shell.sh`, Step 5 `./scripts/user-journey-install-modules.sh` |
| `Run this from the journey toolchain shell` | `./scripts/user-journey-shell.sh` before Step 5 |
| `missing wallet debug config in lez repo` | `./scripts/user-journey-lgs-setup.sh` (fallback copy built in) |
| Stale reads / `account data missing` after a successful write | Wallet mirror lag; `sync_to_chain`, retry read, or continue if write returned success (Step 8) |
| Paused between steps | Run the next step as written; first command is often `sync_to_chain` before a read |
| `initializeVault` fails for vault 0 | Reuse of `$PAYER` from an earlier run: `export VAULT_ID=1` and retry Step 11, or `./scripts/user-journey-reset.sh` and new accounts in Step 7 |
| Deposit rejected | Step 10 pinata for payer |
| Stream not Closed | Run `sync_to_chain`; redo Step 15 |
| Empty claim | Step 14 until `accrued_lo` ≥ `MIN_ACCRUED` |
| AT errors | Step 9 `./scripts/user-journey-auth-transfer.sh`; check `.scaffold/e2e/user-journey-at.jsonl` |
| Pinata no effect | `LEE_WALLET_HOME_DIR` = `$WALLET_HOME`; close wallet before claims (Step 10) |

## Reference

- [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html)
- [Logos build-and-run](https://docs.logos.co/core/build-modules/build-and-run-a-logos-core-module)
- [payment-streams-module chainAction catalogue](../payment-streams-module/README.md#chainaction-catalogue)
