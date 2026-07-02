# Step 32 — step 0 validation (O1, O2)

Date: 2026-07-03  
Sequencer: `https://testnet.lez.logos.co/`  
LEZ pin: `scaffold.toml` → `ps_lez_pin` / `LEZ_OP_REV=a58fbce2ff48c58b7bb5001b1a27e64b9596ee3a`  
Wallet: `~/.cache/logos-scaffold/repos/lez/<pin>/target/release/wallet`

## O1 — `getAccount.program_owner` vs authenticated_transfer

### RPC shape

`getAccount` returns `program_owner` as a JSON array of **eight unsigned 32-bit
integers** (little-endian limbs of the 32-byte program identifier).

Example (owner `DkT97NZPog2attaFURoMUFU5m996QtDnsS53PSnx41oG`, AT-initialized):

```json
"program_owner": [
  3170810844, 2526647253, 999807262, 1205602179,
  3401962591, 3484055895, 2106546407, 1900691388
]
```

Normalize to hex (Python reference):

```python
import struct
limbs = [3170810844, 2526647253, 999807262, 1205602179,
         3401962591, 3484055895, 2106546407, 1900691388]
hex_id = "".join(f"{struct.pack('<I', x).hex()}" for x in limbs)
# dcbbfebcd59399961ed9973b8307dc475fd4c5ca5779aacfe7588f7dbc3f4a71
```

### Compare to AT program

```bash
AT="$HOME/.cargo/git/checkouts/logos-execution-zone-*/a58fbce/artifacts/lez/programs/authenticated_transfer.bin"
spel inspect "$AT"
```

`ImageID (hex bytes)` from spel equals the normalized `program_owner` hex above.
The decimal array matches spel `ProgramId (decimal)`.

### False positive on non-zero check

Vault holding account `55oaHXvNxkDvVtQqZsKNmmTwgyMkqJpwZVtRYPjReNiL` (guest-owned)
also has eight non-zero limbs; normalized hex is the **payment-streams** program
ImageID (`16b95d3701d256eecd41d5a55e4f570331994d787abf0cba44eec209e24f8a44`), not
AT. Confirms Step 32 must compare to AT ImageID, not `any(int(x)!=0 for x in po)`.

### AT ELF path note

Under pin `a58fbce…`, `artifacts/program_methods/authenticated_transfer.bin` may
be absent; use `artifacts/lez/programs/authenticated_transfer.bin` or run
`lgs setup` to populate cache paths documented in Step 32 item 1.

## O2 — LEE vs NSSA for pinned `wallet`

| Env | Result |
| --- | --- |
| `LEE_WALLET_HOME_DIR=.scaffold/e2e/testnet-wallet` | Works; loads testnet wallet storage |
| `NSSA_WALLET_HOME_DIR` only (LEE unset) | Fails; loads `~/.lee/wallet/storage.json` |
| Both unset | Same failure as NSSA-only |

Binary strings and `--help` reference **`LEE_WALLET_HOME_DIR` only**.

**Decision:** Step 32 uses LEE only; remove `NSSA_WALLET_HOME_DIR` re-export from
`module-e2e.sh` (no dual shim on this pin).

## Reproduce

```bash
curl -sf -X POST https://testnet.lez.logos.co/ -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getAccount","params":["DkT97NZPog2attaFURoMUFU5m996QtDnsS53PSnx41oG"]}'

export PATH="$HOME/.cache/logos-scaffold/repos/lez/$(grep -A2 repos.lez scaffold.toml | grep pin | sed 's/.*"\([^"]*\)".*/\1/')/target/release:$PATH"
export LEE_WALLET_HOME_DIR="$PWD/.scaffold/e2e/testnet-wallet"
wallet auth-transfer init --account-id "Public/DkT97NZPog2attaFURoMUFU5m996QtDnsS53PSnx41oG"
```
