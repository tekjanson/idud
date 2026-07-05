# Idempotent Training: Design & Usage

## Overview

The idud training pipeline is **fully idempotent**. You can run `make idud-grow` multiple times safely over weeks, through crashes and code updates. Already-processed repos and issues are automatically skipped.

## Architecture

### Training Cache (`training_cache.json`)

Every processed (repo_url, issue_number) pair is recorded in `data/training_datalake/training_cache.json`:

```json
[
  {
    "repo_url": "https://github.com/tokio-rs/tokio",
    "issue_number": 5123,
    "issue_id": "issue-5123",
    "processed_at": "2026-07-05T10:15:00Z",
    "prediction_run_id": "550e8400-e29b-41d4-a716-446655440001",
    "status": "completed"
  }
]
```

### Key Properties

1. **Per-Issue Tracking**: Each (repo, issue_number) is unique. Same repo, different issue = new training.
2. **Status Field**: Tracks "completed", "failed", or "pending" - currently all are skipped.
3. **Persistent**: Cache survives restarts, crashes, code updates, and deployments.
4. **Fast Lookup**: In-memory HashSet for O(1) duplicate checking during discovery.

## Workflow

### First Run
```bash
make idud-grow REPOS=100 CONCURRENT=10
```

1. Discovers 100 repos with Issues + PRs
2. For each repo: selects 1-3 recent issues
3. Predicts file changes from issue text + dependency graph
4. Validates predictions against actual PR changes
5. **Marks all processed (repo, issue) pairs in cache**
6. Persists training runs to `data/training_datalake/runs/`

**Result**: Cache now has ~200-300 entries

### Second Run (Next Week)
```bash
make idud-grow REPOS=100 CONCURRENT=10
```

1. Discovers 100 repos (may be new or repeated)
2. For each repo: **checks cache first**
   - If (repo, issue) already processed → **SKIP** ✓
   - If new issue → process it
3. Only processes completely new issues
4. **Updates cache with new entries**

**Result**: Accumulates training data over time

### Monitoring Progress
```bash
make cache-status
```

Shows:
- Total entries processed
- Unique repos trained on
- Last processing timestamp
- Sample of processed repos

## Safety Guarantees

### Crash Recovery
Training crashed mid-batch? Just restart:
```bash
make idud-grow REPOS=100  # Restarts where it left off
```

- Already-completed issues are cached → skipped
- New issues are picked up and processed
- No duplication, no data loss

### Code Updates
Updated predictor or validator logic? Run again:
```bash
make idud-grow REPOS=100  # Uses updated code
```

- Previous predictions stay cached
- Only processes new issues with new logic
- Can compare old vs new predictions over time

### Multiple Developers
Different team members can run training:
```bash
# Developer A on Monday
make idud-grow REPOS=50 CONCURRENT=5

# Developer B on Tuesday (different subset)
make idud-grow REPOS=75 CONCURRENT=10

# Both runs accumulate to same cache
```

All entries merge into single `training_cache.json`. No conflicts.

## Scaling Over Weeks

### Week 1: Foundation
```bash
make idud-grow REPOS=100 CONCURRENT=10  # ~200 training runs
```

### Week 2: Iterate & Improve
```bash
# Fix a bug in the predictor
make idud-grow REPOS=100 CONCURRENT=10  # Adds ~100 new repos
```

Cache now tracks ~300 unique repos. Previous 200 still cached → skipped.

### Week 3: Scale Up
```bash
# Scale to larger batch
make idud-grow REPOS=1000 CONCURRENT=20  # Discover 1000 new repos
```

Previous data preserved. Grows incrementally without redundancy.

## Implementation Details

### Cache Module (`src/training/cache.rs`)

```rust
pub struct TrainingCache {
    cache_path: String,
    entries: RwLock<Vec<CacheEntry>>,      // Full history (persistent)
    processed_keys: RwLock<HashSet<String>>, // Fast O(1) lookup
}

// Check before processing
if cache.is_processed(repo_url, issue_number) {
    continue;  // Skip
}

// Mark after successful prediction
cache.mark_processed(repo_url, issue_number, issue_id, run_id)?;
```

### Orchestrator Integration (`src/training/orchestrator.rs`)

1. **Before selection**: Filter out already-processed issues
2. **After prediction**: Mark newly-processed issues
3. **On error**: Mark as "failed" (still skipped but logged)

## Query Cache Status

### Show All Stats
```bash
make cache-status
```

Output:
```
📦 Training Cache Status
   Datalake: ./data/training_datalake

📊 Statistics
   Total processed: 312
   Completed: 308
   Failed: 4
   Pending: 0
   Unique repos: 47
   Last processed: 2026-07-05T10:15:00Z

🏗️  Processed Repositories (47):
   1. https://github.com/tokio-rs/tokio
   2. https://github.com/serde-rs/serde
   ...
```

### Direct CLI
```bash
./target/release/idud cache-status --datalake ./data/training_datalake
```

## Clearing Cache (for full restart)

⚠️ **Use with caution!** This deletes all training history.

```bash
# Via Rust API
let cache = TrainingCache::new("./data/training_datalake/training_cache.json")?;
cache.clear()?;

# Result: Empty cache, next run processes all repos fresh
```

## Limitations & Future Work

### Current Limitations
1. **Same issue, different PR links**: If an issue has multiple linked PRs, only the first is validated
2. **Issue resolution**: Cannot tell if issue was later closed/resolved
3. **Repository history**: Cache doesn't track when repos are deleted or renamed

### Future Improvements
1. **Selective reprocessing**: Mark issue for "re-predict" with new model version
2. **Incremental updates**: Only fetch new GitHub issues since last run
3. **Confidence-based**: Reprocess low-confidence (<0.5 F1) predictions
4. **A/B testing**: Track old vs new model on same issues

## Example: Multi-Week Training Schedule

```bash
# Monday: Initial baseline
make idud-grow REPOS=100 CONCURRENT=5
# Cache: 150 entries

# Wednesday: Bug fixes in selector
git commit -m "Fix false positive file selection"
make idud-grow REPOS=100 CONCURRENT=5
# Cache: 250 entries (100 new + 150 old)

# Friday: Scale up validation
make idud-grow REPOS=500 CONCURRENT=20
# Cache: 700+ entries (ongoing accumulation)

# Next Monday: Analyze results
make cache-status
# Shows: 700 processed, 180 unique repos, improvements over time
```

## Monitoring Long Runs

For long training sessions, check progress while it's running:

```bash
# Terminal 1: Start training
make idud-grow REPOS=1000 CONCURRENT=10

# Terminal 2: Monitor progress
while true; do make cache-status; sleep 60; done
```

Cache updates in real-time as training progresses.

---

**Bottom line**: Build idud's training iteratively. Run `make idud-grow` as many times as you want. The cache handles the rest. Safe, automatic deduplication. Perfect for production scaling over weeks.
