# lez-payment-streams — SPEL Program
#
# Quick start:
#   make build idl deploy setup
#   make cli ARGS="<command> --arg1 value1"


SHELL := /bin/bash
STATE_FILE := .lez_payment_streams-state
IDL_FILE := lez-payment-streams-idl.json
PROGRAMS_DIR := methods/guest/target/riscv32im-risc0-zkvm-elf/docker
PROGRAM_BIN := $(PROGRAMS_DIR)/lez_payment_streams.bin

# Load saved state if it exists
-include $(STATE_FILE)

define save_var
	@grep -v '^$(1)=' $(STATE_FILE) 2>/dev/null > $(STATE_FILE).tmp || true
	@echo '$(1)=$(2)' >> $(STATE_FILE).tmp
	@mv $(STATE_FILE).tmp $(STATE_FILE)
endef

.PHONY: help build idl cli deploy setup program-id status clean seed-fixture wallet-lgx verify-step10a verify-step10b verify-step11a verify-step11d verify-step12 verify-step13 verify-step17 prepare-localnet

help: ## Show this help
	@echo "lez-payment-streams — SPEL Program"
	@echo ""
	@echo "  make build       Build the guest binary (needs risc0 toolchain)"
	@echo "  make idl         Generate IDL from program source"
	@echo "  make cli ARGS=   Run the IDL-driven CLI (pass args via ARGS=)"
	@echo "  make deploy      Deploy program to sequencer"
	@echo "  make setup       Create accounts needed for the program"
	@echo "  make program-id  Show ProgramId for built binary"
	@echo "  make status      Show saved state and binary info"
	@echo "  make seed-fixture Run Step 10a localnet seed script"
	@echo "  make wallet-lgx    Build Step 10b patched logos_execution_zone .lgx"
	@echo "  make verify-step10a Run Step 10a DoD script"
	@echo "  make verify-step10b Run Step 10b DoD script"
	@echo "  make verify-step11a Run Step 11a DoD script"
	@echo "  make verify-step11d Run Step 11d DoD script (LEZ 510 wallet)"
	@echo "  make verify-step12  Run Step 12 DoD script"
	@echo "  make verify-step13  Run Step 13 DoD script"
	@echo "  make verify-step17  Run Step 17 local E2E demo script"
	@echo "  make prepare-localnet  Step 17b restore + create stream (see demo-localnet-prepare.sh)"
	@echo "  make clean       Remove saved state"
	@echo ""
	@echo "Example:"
	@echo "  make build idl deploy"
	@echo "  make cli ARGS=\"--help\""
	@echo "  make cli ARGS=\"-p $(PROGRAM_BIN) <command> --arg1 value1\""

build: ## Build the guest binary
	cargo risczero build --manifest-path methods/guest/Cargo.toml
	@echo ""
	@echo "✅ Guest binary built: $(PROGRAM_BIN)"
	@ls -la $(PROGRAM_BIN) 2>/dev/null || true

idl: ## Generate IDL JSON from program source
	cargo run --manifest-path examples/Cargo.toml --bin generate_idl > $(IDL_FILE)
	@echo "✅ IDL written to $(IDL_FILE)"

cli: ## Run the IDL-driven CLI (ARGS="...")
	cargo run --manifest-path examples/Cargo.toml --bin lez_payment_streams_cli -- -i $(IDL_FILE) $(ARGS)

deploy: ## Deploy program to sequencer (pinned LEZ wallet; set LEE_WALLET_HOME_DIR)
	@test -n "$$LEE_WALLET_HOME_DIR" || (echo "ERROR: set LEE_WALLET_HOME_DIR (see docs/step10a-local-chain-fixture.md)"; exit 1)
	@test -f "$(PROGRAM_BIN)" || (echo "ERROR: Binary not found. Run 'make build' first."; exit 1)
	wallet deploy-program $(PROGRAM_BIN)
	@echo "✅ Program deployed"

program-id: ## Show ProgramId for built binary
	cargo run --manifest-path examples/Cargo.toml --bin lez_payment_streams_cli -- -i $(IDL_FILE) program-id $(PROGRAM_BIN)

setup: ## Create accounts needed for the program
	@echo "Creating signer account..."
	$(eval SIGNER_ID := $(shell wallet account new public 2>&1 | sed -n 's/.*Public\/\([A-Za-z0-9]*\).*/\1/p'))
	@echo "Signer: $(SIGNER_ID)"
	$(call save_var,SIGNER_ID,$(SIGNER_ID))
	@echo ""
	@echo "✅ Account saved to $(STATE_FILE)"

status: ## Show saved state and binary info
	@echo "lez-payment-streams Status"
	@echo "──────────────────────────────────────"
	@if [ -f "$(STATE_FILE)" ]; then cat $(STATE_FILE); else echo "(no state — run 'make setup')"; fi
	@echo ""
	@echo "Binaries:"
	@ls -la $(PROGRAM_BIN) 2>/dev/null || echo "  lez_payment_streams.bin: NOT BUILT (run 'make build')"
	@echo ""
	@echo "IDL:"
	@ls -la $(IDL_FILE) 2>/dev/null || echo "  $(IDL_FILE): NOT GENERATED (run 'make idl')"

clean: ## Remove saved state
	rm -f $(STATE_FILE) $(STATE_FILE).tmp
	@echo "✅ State cleaned"

seed-fixture: ## Step 10a local chain fixture (scripts/seed-localnet-fixture.sh)
	./scripts/seed-localnet-fixture.sh

wallet-lgx: ## Step 10b patched logos_execution_zone .lgx (scripts/build-wallet-lgx.sh)
	./scripts/build-wallet-lgx.sh

verify-step10a: ## Step 10a definition of done (scripts/verify-step10a-dod.sh)
	./scripts/verify-step10a-dod.sh

verify-step10b: ## Step 10b definition of done (scripts/verify-step10b-dod.sh)
	./scripts/verify-step10b-dod.sh

verify-step11a: ## Step 11a definition of done (scripts/verify-step11a-dod.sh)
	chmod +x scripts/verify-step11a-dod.sh
	./scripts/verify-step11a-dod.sh

verify-step11d: ## Step 11d definition of done (scripts/verify-step11d-dod.sh)
	chmod +x scripts/verify-step11d-dod.sh scripts/deploy-program-logoscore.sh scripts/ensure-scaffold-lez-layout.sh
	./scripts/verify-step11d-dod.sh

verify-step12: ## Step 12 definition of done (scripts/verify-step12-dod.sh)
	chmod +x scripts/verify-step12-dod.sh scripts/step12-topup-and-prepare.sh scripts/ensure-scaffold-lez-layout.sh
	./scripts/verify-step12-dod.sh

verify-step13: ## Step 13 definition of done (scripts/verify-step13-dod.sh)
	chmod +x scripts/verify-step13-dod.sh scripts/step12-topup-and-prepare.sh
	./scripts/verify-step13-dod.sh

verify-step17: ## Step 17 local dual-host Store E2E (scripts/demo-e2e-local.sh)
	chmod +x scripts/demo-e2e-local.sh scripts/e2e/*.py scripts/demo-localnet-prepare.sh
	./scripts/demo-e2e-local.sh

prepare-localnet: ## Step 17b restore baseline + create stream (scripts/demo-localnet-prepare.sh)
	chmod +x scripts/demo-localnet-prepare.sh scripts/prefund-localnet.sh scripts/restore-localnet.sh scripts/create-localnet-stream-fixture.sh
	./scripts/demo-localnet-prepare.sh
