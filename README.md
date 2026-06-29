# lez-payment-streams

[LIP-155](https://lip.logos.co/anoncomms/raw/payment-streams.html) payment streams on the Logos
Execution Zone: SPEL guest program, Logos `payment_streams_module`, and a reference Store
integration (eligibility on paid queries).

Documentation hub: [docs/README.md](docs/README.md). Maintainers: [AGENTS.md](AGENTS.md).

## Verify

```bash
MODE=module CHAIN=local ./scripts/e2e.sh local run
./scripts/e2e.sh local run
```

Advanced testnet Store path and artifact expectations:
[docs/reference/verification-matrix.md](docs/reference/verification-matrix.md).
First-time setup: [cold start](docs/reference/verification-matrix.md#cold-start-first-time-on-a-machine).

## License

MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache 2.0 ([LICENSE-APACHE-v2](LICENSE-APACHE-v2)).
