.PHONY: idud idud-grow build test clean help lint fmt check-format cache-status preflight

# Default target
.DEFAULT_GOAL := help

# Color output
BOLD := $(shell tput bold)
GREEN := $(shell tput setaf 2)
YELLOW := $(shell tput setaf 3)
BLUE := $(shell tput setaf 4)
RESET := $(shell tput sgr0)

# Configuration
REPOS ?= 100
CONCURRENT ?= 10
BATCH_SIZE ?= 2
DATALAKE ?= ./data/training_datalake
DURATION_MINUTES ?=
MAX_REPOS ?=

# ============================================================================
# PRIMARY TRAINING TARGETS
# ============================================================================

## idud - Build and run the server
idud:
	@echo "$(BOLD)$(BLUE)🚀 Building idud (release mode)...$(RESET)"
	@cargo build --release 2>&1 | grep -E "(Compiling idud|Finished|error)" || true
	@echo "$(BOLD)$(GREEN)✓ Build complete$(RESET)"
	@echo ""
	@echo "$(BOLD)$(BLUE)🚀 Starting idud server...$(RESET)"
	@echo "$(GREEN)   📡 Running at http://127.0.0.1:3000$(RESET)"
	@echo "$(YELLOW)   Press Ctrl+C to stop$(RESET)"
	@echo ""
	@cargo run --release -- serve

## idud-grow - Train on repositories (default: 100 repos, 10 concurrent, no limit)
idud-grow: preflight
	@echo "$(BOLD)$(BLUE)🌱 Starting training pipeline...$(RESET)"
	@echo "$(GREEN)   Repos: $(REPOS)$(RESET)"
	@echo "$(GREEN)   Concurrent agents: $(CONCURRENT)$(RESET)"
	@echo "$(GREEN)   Batch size: $(BATCH_SIZE)$(RESET)"
	@echo "$(GREEN)   Output: $(DATALAKE)$(RESET)"
	$(if $(DURATION_MINUTES),@echo "$(YELLOW)   Duration limit: $(DURATION_MINUTES) minutes$(RESET)",)
	$(if $(MAX_REPOS),@echo "$(YELLOW)   Max repos: $(MAX_REPOS)$(RESET)",)
	@echo ""
	@mkdir -p $(DATALAKE)
	@echo "$(BOLD)$(BLUE)📦 Cache status before training:$(RESET)"
	@cargo run --release --quiet -- cache-status --datalake $(DATALAKE) 2>/dev/null || echo "   (Cache empty - first run)"
	@echo ""
	@echo "$(BOLD)$(BLUE)📊 Training in progress (idempotent - skips already-processed issues)...$(RESET)"
	@cargo run --release -- train \
		--repos $(REPOS) \
		--concurrent $(CONCURRENT) \
		--batch-size $(BATCH_SIZE) \
		--datalake $(DATALAKE) \
		$(if $(DURATION_MINUTES),--duration-minutes $(DURATION_MINUTES),) \
		$(if $(MAX_REPOS),--max-repos $(MAX_REPOS),)
	@echo ""
	@echo "$(BOLD)$(BLUE)📦 Cache status after training:$(RESET)"
	@cargo run --release --quiet -- cache-status --datalake $(DATALAKE) 2>/dev/null
	@echo ""
	@echo "$(BOLD)$(GREEN)✓ Training complete!$(RESET)"
	@echo "$(YELLOW)   Results saved to: $(DATALAKE)$(RESET)"
	@echo ""

## preflight - Run pre-flight validation checks
preflight:
	@bash scripts/preflight.sh

## cache-status - Show training cache status
cache-status:
	@cargo run --release -- cache-status --datalake $(DATALAKE)

# ============================================================================
# UTILITY TARGETS
# ============================================================================

## build - Build release binary
build:
	@echo "$(BOLD)$(BLUE)🔨 Building release binary...$(RESET)"
	@cargo build --release
	@echo "$(BOLD)$(GREEN)✓ Build complete$(RESET)"

## test - Run all tests
test:
	@echo "$(BOLD)$(BLUE)🧪 Running tests...$(RESET)"
	@cargo test --all
	@echo "$(BOLD)$(GREEN)✓ All tests passed$(RESET)"

## lint - Run clippy linter
lint:
	@echo "$(BOLD)$(BLUE)📝 Running clippy...$(RESET)"
	@cargo clippy --all-targets --all-features
	@echo "$(BOLD)$(GREEN)✓ No lint issues found$(RESET)"

## fmt - Format code with rustfmt
fmt:
	@echo "$(BOLD)$(BLUE)✨ Formatting code...$(RESET)"
	@cargo fmt --all
	@echo "$(BOLD)$(GREEN)✓ Code formatted$(RESET)"

## check-format - Check code formatting
check-format:
	@echo "$(BOLD)$(BLUE)🔍 Checking code format...$(RESET)"
	@cargo fmt --all -- --check
	@echo "$(BOLD)$(GREEN)✓ Code format is correct$(RESET)"

## clean - Remove build artifacts
clean:
	@echo "$(BOLD)$(BLUE)🧹 Cleaning build artifacts...$(RESET)"
	@cargo clean
	@echo "$(BOLD)$(GREEN)✓ Clean complete$(RESET)"

## help - Show this help message
help:
	@echo "$(BOLD)$(BLUE)idud - Contract Ledger Training System$(RESET)"
	@echo ""
	@echo "$(BOLD)PRIMARY TARGETS:$(RESET)"
	@grep -E "^## [a-z-]+" Makefile | sed 's/## /  /' | sed 's/ -//' | awk '{printf "  $(GREEN)%-20s$(RESET) %s\n", $$1, substr($$0, index($$0,$$2))}'
	@echo ""
	@echo "$(BOLD)UTILITY TARGETS:$(RESET)"
	@grep -E "^## (build|test|lint|fmt|check-format|clean|cache-status|preflight)" Makefile | sed 's/## /  /' | sed 's/ -//' | awk '{printf "  $(GREEN)%-20s$(RESET) %s\n", $$1, substr($$0, index($$0,$$2))}'
	@echo ""
	@echo "$(BOLD)ENVIRONMENT VARIABLES:$(RESET)"
	@echo "  $(YELLOW)REPOS=$(REPOS)$(RESET)              - Number of repos for idud-grow (default: 100)"
	@echo "  $(YELLOW)CONCURRENT=$(CONCURRENT)$(RESET)           - Concurrent agents (default: 10)"
	@echo "  $(YELLOW)BATCH_SIZE=$(BATCH_SIZE)$(RESET)             - Batch size per agent (default: 2)"
	@echo "  $(YELLOW)DURATION_MINUTES=$(DURATION_MINUTES)$(RESET)       - Max runtime in minutes (optional)"
	@echo "  $(YELLOW)MAX_REPOS=$(MAX_REPOS)$(RESET)             - Max repos to process (optional)"
	@echo "  $(YELLOW)DATALAKE=$(DATALAKE)$(RESET)  - Training output directory"
	@echo ""
	@echo "$(BOLD)EXAMPLES:$(RESET)"
	@echo "  $(YELLOW)make idud$(RESET)                     - Start the server"
	@echo "  $(YELLOW)make idud-grow$(RESET)               - Train on 100 repos with 10 concurrent agents"
	@echo "  $(YELLOW)make idud-grow REPOS=50 CONCURRENT=5$(RESET)  - Custom training parameters"
	@echo "  $(YELLOW)make idud-grow DURATION_MINUTES=120$(RESET)   - Train for max 2 hours"
	@echo "  $(YELLOW)make idud-grow MAX_REPOS=50$(RESET)          - Train on max 50 new repos"
	@echo "  $(YELLOW)make cache-status$(RESET)             - Show what's been trained so far"
	@echo "  $(YELLOW)make preflight$(RESET)                - Run pre-flight checks before training"
	@echo ""
	@echo "$(BOLD)IDEMPOTENCY:$(RESET)"
	@echo "  Run 'make idud-grow' as many times as you want!"
	@echo "  Already-processed repos/issues are cached and skipped."
	@echo "  Safe to run through crashes and code updates."
	@echo ""
	@echo "$(BOLD)SCALING:$(RESET)"
	@echo "  Run preflight checks: $(YELLOW)make preflight$(RESET)"
	@echo "  Start 2-hour training: $(YELLOW)make idud-grow DURATION_MINUTES=120$(RESET)"
	@echo "  Monitor progress:      $(YELLOW)make cache-status$(RESET)"
	@echo "  Scale over weeks:      $(YELLOW)make idud-grow REPOS=500 CONCURRENT=20$(RESET)"
	@echo ""
