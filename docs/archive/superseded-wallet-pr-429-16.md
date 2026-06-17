# Superseded wallet path — PR 429 and PR 16

Historical reference only. Payment streams uses LEZ `main` (491 merged) and
[logos-execution-zone-module PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19).

Do not pin or build this integration against:

- [logos-execution-zone PR 429](https://github.com/logos-blockchain/logos-execution-zone/pull/429)
  — `wallet_ffi_send_public_transaction` (narrower JSON submit API)
- [logos-execution-zone-module PR 16](https://github.com/logos-blockchain/logos-execution-zone-module/pull/16)
  — Qt JSON wrapper for 429

491 supersedes 429 (`wallet_ffi_send_generic_public_transaction`, account resolution,
serialization helper). PR 19 supersedes 16 as the module bridge for 491.

## 429 / 16 JSON submit shape

429 and PR 16 used a single JSON object with lowercase hex, no `0x` prefix:

```json
{
  "program_id": "hex",
  "accounts": ["hex", "hex", ...],
  "instruction": "hex",
  "signer_account": "hex"
}
```

`logos-rln-module` may still use this shape where deployed. Payment streams and this
demo stack use the 491 generic public path via PR 19, not 429.
