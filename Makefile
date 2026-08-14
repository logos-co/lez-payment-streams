# lez-payment-streams
#
# Quick start:
#   make build idl deploy setup


SHELL := /bin/bash
STATE_FILE := .scaffold/.lez_payment_streams-state
IDL_FILE := program/lez-payment-streams-idl.json
PROGRAMS_DIR := program/methods/guest/target/riscv32im-risc0-zkvm-elf/docker
PROGRAM_BIN := $(PROGRAMS_DIR)/lez_payment_streams.bin

# Load saved state if it exists
-include $(STATE_FILE)

define save_var
	@mkdir -p $(dir $(STATE_FILE))
	@grep -v '^$(1)=' $(STATE_FILE) 2>/dev/null > $(STATE_FILE).tmp || true
	@echo '$(1)=$(2)' >> $(STATE_FILE).tmp
	@mv $(STATE_FILE).tmp $(STATE_FILE)
endef

PHONY_TARGETS := $(shell awk -F: '/^[a-zA-Z0-9][a-zA-Z0-9_.-]*:/ { gsub(/ .*/, "", $$1); print $$1 }' Makefile | sort -u)
.PHONY: $(PHONY_TARGETS)

help: ## Show this help
	@echo "lez-payment-streams"
	@echo ""
	@awk ' \
	  /^[a-zA-Z0-9][a-zA-Z0-9_.-]*:/ { \
	    if (tgt != "" && help != "" && !arch) printf "  make %-42s %s\n", tgt, help; \
	    split($$0, parts, ":"); tgt=parts[1]; \
	    help=""; arch=0; \
	    if (match($$0, /## /)) help=substr($$0, RSTART+3); \
	    next; \
	  } \
	  /verify\/archive\// { arch=1 } \
	  END { if (tgt != "" && help != "" && !arch) printf "  make %-42s %s\n", tgt, help } \
	' $(MAKEFILE_LIST)
	@echo ""
	@echo "Historical DoD scripts: verify/archive/"
	@echo ""
	@echo "Example:"
	@echo "  make build idl deploy"

build: ## Build the guest binary
	cargo risczero build --manifest-path program/methods/guest/Cargo.toml
	@echo ""
	@echo "✅ Guest binary built: $(PROGRAM_BIN)"
	@ls -la $(PROGRAM_BIN) 2>/dev/null || true

idl: ## Generate IDL JSON from program source (stock spel CLI)
	@set -euo pipefail; \
	tmp=$$(mktemp); \
	trap 'rm -f "$$tmp"' EXIT; \
	spel generate-idl program/methods/guest/src/bin/lez_payment_streams.rs > "$$tmp"; \
	mv "$$tmp" $(IDL_FILE)
	@echo "✅ IDL written to $(IDL_FILE)"

deploy: ## Deploy program to sequencer (pinned LEZ wallet; set LEE_WALLET_HOME_DIR)
	@test -n "$$LEE_WALLET_HOME_DIR" || (echo "ERROR: set LEE_WALLET_HOME_DIR"; exit 1)
	@test -f "$(PROGRAM_BIN)" || (echo "ERROR: Binary not found. Run 'make build' first."; exit 1)
	wallet deploy-program $(PROGRAM_BIN)
	@echo "✅ Program deployed"

program-id: ## Show ProgramId for built binary
	@test -f "$(PROGRAM_BIN)" || (echo "ERROR: Binary not found. Run 'make build' first."; exit 1)
	spel inspect $(PROGRAM_BIN)

setup: ## Create accounts needed for the program
	@echo "Creating signer account..."
	$(eval SIGNER_ID := $(shell wallet account new public 2>&1 | sed -n 's/.*Public\/\([A-Za-z0-9]*\).*/\1/p'))
	@echo "Signer: $(SIGNER_ID)"
	$(call save_var,SIGNER_ID,$(SIGNER_ID))
	@echo ""
	@echo "✅ Account saved to $(STATE_FILE)"

clean: ## Remove saved state
	rm -f $(STATE_FILE) $(STATE_FILE).tmp
	@echo "✅ State cleaned"

seed-fixture: ## Local chain fixture (verify/seed/seed.sh)
	./verify/seed/seed.sh

wallet-lgx: ## Patched logos_execution_zone .lgx (verify/lib/build-wallet-lgx.sh)
	./verify/lib/build-wallet-lgx.sh

check-terminology: ## Role-terminology gate
	chmod +x verify/check-terminology.sh
	./verify/check-terminology.sh

check-links: ## Relative-link checker for living markdown
	chmod +x verify/lib/check-relative-links.sh
	./verify/lib/check-relative-links.sh

check: check-terminology check-links ## Terminology and relative links

test-fixture-vault: ## Component tests for fixture.sh / vault_scan
	chmod +x verify/lib/test_fixture_vault.sh
	./verify/lib/test_fixture_vault.sh

verify-module-local: ## Flow A (module only) local happy path (MODE=module verify/e2e.sh local run)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/module-e2e.sh
	MODE=module ./verify/e2e.sh local run

verify-module-local-provider-close: ## Thin provider-close cell (CLOSE_ROLE=provider)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/module-e2e.sh
	MODE=module CLOSE_ROLE=provider ./verify/e2e.sh local run

verify-module-local-provider-close-privacy: ## PF provider-close (CLOSE_ROLE=provider OWNER_PRIVACY=1)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/module-e2e.sh
	MODE=module CLOSE_ROLE=provider OWNER_PRIVACY=1 ./verify/e2e.sh local run

verify-module-local-privacy: ## Owner privacy (OWNER_PRIVACY=1) PseudonymousFunding lifecycle on localnet
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/module-e2e.sh
	MODE=module OWNER_PRIVACY=1 ./verify/e2e.sh local run

verify-module-local-provider-privacy: ## Provider privacy (PROVIDER_PRIVACY=1) shielded claim on localnet
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/module-e2e.sh
	MODE=module PROVIDER_PRIVACY=1 ./verify/e2e.sh local run

verify-module-local-close-negatives: ## Asserted close/create reject tokens on localnet
	chmod +x verify/module-close-negatives.sh
	./verify/module-close-negatives.sh

verify-module-testnet: ## Flow A (module only) testnet happy path
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/module-e2e.sh
	MODE=module ./verify/e2e.sh testnet run

verify-store-local: ## Store integration local dual-host E2E (verify/e2e.sh local run)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/store/*.py
	MODE=store ./verify/e2e.sh local run

verify-store-local-owner-privacy: ## Store local OWNER_PRIVACY=1 (PseudonymousFunding vault, public provider)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/store/*.py
	MODE=store OWNER_PRIVACY=1 ./verify/e2e.sh local run

verify-store-local-provider-privacy: ## Store local PROVIDER_PRIVACY=1 (public vault, private provider claim)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/store/*.py
	MODE=store PROVIDER_PRIVACY=1 ./verify/e2e.sh local run

verify-store-local-full-privacy: ## Store local OWNER_PRIVACY=1 PROVIDER_PRIVACY=1
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/store/*.py
	MODE=store OWNER_PRIVACY=1 PROVIDER_PRIVACY=1 ./verify/e2e.sh local run

verify-store-testnet: ## Store integration public sequencer E2E (verify/e2e.sh testnet run)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh verify/store/*.py
	MODE=store ./verify/e2e.sh testnet run

verify-store-local-lifecycle: ## Maintainer: two Store runs on one ledger (verify/store/store-lifecycle.sh)
	chmod +x verify/store/store-lifecycle.sh
	./verify/store/store-lifecycle.sh

debug-sequencer-latency: ## Probe sequencer RPC latency and block production (verify/testnet/sequencer_latency_probe.py)
	chmod +x verify/testnet/sequencer_latency_probe.py
	REPO="$(CURDIR)" python3 verify/testnet/sequencer_latency_probe.py

full-reset-localnet: ## Reseed funded baseline + snapshot (verify/e2e.sh local prepare)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh
	FULL_RESET=1 SKIP_BUILD=1 ./verify/e2e.sh local prepare

prepare-localnet: ## Restore funded baseline (verify/e2e.sh local prepare)
	chmod +x verify/e2e.sh verify/lifecycle.sh verify/fixture.sh
	SKIP_BUILD=1 ./verify/e2e.sh local prepare

deploy-testnet: ## One-time program deploy (Part B)
	chmod +x verify/testnet/deploy-testnet.sh
	./verify/testnet/deploy-testnet.sh

bootstrap-testnet: ## One-time fixture bootstrap (Part B; verify/testnet/bootstrap-testnet.sh)
	chmod +x verify/testnet/bootstrap-testnet.sh verify/testnet/testnet-common.sh
	./verify/testnet/bootstrap-testnet.sh

bootstrap-testnet-module: ## One-time fixture for module-only testnet (reuses testnet owner)
	chmod +x verify/testnet/bootstrap-testnet-module.sh verify/testnet/testnet-common.sh
	./verify/testnet/bootstrap-testnet-module.sh
