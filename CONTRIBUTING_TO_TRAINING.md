# Contributing to idud Training

idud improves through training: analyzing real GitHub issues and pull requests, predicting file changes, and measuring accuracy. This guide explains how to contribute to that process.

---

## Quick Start: Add a Repository for Training

### Step 1: Identify a Repository

Look for repositories that:
- ✅ Have 50+ stars (established community)
- ✅ Are actively maintained (updated within 30 days)
- ✅ Have open issues and recent PRs
- ✅ Use a language you care about
- ✅ Have non-trivial dependency structure

Good candidates:
- Established frameworks (Rails, Django, Spring Boot)
- Language runtimes (Python, Node.js, Go)
- Distributed systems (Kubernetes, etcd, Consul)
- Libraries (TensorFlow, PyTorch, scikit-learn)

**Avoid:**
- Single-file projects (no dependency learning)
- Inactive repos (stale patterns)
- Closed-source or archived projects
- Configuration-only repos

### Step 2: Propose the Repository

Open an issue with the title **"[Training] Add repo: owner/name"**

**Template:**
```
## Repository
- URL: https://github.com/owner/repo
- Language(s): Go, Python
- Stars: 15.3k
- Last Update: 2024-06-20

## Why This Repo?
Brief explanation of why this repo would help idud:
- Complex interdependencies between packages
- Well-maintained with clear PR practices
- Underrepresented language in training

## Expected Learning
What patterns might idud learn?
- Package initialization ordering
- Middleware composition
- Plugin architecture
```

### Step 3: Verification

idud maintainers will verify:
- Repository meets criteria
- We can access it (public, no auth needed)
- PR practices are good (linked issues, clear file changes)
- Language is supported

### Step 4: Ingestion

Once approved, the repo is added to the training candidate list. Next scheduled training run will ingest it automatically.

---

## Interpreting False Positives vs. False Negatives

When idud makes a prediction that's wrong, understanding *why* helps improve the system.

### False Positive: "File predicted but didn't change"

**What it means:**
```
Issue: "Add caching to API responses"
Predicted: [src/cache.rs, src/api.rs, tests/cache_test.rs]
Actual:    [src/cache.rs, src/api.rs]

❌ False Positive: tests/cache_test.rs (predicted but didn't change)
```

**Root causes:**

1. **Test files always co-change assumption** (most common)
   - idud predicts tests based on source files
   - But maybe tests weren't needed for this issue
   - **Signal:** High false positives on test files = need smarter test prediction

2. **Over-eager graph traversal**
   - Graph says module A depends on module B
   - But the issue only touched A
   - **Signal:** Tighten edge weights or traversal depth

3. **Implicit coupling assumption**
   - idud sees files frequently co-change
   - But this change broke that pattern
   - **Signal:** Temporal weighting could help

**How to debug:**
```bash
# Compare prediction confidence vs. correctness
cargo run --release -- training debug \
  --repo github/owner/repo \
  --issue 12345 \
  --verbose

# Output:
# Predicted files with confidence:
#   src/cache.rs (confidence: 0.92) ✓ correct
#   src/api.rs (confidence: 0.87) ✓ correct
#   tests/cache_test.rs (confidence: 0.63) ✗ false positive
#
# -> Low confidence + wrong = trimming threshold would help
```

**Action:**
- If false positives are high: Report as "Over-prediction" issue
- Include repository name and language
- Add example predictions with confidence scores

### False Negative: "File changed but we didn't predict it"

**What it means:**
```
Issue: "Refactor authentication system"
Predicted: [src/auth/login.rs, src/auth/session.rs]
Actual:    [src/auth/login.rs, src/auth/session.rs, src/auth/middleware.rs]

❌ False Negative: src/auth/middleware.rs (actually changed, not predicted)
```

**Root causes:**

1. **Missing dependency in graph** (most critical)
   - File exists but isn't in our dependency graph
   - Indicates gaps in AST parsing
   - **Signal:** Graph ingestion needs improvement

2. **Implicit relationship not modeled**
   - Middleware is indirectly related to session
   - But graph doesn't capture that connection
   - **Signal:** Need to add implicit edge types

3. **Issue text ambiguous**
   - Middleware not mentioned in issue
   - LLM didn't infer it would change
   - **Signal:** Semantic analysis could be improved

**How to debug:**
```bash
# Check if file is in the graph
cargo run --release -- graph query \
  --repo github/owner/repo \
  --file src/auth/middleware.rs

# Output:
# File: src/auth/middleware.rs
# Signatories: 14
# Inbound Contracts: 8
# Outbound Contracts: 12
# 
# -> If file exists but has few contracts, we're not capturing its role
# -> If file doesn't exist, AST parsing missed it

# Check what connects to session
cargo run --release -- graph query \
  --repo github/owner/repo \
  --search "session" \
  --max-depth 2

# Output:
# Direct connections to session.rs:
#   src/auth/login.rs -> session.rs (explicit import)
#   src/auth/token.rs -> session.rs (explicit import)
# 
# Transitive connections (depth 2):
#   src/auth/middleware.rs -> token.rs -> session.rs
#
# -> Middleware *is* connected, but prediction didn't traverse deep enough
```

**Action:**
- If graph is incomplete: Report as "Graph ingestion gap" 
- Include repository, language, missing file
- Provide minimal example showing what should be connected

- If graph is complete but traversal missed it: Report as "Traversal logic"
- Include example issue and all predicted vs. actual files
- Helps calibrate prediction depth

---

## How to Suggest Improvements Based on Failures

### Analyze Failure Patterns

Run a failure analysis:

```bash
# Generate failure report
cargo run --release -- training analyze-failures \
  --repo github/owner/repo \
  --language rust \
  --output failures.json
```

**Sample output:**
```json
{
  "repository": "rust-lang/rust",
  "language": "Rust",
  "predictions": 45,
  "accuracy": 0.848,
  "failure_patterns": [
    {
      "type": "false_negatives",
      "count": 5,
      "percentage": 11.1,
      "common_files": [
        "src/librustc_hir/hir.rs",
        "src/librustc_error_codes.rs"
      ],
      "hypothesis": "HIR layer changes not predicted from parser changes"
    },
    {
      "type": "false_positives",
      "count": 3,
      "percentage": 6.7,
      "common_files": [
        "tests/ui/parser_tests.rs"
      ],
      "hypothesis": "Parser test prediction too aggressive"
    }
  ]
}
```

### Formulate Hypothesis

Based on failures, propose a specific improvement:

```markdown
## Issue: [Improvement] Better detection of HIR transformations in Rust

### Problem
In rust-lang/rust, false negatives on HIR files (src/librustc_hir/*.rs) 
are 11% of failures. When parser changes, HIR should often change too, 
but our graph doesn't capture this relationship.

### Root Cause
The HIR layer sits between AST and type-checking. Our graph sees:
- parser.rs -> ast.rs ✓ (explicit)
- ast.rs -> typeck.rs ✓ (explicit)

But misses:
- parser.rs -> hir.rs ✗ (implicit via compiler stages)

### Proposed Fix
Add "implicit layer" analysis for Rust compiler:
1. Recognize compiler pipeline stages (lexer → parser → HIR → typeck → codegen)
2. When files in stage N change, predict files in stages N+1
3. Weight by distance (stage N+1 higher probability than N+2)

### Expected Impact
- F1 improvement: +0.03 (estimated)
- Mainly from false negative reduction
```

### Submit as Issue

Open an issue with title: **"[Training] Improvement: [specific suggestion]"**

Include:
- Repository and language
- Specific failure pattern (with numbers)
- Root cause hypothesis
- Proposed solution
- Expected impact on metrics

---

## Understanding idud's Codebase

### Training Pipeline Components

```
┌─────────────────────────────────────────────────────────────────┐
│                   Training Module (src/training/)                │
└─────────────────────────────────────────────────────────────────┘
     │
     ├─ discovery.rs         Find candidate repos (GitHub API)
     ├─ predictor.rs         Predict files from issues (LLM)
     ├─ validator.rs         Validate & calculate metrics
     ├─ orchestrator.rs      Coordinate training pipeline
     └─ mod.rs               Module exports
```

### Key Data Structures

**`RepoCandidate`** — Repository metadata for training
```rust
pub struct RepoCandidate {
    pub url: String,
    pub name: String,
    pub stars: u32,
    pub language: Option<String>,
    pub issue_count: u32,
    pub pr_count: u32,
    pub updated_at: String,
}
```

**`IssueWithPR`** — Issue + linked PR data
```rust
pub struct IssueWithPR {
    pub issue_title: String,
    pub issue_body: String,
    pub issue_number: u32,
    pub pr_number: Option<u32>,
    pub pr_files: Vec<String>,  // Ground truth
}
```

**`PredictionResponse`** — Predicted files from LLM
```rust
pub struct PredictionResponse {
    pub predicted_files: Vec<String>,  // What we think will change
    pub token_usage: TokenUsage,
}
```

**`ValidationMetrics`** — Calculated accuracy metrics
```rust
pub struct ValidationMetrics {
    pub precision: f32,
    pub recall: f32,
    pub f1: f32,
    pub true_positives: u32,
    pub false_positives: u32,
    pub false_negatives: u32,
}
```

### API Endpoints

```
POST   /api/training/discover?limit=100
       → Returns: RepoCandidate[] for training

GET    /api/training/issue/{owner}/{name}/{issue_id}
       → Returns: IssueWithPR (issue + linked PR files)

POST   /api/training/predict
       → Body: PredictionRequest (repo, issue text)
       → Returns: PredictionResponse (predicted files)

POST   /api/training/validate
       → Body: ValidationRequest (predicted + actual files)
       → Returns: ValidationMetrics + run_id

GET    /api/training/metrics
       → Returns: AggregatedMetrics + LanguageMetrics
```

### Running Training Locally

```bash
# Full training session (requires API keys)
cargo run --release -- training --batch-size 10

# Query metrics from previous runs
cargo run --release -- training metrics

# Analyze specific repository
cargo run --release -- training analyze-repo \
  --url https://github.com/owner/repo

# Debug a single prediction
cargo run --release -- training predict \
  --repo-url https://github.com/owner/repo \
  --issue-id 12345
```

### Configuration

Environment variables (see `.env.example`):
```bash
GITHUB_TOKEN=ghp_xxx           # (optional) Increase rate limit
ANTHROPIC_API_KEY=sk-xxx       # Required for LLM predictions
LOG_LEVEL=debug                # (optional) Verbose logging
```

---

## Common Questions

### Q: Can I train on private repositories?

**A:** Not currently. Training discovery uses public GitHub API and can only access public repos. Private repos would require authentication and special handling. If you want to train idud on your private codebase, please open an issue.

### Q: Why is my repository rejected?

Common reasons:
- **Too new:** Less than 50 stars (signal quality concern)
- **Inactive:** No updates in 30 days (stale patterns)
- **Already training:** Repository already in the dataset
- **No issues/PRs:** Need linked PR data for ground truth
- **Unsupported language:** Only major languages currently supported

### Q: How often are training runs scheduled?

**A:** Weekly (every Monday 00:00 UTC). Results published in [TRAINING_RESULTS.md](TRAINING_RESULTS.md).

Can run manually with `cargo run --release -- training --mode manual`.

### Q: Can I benchmark idud on my own repositories?

**A:** Yes! Run:

```bash
cargo run --release -- training benchmark \
  --local-repo /path/to/repo \
  --issues-file issues.json
```

You'd need to provide:
- Local repository path
- JSON file with issue descriptions
- CSV file with actual changed files (PR ground truth)

### Q: What happens to my failure reports?

They're analyzed in the following weekly training session:
1. Pattern matching groups similar failures
2. Hypothesis generation suggests graph improvements
3. Maintainers prioritize fixes based on impact
4. Next training run validates improvements
5. Results published in [TRAINING_RESULTS.md](TRAINING_RESULTS.md)

### Q: Can I help improve graph analysis?

**A:** Absolutely! If you understand a language well:
- Review [src/pipelines/](../src/pipelines/) for language-specific extractors
- Propose improvements to AST parsing or contract extraction
- Validate that our graph captures real dependencies
- Report false patterns that inflate the graph

---

## Review Process

When you submit a training contribution:

1. **Initial Review** (maintainer) ← 24 hours
   - Is repository appropriate?
   - Does it meet criteria?
   - Any duplicate / overlap?

2. **Inclusion** (automatic)
   - Repository added to candidate pool
   - Ingested in next training run

3. **Validation** (1 week)
   - Training run completes
   - Results analyzed
   - Published in TRAINING_RESULTS.md

4. **Feedback** (optional)
   - You receive results for your repository
   - Can see how idud performs on your code
   - Can suggest improvements

---

## Community Recognition

Contributors who help idud improve are recognized:

- **In TRAINING_RESULTS.md:** "Special Thanks" section
- **In commit messages:** "Suggested by @username"
- **In major releases:** Contributors listed in CHANGELOG

---

## See Also

- **[TRAINING_METHODOLOGY.md](TRAINING_METHODOLOGY.md)** — How training works
- **[TRAINING_RESULTS.md](TRAINING_RESULTS.md)** — Latest training results
- **[TRAINING_VALIDATION.md](TRAINING_VALIDATION.md)** — Metrics deep dive
- **[TRAINING_DISCOVERY.md](TRAINING_DISCOVERY.md)** — Discovery mechanics
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — General contribution guidelines

---

## Contact

Questions? 
- Open an issue on GitHub
- Check [TRAINING_RESULTS.md](TRAINING_RESULTS.md) for recent updates
- Review existing training documentation

**Thank you for helping idud learn!** 🚀
