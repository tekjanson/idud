# AI-Augmented Dependency Linking for idud

## Overview

The AI Linker module (`src/analysis/ai_linker.rs`) uses Copilot CLI to infer semantic dependencies that AST analysis misses. This includes:

- Duck typing patterns
- Shared protocols and interfaces  
- Implicit type conversions
- Behavioral contracts
- Architectural patterns

## Architecture

### Core Components

1. **AILinker struct**: Main linking orchestrator
   - `link_files()`: Process signatories in batches, call Copilot CLI
   - Batching strategy: 5-10 files per call (configurable)
   - Confidence range: 0.40-0.75 (lower than AST to reflect uncertainty)

2. **AILinkerConfig**: Controls linking behavior
   - `batch_size`: Files per Copilot invocation (default: 8)
   - `min_confidence`: Minimum confidence threshold (default: 0.40)
   - `max_confidence`: Maximum confidence for AI links (default: 0.65)
   - `verbose`: Enable debug logging

3. **Token Budget**: Each batch costs ~400 tokens
   - File list: ~100 tokens
   - Inference: ~300 tokens max
   - For 100 files: ~5000 tokens total (5% of 50k/month budget)

### Prompting Strategy

The AI linker uses a minimal, focused prompt:

```
You are analyzing source files for implicit semantic dependencies.

FILES TO ANALYZE:
[compact file listing]

TASK: Identify pairs of files that likely interact through duck typing, 
shared protocols, or implicit patterns.

RESPOND with ONLY a JSON array. NO EXPLANATION, NO MARKDOWN.
```

This saves ~90% of tokens vs verbose prompts by:
- Compact file format (one line per file)
- Minimal system prompt (~5 tokens vs 400)
- JSON-only output (no explanations)
- Discarding verbose reasoning

## Integration

### Phase Integration

AI linking runs as an optional post-processing step after broad sweep ingestion:

1. **PHASE 3.1 (Broad Sweep)**: Deterministic signatory registration
2. **PHASE 3.2 (AI Linking)**: Optional semantic dependency inference

### Environment Variable

Control AI linking with `ENABLE_AI_LINKING`:

```bash
# Enable (default)
ENABLE_AI_LINKING=true cargo run

# Disable
ENABLE_AI_LINKING=false cargo run
```

Defaults to `true` if not set.

### Usage in Code

```rust
use idud::{AILinker, AILinkerConfig, RepositoryTraverser, RepositoryIngestionConfig};

// After ingestion
let config = AILinkerConfig::default();
let mut linker = AILinker::new(config);

let ai_contracts = linker.link_files(
    &signatories,           // All extracted signatories
    &existing_contracts,    // Pre-existing contracts from AST
)?;
```

## Contract Confidence Scores

AI-inferred contracts use lower confidence than AST contracts:

| Source | Confidence Range | Reasoning |
|--------|------------------|-----------|
| AST (explicit imports) | 0.90-1.00 | Deterministic parsing |
| AI semantic | 0.40-0.75 | Probabilistic inference |
| AI default | 0.525 | Midpoint between min/max |

## Performance Characteristics

### Token Efficiency

```
Scenario: 100 files
Batch size: 8 files per call
Total batches: ~13

Token Usage:
- Per batch: ~400 tokens (100 for list + 300 for inference)
- Total: ~5200 tokens for 100 files
- Per file: ~52 tokens

Budget Impact:
- 50,000 tokens/month budget
- 100-file analysis: ~5% of monthly budget
- Recommended for training on 1000s of repos
```

### Computational Efficiency

- Local AST parsing: O(1) per file (done once)
- Batching: O(n/batch_size) Copilot calls
- Response parsing: O(n) local, free
- No network I/O beyond Copilot CLI

## Response Format

Copilot returns JSON with inferred pairs:

```json
[
  {"from": "src/main.rs", "to": "src/lib.rs", "reason": "Both implement Stream protocol"},
  {"from": "src/lib.rs", "to": "src/utils.rs", "reason": "file2 creates instances of file3 types"},
  {"from": "src/handlers.rs", "to": "src/middleware.rs", "reason": "Duck-typed middleware pattern"}
]
```

Response parsing:
1. Extract JSON array from response
2. Filter pairs to only those in current batch
3. Lookup signatories by label (from/to)
4. Create Contract objects with AI confidence
5. Skip duplicates (already in existing contracts)

## Validation & Safety

### Deduplication

AI linking avoids creating duplicate contracts:
- Checks against all existing contracts
- Uses (principal_id, guarantor_id) tuple as key
- Silently skips if contract already exists

### Error Handling

If AI linking fails:
1. Logs warning with error message
2. Continues with partial results
3. Never fails the entire ingestion
4. Returns gracefully to broad sweep orchestrator

### Testing

Unit tests validate:
- Batch formatting (`test_format_batch_for_analysis`)
- Prompt generation (`test_build_linking_prompt`)
- Response parsing (`test_parse_linking_response_*`)
- Config defaults (`test_ai_linker_config_defaults`)
- Deduplication (`test_build_contract_set`)

Run tests: `cargo test analysis::ai_linker`

## Typical Workflow

```bash
# 1. Clone and setup
git clone https://github.com/tekjanson/idud.git
cd idud

# 2. Build (compiles AI linker module)
cargo build --release

# 3. Run with AI linking enabled (default)
ENABLE_AI_LINKING=true cargo run -- --repo https://github.com/example/repo

# 4. View results
# - Deterministic contracts from AST
# - Semantic contracts from AI (confidence 0.40-0.75)
# - Combined in contract_ledger

# 5. Disable if tokens are high
ENABLE_AI_LINKING=false cargo run -- --repo https://github.com/example/repo
```

## Future Enhancements

1. **Adaptive batching**: Adjust batch size based on file complexity
2. **Confidence tuning**: Machine-learning on validation metrics
3. **Protocol detection**: Specialized prompts for trait/interface patterns
4. **Incremental linking**: Only re-link modified files
5. **Cache warming**: Store common patterns locally

## References

- AST analyzer: `src/analysis/ast_analyzer.rs`
- Token meter: `src/training/token_meter.rs`
- Broad sweep orchestrator: `src/pipelines/broad_sweep.rs`
- Contract types: `src/types.rs` (Contract, ClauseType)
