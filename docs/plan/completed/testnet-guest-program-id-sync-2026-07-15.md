# Completed — testnet guest redeploy (2026-07-15)

Supersedes raw TODO [testnet-guest-program-id-sync.md](../raw-todos/testnet-guest-program-id-sync.md).

## Outcome

Public testnet payment-streams guest ImageID updated from `16b95d37…` to
`de17c0db368abf9f6476f4d67a56ad24e89ddb23bc49b58f7effb566146c1677`.

| Field | Value |
| --- | --- |
| Deploy date | 2026-07-15 |
| Source commit | `6772238bed072d87e62f57f5194d717d9b4ee0b9` |
| ELF size (bytes) | 361716 |
| Guest lock pin | `ruint` 1.17.0, `enum-ordinalize` 4.3.0 (Docker rustc 1.88) |
| Operator | `make build` → `make deploy-testnet` |

Fixtures and script defaults updated to the new `program_id_hex`. Module testnet E2E uses
logoscore chainAction with fresh vault resolution under the fixture owner; full
`bootstrap-testnet-module` PDAs are optional for Flow A.

## Verification

- `make program-id` ImageID matches `fixtures/testnet-module.json`.
- `MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run` — passed 2026-07-15; artifact
  `.scaffold/e2e/artifacts/module-e2e-20260715T181611.log`.
