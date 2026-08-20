# Payment streams on LEZ

Published:
[Payment streams on LEZ](https://forum.research.logos.co/t/payment-streams-on-lez/725)
(2026-08-20).

This post summarizes progress on payment streams and the first implementation on the Logos Execution Zone (LEZ).

A payment stream lets a user pay a provider over time from a vault. It can be used as a separate off-chain payment protocol, or as eligibility on a request-response protocol. The protocol is specified in [LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html). This work builds on [Off-chain Payment Protocols on LEE](https://forum.research.logos.co/t/off-chain-payment-protocols-on-lee/674).

The implementation lives in [lez-payment-streams](https://github.com/logos-co/lez-payment-streams). A Logos module (`payment_streams_module`) exposes vault and stream lifecycle. It runs on Logos TestNet v0.2, including private execution.

## Independent payment protocol

Payment streams allow fine-grained payments with a limited on-chain footprint.

The user funds a vault, deposits tokens, and opens a stream to a provider at a rate. Value accrues while the stream is active. The vault owner or the provider can close the stream. Unaccrued allocation returns to the vault at close and can be withdrawn. Claimable accrued funds stay in the stream until the provider claims. Allocated tokens stay locked while a stream is Active or Paused.

Walkthrough:

[docs/reproduce/module.md](https://github.com/logos-co/lez-payment-streams/blob/master/docs/reproduce/module.md)

An early Basecamp UI plugin covers the same lifecycle for public execution:

[docs/reproduce/basecamp-ui.md](https://github.com/logos-co/lez-payment-streams/blob/master/docs/reproduce/basecamp-ui.md)

## Eligibility for request-response protocols

A protocol developer can amend a request-response protocol so it supports eligibility. Requests then carry an eligibility proof. The provider verifies the proof against an active payment stream before serving.

Store is a working example. A paid historical-query request carries a LIP-155 eligibility proof. The provider checks on-chain state before serving. Store follows [WAKU2-INCENTIVIZATION](https://lip.logos.co/messaging/core/raw/incentivization.html) (proof on request, status on response) with LIP-155 proof bytes.

Integration approach:

[docs/integrate.md](https://github.com/logos-co/lez-payment-streams/blob/master/docs/integrate.md)

Walkthrough:

[docs/reproduce/store.md](https://github.com/logos-co/lez-payment-streams/blob/master/docs/reproduce/store.md)

## Private execution

LEZ private accounts and shielded transactions support [user unlinkability](https://lip.logos.co/anoncomms/raw/payment-streams.html#privacy-goals) and provider unlinkability. User unlinkability uses a `PseudonymousFunding` vault so the funding address stays unlinked from the on-chain vault owner. Provider unlinkability uses a private provider account so the on-chain stream provider stays unlinked from the account that claims accrued funds.

Privacy model and limits:

[LIP-155 Security and Privacy Considerations](https://lip.logos.co/anoncomms/raw/payment-streams.html#security-and-privacy-considerations)

Reproduce notes:

[module.md Private execution notes](https://github.com/logos-co/lez-payment-streams/blob/master/docs/reproduce/module.md#private-execution-notes)

## References

[lez-payment-streams](https://github.com/logos-co/lez-payment-streams)

[LIP-155 Payment Streams](https://lip.logos.co/anoncomms/raw/payment-streams.html)

[WAKU2-INCENTIVIZATION](https://lip.logos.co/messaging/core/raw/incentivization.html) (eligibility envelope)

[Off-chain Payment Protocols on LEE](https://forum.research.logos.co/t/off-chain-payment-protocols-on-lee/674)