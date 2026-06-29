# Step 11d — wallet runtime at LEZ PR 510

Status: landed in tree (pins, wrapper patches, `./scripts/archive/verify-step11d-dod.sh`).
Strict 11b E2E on a fresh fixture may still need `REINIT_WALLET=1` and reinstalled `.lgx` after
`lgs setup`. Step 12 strict verify uses `REQUIRE_STREAM_PROOF=1` (see
[`step12-user-eligibility.md`](step12-user-eligibility.md)).

Upgrade reference: payment-streams moved the wallet stack from the earlier 491-era pin to LEZ `main` at
[PR 510](https://github.com/logos-blockchain/logos-execution-zone/pull/510) merge
(`62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60`).

510 adds `wallet_ffi_program_deployment` and program ELF helpers on the Rust FFI side.
The Qt wallet module on
[PR 19](https://github.com/logos-blockchain/logos-execution-zone-module/pull/19)
already exposes `send_program_deployment_transaction` and generic public submit; this step
pins LEZ 510, rebuilds the patched `logos_execution_zone` `.lgx`, and adds
`send_generic_public_transaction_json` in the payment-streams wrapper (used by
`payment_streams_module` chain writes).

Pins and flake layout: [`feature-branch-pins.md`](feature-branch-pins.md).
Prerequisites: Step 10a fixture, Step 10b wallet install pattern.
Step 12 eligibility does not require 11d for offline DoD; logoscore `stream_proof` with
`REQUIRE_STREAM_PROOF=1` expects 11b writes and a funded stream (top-up helper in verify script).

## Pin bump

After pulling this tree (details also in [`feature-branch-pins.md`](feature-branch-pins.md)):

```bash
# Match scaffold localnet + FFI artifacts to LEZ 510
lgs setup

# Rebuild FFI (symlinks LEZ artifacts/)
nix build .#payment-streams-ffi

# Rebuild patched wallet + payment_streams .lgx
./scripts/archive/build-wallet-lgx.sh
nix build ./logos-payment-streams-module#lgx
```

Install order unchanged (wallet first, then payment_streams_module). Build and install commands
live in [`step10b-wallet-runtime.md`](step10b-wallet-runtime.md); this doc only lists the
extra steps after a pin bump (`lgs setup`, FFI rebuild).

## Patched wrapper (payment-streams only)

Directory:
`logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`

| Input | Rev / ref |
| --- | --- |
| LEZ (`lez-wallet-ffi-patched`) | `62d9ba10f8f86db3a1f04b329a1bd9d5b893bf60` |
| Wallet module `upstream` | PR 19 head (follows LEZ input) |
| Rust patch | `lez-rust-sign-public-payload.patch` on `wallet_ffi` |
| Qt patches | guest ELF from `PAYMENT_STREAMS_GUEST_BIN`, `sign_public_payload`, `send_generic_public_transaction_json` |

`PAYMENT_STREAMS_GUEST_BIN` remains until chain submit reliably uses wallet-held program ELF
after deploy; 11d does not remove the guest-env patch.

## Offline API check

```bash
export MODULES="$HOME/Downloads/software/waku/lez-related/logos-cli/modules"
lm methods "$MODULES/logos_execution_zone/logos_execution_zone_plugin.so" \
  | rg 'send_program_deployment_transaction|send_generic_public_transaction_json|sign_public_payload'
```

## Operator paths

Fixture seed (default): `make deploy` via scaffold `wallet` CLI during
`./scripts/seed-localnet-fixture.sh` — unchanged.

Logoscore deploy smoke: `./scripts/deploy-program-logoscore.sh` checks
`send_program_deployment_transaction` is wired (empty ELF → structured failure).
Full program bytes are too large for logoscore CLI arguments; use `make deploy` or
`wallet deploy-program` with the same wallet home as seed.

Chain writes: `payment_streams_module` `chainAction` →
`send_generic_public_transaction_json` on `logos_execution_zone` (Step 11b).
See [`step11b-chain-writes.md`](step11b-chain-writes.md) for env vars and E2E scripts.

## Verification

```bash
./scripts/archive/verify-step11d-dod.sh
./scripts/archive/verify-step10b-dod.sh
./scripts/archive/verify-step11b-dod.sh
```

Fresh blank slate before strict E2E:

```bash
make full-reset-localnet
```

Then Step 12 with a new persistence dir (see [`demo-localnet-recovery.md`](demo-localnet-recovery.md)).

## Definition of done

1. Patched wallet `.lgx` builds against LEZ ≥ 510; `lm methods` lists deploy + JSON public submit.
2. `./scripts/archive/verify-step11d-dod.sh` exits 0 (offline checks; logoscore section when localnet is up).
3. Re-run 10b / 11b verifies on the upgraded stack after `lgs setup` and reinstalling `.lgx` files.
4. Step 12 strict verify: `REQUIRE_STREAM_PROOF=1 ./scripts/archive/verify-step12-dod.sh` (or `make verify-step12`
   with env set) after fresh fixture when localnet is up.
