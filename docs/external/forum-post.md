# Payment streams on LEZ

Step 51 owns publish.
TODO: wait for Step 52 wrap-up verification before stating wrap-up in public.

Payment streams let a user pay a provider over time from a vault on the Logos Execution Zone.
A Logos module (`payment_streams_module`) exposes vault and stream lifecycle.
Store is one application. A paid historical-query request carries a LIP-155 eligibility proof.
The provider checks on-chain state before serving.

Public and private LEZ execution are both supported.
See Private execution notes in the reproduce docs linked below.

## As a protocol user

You fund a vault, deposit tokens, open a stream to a provider at a rate, wait while value accrues, close the stream, and the provider claims what accrued.
Unaccrued allocation returns to the vault at close.
Claimable accrued stays until the provider claims.

Walkthrough (module, testnet):
[docs/reproduce/payment-streams.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/reproduce/payment-streams.md)

## As a protocol developer

Any request-response protocol can attach an eligibility proof to a request and a verdict to the response.
The provider verifies the proof against an active payment stream before serving.
Store follows RFC 73 (proof on request, status on response) with LIP-155 proof bytes.

Integration approach:
[docs/integrate/eligibility.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/integrate/eligibility.md)

Store reproduction (orchestrator, local primary):
[docs/reproduce/store-eligibility.md](https://github.com/logos-co/lez-payment-streams/blob/main/docs/reproduce/store-eligibility.md)

## Links

Repository and install:
[lez-payment-streams](https://github.com/logos-co/lez-payment-streams)

```bash
git clone https://github.com/logos-co/lez-payment-streams.git
cd lez-payment-streams
```

Testing recipes: repository README Testing section.

LIP-155:
[Payment Streams](https://lip.logos.co/anoncomms/raw/payment-streams.html)
