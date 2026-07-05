.PHONY: idud idud-grow datalake-grow datalake-status build test clean help lint fmt check-format cache-status preflight

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

## datalake-grow - Grow datalake by ingesting repositories from registry
datalake-grow: build
	@echo "$(BOLD)$(BLUE)🌱 Growing data lake from repository registry...$(RESET)"
	@echo "$(GREEN)   Registry: data/repos_to_ingest.json$(RESET)"
	@echo "$(GREEN)   Output: data/$(RESET)"
	$(if $(MAX_REPOS),@echo "$(YELLOW)   Max repos: $(MAX_REPOS)$(RESET)",)
	$(if $(DURATION_MINUTES),@echo "$(YELLOW)   Timeout: $(DURATION_MINUTES) minutes$(RESET)",)
	@echo ""
	@mkdir -p data
	@echo "$(BOLD)$(BLUE)📋 Ingestion log status:$(RESET)"
	@test -f data/ingestion-log.json && echo "   Found existing log:" && jq '.[] | "\(.repo_name): \(.status)"' data/ingestion-log.json 2>/dev/null | head -5 || echo "   (First run - no previous log)"
	@echo ""
	@echo "$(BOLD)$(BLUE)📦 Starting repository ingestion...$(RESET)"
	@cargo run --release -- grow-datalake \
		--registry data/repos_to_ingest.json \
		--output data \
		--skip-ingested \
		$(if $(MAX_REPOS),--max-repos $(MAX_REPOS),) \
		$(if $(DURATION_MINUTES),--timeout-minutes $(DURATION_MINUTES),)
	@echo ""
	@echo "$(BOLD)$(BLUE)📊 Ingestion complete! Checking results...$(RESET)"
	@if [ -f data/ingestion-log.json ]; then \
		echo "$(BOLD)$(GREEN)✓ Ingestion log updated$(RESET)"; \
		jq 'length' data/ingestion-log.json | xargs -I {} echo "   Total ingested: {} repos"; \
	fi
	@if [ -f DATALAKE_LOG.md ]; then \
		echo "$(BOLD)$(GREEN)✓ Progress logged to DATALAKE_LOG.md$(RESET)"; \
	fi
	@echo ""

## datalake-status - Show current data lake ingestion status
datalake-status:
	@echo "$(BOLD)$(BLUE)📊 Data Lake Status$(RESET)"
	@echo ""
	@if [ -f data/ingestion-log.json ]; then \
		echo "$(BOLD)Ingestion Log:$(RESET)"; \
		jq 'group_by(.status) | map({status: .[0].status, count: length})' data/ingestion-log.json; \
		echo ""; \
		echo "$(BOLD)Recent Ingestions:$(RESET)"; \
		jq -r '.[] | "\(.repo_name): \(.status) (\(.files_processed) files, \(.signatories) sig, \(.contracts) contracts)"' data/ingestion-log.json | head -10; \
	else \
		echo "$(YELLOW)No ingestion log found. Run 'make datalake-grow' to start.$(RESET)"; \
	fi
	@echo ""
	@if [ -f DATALAKE_LOG.md ]; then \
		echo "$(BOLD)Latest Progress (from DATALAKE_LOG.md):$(RESET)"; \
		head -20 DATALAKE_LOG.md; \
	fi
	@echo ""

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

## cache-status - Show training cache status
cache-status:
	@cargo run --release -- cache-status --datalake $(DATALAKE)

## help - Show this help message
help:
	@echo "$(BOLD)$(BLUE)idud - Contract Ledger Training System (Copilot CLI)$(RESET)"
	@echo ""
	@echo "$(BOLD)PRIMARY TARGETS:$(RESET)"
	@grep -E "^## [a-z-]+" Makefile | sed 's/## /  /' | sed 's/ -//' | awk '{printf "  $(GREEN)%-20s$(RESET) %s\n", $$1, substr($$0, index($$0,$$2))}'
	@echo ""
	@echo "$(BOLD)UTILITY TARGETS:$(RESET)"
	@grep -E "^## (build|test|lint|fmt|check-format|clean|cache-status|datalake-status|preflight)" Makefile | sed 's/## /  /' | sed 's/ -//' | awk '{printf "  $(GREEN)%-20s$(RESET) %s\n", $$1, substr($$0, index($$0,$$2))}'
	@echo ""
	@echo "$(BOLD)REQUIREMENTS:$(RESET)"
	@echo "  $(YELLOW)Copilot CLI$(RESET)              - Install from: https://github.com/github/gh-copilot"
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
	@echo "  $(BOLD)$(GREEN)Server & Web UI:$(RESET)"
	@echo "    $(YELLOW)make idud$(RESET)                     - Start the server"
	@echo ""
	@echo "  $(BOLD)$(GREEN)Training Validation (AI-assisted):$(RESET)"
	@echo "    $(YELLOW)make idud-grow$(RESET)               - Train on 100 repos with 10 concurrent agents"
	@echo "    $(YELLOW)make idud-grow REPOS=50 CONCURRENT=5$(RESET)  - Custom training parameters"
	@echo "    $(YELLOW)make idud-grow MAX_REPOS=50$(RESET)          - Train on max 50 new repos"
	@echo "    $(YELLOW)make cache-status$(RESET)             - Show what's been trained so far"
	@echo ""
	@echo "  $(BOLD)$(GREEN)Data Lake Growth (AST-based):$(RESET)"
	@echo "    $(YELLOW)make datalake-grow$(RESET)           - Ingest 24 curated repos (AST-only, no AI)"
	@echo "    $(YELLOW)make datalake-grow MAX_REPOS=5$(RESET)      - Ingest first 5 repos"
	@echo "    $(YELLOW)make datalake-grow DURATION_MINUTES=30$(RESET) - Ingest for max 30 minutes"
	@echo "    $(YELLOW)make datalake-status$(RESET)         - Show ingestion progress and logs"
	@echo ""
	@echo "$(BOLD)IDEMPOTENCY:$(RESET)"
	@echo "  Run commands as many times as you want!"
	@echo "  Already-processed repos are skipped."
	@echo "  Safe to run through crashes and code updates."
	@echo ""
	@echo "$(BOLD)SCALING:$(RESET)"
	@echo "  $(BOLD)Training (with AI validation):$(RESET)"
	@echo "    Start 2-hour training: $(YELLOW)make idud-grow DURATION_MINUTES=120$(RESET)"
	@echo "    Monitor progress:      $(YELLOW)make cache-status$(RESET)"
	@echo ""
	@echo "  $(BOLD)Data Lake (AST-based collection):$(RESET)"
	@echo "    Grow datalake: $(YELLOW)make datalake-grow$(RESET)"
	@echo "    Check status:  $(YELLOW)make datalake-status$(RESET)"
	@echo ""
