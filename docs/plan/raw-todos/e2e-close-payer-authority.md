# Raw TODO — E2E closeStream payer authority

Ideas not yet scheduled as a plan step. Index: [../index.md](../index.md).

## Problem

`scripts/module-e2e.sh` narrates “Alice closes the stream” but calls `closeStream` with
`signer` = owner (payer) and `authority` = provider (payee). The module signs with `authority`,
so the payee key submits close today.

On chain, either payer or payee may close (LIP-155). The User Journey walkthrough (Step 34)
teaches **payer-led close**.

## Proposed change

In `module-e2e.sh` (and any mirrored JSON in fixtures/docs):

- Set `closeStream` `authority` to the payer (same as `signer`), or omit `authority` so it
  defaults to `signer`.

Re-run `MODE=module` local and testnet E2E; update narrative only if needed (already payer-led).

## Verification

- Artifact phases `close_stream`, `close_state` still `ok:true`.
- Optional: align DEVELOPER_JOURNEY / Store teardown close if it copies the same pattern.

## Promotion

When done, close this raw TODO or fold into a small maintenance step; mention in E2E.md phase
expectations if narrative strings are documented there.
