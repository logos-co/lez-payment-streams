# Payment streams module

Universal Logos module (`payment_streams_module`) exposing LIP-155 vault and stream lifecycle via
`chainAction`. Assumes familiarity with Logos (logoscore, `.lgx` modules, LEZ wallet).

## chainAction catalogue

SSOT for module I/O. Invoke:

```bash
logoscore call payment_streams_module chainAction <operation> '<paramsJson>'
```

`paramsJson` is a compact JSON object. Writes return submit JSON (`status`, `tx_hash`, …). Callers
sync via `logos_execution_zone sync_to_block` and poll status ops below.
The invokable return is that compact JSON string (`status` `ok` or `error`, optional `message`,
payload keys merged in). Basecamp `logos.callModule` may wrap it as `{success, data}` with `data`
the same string; `payment_streams_ui` unwraps that envelope.
Status numeric fields (`accrued_lo`, `allocation_lo`, …) are inserted as `qint64` JSON numbers
(IEEE doubles). Values above `2^53 − 1` lose precision on the read path. v1 demo amounts fit in
`*_lo` well below that.
Historical step runbook: [docs/archive/steps/module-chain-writes-runbook.md](../docs/archive/steps/module-chain-writes-runbook.md)
(points here).

Walkthrough = exercised in [docs/reproduce/module.md](../docs/reproduce/module.md).

### Writes

| operation | JSON keys | Semantics | Walkthrough |
| --- | --- | --- | --- |
| `initializeVault` | `owner`, `vault_id` | Create empty native vault. Optional `privacy_tier`. Optional `token_id` (64-hex or base58). Omit `token_id` for native (32 zero octets). | yes |
| `deposit` | `owner`, `vault_id`, `amount_lo`, `amount_hi` | Credit vault from owner balance | yes |
| `withdraw` | `owner`, `vault_id`, `amount_lo`, `amount_hi`, optional `withdraw_to` | Debit vault to owner or `withdraw_to` | no |
| `createStream` | `owner`, `vault_id`, `stream_id`, `provider`, `rate`, `allocation_lo`, `allocation_hi` | Open stream to provider (`provider` base58) | yes |
| `pauseStream` | `owner`, `vault_id`, `stream_id` | Pause accrual | no |
| `resumeStream` | `owner`, `vault_id`, `stream_id` | Resume paused stream | no |
| `topUpStream` | `owner`, `vault_id`, `stream_id`, `increase_lo`, `increase_hi` | Increase stream allocation | no |
| `closeStream` | `owner`, `vault_id`, `stream_id`, optional `provider` | Close stream. Unaccrued returns to vault. Omit `provider` (or match `owner`) for owner-close. Distinct `provider` selects provider-close after stream pre-read. | yes |
| `claim` | `owner`, `provider`, `vault_id`, `stream_id` | Provider claims accrued on stream | yes |

### Reads (via chainAction)

| operation | JSON keys | Semantics | Walkthrough |
| --- | --- | --- | --- |
| `getVaultStatus` | `owner`, `vault_id` | Vault holding balance hex, owner wallet balance hex, and config (`privacy_tier`, `total_allocated_lo`, …) | yes |
| `getStreamStatus` | `owner`, `vault_id`, `stream_id` | Fold at clock (`as_of`, `accrued_*`, `unaccrued_*`), `stream_state` (0 Active, 1 Paused, 2 Closed), plus config `rate`, `allocation_*`, and `accrued_as_of` | yes |

### Low-level decode helpers (separate invokables)

| Method | Purpose |
| --- | --- |
| `ensureWalletOpen` | Open `logos_execution_zone` from `WALLET_HOME` (else `LEE_WALLET_HOME_DIR` / `NSSA_WALLET_HOME_DIR`) when the handle is closed. Creates `statistics.json` beside storage if missing. Returns `sequencer_addr`, `wallet_home`, `fixture_path`, `program_id_hex`. `chainAction` calls the same helper first. |
| `readVaultConfigDecoded` | Decode vault config PDA by base58 account id |
| `readVaultHoldingDecoded` | Decode vault holding PDA |
| `readStreamConfigDecoded` | Decode stream config PDA |
| `readClockDecoded` | Clock PDA |
| `readClock10Decoded` | Default clock-10 account from fixture |

## Required verification

```bash
MODE=module ./verify/e2e.sh local run
```

Testnet:

```bash
make bootstrap-testnet-module   # one-time
make verify-module-testnet
```

Success: exit code 0 and JSON-lines under `.scaffold/e2e/artifacts/` (`module-e2e-*.log`) with phases
`vault_init`, `deposit`, `create_stream`, `close_stream`, `claim`, `module_e2e_complete`
(close then claim). Localnet module E2E uses `e2e/user/wallet-local`. Testnet uses
`e2e/testnet-wallet`. Layout:
[names.md](../docs/reference/names.md#scaffold-layout).

Orchestrated recipes: [docs/reproduce/store.md](../docs/reproduce/store.md).
Manual testnet walkthrough: [docs/reproduce/module.md](../docs/reproduce/module.md).

Prepare only:

```bash
MODE=module ./verify/e2e.sh local prepare
```

Orchestrator: [verify/module-e2e.sh](../verify/module-e2e.sh).
Matrix: [docs/reference/matrix.md](../docs/reference/matrix.md).
First machine: [cold start](../docs/reference/matrix.md#cold-start-first-time-on-a-machine)
in the verification matrix.

## Setup

Tooling example:

```bash
nix shell \
  github:logos-co/logos-package-manager \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-module#lm \
  --command bash
```

Scaffold: `lgs init`, `lgs setup`, `lgs localnet start`. `make seed-fixture` for chain seed script.

Build module (no delivery):

```bash
MODE=module ./verify/e2e.sh build
# or: nix build ./module#lgx
```

Patched `logos_execution_zone` wallet: [docs/reference/pins.md](../docs/reference/pins.md).

Guest ELF for logoscore:

```bash
export PAYMENT_STREAMS_GUEST_BIN="$REPO/program/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
cargo risczero build --manifest-path program/methods/guest/Cargo.toml
```

Set `PAYMENT_STREAMS_GUEST_BIN` on the logoscore daemon process before writes.

## Host boundary

One logoscore process loads `logos_execution_zone` and `payment_streams_module`. Store integration
adds `delivery_module` on provider and user hosts. See [docs/reproduce/store.md](../docs/reproduce/store.md).
CLI callers open the wallet before `chainAction`. Basecamp `payment_streams_ui` calls `ensureWalletOpen`
so `logos_host` inherits `WALLET_HOME` from `make basecamp-ui-run`.

## Recovery

[docs/reference/localnet-recovery.md](../docs/reference/localnet-recovery.md).

## Related

Store eligibility: [docs/reproduce/store.md](../docs/reproduce/store.md).
Eligibility for other protocols: [docs/integrate.md](../docs/integrate.md).
