# Testnet claim known issue and demo funding policy

Status as of 2026-06-28.
Scope: public testnet (`https://testnet.lez.logos.co/`) only.
Localnet is unaffected; local close and claim both confirm.

## Summary

On public testnet the provider `claim` instruction has not reliably confirmed,
while every other instruction in the same wallet and nonce sequence
(create, deposit, close) lands.
Because claim is a teardown and accounting step, not the demo headline,
the demo treats claim as optional (see Demo policy below) and the
end-to-end run does not depend on it.

## Observed behavior

Two provider-signed `claim-onchain` submissions for stream 5 (vault 0,
owner `DkT97...`, provider `BhyL...`) never confirmed.
After each submission `provider_bal` stayed `0` and the vault
`total_allocated` was unchanged, even after waiting more than 10 minutes.
In the same wallet and nonce sequence every provider-signed `close`
landed on chain (streams 3, 4, 5 reached `stream_state=2`).
The signature is a dropped or rejected transaction at sequencer validation
rather than slow inclusion.

## Why this is not a core protocol bug

The `claim-onchain` path is wired
(`examples/src/bin/seed_localnet_fixture.rs`, subcommand `claim-onchain`)
and the core accrual semantics support paying residual on a `Closed` stream.
`lez-payment-streams-core/src/stream_config.rs` covers this with the test
`n_closed_residual_succeeds`: `claim_at_time` on a closed stream pays the
accrued residual, lowers allocation by the payout, clears accrued, and
revalidates invariants.
Account order and signer for claim are identical to close, which works,
so the encoding and witness construction are not the obvious cause.

## Next diagnostic step

Capture the sequencer reject reason for `Instruction::Claim`
(verbose submit or mempool inspection) and compare the claim message bytes
against a known-good close message.
Since account order and signer are identical, the difference is the
instruction variant in the message payload and the guest dispatch for it.

## Demo policy claim is optional

`scripts/e2e/run_local_e2e.py` `demo_teardown` treats claim as optional.
After close, if the stream has residual accrued it attempts the claim via the
direct-submit seed path, polls for confirmation, and on testnet logs
`demo_claim` with `optional=True`, `claimed=False`,
`reason=claim_optional_unconfirmed` instead of failing the run.

- Default: claim is optional on testnet (`CHAIN=testnet`), required on localnet.
- Override: set `E2E_CLAIM_OPTIONAL=1` to force optional on localnet, or
  `E2E_CLAIM_OPTIONAL=0` to require a confirmed claim on testnet.

A run is green when create, fundable, paid Store query, and close succeed;
an unconfirmed claim does not fail the run.

## Funding must be sufficient without claim

Vault accounting on chain:

- `unallocated = holding - total_allocated`.
- Only `deposit`, or a `close` that returns unaccrued, raises `unallocated`.
- `claim` pays the provider out of `holding` and lowers `total_allocated` by
  the same amount, so it does not free `unallocated` headroom.

Implication for demos:
claim never recycles spendable headroom, so the demo vault must be funded so a
run completes without relying on claim.
Each run needs `unallocated >= stream_allocation` at create time
(the create preflight rejects `allocation > unallocated`).
`close` returns only the unaccrued portion; the accrued portion stays in
`total_allocated`, owed to the provider until claimed.

On slow testnet a `rate=1` stream accrues about 1 unit per second of real time.
A clean run with the close fix closes in roughly 7 minutes, so the stream is
only partially accrued and `close` returns most of the allocation, but each
run still draws down `holding` by the accrued amount.
Size the vault for the expected number of runs, or top it up between runs via
`pinata claim` plus `seed_localnet_fixture deposit-onchain`.

Reference values used during this work: `fixtures/testnet.json` `stream_rate=1`,
`stream_allocation=1000`; vault 0 holding was raised to `4550` to cover
several runs without reclaiming.

## Pointers

- Orchestrator teardown: `scripts/e2e/run_local_e2e.py`
  (`demo_teardown`, `seed_claim_onchain`).
- Claim CLI: `examples/src/bin/seed_localnet_fixture.rs` (`claim-onchain`).
- Core semantics: `lez-payment-streams-core/src/stream_config.rs`
  (`claim_at_time`, test `n_closed_residual_succeeds`).
- Testnet packet: [plan/completed/step-18-public-testnet-demo.md](plan/completed/step-18-public-testnet-demo.md).
