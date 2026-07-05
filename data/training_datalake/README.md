# Training Data Lake

The **Training Data Lake** is the authoritative repository for all idud self-validation training data. This directory contains:

1. **schemas/**: JSON schema definitions for data validation
   - `repo_metadata.schema.json` - Repository metadata structure
   - `training_run.schema.json` - Individual training run results
   - `aggregated_metrics.schema.json` - Performance metrics summaries

2. **repos/**: Repository metadata files
   - Files follow naming convention: `{repo_name}.repo_metadata.json`
   - Contains metadata about repositories used for training

3. **runs/**: Training run results (predictions and validations)
   - Files follow naming convention: `{uuid}.training_run.json`
   - Each file represents a single prediction validation

4. **metrics/**: Aggregated performance metrics
   - Files follow naming convention: `{uuid}.aggregated_metrics.json`
   - Summaries of training performance over time windows

## Key Principles

- **Single Source of Truth**: All training validation data flows through this datalake
- **JSON-Based Storage**: Language-agnostic format for easy interoperability
- **Schema Validation**: Every file must conform to its JSON schema
- **Immutable Records**: Training runs are append-only; never modify existing records
- **Local-First**: Data lives in `/data/training_datalake/` within the repository

## Rust API

The `TrainingDataLake` struct provides type-safe I/O:

```rust
use idud::TrainingDataLake;

let datalake = TrainingDataLake::new("./data/training_datalake")?;

// Write training results
datalake.write_training_run(&run)?;
datalake.write_repo_metadata(&metadata)?;
datalake.write_aggregated_metrics(&metrics)?;

// List all records
let runs = datalake.list_training_runs()?;
let repos = datalake.list_repo_metadata()?;
let metrics = datalake.list_aggregated_metrics()?;
```

See `TRAINING_DATALAKE_SCHEMA.md` for complete documentation.

## File Naming Convention

- **Repo Metadata**: `{repo_name}.repo_metadata.json`
- **Training Runs**: `{run_uuid}.training_run.json`
- **Metrics**: `{metric_uuid}.aggregated_metrics.json`

This convention ensures easy filtering and discovery.

## Example Integration

```rust
// Training pipeline writes results
let run = TrainingRun::new(
    repo_url,
    issue_id,
    issue_text,
    predicted_files,
    actual_files,
);
datalake.write_training_run(&run)?;

// Analytics queries results
let all_runs = datalake.list_training_runs()?;
let avg_f1 = all_runs.iter().map(|r| r.f1).sum::<f64>() / all_runs.len() as f64;
```

---

For detailed field documentation, see **TRAINING_DATALAKE_SCHEMA.md** at the repository root.
