# Training Module Developer Guide

Comprehensive reference for developers extending idud's training system.

---

## Overview

The training module coordinates the self-validation pipeline:

```
Discovery → Ingest → Predict → Validate → Improve
```

It integrates with:
- **GitHub API** (discovery, ground truth)
- **LLM APIs** (Claude for predictions)
- **Repository Ingestion** (dependency graph building)
- **Training Datalake** (metrics persistence)
- **UI/Visualization** (results presentation)

---

## Module Architecture

```
src/training/
├── mod.rs               # Module exports & organization
├── discovery.rs         # GitHub repository discovery
├── predictor.rs         # LLM-based file prediction
├── validator.rs         # Metric calculation & persistence
└── orchestrator.rs      # Pipeline coordination
```

### Key Files at a Glance

| File | Purpose | Key Functions |
|------|---------|---------------|
| `discovery.rs` | Find training candidates | `discover_training_repos()`, `fetch_issue_and_linked_pr()` |
| `predictor.rs` | Predict files from issues | `predict_files_from_issue()`, `PredictionRequest` |
| `validator.rs` | Validate & calculate metrics | `validate_prediction()`, `calculate_aggregate_metrics()` |
| `orchestrator.rs` | Coordinate pipeline | `TrainingOrchestrator::run()`, batch processing |

---

## Component Deep Dive

### 1. Discovery Module (`discovery.rs`)

**Purpose:** Find public repositories suitable for training validation.

**Main Functions:**

```rust
pub async fn discover_training_repos(limit: usize) -> Result<Vec<RepoCandidate>>
```
- Queries GitHub GraphQL API for repositories
- Filters by: 50+ stars, recent updates, active issues/PRs
- Returns: Structured candidate metadata
- Rate limit: Respects GitHub 60/hour unauthenticated limit

**Example:**
```rust
let candidates = discover_training_repos(100).await?;
for repo in candidates {
    println!("{}: {} stars", repo.name, repo.stars);
}
```

```rust
pub async fn fetch_issue_and_linked_pr(
    owner: &str, 
    name: &str, 
    issue_id: u32
) -> Result<IssueWithPR>
```
- Retrieves issue details from GitHub
- Finds linked PR via timeline events
- Extracts changed files from PR
- Returns: Issue text + actual file changes (ground truth)

**Example:**
```rust
let issue_data = fetch_issue_and_linked_pr("rust-lang", "rust", 51747).await?;
println!("Issue: {}", issue_data.issue_title);
println!("Files changed: {:?}", issue_data.pr_files);  // Ground truth
```

**Rate Limiting:**
- Unauthenticated: 60 requests/hour
- Authenticated (with `GITHUB_TOKEN`): 5,000 requests/hour
- Module handles 429 responses and returns `DiscoveryError::RateLimited`

**Error Types:**
```rust
pub enum DiscoveryError {
    HttpError(reqwest::Error),
    ApiError(String),
    JsonError(serde_json::Error),
    RateLimited,
    RepoNotFound(String),
}
```

**Data Structures:**

```rust
pub struct RepoCandidate {
    pub url: String,                    // https://github.com/owner/repo
    pub name: String,                   // repo name
    pub owner: String,                  // owner name
    pub stars: u32,                     // star count
    pub language: Option<String>,       // primary language
    pub issue_count: u32,               // open issues
    pub pr_count: u32,                  // open PRs
    pub last_issue_id: Option<String>,  // GraphQL ID of latest issue
    pub last_pr_id: Option<String>,     // GraphQL ID of latest PR
    pub updated_at: String,             // ISO 8601 timestamp
}

pub struct IssueWithPR {
    pub issue_title: String,
    pub issue_body: String,
    pub issue_number: u32,
    pub pr_number: Option<u32>,
    pub pr_files: Vec<String>,          // Files changed in linked PR
}
```

**Integration Points:**
- Called by `TrainingOrchestrator::discover_candidates()`
- Results cached in training datalake

---

### 2. Predictor Module (`predictor.rs`)

**Purpose:** Predict which files should change given an issue description.

**Main Function:**

```rust
pub async fn predict_files_from_issue(
    request: PredictionRequest
) -> Result<PredictionResponse>
```

**Input:**
```rust
pub struct PredictionRequest {
    pub repository_name: String,        // repo name (for context)
    pub issue_title: String,            // issue title
    pub issue_body: String,             // issue description
    pub graph: &ContractLedger,         // dependency graph
    pub max_predictions: usize,         // limit returned files
}
```

**Output:**
```rust
pub struct PredictionResponse {
    pub predicted_files: Vec<String>,   // Predicted file paths
    pub token_usage: TokenUsage,        // LLM tokens consumed
}

pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}
```

**How It Works:**

1. **Issue Analysis** (LLM)
   - Claude Haiku analyzes issue text
   - Extracts keywords, intent, affected areas
   - Queries dependency graph for related signatories

2. **Graph Traversal**
   - Uses idud's contract ledger to find related files
   - Starts from keywords extracted by LLM
   - Traverses contracts up to configured depth
   - Ranks candidates by traversal distance + LLM confidence

3. **Prediction Assembly**
   - Returns top N files by combined score
   - Includes confidence scores if available
   - Respects `max_predictions` limit

**Example:**

```rust
use idud::{predict_files_from_issue, PredictionRequest};

let request = PredictionRequest {
    repository_name: "rust-lang/rust".to_string(),
    issue_title: "Implement const generics for arrays".to_string(),
    issue_body: "Add support for const generic parameters in array types...".to_string(),
    graph: &contract_ledger,
    max_predictions: 10,
};

let response = predict_files_from_issue(request).await?;
for file in response.predicted_files {
    println!("Predicted: {}", file);
}
println!("Tokens used: {}", response.token_usage.prompt_tokens);
```

**LLM Integration:**
- Model: Claude Haiku (fast, cheap, good for code understanding)
- API: Anthropic (requires `ANTHROPIC_API_KEY` env var)
- Prompt: See `HAIKU_PREDICTION_PROMPT.md`

**Cost & Performance:**
- ~0.01 USD per prediction (Haiku pricing)
- ~2 seconds per prediction (API latency)
- Token efficiency: ~500 prompt tokens, ~100 completion tokens

---

### 3. Validator Module (`validator.rs`)

**Purpose:** Calculate prediction accuracy metrics and persist results.

**Core Functions:**

```rust
pub fn validate_prediction(
    predicted_files: Vec<String>,
    actual_files: Vec<String>,
) -> ValidationMetrics
```

Calculates: Precision, Recall, F1, confusion matrix.

```rust
pub struct ValidationMetrics {
    pub precision: f32,        // TP / (TP + FP)
    pub recall: f32,           // TP / (TP + FN)
    pub f1: f32,               // Harmonic mean
    pub true_positives: u32,   // Files we predicted correctly
    pub false_positives: u32,  // Files we predicted but didn't change
    pub false_negatives: u32,  // Files changed but we didn't predict
}
```

**Example:**
```rust
use idud::validate_prediction;

let metrics = validate_prediction(
    vec!["src/auth.rs", "src/session.rs"],
    vec!["src/auth.rs", "src/session.rs", "tests/auth_test.rs"],
);

println!("Precision: {:.3}", metrics.precision);  // 2/2 = 1.0
println!("Recall: {:.3}", metrics.recall);        // 2/3 = 0.667
println!("F1: {:.3}", metrics.f1);                // 0.8
```

**Persistence:**

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

Stores result to JSONL datalake for historical analysis.

**Aggregation:**

```rust
pub fn calculate_aggregate_metrics(
    datalake: &TrainingDataLake,
) -> Result<AggregatedMetrics>
```

Computes metrics across all stored training runs:
- Average precision, recall, F1
- Improvement over time (checkpoints)
- Percentile distribution (p50, p75, p90, p95)

**Language-Specific Analysis:**

```rust
pub fn calculate_metrics_by_language(
    datalake: &TrainingDataLake,
) -> Result<HashMap<String, LanguageMetrics>>
```

Groups performance by programming language.

```rust
pub struct LanguageMetrics {
    pub language: String,
    pub repo_count: u32,
    pub prediction_count: u32,
    pub avg_precision: f32,
    pub avg_recall: f32,
    pub avg_f1: f32,
}
```

---

### 4. Orchestrator Module (`orchestrator.rs`)

**Purpose:** Coordinate the entire training pipeline.

**Main Component:**

```rust
pub struct TrainingOrchestrator {
    config: TrainingConfig,
    datalake: Arc<TrainingDataLake>,
    client: HttpClient,
}

impl TrainingOrchestrator {
    pub async fn run(&self) -> Result<TrainingResults> { ... }
}
```

**Pipeline Stages:**

```rust
pub async fn run(&self) -> Result<TrainingResults> {
    // 1. DISCOVER: Find candidate repositories
    let candidates = self.discover_candidates().await?;
    
    // 2. BATCH: Split into parallel groups
    let batches = self.create_batches(candidates);
    
    // 3. PROCESS: For each batch, ingest & validate
    let mut all_metrics = Vec::new();
    for batch in batches {
        let metrics = self.process_batch(batch).await?;
        all_metrics.extend(metrics);
    }
    
    // 4. AGGREGATE: Calculate overall metrics
    let aggregated = calculate_aggregate_metrics(&self.datalake)?;
    
    // 5. RETURN: Training results
    Ok(TrainingResults {
        run_id: Uuid::new_v4().to_string(),
        started_at: Utc::now(),
        completed_at: Utc::now(),
        total_repos_processed: all_metrics.len(),
        repo_metrics: all_metrics,
        aggregated_metrics: Some(aggregated),
        status: TrainingStatus::Completed,
    })
}
```

**Configuration:**

```rust
pub struct TrainingConfig {
    pub batch_size: usize,              // Repos per parallel batch
    pub max_repos: Option<usize>,       // Total repos to process
    pub language_filter: Vec<String>,   // Only train on these languages
    pub min_stars: u32,                 // Minimum repository stars
    pub max_issues_per_repo: usize,     // Issues to sample per repo
    pub concurrency_limit: usize,       // Parallel processing limit
}
```

**Usage:**

```rust
use idud::{TrainingOrchestrator, TrainingConfig};

let config = TrainingConfig {
    batch_size: 10,
    max_repos: Some(100),
    language_filter: vec!["Rust".into(), "Go".into()],
    min_stars: 50,
    max_issues_per_repo: 50,
    concurrency_limit: 5,
};

let datalake = TrainingDataLake::new("./data/training_datalake")?;
let orchestrator = TrainingOrchestrator::new(config, datalake)?;
let results = orchestrator.run().await?;

println!("Processed: {} repos", results.total_repos_processed);
println!("Avg F1: {:.3}", results.aggregated_metrics.unwrap().avg_f1);
```

**Error Handling:**
- Failures in one batch don't stop others (resilience)
- Errors collected and reported in results
- Partial results can be analyzed even if batch fails

---

## Data Flow: Step by Step

### Complete Training Run

```
1. DISCOVER
   discover_training_repos(50)
   → Vec<RepoCandidate>
   
2. BATCH
   Split candidates into groups of batch_size
   → Vec<TrainingBatch>
   
3. INGEST (for each repo)
   Clone repository → Build AST → Extract graph → In-memory ledger
   
4. PREDICT (for each issue)
   fetch_issue_and_linked_pr(owner, name, issue_id)
   → IssueWithPR { issue_text, pr_files }
   
   predict_files_from_issue(issue_text, graph)
   → PredictionResponse { predicted_files }
   
5. VALIDATE
   validate_prediction(predicted_files, pr_files)
   → ValidationMetrics { precision, recall, f1 }
   
   write_training_result(...)
   → Persisted to datalake
   
6. AGGREGATE
   calculate_aggregate_metrics(datalake)
   calculate_metrics_by_language(datalake)
   → AggregatedMetrics & LanguageMetrics
   
7. RETURN
   TrainingResults {
     run_id,
     total_repos_processed,
     repo_metrics: Vec<RepoTrainingMetrics>,
     aggregated_metrics: AggregatedMetrics,
     status: TrainingStatus::Completed,
   }
```

---

## Extension Points

### Adding a New Extraction Method

To support analyzing a new code pattern (e.g., async/await contracts):

1. **Add to Graph Model**
   - Extend `ContractType` enum in `types.rs`
   - Define new signatory relationships

2. **Update Parser**
   - Add language-specific analysis in `pipelines/`
   - Extract new contract patterns

3. **Test on Sample Repo**
   ```bash
   cargo run --release -- training analyze-repo \
     --url https://github.com/owner/repo \
     --verbose
   ```

4. **Measure Impact**
   - Run training session with new extractor
   - Compare F1 scores vs. baseline
   - Iterate if results don't improve

### Adding Support for New Language

1. **Create Language Extractor**
   - Add `pipelines/extract_<language>.rs`
   - Implement AST parsing and contract extraction

2. **Register in Pipeline**
   - Update `pipelines/mod.rs` to dispatch by language
   - Add language-specific configuration

3. **Test Thoroughly**
   - Run on 3-5 public repositories in that language
   - Verify contract extraction is accurate
   - Add to TRAINING_RESULTS language breakdown

4. **Document**
   - Note language support in README
   - Add example in training docs

---

## Integration with Other Modules

### With Repository Ingestion

Training uses the same graph ingestion as normal operation:

```rust
use idud::RepositoryIngestionConfig;

let config = RepositoryIngestionConfig {
    repo_url: "https://github.com/owner/repo",
    branch: "main",
    ...
};

let graph = ingest_repository(&config)?;
```

The graph is then passed to `predict_files_from_issue()` for traversal.

### With Training Datalake

Results are persisted to the datalake for trend analysis:

```rust
use idud::TrainingDataLake;

let datalake = TrainingDataLake::new("./data/training_datalake")?;
write_training_result(&datalake, ...)?;

// Later: retrieve and analyze
let metrics = calculate_aggregate_metrics(&datalake)?;
```

### With Web Server

Training results are exposed via HTTP API:

```
GET /api/training/metrics
    → AggregatedMetrics + LanguageMetrics

POST /api/training/predict
    → Synchronous prediction for UI
    
GET /api/training/discover?limit=100
    → Repository candidates
```

---

## CLI Commands

### Run Training

```bash
cargo run --release -- training
```

Options:
- `--batch-size 50` — Repos per batch
- `--language rust` — Filter by language
- `--max-repos 100` — Limit total repos
- `--verbose` — Debug output

### Analyze Results

```bash
cargo run --release -- training metrics
```

### Debug Specific Repo

```bash
cargo run --release -- training analyze-repo \
  --url https://github.com/owner/repo \
  --verbose
```

### Single Prediction

```bash
cargo run --release -- training predict \
  --repo-url https://github.com/owner/repo \
  --issue-id 12345
```

---

## Performance Considerations

### Optimization Strategy

1. **Discovery:** Cached for 1 hour (rate limit optimization)
2. **Ingestion:** Parallel cloning + AST parsing (IO-bound)
3. **Prediction:** Batched LLM calls (API rate limit)
4. **Validation:** Local computation (CPU-bound but fast)
5. **Aggregation:** Streaming JSON parsing (memory-efficient)

### Bottlenecks

| Stage | Bottleneck | Mitigation |
|-------|-----------|-----------|
| Discovery | GitHub API rate limit | Cache, batching |
| Ingestion | Repository cloning | Parallel downloads |
| Prediction | LLM latency | Batched requests |
| Aggregation | Disk I/O | Stream processing |

### Typical Performance

- **Full training session (50 repos):** 2-3 hours
- **Per-repo time:** ~3 minutes
- **Per-prediction time:** ~2 seconds
- **Aggregation time:** ~30 seconds

---

## Testing

### Unit Tests

```bash
cargo test training::discovery::tests
cargo test training::validator::tests
cargo test training::orchestrator::tests
```

### Integration Tests

```bash
cargo test --test training_integration
```

Tests against live GitHub API (requires `GITHUB_TOKEN`).

### Local Testing

For development without hitting external APIs:

```bash
cargo test -- --ignored  # Tests with #[ignore] marker
```

---

## Monitoring & Debugging

### Log Output

```bash
RUST_LOG=debug cargo run --release -- training
```

Look for:
- Rate limit hits
- Prediction confidence scores
- Metric calculations
- Anomalies

### Metrics Inspection

```bash
# View latest aggregated metrics
cargo run --release -- training metrics --format json | jq

# View language-specific metrics
cargo run --release -- training metrics --language rust

# View historical trend
cargo run --release -- training metrics --days 30
```

---

## Future Work

- [ ] Real-time training pipeline (streaming)
- [ ] Active learning (request human feedback on uncertain predictions)
- [ ] Federated training (allow external graph contributions)
- [ ] Multi-modal analysis (combine graph + test coverage + documentation)
- [ ] Confidence calibration (uncertainty quantification)
- [ ] Anomaly detection (flag unusual failure patterns)

---

## See Also

- **[TRAINING_METHODOLOGY.md](../TRAINING_METHODOLOGY.md)** — High-level overview
- **[TRAINING_VALIDATION.md](../TRAINING_VALIDATION.md)** — Metric calculations
- **[TRAINING_DISCOVERY.md](../TRAINING_DISCOVERY.md)** — Discovery mechanics
- **[CONTRIBUTING_TO_TRAINING.md](../CONTRIBUTING_TO_TRAINING.md)** — Contribution guide
- **[src/training/mod.rs](../src/training/mod.rs)** — Module exports
