# Hand-off - deposit / createStream execution failure (zero instruction)

Source of truth investigation for the LIP-155 payment streams User Journey demo on localnet.
Created 2026-07-02. Read alongside [step-29](plan/completed/step-29-e2e-script-ux.md) and [USER_JOURNEY](journeys/USER_JOURNEY.md).

## TL;DR (the proven root cause)

The module-submitted `deposit` (and `createStream`) transactions are included on chain, but their
`instruction_data` field is **16 words, ALL ZEROS**. The on-chain program therefore receives an
empty/zero instruction: `deposit` sees `amount == 0` and reverts with
`Program error 6001: zero deposit amount`; `createStream` reverts with an account-read error
(`invalid length: expected 32 bytes, got 0`).

`initializeVault` "works" only because an all-zero instruction deserializes to
`InitializeVault { vault_id: 0, privacy_tier: Public }`, which is still a valid instruction and
creates vault 0 - masking the bug for that one operation.

So the bug is NOT in the guest program, NOT in the FFI serialization of the `Instruction` enum,
and NOT a stale-deploy / version mismatch. It is in the module -> wallet submission path: the
instruction bytes produced by the FFI are not reaching the wallet FFI as the correct `u32` words.

## How it was proven (empirical, not guessed)

All wire formats below were read from first-hand LEZ source, not assumed:

- `PublicTransaction` is `BorshSerialize` of `{ message: Message, witness_set: WitnessSet }`
  (`logos-execution-zone/lee/state_machine/src/public_transaction/transaction.rs`).
- `Message` fields are `{ program_id: ProgramId([u8;32]), account_ids: Vec<AccountId>, nonces: Vec<Nonce(u128)>, instruction_data: Vec<u32> }`
  (`.../public_transaction/message.rs`).
- The bytes returned by sequencer RPC `getTransaction` are: 1-byte tx-type tag, then the borsh
  `PublicTransaction`.

Parsed the on-chain `deposit` tx `16d07d3e2d7e966b12c08d09a005f846c262a3624791fde9dd0090997f12d643`
(base64 from `getTransaction`) via a Python borsh parser:

```
tag         : 0
program_id  : de17c0db368abf9f6476f4d67a56ad24e89ddb23bc49b58f7effb566146c1677   (= current ps program id)
accounts    : [vault_config 470c2719..., vault_holding 31ed9558..., owner 71fcb1c6...]   (correct deposit plan)
nonces      : [(1, 0)]   (owner nonce 1, correct at submit time)
instruction_data word count : 16
instruction_data words      : [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]   <-- ALL ZEROS
sig count   : 1
```

Accounts, nonce, program id, and signature are all correct. Only `instruction_data` is zeroed.

The 16-word length is itself meaningful: a `Deposit { vault_id, amount, program_id }` instruction
serializes (via `risc0_zkvm::serde`) to exactly 16 `u32` words
(discriminant + vault_id(2) + amount(4) + program_id(8), plus the serde length-prefix word),
i.e. 64 bytes. So the wallet is allocating the right number of words - it is just filling them
with zeros.

The same all-zero `instruction_data` is what makes `initializeVault` appear to succeed (zero
fields are valid for that variant) and what makes `deposit`/`createStream` fail (zero amount /
zero account fields are rejected).

Sequencer log lines confirming the on-chain failures:

- `deposit`     -> `Program error [12001]: Program error 6001: zero deposit amount`
- `createStream`-> `invalid length: expected 32 bytes, got 0`

## The submission path (where the zeroing happens)

Module side (`logos-payment-streams-module/src/payment_streams_module_writes.cpp`):

1. `PaymentStreamsModuleImpl::deposit` (line 796) calls
   `ps_ffi_serialize_deposit(vid, lo, hi, transferPid, ptr, cap, len)` via
   `ffiSerializeTwoPhase` -> `instruction` (QByteArray of the LE-byte expansion of the
   risc0-serde instruction words). `lo=100, hi=0` are passed correctly from `chainAction`.
2. `buildAndSubmit` (line 681) -> `instructionBytesForWallet` (line 416) -> `QList<uint8_t>`
   of those raw bytes (non-empty check passes).
3. `submitGenericPublic` (line 665) -> `buildGenericPublicPayloadJson` (line 442) puts
   `instruction_hex = hex(instructionBytes)` into the payload JSON.
4. `submitGenericPublicViaFfi` (line 470):
   - PRIMARY: `invokeWalletQtString(qtClient, "send_generic_public_transaction_json", payloadJson)`.
   - FALLBACK (only if primary returns empty, lines 481-511): re-parse the JSON and call the
     typed `qtClient->invokeRemoteMethod("logos_execution_zone", "send_generic_public_transaction",
     accountHexIds, signingFlags, instructionList, programIdHex)` where
     `instructionList` is a `QList<uint8_t>` (raw bytes).

Wallet side (`logos-execution-zone-module/src/logos_execution_zone_wallet_module.{h,cpp}`):

- The typed method is
  `send_generic_public_transaction(const std::vector<std::string>& account_ids, const std::vector<bool>& signing_requirements, const std::vector<uint32_t>& instruction, const std::string& program_id_hex)`
  (header line 80) -> `wallet_ffi_send_generic_public_transaction(... instruction_words, size ...)`.
- The `_json` variant (`send_generic_public_transaction_json`) is a "repo-local Qt-only patch
  (N10)" referenced in code comments and in step-26 docs, but it is **NOT present** in the
  checked-out `logos-execution-zone-module` source tree
  (`rg send_generic_public_transaction_json` finds only the call site in the payment-streams
  module, no implementation). So on the currently built/loaded wallet module, the primary
  invocation returns empty and the FALLBACK runs.

The fallback's type mismatch is the prime suspect: it passes `QList<uint8_t>` (raw bytes) into a
parameter declared `std::vector<uint32_t>` (words). The Qt `invokeRemoteMethod` marshalling does
not correctly reinterpret the bytes as `u32` words, and the wallet FFI ends up with 16 zero
words (right count, zero content). This matches the on-chain evidence exactly.

Caveat / open question: it is also possible that a patched wallet module IS loaded (built from a
source tree not present in this workspace) and the `_json` primary path is actually taken, in
which case the zeroing would be inside that patch's `instruction_hex` -> `wallet_ffi_serialization_helper`
handling. Either way the symptom and the fix target are the same: the wallet must receive the
correct `u32` instruction words.

## What is NOT the cause (rules out, with evidence)

- Stale deployed program / ImageID mismatch: ruled out. The on-chain `program_id`
  `de17c0db368abf9f6476f4d67a56ad24e89ddb23bc49b58f7effb566146c1677` equals the current
  `ps_program_id_hex` derived from the current guest build, and equals the program id in the
  restored `funded` snapshot. The seeder (`examples/src/bin/seed_localnet_fixture.rs`) successfully
  deposits 1000 with the same program, so the guest `deposit` logic is sound.
- FFI / core instruction serialization: ruled out. `lez-payment-streams-core` test
  `all_variants_round_trip_via_instruction_words` round-trips `Deposit` (and all variants) through
  `Program::serialize_instruction` = `risc0_zkvm::serde::to_vec`. The C bridge
  (`ps_ffi_serialize_deposit`) is a thin pass-through of `lo`/`hi` to
  `payment_streams_ffi_serialize_deposit_instruction`, and `balance_from_lo_hi(100,0)` = 100.
- Account planning: ruled out. The on-chain accounts are exactly `vault_config`,
  `vault_holding`, owner - the correct deposit account set.
- Nonce: ruled out. Owner nonce in the tx is 1, matching the chain state at submit time
  (AT-init consumed nonce 0, vault_init consumed nonce 1... see nonce notes below).

## Environment / reproduce notes

- Chain: localnet via `./scripts/lifecycle.sh` (sequencer RPC at `http://127.0.0.1:3040`).
- State was restored from the `funded` snapshot
  (`./scripts/lifecycle.sh snapshot restore funded`) so the payment_streams program is deployed
  and the seeder's owner has funds.
- Fixture manifest: `fixtures/localnet.json` must exist for the module. It was created by copying
  `fixtures/localnet-debug.json` (which carries a valid `program_id_hex`). Without it,
  `vault_init` fails with "cannot open fixture manifest".
- Owner used by the module demo: `8fxZ8wwrc15EYnPoqqLEFHFAvr9Ft2o1yviMpbCdX962` (base58),
  hex `71fcb1c6830bf2d1b18d19a2b2fe0720e162801b279b00cf23846be152821313`.
- `getTransaction` has a severe indexing lag on localnet: a tx that is included in a block
  (~15s block time) can take **2-5 minutes** to appear in `getTransaction`. The wallet CLI config
  reflects this (`seq_tx_poll_max_blocks: 22` ~ 5.5 min). The script's `await_inclusion` budget
  must account for this, or false "not included" failures occur.

## AT-init side-fix already landed (script-only, keep it)

A separate, real bug was fixed in `scripts/module-e2e.sh`: `deposit`/`claim` chain into the
`authenticated_transfer` program, which requires the owner/provider accounts to be AT-initialized
first (`register_public_account` / `wallet auth-transfer init`) while still default-owned. The
module flow was missing this. Fix added an `auth_transfer_init` helper (uses the
`logos_execution_zone` module's already-exposed `register_public_account` primitive via logoscore)
and calls it for OWNER and PROVIDER right after account creation and before topup, with inclusion
confirmation. This is correct and necessary; do not revert it. It is a prerequisite for deposit to
even reach the program, but it is NOT the cause of the zero-instruction bug (deposit still fails
with "zero deposit amount" after AT-init succeeds).

## Script-side work already in place (keep, but it depends on the bug above)

`scripts/module-e2e.sh` was extended for "chain as source of truth":

- On-chain read helpers: `read_vault`, `read_stream`, `stream_state_name`, `poll_read`,
  `_le_u128_to_int` (Python).
- Mandatory tx-inclusion confirmation: `seq_tx_included` + `await_inclusion` poll
  `getTransaction(tx_hash)` after every `call_ps`; `call_ps` flags `ok:false` with
  `inclusion:timeout` if a submitted tx never appears.
- `auth_transfer_init` helper + OWNER/PROVIDER init calls (see above).
- `USER_JOURNEY.md` rewritten to the Step-29 flow (TestNet v0.2, two demo commands, on-chain
  verification phases, removed manual step-by-step, accounts-generated-locally note).

These changes are all valid and should stay; they simply cannot turn the demo green until the
zero-instruction bug is fixed, because every deposit/createStream/claim will keep failing on chain.

## Recommended next steps (for a fresh chat)

1. Confirm which wallet module is actually loaded by logoscore and whether
   `send_generic_public_transaction_json` exists on it at runtime
   (e.g. add a temporary debug log in `submitGenericPublicViaFfi` printing whether the primary
   returned non-empty, and the `instructionList` byte length / first 16 bytes). This decides
   whether the fix is "make the fallback pass words" or "fix the _json patch".
2. Fix the instruction marshalling so the wallet FFI receives the correct `u32` words:
   - Preferred: implement/restore the `send_generic_public_transaction_json` Qt patch (N10) in
     `logos-execution-zone-module` so it calls `wallet_ffi_serialization_helper` on
     `instruction_hex` to produce the `u32` words (this is the path the module's primary branch
     already targets). OR
   - In the fallback, convert the instruction bytes to `u32` words (little-endian, 4 bytes ->
     1 word) before passing them as `std::vector<uint32_t>` to `send_generic_public_transaction`,
     instead of passing a `QList<uint8_t>`.
3. Rebuild the wallet module, restart logoscore, rerun `scripts/module-e2e.sh` with `CHAIN=local`,
   and re-parse a fresh `deposit` tx's `instruction_data` with the Python borsh parser above to
   confirm it is now non-zero (discriminant 1, amount 100, etc.).
4. Once deposit/createStream execute, recheck `claim` end-to-end (AT-init for provider is already
   in place) and finalize the `await_inclusion` timeout so it tolerates the multi-minute
   `getTransaction` indexing lag (or switch inclusion confirmation to a block-scan method that
   does not depend on `getTransaction` indexing).

## Key files

- `logos-payment-streams-module/src/payment_streams_module_writes.cpp` - `deposit` (796),
  `buildAndSubmit` (681), `instructionBytesForWallet` (416), `buildGenericPublicPayloadJson` (442),
  `submitGenericPublicViaFfi` (470, primary + fallback), `submitGenericPublic` (665),
  `chainAction` (1341).
- `logos-execution-zone-module/src/logos_execution_zone_wallet_module.{cpp,h}` - typed
  `send_generic_public_transaction` (cpp:970, h:80) expecting `std::vector<uint32_t>`; missing
  `_json` patch.
- `logos-execution-zone/lee/state_machine/src/public_transaction/{transaction,message}.rs` -
  `PublicTransaction`/`Message` borsh wire format.
- `logos-execution-zone/lez/wallet-ffi/src/generic_transaction.rs` -
  `wallet_ffi_send_generic_public_transaction` (takes `u32` words) and
  `wallet_ffi_serialization_helper` (bytes -> words via `risc0_zkvm::serde::to_vec(slice)[1..]`).
- `lez-payment-streams-core/src/instruction_wire.rs` - `instruction_words_for_public_transaction`
  and `instruction_bytes_le_from_words` (canonical encoding used by the FFI).
- `lez-payment-streams-core/src/instruction.rs` - `Instruction` enum (`Deposit` field order:
  `vault_id, amount, authenticated_transfer_program_id`).
- `examples/src/bin/seed_localnet_fixture.rs` - known-good submission path (`Message::try_new`
  with explicit nonce fetch + poll) that successfully deposits 1000 with the same program.
- `scripts/module-e2e.sh` - demo script with on-chain reads, inclusion confirmation, AT-init.
- `fixtures/localnet.json` - required fixture manifest (copied from `localnet-debug.json`).

## Reproduce the parse (decisive check)

```python
import urllib.request, json, base64, struct
url="http://127.0.0.1:3040"
def rpc(m,p):
    r=urllib.request.Request(url,data=json.dumps({"jsonrpc":"2.0","id":1,"method":m,"params":p}).encode(),headers={"Content-Type":"application/json"})
    return json.load(urllib.request.urlopen(r,timeout=5))
h="<tx_hash>"
raw=base64.b64decode(rpc("getTransaction",[h])["result"])
o=0; tag=raw[o]; o+=1; pid=raw[o:o+32]; o+=32
na=struct.unpack_from("<I",raw,o)[0]; o+=4; o+=32*na
nn=struct.unpack_from("<I",raw,o)[0]; o+=4; o+=16*nn
ni=struct.unpack_from("<I",raw,o)[0]; o+=4
inst=[struct.unpack_from("<I",raw,o+4*i)[0] for i in range(ni)]
print("instruction_data words:", inst)  # expect non-zero for a healthy tx
```
