# Payment streams on LEZ

Step 51 owns publish.
Wrap-up claims match the Step 52 gate log.
Withdraw to owner is written as if Step 54 is complete.
Venue TBD.

Payment streams let a user pay a provider over time from a vault on the Logos Execution Zone.
A Logos module (`payment_streams_module`) exposes vault and stream lifecycle.
Store is one application.
A paid historical-query request carries a LIP-155 eligibility proof.
The provider checks on-chain state before serving.

In August 2026 we completed a wrap-up of that stack on localnet and public testnet.
Fast checks passed.
Public local E2E passed for the module lifecycle (init, deposit, create stream, owner close, claim), provider close, close and create rejects, and Store eligibility (paid query, missing-proof reject, close, claim).
Private local runs passed for the module and for Store with stub receipts.
A local module run with real proving passed.
Public testnet passed for the module and for Store.
Private testnet with real proving passed for the module and for Store, including a paid Store query and a missing-proof reject.
CPU proving is enough for those private legs.
Public and private LEZ execution are both supported.
See Private execution notes in the reproduce docs linked below.

After that wrap-up we shipped withdraw to owner.
The vault owner can take leftover unallocated tokens back to their own account.
They can also send leftover to a different account.
The Basecamp UI withdraws leftover to the owner and prefills the amount from unallocated.
Live guest identity is in the repository README Public testnet guest program section.

## As a protocol user

You fund a vault, deposit tokens, and open a stream to a provider at a rate.
Value accrues while the stream is open.
The vault owner or the provider can close the stream.
Unaccrued allocation returns to the vault at close.
Claimable accrued stays until the provider claims.

After claim, leftover unallocated tokens remain in the vault holding.
The vault owner withdraws them.
If the destination is omitted or is the owner, the owner account is credited.
If the destination is a different account, that account is credited.
Allocated tokens stay locked while a stream is Active or Paused.

Walkthrough (module, testnet):
[docs/reproduce/module.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/reproduce/module.md)

Private execution notes:
[module.md Private execution notes](https://github.com/logos-co/lez-payment-streams/blob/main/docs/reproduce/module.md#private-execution-notes)

Basecamp UI (init through claim, then leftover withdraw to owner):
[docs/reproduce/basecamp-ui.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/reproduce/basecamp-ui.md)

## As a protocol developer

Any request-response protocol can attach an eligibility proof to a request and a verdict to the response.
The provider verifies the proof against an active payment stream before serving.
Store follows RFC 73 (proof on request, status on response) with LIP-155 proof bytes.

Integration approach:
[docs/integrate.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/integrate.md)

Store reproduction (orchestrator, local primary):
[docs/reproduce/store.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/reproduce/store.md)

Store private execution notes:
[store.md Private execution notes](https://github.com/logos-co/lez-payment-streams/blob/main/docs/reproduce/store.md#private-execution-notes)

## Links

Repository, install, and Testing recipes:
[lez-payment-streams](https://github.com/logos-co/lez-payment-streams)

```bash
git clone https://github.com/logos-co/lez-payment-streams.git
cd lez-payment-streams
```

Testing recipes: [README Testing](https://github.com/logos-co/lez-payment-streams/blob/main/README.md#testing).

LIP-155:
[Payment Streams](https://lip.logos.co/anoncomms/raw/payment-streams.html)
