# Synthetic Understanding for idud

Generated at: 2026-07-13T00:56:17.996538158+00:00

idud appears to be a product-oriented repository with 56 top-level areas, 2 inferred journey candidates, and 4 tests. Its structure suggests domains such as documentation and operating workflows, quality assurance and regression coverage, application logic and integrations.

## Customer journeys
- core workflow
  - Evidence: test_rust_regex.sh, test_ai_linker_subset.sh
  - Related files: src/analysis/ai_linker.rs
- data operations
  - Evidence: test_validator_integration.md, test_ai_linker_waymark.sh
  - Related files: src/training/validator.rs, src/training/waymark_validator.rs, src/analysis/ai_linker.rs, tests/pr_prediction_waymark.rs, tests/waymark_integration.rs

## Test inventory
- ai linker subset sh [unit]
  - Path: test_ai_linker_subset.sh
  - Linked files: src/analysis/ai_linker.rs
- ai linker waymark sh [unit]
  - Path: test_ai_linker_waymark.sh
  - Linked files: src/analysis/ai_linker.rs, src/training/waymark_validator.rs, tests/pr_prediction_waymark.rs, tests/waymark_integration.rs
- rust regex sh [unit]
  - Path: test_rust_regex.sh
- validator md [integration]
  - Path: test_validator_integration.md
  - Linked files: src/training/validator.rs, src/training/waymark_validator.rs

## Top-level directories
- src: 35 files
- data: 25 files
- tests: 6 files
- .env.example: 1 files
- .github: 1 files
- .gitignore: 1 files
- AI_LINKER_DOCUMENTATION.md: 1 files
- AI_LINKER_FINAL_REPORT.txt: 1 files
- AI_LINKER_OPTIMIZATION_REPORT.md: 1 files
- AST_ANALYZER_IMPLEMENTATION.md: 1 files
- COMPLETION_REPORT.md: 1 files
- CONTRIBUTING.md: 1 files
- CONTRIBUTING_TO_TRAINING.md: 1 files
- Cargo.lock: 1 files
- Cargo.toml: 1 files
- DATALAKE_LOG.md: 1 files
- DELIVERABLES.md: 1 files
- FLEET_COMPLETION_REPORT.md: 1 files
- HAIKU_INTEGRATION_EXAMPLE.md: 1 files
- HAIKU_PREDICTION_PROMPT.md: 1 files
- IMPLEMENTATION_SUMMARY.md: 1 files
- IMPORT_FEATURE_SUMMARY.md: 1 files
- KB_IMPORT_SUMMARY.md: 1 files
- KNOWLEDGE_BASE_FEATURE.md: 1 files
- LICENSE: 1 files
- Makefile: 1 files
- Makefile.md: 1 files
- PRE_FLIGHT_REVIEW.md: 1 files
- README.md: 1 files
- READY_TO_GROW.md: 1 files
- REPO_ORCHESTRATOR_GUIDE.md: 1 files
- SCALING_COMPLETE_REPORT.md: 1 files
- SETUP.md: 1 files
- TASK_COMPLETION_AI_LINKER.md: 1 files
- TASK_COMPLETION_REPORT.md: 1 files
- TRAINING_DATALAKE_SCHEMA.md: 1 files
- TRAINING_DISCOVERY.md: 1 files
- TRAINING_DISCOVERY_IMPLEMENTATION.md: 1 files
- TRAINING_IDEMPOTENCY.md: 1 files
- TRAINING_METHODOLOGY.md: 1 files
- TRAINING_ORCHESTRATOR.md: 1 files
- TRAINING_RESULTS.md: 1 files
- TRAINING_VALIDATION.md: 1 files
- TRAINING_VALIDATION_REPORT.md: 1 files
- VISUALIZATION.md: 1 files
- WAYMARK_INGESTION_LOG.md: 1 files
- WAYMARK_SUCCESS_REPORT.md: 1 files
- idud.db: 1 files
- package.json: 1 files
- scripts: 1 files
- test_ai_linker_subset.sh: 1 files
- test_ai_linker_waymark.sh: 1 files
- test_rust_regex.sh: 1 files
- test_validator_integration.md: 1 files
- training: 1 files
- validate_ai_linker.sh: 1 files

## Inferred domains
- documentation and operating workflows
- quality assurance and regression coverage
- application logic and integrations

## Notable files
- README.md
- training/README.md
- data/training_datalake/README.md
- package.json

## Synthetic brief
Synthetic understanding for idud:
- Primary areas:
  - src: 35 files
  - data: 25 files
  - tests: 6 files
  - .env.example: 1 files
  - .github: 1 files
  - .gitignore: 1 files
- Inferred domains:
  - documentation and operating workflows
  - quality assurance and regression coverage
  - application logic and integrations
- Customer journeys:
  - core workflow (evidence: 2)
  - data operations (evidence: 2)
- Test inventory:
  - ai linker subset sh [unit] -> test_ai_linker_subset.sh
  - ai linker waymark sh [unit] -> test_ai_linker_waymark.sh
  - rust regex sh [unit] -> test_rust_regex.sh
  - validator md [integration] -> test_validator_integration.md
- Dependency hints:
  - src/training/repo_understanding.rs -> ./lib (import)
  - src/analysis/extractors.rs -> external (import)
  - src/analysis/ast_analyzer.rs -> external (import)
  - src/analysis/ast_analyzer.rs -> ./utils (import)
  - tests/integration_ast_analysis.rs -> external (import)
  - test_ai_linker_subset.sh -> src/analysis/ai_linker.rs (test-link)
  - test_validator_integration.md -> src/training/validator.rs (test-link)
  - test_validator_integration.md -> src/training/waymark_validator.rs (test-link)