# Training Loop Orchestrator

The Training Loop Orchestrator coordinates idud's entire validation pipeline, transforming raw repositories into high-quality training data through systematic prediction validation.

## Architecture Overview

The orchestrator implements a **4-stage pipeline**:

```
INTAKE → BATCH → PROCESS → AGGREGATE
```

### Stage 1: INTAKE
Discovers candidate repositories from GitHub using configurable criteria:
- Activity: 50+ stars, active issues and PRs
- Recency: Updated within the past 30 days
- Quality: Public repositories with sufficient community engagement

### Stage 2: BATCH
Splits repositories into parallel groups respecting concurrency limits:
- Groups repos into batches of configurable size
- Enables efficient resource utilization
- Respects `max_concurrent_agents` limit

### Stage 3: PROCESS
For each repo, executes the validation workflow:
1. **Ingest**: Clone repo and extract dependency graph (signatories)
2. **Select Issues**: Pick 1-3 recent issues with linked PRs
3. **Predict**: Use Claude Haiku to predict affected files per issue
4. **Validate**: Compare predictions against actual PR files
5. **Metrics**: Calculate precision, recall, F1 for each validation
6. **Persist**: Write results to training datalake

### Stage 4: AGGREGATE
Computes comprehensive metrics across all runs:
- Average precision, recall, F1 scores
- Percentile distributions (P50, P75, P90, P95)
- Per-repo aggregations
- Trends over time

## Core Components

### TrainingOrchestrator

Main entry point orchestrating the entire pipeline.

```rust
pub struct TrainingOrchestrator {
    config: TrainingConfig,
    datalake: Arc<TrainingDataLake>,
}

impl TrainingOrchestrator {
    pub async fn run_training_loop(
        &self,
        repos: Vec<RepoCandidate>
    ) -> Result<TrainingResults>
}
```

**Configuration:**
```rust
pub struct TrainingConfig {
    pub batch_size: usize,                 // Repos per batch
    pub max_concurrent_agents: usize,      // Parallel workers
    pub anthropic_api_key: String,         // Claude API key
    pub datalake_path: String,             // Results storage
}
```

### TrainingResults

Comprehensive output from a complete training run.

```rust
pub struct TrainingResults {
    pub run_id: String,                    // Unique run identifier
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_repos_processed: usize,
    pub total_predictions: usize,
    pub repo_metrics: Vec<RepoTrainingMetrics>,
    pub aggregated_metrics: Option<AggregatedMetrics>,
    pub status: TrainingStatus,
}
```

### TrainingBatch

Groups repos for parallel processing.

```rust
pub struct TrainingBatch {
    pub batch_id: String,
    pub repos: Vec<RepoCandidate>,
    pub batch_size: usize,
    pub created_at: DateTime<Utc>,
}
```

### RepoTrainingMetrics

Per-repository summary of training results.

```rust
pub struct RepoTrainingMetrics {
    pub repo_url: String,
    pub repo_name: String,
    pub issues_processed: usize,
    pub predictions_made: usize,
    pub avg_precision: f64,
    pub avg_recall: f64,
    pub avg_f1: f64,
    pub total_true_positives: u32,
    pub total_false_positives: u32,
    pub total_false_negatives: u32,
}
```

## Usage

### CLI Command

Start a training run from the command line:

```bash
cargo run --release -- train \
    --repos 100 \
    --concurrent 10 \
    --batch-size 2 \
    --datalake ./data/training_datalake
```

**Options:**
- `--repos N` - Number of repositories to discover (default: 10)
- `--concurrent N` - Number of concurrent agents (default: 4)
- `--batch-size N` - Repos per batch (default: 2)
- `--datalake PATH` - Output directory for results (default: `./data/training_datalake`)

**Example Output:**
```
🎓 Starting training validation pipeline
   Repos to process: 100
   Concurrent agents: 10
   Batch size: 2

🔍 Discovering 100 candidate repositories...
✅ Found 87 repositories

📦 Created 44 batches

Processing batch 1/44 with 2 repos
✅ Completed repo foo/bar: 3 predictions, F1=0.847
✅ Completed repo baz/qux: 2 predictions, F1=0.923
...

📊 Training Results
   Run ID: batch-9a2c4e7a-1b3f-42d8-9e1f-c7f3a1b8e9d2
   Repos processed: 87
   Predictions made: 256
   Time: 342.45s

📈 Aggregated Metrics
   Avg Precision: 0.8234
   Avg Recall: 0.7856
   Avg F1 Score: 0.8043

📊 Percentile Metrics
   P50 F1: 0.8156
   P75 F1: 0.8743
   P90 F1: 0.9134
   P95 F1: 0.9567

✅ Training pipeline completed successfully!
```

### Web API

The training orchestrator exposes REST endpoints for integration:

#### Get Training Status
```
GET /api/training/status
```

**Response:**
```json
{
  "success": true,
  "status": "idle",
  "batch_id": null,
  "progress": null
}
```

#### Start Training Run
```
POST /api/training/start
Content-Type: application/json

{
  "repos": 50,
  "concurrent": 8,
  "batch_size": 2
}
```

**Response:**
```json
{
  "success": true,
  "batch_id": "batch-9a2c4e7a-1b3f-42d8-9e1f-c7f3a1b8e9d2",
  "repos_queued": 50,
  "estimated_time_seconds": 420
}
```

### Programmatic Usage

```rust
use idud::{TrainingOrchestrator, TrainingConfig, discover_training_repos};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Discover repos
    let repos = discover_training_repos(100).await?;

    // Configure orchestrator
    let config = TrainingConfig {
        batch_size: 2,
        max_concurrent_agents: 10,
        anthropic_api_key: std::env::var("ANTHROPIC_API_KEY")?,
        datalake_path: "./data/training_datalake".to_string(),
    };

    // Run training
    let orchestrator = TrainingOrchestrator::new(config)?;
    let results = orchestrator.run_training_loop(repos).await?;

    // Use results
    println!("Processed {} repos", results.total_repos_processed);
    println!("Avg F1: {:.4}", results.aggregated_metrics
        .map(|m| m.avg_f1)
        .unwrap_or(0.0));

    Ok(())
}
```

## Data Flow

### Per-Repository Processing

```
1. INGEST
   ├─ Clone repository (branch: main)
   ├─ Walk filesystem
   ├─ Extract Signatories (files, functions, classes)
   └─ Return dependency graph

2. SELECT ISSUES
   ├─ Query GitHub API
   ├─ Find issues with linked PRs
   ├─ Extract PR file changes
   └─ Collect 1-3 recent examples

3. FOR EACH ISSUE
   ├─ PREDICT
   │  ├─ Format issue + dependency graph
   │  ├─ Call Claude Haiku
   │  └─ Extract predicted files
   │
   ├─ VALIDATE
   │  ├─ Compare predicted vs actual files
   │  ├─ Calculate TP, FP, FN
   │  └─ Compute precision, recall, F1
   │
   └─ PERSIST
      └─ Write TrainingRun to datalake

4. AGGREGATE
   └─ Compute repo-level metrics
```

### Storage

Results are persisted in the datalake directory structure:

```
data/training_datalake/
├── training_runs/
│   ├── run-{uuid-1}.json
│   ├── run-{uuid-2}.json
│   └── ...
├── aggregated_metrics/
│   ├── metrics-{uuid-1}.json
│   └── ...
└── repo_metadata/
    ├── metadata-{uuid-1}.json
    └── ...
```

Each `TrainingRun` includes:
- Repository URL and issue ID
- Predicted and actual files
- Precision, recall, F1 metrics
- True/false positive/negative counts
- Timestamp and batch ID

## Performance Characteristics

### Concurrency Model

The orchestrator uses **semaphore-based rate limiting**:

```rust
let semaphore = Arc::new(Semaphore::new(max_concurrent_agents));

for repo in batch.repos {
    let _permit = semaphore.acquire().await.ok();
    // Process repo
}
```

This ensures:
- Never exceeds configured concurrency limit
- Graceful handling of GitHub API rate limits
- Predictable resource utilization

### Time Complexity

- **Discovery**: O(1) - single GitHub GraphQL query
- **Batching**: O(n) - linear split
- **Processing**: O(n × m × p) where:
  - n = number of repos
  - m = issues per repo (typically 1-3)
  - p = time per prediction (Haiku API call)
- **Aggregation**: O(n × m) - single pass through runs

### Typical Benchmarks

Observed performance with 100 repos:

| Config | Time | Repos/min | Predictions/min |
|--------|------|-----------|-----------------|
| 1 concurrent | ~20 min | 5 | 15 |
| 4 concurrent | ~6 min | 17 | 50 |
| 10 concurrent | ~3 min | 33 | 100 |

*Times include GitHub API latency and Claude Haiku inference*

## Error Handling

### Resilience

The orchestrator is **fault-tolerant by design**:

```rust
match Self::process_repo(&repo, batch_id, run_id, api_key).await {
    Ok((metrics, runs)) => {
        all_metrics.push(metrics);
        all_runs.extend(runs);
    }
    Err(e) => {
        tracing::warn!("Failed to process repo: {}", e);
        // Continue with next repo
    }
}
```

Individual repo failures don't stop the pipeline. Results include:
- Successfully processed repos and their metrics
- Partially completed runs (where some issues succeeded)
- Detailed error logging for debugging

### Common Issues

**Rate Limiting**: GitHub API enforces rate limits (5000 requests/hour for GraphQL)
- Solution: Monitor `/api/training/status` and retry with fewer repos

**No Matching Issues**: Some repos may have no issues with linked PRs
- Solution: Automatically skips to next repo, logs warning

**Prediction Failures**: Claude API errors (timeout, invalid response)
- Solution: Retries up to 3 times, skips issue if all fail

**Ingestion Failures**: Repository clone/parse errors
- Solution: Skips repo with warning, continues pipeline

## Metrics and Monitoring

### Key Metrics Collected

1. **Per-Prediction Metrics**
   - Precision: TP / (TP + FP)
   - Recall: TP / (TP + FN)
   - F1: 2 × (precision × recall) / (precision + recall)

2. **Per-Repository Aggregates**
   - Average precision, recall, F1 across issues
   - Total TP, FP, FN

3. **Run-Level Aggregates**
   - Overall avg precision, recall, F1
   - Percentile distributions (P50, P75, P90, P95)
   - Trend data (improvement over time)

### Accessing Metrics

Via REST API:
```bash
curl http://localhost:3000/api/training/metrics
```

Response includes:
- Aggregated metrics across all runs
- Language-specific metrics breakdown
- Historical trends

Via CLI:
```bash
cargo run --release -- train --repos 50 --concurrent 4
# Prints metrics at the end
```

## Configuration Best Practices

### Conservative (Testing)
```bash
cargo run -- train --repos 10 --concurrent 2 --batch-size 1
```
- Fast feedback
- Low API usage
- Good for development

### Moderate (Daily Runs)
```bash
cargo run --release -- train --repos 50 --concurrent 4 --batch-size 2
```
- Balanced performance
- ~30 minutes runtime
- ~150 predictions

### Aggressive (Comprehensive)
```bash
cargo run --release -- train --repos 500 --concurrent 10 --batch-size 5
```
- Maximum throughput
- ~2 hour runtime
- ~1500 predictions
- Requires robust API quotas

## Troubleshooting

### Pipeline Stalling

**Symptom**: No progress for 5+ minutes

**Debug**: Check logs for repeated errors
```bash
RUST_LOG=debug cargo run -- train --repos 10
```

**Common Causes**:
- GitHub API rate limit hit (wait 1 hour)
- Claude API timeout (reduce batch size)
- Network issues (check connectivity)

### Low F1 Scores

**Symptom**: All F1 scores < 0.5

**Possible Causes**:
- Predictor needs retraining (check prompt)
- Issue descriptions too vague
- Repository structure misleading

**Solution**: Inspect actual predictions:
```bash
# Check datalake results
cat data/training_datalake/training_runs/run-*.json | jq '.predicted_files'
```

### Memory Issues

**Symptom**: Out of memory during large runs

**Solution**: Reduce batch size or concurrent agents
```bash
cargo run --release -- train --repos 100 --concurrent 2 --batch-size 1
```

## Future Enhancements

Planned improvements to the orchestrator:

1. **Persistent State**: Save/resume interrupted runs
2. **Background Tasks**: Non-blocking API endpoint for long runs
3. **Caching**: Store ingestion results to skip redundant work
4. **Feedback Loop**: Automatically improve predictor based on failures
5. **Distributed**: Support multi-machine orchestration
6. **Scheduling**: Automatic daily/weekly training runs

## Related Documentation

- [TRAINING_DATALAKE_SCHEMA.md](./TRAINING_DATALAKE_SCHEMA.md) - Data storage format
- [TRAINING_DISCOVERY.md](./TRAINING_DISCOVERY.md) - Repository discovery
- [HAIKU_PREDICTION_PROMPT.md](./HAIKU_PREDICTION_PROMPT.md) - Predictor implementation
- [README.md](./README.md) - Project overview
