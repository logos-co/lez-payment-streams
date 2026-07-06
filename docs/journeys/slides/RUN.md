# User Journey, localnet
MODE=module CHAIN=local ./scripts/e2e.sh local run --verbosity verbose

# User Journey, testnet
MODE=module CHAIN=testnet ./scripts/e2e.sh testnet run --verbosity verbose

# Developer Journey, localnet
E2E_VERBOSITY=verbose MODE=store CHAIN=local ./scripts/e2e.sh local run

# Developer Journey, testnet
E2E_VERBOSITY=verbose MODE=store CHAIN=testnet ./scripts/e2e.sh testnet run