# Training Data Lake Schema Documentation

## Overview
The idud Training Data Lake is the single source of truth for all training data used in the self-validation system. It provides a structured, JSON-based storage system with three primary data types: repository metadata, training runs, and aggregated metrics.

**Directory Structure:**
```
./data/training_datalake/
├── repos/                          # Repository metadata files
│   └── *.repo_metadata.json
├── runs/                           # Individual training run results
│   └── *.training_run.json
├── metrics/                        # Aggregated performance metrics
│   └── *.aggregated_metrics.json
├── repo_metadata.schema.json       # JSON schema definitions
├── training_run.schema.json
└── aggregated_metrics.schema.json
```

---

## Repository Metadata (`repo_metadata.json`)

Stores metadata about repositories ingested into the training datalake.

### Schema Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | ✓ | HTTPS URL of the repository (e.g., `https://github.com/owner/repo`) |
| `owner` | string | ✓ | Repository owner: username or organization name |
| `name` | string | ✓ | Repository name (should be unique within owner) |
| `stars` | integer | ✗ | Number of GitHub stars (default: 0) |
| `forks` | integer | ✗ | Number of GitHub forks (default: 0) |
| `language` | string | ✓ | Primary programming language (e.g., "Rust", "Python", "JavaScript") |
| `last_activity` | ISO 8601 datetime | ✗ | Timestamp of the last commit on the main branch |
| `issue_count` | integer | ✗ | Total number of open issues (default: 0) |
| `pr_count` | integer | ✗ | Total number of open pull requests (default: 0) |
| `added_at` | ISO 8601 datetime | ✓ | Timestamp when repo was added to datalake |
| `ingestion_status` | enum | ✓ | One of: `pending`, `ingested`, `failed` |

### Ingestion Status Values

- **pending**: Repository metadata added but AST ingestion not yet started
- **ingested**: Repository successfully ingested; code graph and contracts available
- **failed**: Ingestion failed; see logs for details

### Example
```json
{
  "url": "https://github.com/example-org/example-repo",
  "owner": "example-org",
  "name": "example-repo",
  "stars": 1250,
  "forks": 340,
  "language": "Rust",
  "last_activity": "2024-07-04T15:30:00Z",
  "issue_count": 42,
  "pr_count": 8,
  "added_at": "2024-07-01T09:00:00Z",
  "ingestion_status": "ingested"
}
```

---

## Training Run (`training_run.json`)

Records a single prediction validation result from the idud engine.

### Schema Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `run_id` | UUID | ✓ | Unique identifier (v4 UUID) for this training run |
| `timestamp` | ISO 8601 datetime | ✓ | When this prediction was executed |
| `repo_url` | string | ✓ | URL of repository in which this prediction was made |
| `batch_id` | string | ✗ | Batch identifier for grouping multiple related runs |
| `issue_id` | string | ✓ | Unique identifier for test case (e.g., GitHub issue number or custom ID) |
| `issue_text` | string | ✓ | The problem description or query text |
| `predicted_files` | string[] | ✓ | File paths predicted by idud as related to the issue |
| `actual_files` | string[] | ✓ | Ground truth: actual files that were changed or relevant |
| `precision` | number (0-1) | ✓ | TP / (TP + FP) — fraction of predicted files that were correct |
| `recall` | number (0-1) | ✓ | TP / (TP + FN) — fraction of actual files that were predicted |
| `f1` | number (0-1) | ✓ | Harmonic mean: 2 × (precision × recall) / (precision + recall) |
| `true_positives` | integer | ✗ | Number of correctly predicted files (TP) |
| `false_positives` | integer | ✗ | Number of incorrectly predicted files (FP) |
| `false_negatives` | integer | ✗ | Number of actual files not predicted (FN) |

### Metric Definitions

- **Precision**: How many of the predicted files were actually relevant?
  - Formula: `TP / (TP + FP)`
  - Interpretation: If precision = 0.8, then 80% of predictions were correct

- **Recall**: How many of the actual files did we find?
  - Formula: `TP / (TP + FN)`
  - Interpretation: If recall = 0.9, then we found 90% of the relevant files

- **F1 Score**: Harmonic mean balancing precision and recall
  - Formula: `2 × (precision × recall) / (precision + recall)`
  - Range: 0 to 1 (1.0 is perfect)

### Example
```json
{
  "run_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-07-05T10:15:00Z",
  "repo_url": "https://github.com/example-org/example-repo",
  "batch_id": "batch-20240705-01",
  "issue_id": "issue-12345",
  "issue_text": "Fix memory leak in concurrent HashMap access",
  "predicted_files": [
    "src/concurrent/hashmap.rs",
    "src/concurrent/mod.rs",
    "tests/concurrent_tests.rs",
    "benches/hashmap_bench.rs"
  ],
  "actual_files": [
    "src/concurrent/hashmap.rs",
    "src/concurrent/mod.rs",
    "CHANGELOG.md"
  ],
  "precision": 0.75,
  "recall": 1.0,
  "f1": 0.857,
  "true_positives": 2,
  "false_positives": 1,
  "false_negatives": 0
}
```

---

## Aggregated Metrics (`aggregated_metrics.json`)

Summary statistics of training run performance, computed from multiple runs.

### Schema Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `metric_id` | UUID | ✓ | Unique identifier (v4 UUID) for this metrics snapshot |
| `generated_at` | ISO 8601 datetime | ✓ | When these metrics were computed |
| `period` | string | ✓ | Time period label: `daily`, `weekly`, `all-time`, etc. |
| `time_window` | object | ✓ | Start and end timestamps of the aggregation window |
| `time_window.start` | ISO 8601 datetime | ✓ | Window start time |
| `time_window.end` | ISO 8601 datetime | ✓ | Window end time |
| `total_repos` | integer | ✓ | Count of unique repositories in this window |
| `total_predictions` | integer | ✓ | Total number of predictions made in this window |
| `avg_precision` | number (0-1) | ✓ | Mean precision across all predictions |
| `avg_recall` | number (0-1) | ✓ | Mean recall across all predictions |
| `avg_f1` | number (0-1) | ✓ | Mean F1 score across all predictions |
| `improvement_over_time` | object[] | ✗ | Historical F1 progression (one checkpoint per day/week) |
| `improvement_over_time[].checkpoint` | ISO 8601 datetime | ✓ | Checkpoint timestamp |
| `improvement_over_time[].avg_f1` | number (0-1) | ✓ | F1 score at that checkpoint |
| `percentiles` | object | ✗ | Distribution metrics |
| `percentiles.p50_f1` | number (0-1) | ✗ | 50th percentile F1 score (median) |
| `percentiles.p75_f1` | number (0-1) | ✗ | 75th percentile F1 score |
| `percentiles.p90_f1` | number (0-1) | ✗ | 90th percentile F1 score |
| `percentiles.p95_f1` | number (0-1) | ✗ | 95th percentile F1 score |

### Interpretation

- **avg_f1**: Overall system accuracy during this period (0 = worst, 1 = perfect)
- **percentiles**: Distribution shape; if p50_f1 ≈ avg_f1, distribution is balanced

### Example
```json
{
  "metric_id": "660e8400-e29b-41d4-a716-446655440001",
  "generated_at": "2024-07-05T10:30:00Z",
  "period": "daily",
  "time_window": {
    "start": "2024-07-05T00:00:00Z",
    "end": "2024-07-05T23:59:59Z"
  },
  "total_repos": 12,
  "total_predictions": 156,
  "avg_precision": 0.823,
  "avg_recall": 0.912,
  "avg_f1": 0.865,
  "improvement_over_time": [
    { "checkpoint": "2024-07-01T00:00:00Z", "avg_f1": 0.742 },
    { "checkpoint": "2024-07-02T00:00:00Z", "avg_f1": 0.768 },
    { "checkpoint": "2024-07-03T00:00:00Z", "avg_f1": 0.801 },
    { "checkpoint": "2024-07-04T00:00:00Z", "avg_f1": 0.838 },
    { "checkpoint": "2024-07-05T00:00:00Z", "avg_f1": 0.865 }
  ],
  "percentiles": {
    "p50_f1": 0.872,
    "p75_f1": 0.918,
    "p90_f1": 0.952,
    "p95_f1": 0.968
  }
}
```

---

## Rust API Reference

The `TrainingDataLake` struct provides type-safe I/O operations:

```rust
pub struct TrainingDataLake { ... }

impl TrainingDataLake {
    /// Create new datalake with automatic directory setup
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self>

    // Repository metadata operations
    pub fn write_repo_metadata(&self, metadata: &RepoMetadata) -> Result<PathBuf>
    pub fn read_repo_metadata<P: AsRef<Path>>(&self, path: P) -> Result<RepoMetadata>
    pub fn list_repo_metadata(&self) -> Result<Vec<RepoMetadata>>

    // Training run operations
    pub fn write_training_run(&self, run: &TrainingRun) -> Result<PathBuf>
    pub fn read_training_run<P: AsRef<Path>>(&self, path: P) -> Result<TrainingRun>
    pub fn list_training_runs(&self) -> Result<Vec<TrainingRun>>

    // Aggregated metrics operations
    pub fn write_aggregated_metrics(&self, metrics: &AggregatedMetrics) -> Result<PathBuf>
    pub fn read_aggregated_metrics<P: AsRef<Path>>(&self, path: P) -> Result<AggregatedMetrics>
    pub fn list_aggregated_metrics(&self) -> Result<Vec<AggregatedMetrics>>

    pub fn base_path(&self) -> &Path
}
```

### Example Usage

```rust
use idud::{TrainingDataLake, TrainingRun, RepoMetadata, IngestionStatus};
use chrono::Utc;

let datalake = TrainingDataLake::new("./data/training_datalake")?;

// Create and save a training run
let run = TrainingRun::new(
    "https://github.com/org/repo".to_string(),
    "issue-1".to_string(),
    "Fix the bug".to_string(),
    vec!["src/main.rs".to_string()],
    vec!["src/main.rs".to_string(), "tests/main_test.rs".to_string()],
);
datalake.write_training_run(&run)?;

// List all training runs
let all_runs = datalake.list_training_runs()?;
for run in all_runs {
    println!("Run {} - F1: {}", run.run_id, run.f1);
}
```

---

## Best Practices

1. **Batch Operations**: Use `batch_id` to group related predictions for easier analysis
2. **Timestamps**: Always use ISO 8601 format with UTC timezone (`Z` suffix)
3. **File Paths**: Store relative paths from repository root (e.g., `src/main.rs`, not `/home/user/repo/src/main.rs`)
4. **Ground Truth**: Ensure `actual_files` comes from version control history or issue labels
5. **Naming**: File names follow pattern `{identifier}.{type}.json` for easy filtering
6. **Archival**: Move old metrics to separate directories if datalake grows very large

---

## Integration Points

- **Pipelines**: Use `TrainingDataLake::write_training_run()` after each prediction
- **Web UI**: Query `list_training_runs()` and `list_aggregated_metrics()` for dashboards
- **Analytics**: Compute percentiles from historical runs to track improvement over time
- **Validation**: Use JSON schemas to validate before writing to prevent corruption
