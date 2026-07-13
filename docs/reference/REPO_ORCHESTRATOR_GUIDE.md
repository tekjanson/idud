# Repository Ingestion Orchestrator: Scaling idud Across 100+ Repos

## Overview

The Repository Ingestion Orchestrator scales contract discovery across multiple codebases for training data collection. It uses **AST-based analysis only** (no AI linking) for fast, deterministic ingestion.

**Key Benefits:**
- ✅ Ingest 20+ repos in ~10 minutes (AST-only, no AI latency)
- ✅ Idempotent: skip already-ingested repos automatically
- ✅ Production-ready: error handling, timeouts, progress logging
- ✅ Git-tracked: progress logged to `DATALAKE_LOG.md` for visibility
- ✅ Scalable: from 20 to 100+ repos with simple flags

## Quick Start

### Ingest the Curated Registry (24 diverse repos)

```bash
# Ingest first 5 repos to test
make datalake-grow MAX_REPOS=5

# Ingest all 24 repos (takes ~10-15 minutes)
make datalake-grow

# Ingest with 30-minute timeout
make datalake-grow DURATION_MINUTES=30
```

### Check Ingestion Status

```bash
# Show current progress
make datalake-status

# Show detailed log
cat DATALAKE_LOG.md
cat data/ingestion-log.json | jq '.'
```

## Architecture

### Three-Tier Design

```
┌─────────────────────────────────────────┐
│ Makefile (datalake-grow / datalake-status)
├─────────────────────────────────────────┤
│ CLI (grow-datalake command)
├─────────────────────────────────────────┤
│ RepositoryIngestionOrchestrator
│  ├─ Load registry (repos_to_ingest.json)
│  ├─ Load existing log (idempotency)
│  ├─ Loop through repos with timeout/limit
│  │   └─ RepositoryTraverser.ingest() [AST-only]
│  ├─ Track progress (ingestion-log.json)
│  └─ Generate markdown log (DATALAKE_LOG.md)
└─────────────────────────────────────────┘
```

### Data Flow

```
repos_to_ingest.json (registry)
        ↓
RepositoryIngestionOrchestrator
        ↓ (load existing log)
ingestion-log.json (skip already-done repos)
        ↓ (for each repo)
RepositoryTraverser.ingest()
        ├─ Clone repo to temp dir
        ├─ AST parse all files
        ├─ Extract signatories & contracts
        └─ Save results
        ↓ (aggregate)
DATALAKE_LOG.md (progress report)
ingestion-log.json (updated log)
```

## Configuration

### Repository Registry (`data/repos_to_ingest.json`)

A JSON file with 24 carefully selected repositories:

```json
{
  "metadata": {...},
  "repositories": [
    {
      "repo_url": "https://github.com/lodash/lodash",
      "repo_name": "lodash",
      "owner": "lodash",
      "stars": 59000,
      "language": "JavaScript",
      "priority": 1,
      "reason": "Most-used utility library..."
    },
    ...
  ]
}
```

**Selection Criteria:**
- ✅ Popular (community signal)
- ✅ Active (recent commits)
- ✅ Diverse languages (TypeScript, JavaScript, Rust, Python, Go, Java)
- ✅ Medium-sized (500-5000 files)
- ✅ Production-quality code

### Ingestion Log (`data/ingestion-log.json`)

Tracks all ingested repos for idempotency:

```json
[
  {
    "repo_name": "lodash",
    "repo_url": "https://github.com/lodash/lodash",
    "timestamp": "2026-07-05T12:35:00Z",
    "status": "success",
    "files_processed": 926,
    "signatories": 6174,
    "contracts": 3200,
    "duration_secs": 45
  },
  ...
]
```

### Progress Log (`DATALAKE_LOG.md`)

Git-tracked markdown report updated after each run:

```markdown
# Data Lake Ingestion Log

**Last Updated**: 2026-07-05 12:45:30 UTC

## Current Status

- **Run ID**: abc-def-123
- **Duration**: 543 seconds (9.1 minutes)
- **Repos Processed**: 5/24
- **Success**: 5 | **Failed**: 0

## Aggregated Metrics

- **Total Files**: 5,430
- **Total Signatories**: 32,400
- **Total Contracts**: 15,200

## Repository Breakdown

| Repo | Status | Files | Signatories | Contracts | Time (s) |
|------|--------|-------|-------------|-----------|----------|
| lodash | ✅ | 926 | 6174 | 3200 | 45 |
| react | ✅ | 1200 | 8500 | 4100 | 52 |
| ...
```

## Usage Patterns

### Pattern 1: Quick Test (5 repos in <2 minutes)

```bash
make datalake-grow MAX_REPOS=5
```

### Pattern 2: Grow Overnight (4-hour window)

```bash
make datalake-grow DURATION_MINUTES=240
# Runs as many repos as possible in 4 hours
```

### Pattern 3: Full Ingest (all 24 repos)

```bash
make datalake-grow
# Takes ~10-15 minutes with default settings
```

### Pattern 4: Resume After Crash

```bash
# Re-run with same flags - already-ingested repos are skipped
make datalake-grow
```

### Pattern 5: Monitor Progress

```bash
# In one terminal:
make datalake-grow

# In another terminal (while running):
watch -n 5 'make datalake-status'
```

## CLI Reference

### `cargo run -- grow-datalake` Options

```
-r, --registry <REGISTRY>
    Registry JSON file (default: data/repos_to_ingest.json)

-o, --output <OUTPUT>
    Output directory (default: data)

-m, --max-repos <MAX_REPOS>
    Maximum number of repos to ingest (optional)

-t, --timeout-minutes <TIMEOUT_MINUTES>
    Maximum duration in minutes (optional)

-s, --skip-ingested
    Skip already-ingested repos (default: true)
```

## Implementation Details

### Orchestrator (`src/training/repo_ingestion_orchestrator.rs`)

**Key Components:**

1. **RepositoryIngestionOrchestrator**
   - Loads registry from JSON
   - Manages idempotency via ingestion log
   - Loops through repos with timeout/limit control
   - Aggregates results and generates reports

2. **IngestionMetrics**
   - Tracks per-repo stats (files, signatories, contracts, time)
   - Supports success/failed/skipped status

3. **IngestionLogEntry**
   - Durable log of all ingestion attempts
   - Used for idempotency and audit trail

### Key Methods

```rust
pub async fn run(&mut self) -> Result<IngestionResults>
    // Main orchestration loop

pub fn load_ingestion_log(&self) -> Result<HashMap<String, IngestionLogEntry>>
    // Load existing log for idempotency

pub fn save_ingestion_log(&self) -> Result<()>
    // Persist log after each run

fn update_markdown_log(&self, results: &IngestionResults) -> Result<()>
    // Update DATALAKE_LOG.md
```

### Idempotency Strategy

```rust
// At start of each repo:
if self.config.skip_already_ingested && existing_log.contains_key(&repo.repo_name) {
    println!("⏭️  Already ingested, skipping");
    continue;
}

// After successful ingest:
self.log_entries.push(IngestionLogEntry {
    repo_name,
    status: "success",
    ...
});
self.save_ingestion_log()?;
```

**Result:** Running `make datalake-grow` twice:
1. **First run**: Ingests all repos
2. **Second run**: Skips all, completes in ~1 second

## Performance Characteristics

### Speed

| Scenario | Time | Notes |
|----------|------|-------|
| 5 small repos | ~2 min | TypeScript/JavaScript |
| 10 medium repos | ~5 min | Mixed languages |
| 20 diverse repos | ~10 min | AST-only (no AI) |
| Full registry (24) | ~15 min | All languages |

### Resource Usage

- **CPU**: Moderate (parallel AST parsing)
- **Memory**: ~500MB (registry + logs in memory)
- **Disk**: ~100MB per repo (cloned and analyzed)
- **Network**: ~10-50MB per repo (clone from GitHub)

### Scaling Path

```
1 repo:    5 sec   ✓ works
5 repos:   2 min   ✓ works
20 repos:  10 min  ✓ works
50 repos:  25 min  ← next milestone (use DURATION_MINUTES=30)
100 repos: 50 min  ← achievable with more infrastructure
```

## Troubleshooting

### Issue: "Repository not found"

**Cause**: Network error or repo deleted

**Solution**:
```bash
# Re-run - will skip this repo and continue
make datalake-grow
```

### Issue: Build takes too long

**Cause**: First build compiles all dependencies

**Solution**:
```bash
# Subsequent runs are much faster
make datalake-grow  # First: ~2m build + ingest
make datalake-grow  # Second: ~10m ingest only
```

### Issue: Want to re-ingest a specific repo

**Solution**:
```bash
# Edit ingestion-log.json and remove the repo entry
# Then re-run
make datalake-grow
```

### Issue: Progress seems stuck

**Solution**:
```bash
# Monitor in real time
watch -n 5 'tail -20 DATALAKE_LOG.md'

# Or check the actual build output
cargo run --release -- grow-datalake --max-repos=3
```

## Integration with Training Pipeline

The ingested repos feed two downstream workflows:

### 1. Data Lake (AST-based)
```
repos_to_ingest.json
    ↓ (orchestrator)
data/ingestion-log.json
data/contracts-*.json
DATALAKE_LOG.md
    → Used for contract analysis & visualization
```

### 2. Training Validation (AI-assisted)
```
Discovered repos (from GitHub API)
    ↓ (idud-grow training pipeline)
data/training_datalake/
    → Used for ML training & evaluation
```

**Note**: These are independent workflows. Data lake grows repos automatically, while training validates specific issues using AI predictions.

## Future Enhancements

### Phase 2: Parallel Ingestion
- [ ] Parallel clone + ingest (2-3x speedup)
- [ ] Batch multiple repos per task

### Phase 3: Selective Ingestion
- [ ] Filter by language
- [ ] Filter by size range
- [ ] Filter by update recency

### Phase 4: Contract Export
- [ ] Save contracts to `data/contracts-<repo>.json`
- [ ] Aggregate contracts across all repos
- [ ] Generate training dataset from contracts

### Phase 5: Distributed Ingestion
- [ ] Split work across multiple machines
- [ ] Shared ingestion log (S3/cloud storage)
- [ ] Horizontal scaling to 1000+ repos

## Testing

Run integration tests:

```bash
cargo test --test integration_repo_orchestrator
# Tests registry loading, structure, idempotency, logging, etc.
```

Expected output:
```
running 11 tests
test tests::test_registry_loads_successfully ... ok
test tests::test_registry_structure ... ok
test tests::test_idempotency_log_format ... ok
test tests::test_config_validation ... ok
test tests::test_default_config ... ok
test tests::test_ingestion_log_persistence ... ok
test tests::test_registry_has_diverse_languages ... ok
test tests::test_markdown_log_creation ... ok
test tests::test_output_directory_structure ... ok
test tests::test_registry_priority_ordering ... ok
test tests::test_metrics_calculation ... ok

test result: ok. 11 passed
```

## References

- **Orchestrator Code**: `src/training/repo_ingestion_orchestrator.rs`
- **Registry**: `data/repos_to_ingest.json`
- **Progress Log**: `DATALAKE_LOG.md`
- **Ingestion Log**: `data/ingestion-log.json`
- **Integration Tests**: `tests/integration_repo_orchestrator.rs`
- **Makefile Targets**: `make datalake-grow`, `make datalake-status`
