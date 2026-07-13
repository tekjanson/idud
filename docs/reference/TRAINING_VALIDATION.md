# Training Validation System

**Purpose:** Validates idud's file change predictions against ground truth by measuring precision, recall, and F1 score across training runs.

## Overview

The prediction validation system enables idud to:
- Validate predictions from the dependency graph analysis against actual file changes in pull requests
- Calculate comprehensive metrics (precision, recall, F1) for each prediction
- Track performance trends over time and across programming languages
- Identify improvements and areas for refinement

## Metrics

### Individual Prediction Metrics

Each prediction produces three primary metrics:

| Metric | Formula | Interpretation |
|--------|---------|-----------------|
| **Precision** | TP / (TP + FP) | Of files we predicted, how many were right? |
| **Recall** | TP / (TP + FN) | Of actual files that changed, how many did we predict? |
| **F1 Score** | 2 × (P × R) / (P + R) | Harmonic mean balancing precision and recall |

Where:
- **TP (True Positives):** Predicted files that actually changed
- **FP (False Positives):** Predicted files that didn't change
- **FN (False Negatives):** Files that changed but we didn't predict

### Confusion Matrix Components

Each training run tracks:
- `true_positives` - Files correctly predicted
- `false_positives` - Files predicted but didn't change
- `false_negatives` - Files that changed but weren't predicted

## Core API

### `validate_prediction()`

Validates a single prediction against ground truth.

```rust
pub fn validate_prediction(
    predicted_files: Vec<String>,
    actual_files: Vec<String>,
) -> ValidationMetrics {
    // precision, recall, f1, true_positives, false_positives, false_negatives
}
```

**Usage:**
```rust
use idud::validate_prediction;

let predicted = vec!["src/main.rs", "src/lib.rs"];
let actual = vec!["src/main.rs", "src/db.rs"];

let metrics = validate_prediction(predicted, actual);
println!("Precision: {}", metrics.precision);  // 0.5
println!("Recall: {}", metrics.recall);        // 0.5
println!("F1: {}", metrics.f1);                // 0.5
```

### `write_training_result()`

Persists a training result to the datalake for historical analysis.

```rust
pub fn write_training_result(
    datalake: &TrainingDataLake,
    repo_url: String,
    issue_id: String,
    issue_text: String,
    predicted_files: Vec<String>,
    actual_files: Vec<String>,
) -> Result<Uuid>
```

**Usage:**
```rust
use idud::{TrainingDataLake, write_training_result};

let datalake = TrainingDataLake::new("./data/training_datalake")?;
let run_id = write_training_result(
    &datalake,
    "https://github.com/rust-lang/rust".into(),
    "issue-12345".into(),
    "Fix compilation error in parser".into(),
    vec!["src/parser.rs".to_string()],
    vec!["src/parser.rs".to_string()],
)?;
```

### `calculate_aggregate_metrics()`

Computes aggregated performance metrics across all stored training runs.

```rust
pub fn calculate_aggregate_metrics(
    datalake: &TrainingDataLake,
) -> Result<AggregatedMetrics>
```

**Returns:**
- `avg_precision` - Average precision across all runs
- `avg_recall` - Average recall across all runs
- `avg_f1` - Average F1 score across all runs
- `improvement_over_time` - Daily F1 score checkpoints
- `percentiles` - F1 score distribution (p50, p75, p90, p95)

**Usage:**
```rust
use idud::{TrainingDataLake, calculate_aggregate_metrics};

let datalake = TrainingDataLake::new("./data/training_datalake")?;
let metrics = calculate_aggregate_metrics(&datalake)?;

println!("Average F1: {:.3}", metrics.avg_f1);
println!("Total predictions: {}", metrics.total_predictions);
for checkpoint in &metrics.improvement_over_time {
    println!("Date: {}, F1: {:.3}", checkpoint.checkpoint, checkpoint.avg_f1);
}
```

### `calculate_metrics_by_language()`

Groups performance metrics by programming language.

```rust
pub fn calculate_metrics_by_language(
    datalake: &TrainingDataLake,
) -> Result<HashMap<String, LanguageMetrics>>
```

**Usage:**
```rust
use idud::{TrainingDataLake, calculate_metrics_by_language};

let datalake = TrainingDataLake::new("./data/training_datalake")?;
let lang_metrics = calculate_metrics_by_language(&datalake)?;

for (language, metrics) in lang_metrics {
    println!("{}: {} repos, avg F1 = {:.3}", 
        language, metrics.repo_count, metrics.avg_f1);
}
```

## HTTP API Endpoints

### POST `/api/training/validate`

Validates a prediction and stores the result.

**Request:**
```json
{
  "repo_url": "https://github.com/owner/repo",
  "issue_id": "issue-123",
  "issue_text": "Fix bug in authentication module",
  "predicted_files": ["src/auth/login.rs", "src/auth/session.rs"],
  "actual_files": ["src/auth/login.rs", "src/auth/session.rs", "tests/auth.rs"],
  "batch_id": "batch-2024-01-15" // optional
}
```

**Response:**
```json
{
  "success": true,
  "run_id": "550e8400-e29b-41d4-a716-446655440000",
  "metrics": {
    "precision": 0.667,
    "recall": 0.667,
    "f1": 0.667,
    "true_positives": 2,
    "false_positives": 1,
    "false_negatives": 1
  }
}
```

### GET `/api/training/metrics`

Returns aggregated metrics and language-specific analysis.

**Response:**
```json
{
  "success": true,
  "aggregated_metrics": {
    "metric_id": "550e8400-e29b-41d4-a716-446655440001",
    "generated_at": "2024-01-15T10:30:00Z",
    "period": "20240101_20240115",
    "total_repos": 42,
    "total_predictions": 1250,
    "avg_precision": 0.738,
    "avg_recall": 0.692,
    "avg_f1": 0.714,
    "improvement_over_time": [
      {
        "checkpoint": "2024-01-01T12:00:00Z",
        "avg_f1": 0.650
      },
      {
        "checkpoint": "2024-01-15T12:00:00Z",
        "avg_f1": 0.714
      }
    ],
    "percentiles": {
      "p50_f1": 0.750,
      "p75_f1": 0.850,
      "p90_f1": 0.920,
      "p95_f1": 0.950
    }
  },
  "language_metrics": {
    "Rust": {
      "language": "Rust",
      "repo_count": 15,
      "prediction_count": 450,
      "avg_precision": 0.760,
      "avg_recall": 0.710,
      "avg_f1": 0.734
    },
    "Python": {
      "language": "Python",
      "repo_count": 12,
      "prediction_count": 380,
      "avg_precision": 0.720,
      "avg_recall": 0.680,
      "avg_f1": 0.699
    }
  }
}
```

## Data Storage

All training results are persisted to **JSONL** format in `./data/training_datalake/runs/`:

```
{run_id}.training_run.json
```

Each line is a TrainingRun object:
```json
{
  "run_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-15T09:30:00Z",
  "repo_url": "https://github.com/owner/repo",
  "batch_id": null,
  "issue_id": "issue-123",
  "issue_text": "Fix authentication bug",
  "predicted_files": ["src/auth/login.rs"],
  "actual_files": ["src/auth/login.rs", "tests/auth.rs"],
  "precision": 0.5,
  "recall": 0.5,
  "f1": 0.5,
  "true_positives": 1,
  "false_positives": 1,
  "false_negatives": 1
}
```

Aggregated metrics are stored in `./data/training_datalake/metrics/`:

```
{metric_id}.aggregated_metrics.json
```

## Integration with Training Orchestrator

The validation system integrates with the training orchestrator for automated batch validation:

```rust
use idud::{TrainingOrchestrator, TrainingConfig};

let config = TrainingConfig {
    // ... configuration
};

let orchestrator = TrainingOrchestrator::new(config, datalake)?;
let results = orchestrator.validate_batch(predictions)?;
```

## Best Practices

1. **Include Sufficient Context:** Provide complete issue text for meaningful predictions
2. **Normalize File Paths:** Ensure predicted and actual files use consistent path formats (relative from repo root)
3. **Batch Validation:** Group related predictions with batch_id for trend analysis
4. **Monitor Trends:** Track metrics over time to identify improvements and regressions
5. **Language Analysis:** Use language-specific metrics to understand performance differences
6. **Percentile Monitoring:** Watch p75-p95 F1 scores to understand tail performance

## Interpretation Guide

| F1 Score Range | Interpretation | Action |
|---|---|---|
| 0.90 - 1.00 | Excellent | Model is working well for this type of change |
| 0.70 - 0.89 | Good | Model performs adequately; minor improvements possible |
| 0.50 - 0.69 | Moderate | Model needs refinement; consider retraining |
| 0.30 - 0.49 | Poor | Model may need significant changes to graph analysis |
| 0.00 - 0.29 | Very Poor | Critical issues; investigate prediction logic |

## Examples

### Example 1: Single Prediction Validation

```rust
use idud::validate_prediction;

let metrics = validate_prediction(
    vec!["src/main.rs".into(), "src/utils.rs".into()],
    vec!["src/main.rs".into()],
);

assert_eq!(metrics.true_positives, 1);
assert_eq!(metrics.false_positives, 1);
assert_eq!(metrics.false_negatives, 0);
// Precision = 1/(1+1) = 0.5
// Recall = 1/(1+0) = 1.0
// F1 = 2*(0.5*1.0)/(0.5+1.0) ≈ 0.667
```

### Example 2: Persistence and Aggregation

```rust
use idud::{TrainingDataLake, write_training_result, calculate_aggregate_metrics};

let datalake = TrainingDataLake::new("./data/training_datalake")?;

// Store multiple predictions
for prediction in predictions {
    write_training_result(
        &datalake,
        prediction.repo_url,
        prediction.issue_id,
        prediction.issue_text,
        prediction.predicted_files,
        prediction.actual_files,
    )?;
}

// Analyze aggregate performance
let metrics = calculate_aggregate_metrics(&datalake)?;
println!("Overall F1: {:.3}", metrics.avg_f1);
```

### Example 3: Language-Specific Analysis

```rust
use idud::{TrainingDataLake, calculate_metrics_by_language};

let datalake = TrainingDataLake::new("./data/training_datalake")?;
let by_lang = calculate_metrics_by_language(&datalake)?;

// Find best-performing language
let best = by_lang.values().max_by(|a, b| {
    a.avg_f1.partial_cmp(&b.avg_f1).unwrap()
});

if let Some(lang_metrics) = best {
    println!("Best performing: {} with F1 = {:.3}", 
        lang_metrics.language, lang_metrics.avg_f1);
}
```

## Performance Characteristics

- **Validation:** O(n) where n = number of predicted + actual files (typically < 100)
- **Write:** O(1) - single file write to datalake
- **Aggregation:** O(m) where m = total number of training runs
- **Language Grouping:** O(m + r) where r = number of repos

## Future Enhancements

- [ ] Confidence score thresholds for per-prediction tuning
- [ ] Confusion matrix heatmaps by file type
- [ ] Anomaly detection for outlier predictions
- [ ] Integration with continuous training pipelines
- [ ] Per-issue-type metric breakdown
- [ ] File-path-specific performance analysis
