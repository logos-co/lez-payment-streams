# Naming conventions

Use this vocabulary consistently in product docs and runbooks.

## Role layers (LIP-155 / module)

Keep service, chain, submitter, and funding as separate layers.
Formerly `payer` / `payee` map as below. Living surfaces use the table terms (see Step 47).

| Layer | Terms | Use for |
| --- | --- | --- |
| Service / protocol (LIP Roles, Store hosts) | `user`, `provider` | Who requests service vs who delivers. `provider` is the same party as the chain-layer provider (cross-layer term). |
| On-chain identity (guest, module JSON, manifests) | `owner`, `provider` | `VaultConfig.owner`, `StreamConfig.provider` / claim account. |
| Tx submitter (derived) | `submitter` | Who signs/submits this tx. Not a module JSON key. Precedence for close/claim: `provider` > `owner`. Owner writes (`createStream`, …) keep owner as submitter even when `provider` appears as instruction data. |
| Privacy funding (ops only) | `funder` / `PUBLIC_FUNDER` | Public account that shields into a private owner. Distinct from the vault owner. |
| Informal (retired) | `payer`, `payee` | Historical slang only. |

Module `chainAction` JSON (writes + close): vault owner id is `owner`. Stream
provider id is `provider` (close). Store verify peer param is `userPeerId`
(symmetry with `providerPeerId`). LEZ `#[account(signer)]` / IDL `"signer":
true` means must-sign only.

Account-id write params accept base58 or 64-hex via one helper (fixed order):

1. Trimmed input exactly 64 hex characters (`[0-9a-fA-F]{64}`) → decode hex to 32 bytes.
2. Else → decode as base58; require decoded length 32.
3. Otherwise → error.

Inventory (not renamed): localnet fixture state still keys `SIGNER_ID`. Scripts
alias it to local `OWNER` at the state-file boundary.

## External product names

| Term | Meaning |
| --- | --- |
| Module verification | Single-host `payment_streams_module` happy path (`MODE=module`). |
| Store integration | Dual-host Store demo with eligibility (`MODE=store`, default). |

## Verification flows (`MODE`)

| Term | Meaning |
| --- | --- |
| `MODE=module` | Payment streams in isolation (no Store, no eligibility gate). |
| `MODE=store` | Default. Dual-host demo with `delivery_module` and LIP-155 eligibility on Store. |

Makefile targets use `verify-module-*` and `verify-store-*`.

## N18 demo tracks (plan index)

| Term | Meaning |
| --- | --- |
| N18 | Protocol vs Store eligibility tracks. Step 20 formal logos-docs publish wontfix (logos-docs#369). Historical protocol CLI complete (Step 22 / logos-docs#370). Protocol UI Step 21 upcoming. Step 46: reproduce + integrate docs. Step 51: forum links. Formerly User Journey / Developer Journey as living labels. |

## Logos and protocol names

| Term | Use |
| --- | --- |
| payment streams module | Prose description of the Logos plugin. |
| `payment_streams_module` | Runtime module id. |
| Store | Waku/Logos Store protocol (capitalize). |
| `logos-delivery` | Repository for Store and liblogosdelivery. |
| `delivery_module` | Logos plugin for Delivery/Store. |
| `logos_execution_zone` | LEZ wallet Logos module id. |
| LIP-155 | Hyphenated spec name. |

## Makefile targets

Primary (step-free):

| Make target | Matrix cell |
| --- | --- |
| `make verify-module-local` | Module × localnet |
| `make verify-store-local` | Store × localnet |
| `make verify-store-testnet` | Store × testnet |
| `make verify-store-local-lifecycle` | Maintainer only (two runs, one ledger) |

Legacy aliases: `verify-step17`, `verify-step18`, `verify-step17-back-to-back`.

Canonical commands: [verification-matrix.md](verification-matrix.md),
[scripts/README.md](../../scripts/README.md).

## Scaffold layout

Gitignored state under `$REPO_ROOT/.scaffold/`. Path helpers live in
`scripts/lib/common.sh` (`ps_e2e_*`, `ps_scaffold_*`).

| Path | Mode | Role |
| --- | --- | --- |
| `e2e/user/modules` | module (+ Store client install) | `lgpm` install tree (`MODULES_USER`) |
| `e2e/user/logoscore`, `e2e/user/persist` | module / Store client host | Dual-host logoscore daemon state |
| `e2e/user/wallet-local` | module (localnet module E2E) | Isolated wallet; reset each `module-e2e.sh` local run (`WALLET_E2E_DIR`) |
| `e2e/provider/modules`, `e2e/provider/logoscore`, `e2e/provider/persist` | Store provider | Provider host |
| `e2e/testnet-wallet` | module + Store (testnet) | Testnet wallet home (`ps_chain_wallet_home` when `CHAIN=testnet`) |
| `e2e/artifacts` | E2E verification | JSONL logs (`module-e2e-*.log`, `e2e-*.log`, …) |
| `e2e/provider-advertisement.json` | Store | Off-band provider ad file (orchestrator) |
| `wallet` | Localnet (scaffold) | Default localnet wallet when not using `e2e/user/wallet-local` |

Override defaults with env vars (`MODULES_USER`, `MODULES_PROVIDER`, `WALLET_E2E_DIR`,
`TESTNET_WALLET_DIR`, …). Manual protocol path on testnet uses the same paths as
`MODE=module` E2E: [payment-streams.md](../reproduce/payment-streams.md).
