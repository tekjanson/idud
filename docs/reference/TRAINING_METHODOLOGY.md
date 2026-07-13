# Training Methodology: idud Self-Validation System

## Executive Summary

**idud Self-Validation** is the mechanism by which idud learns to improve itself. Rather than requiring manual annotation or external oracles, idud validates its dependency predictions against ground truth from real GitHub issues and pull requests.

The fundamental insight: **Every merged pull request is a training signal.** When a developer fixes an issue, the files they changed are the ground truth. By predicting which files would change based on the issue description, and comparing predictions to reality, idud measures and improves its contract mapping accuracy.

This is "Code as Customer"—the codebase itself teaches idud how to get better.

---

## Architecture: Discovery → Ingest → Predict → Validate → Improve

### Phase 1: Discovery
**Goal:** Find public repositories suitable for training validation.

```
GitHub (Public API)
    ↓
Discovery Engine
    ↓
Repository Candidates (50+ stars, active, recent updates)
```

- Filters for: ≥50 stars, recent updates (30 days), active issues/PRs
- Returns structured metadata: URL, language, stars, issue count, PR count
- Selection ensures **signal quality**: mature projects with meaningful dependencies
- **Why 50 stars?** Indicates established community, correlates with codebase complexity

### Phase 2: Ingest
**Goal:** Parse candidate repository and build the dependency graph.

```
Repository Candidate
    ↓
Clone/Parse Repository
    ↓
AST Analysis
    ↓
Extract Signatories & Contracts
    ↓
In-Memory Graph (petgraph)
    ↓
Export JSON Ledger
```

- Uses Rust AST parsing for accurate dependency extraction
- Builds topological graph of code dependencies
- Exports to JSON for zero-latency querying
- **Zero external DB:** All analysis happens locally

### Phase 3: Predict
**Goal:** Given a GitHub issue description, predict which files the fix should touch.

```
GitHub Issue (Title + Body)
    ↓
Semantic Analysis
    ↓
Graph Traversal (from issue context)
    ↓
Predicted Files {f1, f2, ...}
```

- Uses LLM (Claude Haiku) to understand issue semantics
- Queries idud's in-memory graph to find related signatories
- Produces list of predicted files + confidence scores
- **Token Efficient:** Graph traversal is free; only LLM call is issue analysis

### Phase 4: Validate
**Goal:** Compare predictions to actual files changed in the linked PR.

```
Predicted Files: {f1, f2}
Actual Files (PR):  {f1, f3}
    ↓
Calculate Metrics
    ↓
Precision, Recall, F1 Score
    ↓
Store Result
```

- **Precision:** Of predicted files, how many changed? (False positive cost)
- **Recall:** Of changed files, how many did we predict? (False negative cost)
- **F1 Score:** Harmonic mean (single summary metric for trend tracking)
- Results persisted to JSONL datalake for historical analysis

### Phase 5: Improve
**Goal:** Identify patterns, surface failures, refine graph models.

```
Training Results (1000s of predictions)
    ↓
Aggregate Metrics (by language, by repo, by issue type)
    ↓
Trend Analysis
    ↓
Identify Improvements
    ↓
Update Graph Model
```

- Language-specific analysis reveals which codebases are predictable
- Percentile tracking (p50, p75, p90, p95) shows tail performance
- Checkpoint tracking reveals improvement over time
- Failures become debugging signals for graph analysis

---

## Data Flow: From Issue to Training Results

### Example: Tracing One Issue Through the Full Pipeline

**Repository:** `rust-lang/rust`  
**Issue:** "Implement const generics for array types"  
**Issue #:** 51747  

#### Step 1: Discovery
```
✓ Found rust-lang/rust in candidate list
  - Stars: 98k
  - Language: Rust
  - Recent activity: Yes
  - Open issues: 12,500+
```

#### Step 2: Ingest
```
✓ Cloned rust-lang/rust
✓ Parsed AST for Rust code
✓ Extracted signatories:
  - src/librustc_typeck/mod.rs (type checker)
  - src/librustc_ast/ast.rs (AST definitions)
  - src/test/ui/const-generics/ (test module)
✓ Built dependency graph (23,451 signatories, 87,392 contracts)
```

#### Step 3: Predict
```
Input Issue:
  Title: "Implement const generics for array types"
  Body: "Currently, we can't use const generics in array type
         declarations. This blocks generic fixed-size arrays.
         We need to update the type checker and parser to handle
         const parameters in array brackets."

Prediction Process:
  1. LLM analyzes issue → keywords: "const generics", "array types",
     "type checker", "parser"
  2. Query graph for signatories matching these keywords
  3. Traverse contracts → related modules
  
Predicted Files:
  - src/librustc_ast/ast.rs (AST changes)
  - src/librustc_typeck/mod.rs (type checking)
  - src/librustc_resolve/mod.rs (resolution)
  - tests/ui/const-generics/array.rs (test)
  - src/librustc_parser/parser.rs (parsing)
```

#### Step 4: Validate
```
PR #51748 (merged, fixes issue #51747)

Actual Files Changed:
  - src/librustc_ast/ast.rs ✓ (predicted)
  - src/librustc_typeck/mod.rs ✓ (predicted)
  - src/librustc_typeck/collect.rs (NOT predicted)
  - src/librustc_resolve/mod.rs ✓ (predicted)
  - tests/ui/const-generics/array.rs ✓ (predicted)
  - src/librustc_parser/parser.rs ✓ (predicted)
  - src/librustc_ast_passes/ast_validation.rs (NOT predicted)

Confusion Matrix:
  TP (True Positives): 4 files correctly predicted
  FP (False Positives): 1 file predicted but didn't change
  FN (False Negatives): 2 files changed but not predicted

Metrics:
  Precision = TP / (TP + FP) = 4/5 = 0.80
  Recall = TP / (TP + FN) = 4/6 ≈ 0.67
  F1 = 2 * (0.80 * 0.67) / (0.80 + 0.67) ≈ 0.73
```

#### Step 5: Store & Learn
```
Stored Training Run:
  {
    "run_id": "2024-07-05-001",
    "repo": "rust-lang/rust",
    "issue_id": "51747",
    "precision": 0.80,
    "recall": 0.67,
    "f1": 0.73,
    "timestamp": "2024-07-05T10:00:00Z"
  }

Aggregated After 1,000 Predictions:
  Across all Rust repos:
    - avg_precision: 0.76
    - avg_recall: 0.71
    - avg_f1: 0.73
  
  Trend: F1 score improved 0.68 → 0.73 over past month
```

---

## Metrics Explained: Why They Matter

### Precision: "Of Files We Predicted, How Many Were Right?"

**Formula:** `Precision = TP / (TP + FP)`

**Why it matters:**
- High false positives = noisy predictions → developers waste time checking irrelevant files
- Low precision = over-predicting
- **For users:** "Can I trust this recommendation?"

**Example:**
```
Predicted: [auth.rs, session.rs, middleware.rs]
Actual:    [auth.rs, session.rs]

TP = 2 (auth.rs, session.rs changed)
FP = 1 (middleware.rs didn't change)
Precision = 2/3 ≈ 0.67

Interpretation: "Of 3 files we recommended, 2 were right. 
Not terrible, but we're suggesting 1 wasted file."
```

### Recall: "Of Files That Changed, How Many Did We Predict?"

**Formula:** `Recall = TP / (TP + FN)`

**Why it matters:**
- High false negatives = missing important files → incomplete understanding
- Low recall = under-predicting
- **For debugging:** "Did we miss important dependencies?"

**Example:**
```
Predicted: [auth.rs, session.rs]
Actual:    [auth.rs, session.rs, middleware.rs]

TP = 2
FN = 1 (middleware.rs changed but we missed it)
Recall = 2/3 ≈ 0.67

Interpretation: "Of 3 files that actually changed, we only found 2. 
We missed middleware.rs—indicating a gap in our dependency graph."
```

### F1 Score: "Balanced Summary of Prediction Quality"

**Formula:** `F1 = 2 * (Precision * Recall) / (Precision + Recall)`

**Why it matters:**
- Single metric for trend tracking
- Harmonic mean: penalizes extreme imbalance
- **For monitoring:** Track weekly/monthly F1 to detect regressions

**Interpretation Table:**

| F1 Range | Quality | Meaning |
|----------|---------|---------|
| 0.90–1.00 | Excellent | Graph analysis is very accurate |
| 0.70–0.89 | Good | Predictions are reliable; minor gaps exist |
| 0.50–0.69 | Moderate | Predictions need refinement |
| 0.30–0.49 | Poor | Significant gaps in dependency model |
| 0.00–0.29 | Very Poor | Critical issues; investigate graph logic |

**Example Scenarios:**

- **F1 = 0.95** (Precision 0.94, Recall 0.96)
  - Nearly perfect prediction
  - Only minor files missed or over-predicted
  - No action needed; model is working well

- **F1 = 0.65** (Precision 0.80, Recall 0.53)
  - Predicting well but missing some files
  - False negatives are the problem
  - Action: Audit graph traversal logic; check for missing implicit contracts

- **F1 = 0.65** (Precision 0.55, Recall 0.80)
  - Finding most files but over-predicting
  - False positives are the problem
  - Action: Tighten prediction thresholds; reduce noisy edges in graph

---

## Results Interpretation: What Good Results Look Like

### Baseline Expectations

Based on public repos with moderate complexity:
- **Baseline F1:** 0.65–0.72 (realistic starting point)
- **Good F1:** 0.75–0.85 (model is reliable)
- **Excellent F1:** 0.85+ (production-ready)

### By Programming Language

Different languages have different predictability:

```
Language         Repos  Avg F1   Reason
───────────────────────────────────────
Rust             28     0.76     Type system = explicit contracts
Python           24     0.69     Dynamic = implicit contracts
Go               18     0.78     Interface-based = clear contracts
JavaScript       22     0.62     Loose coupling = hard to predict
Java             15     0.74     Class hierarchy = traceable
```

**Why the variation?**
- **Type systems are helpful:** Rust, Go, Java have explicit signatures
- **Dynamic languages are harder:** Python, JavaScript have runtime inference
- **Architecture matters:** Well-separated concerns score higher

### Percentile Distribution

Track the distribution, not just average:

```
Metric    Value   Meaning
──────────────────────────────────────
p50 F1    0.75    "Half of our predictions are better than 0.75"
p75 F1    0.83    "75% of predictions score > 0.83"
p90 F1    0.91    "90% of predictions are very good"
p95 F1    0.94    "Our best 5% are nearly perfect"
```

**Healthy tail distribution:**
- p75 > 0.80 (most predictions are solid)
- p90 > 0.88 (tail performance is strong)
- p95 > 0.92 (best cases are excellent)

### Improvement Signals

**Weekly Checkpoint View:**
```
Week       Avg F1    Trend
────────────────────────────
Jan 1      0.68      Starting baseline
Jan 8      0.70      +0.02 ✓ (tweaked graph)
Jan 15     0.73      +0.03 ✓ (added contracts)
Jan 22     0.70      -0.03 ✗ (regression—investigate)
Jan 29     0.74      +0.04 ✓ (fixed contracts)
```

**When to celebrate:** F1 increases consistently week-over-week  
**When to investigate:** F1 drops > 0.02 points  
**When to refactor:** F1 plateaus for 3+ weeks

---

## Limitations: What We DON'T Measure

### 1. **Non-code Changes**
We measure file-level predictions. We miss:
- Documentation-only changes
- Configuration changes (`.env`, `.yml`)
- Comments or docstrings

**Implication:** True recall may be higher than we measure (we're not penalizing these)

### 2. **Implicit Dependencies (Test Files)**
Test files often change alongside source, but relationship is implicit:
```
Issue: "Fix bug in authentication"
  →  src/auth.rs changes
  →  tests/auth_test.rs also changes (we might miss)
```

**Implication:** False negatives on tests are common but expected

### 3. **External Codebases**
We only analyze within-repo dependencies. We don't see:
- Library imports
- Third-party contracts
- Type inference from external packages

**Implication:** Cross-repo dependencies are invisible to our graph

### 4. **Historical vs. Current**
Training data reflects PR changes at merge time, not evolution:
- File was refactored since then?
- Code structure is different now?
- Historical contracts may not reflect current graph

**Implication:** Very old training data (>6 months) may not reflect current codebase

### 5. **Semantic Ambiguity**
Some issues describe multi-faceted changes:
```
Issue: "Improve performance and fix security bug"
  → Could touch auth code, caching, encryption
  → Prediction must choose; ground truth is single PR
```

**Implication:** High ambiguity issues produce lower F1 scores legitimately

---

## Future Improvements: Where We're Going

### Short Term (Weeks)
- [ ] **Multi-language analysis:** Separate metrics for each language
- [ ] **Confidence scoring:** Per-prediction confidence for uncertainty quantification
- [ ] **Failure clustering:** Identify categories of wrong predictions
- [ ] **Weekly automated reports:** Public results published automatically

### Medium Term (Months)
- [ ] **Continuous retraining:** Automated daily training runs
- [ ] **Anomaly detection:** Flag unusual prediction failures
- [ ] **Comparative analysis:** "This repo has F1 0.80, why is that repo 0.60?"
- [ ] **File-type breakdown:** Separate metrics for tests vs. source vs. config
- [ ] **Issue-type tagging:** "Bug fixes" vs. "features" vs. "refactors"

### Long Term (Quarters)
- [ ] **Active learning loop:** Automatically request human feedback on uncertain predictions
- [ ] **Graph refinement:** Use failures to refine edge weights in dependency graph
- [ ] **Transfer learning:** Train on high-signal repos, apply to new ones
- [ ] **Developer feedback:** Integration with IDEs to collect live predictions
- [ ] **Cross-language generalization:** Learn patterns that transcend languages
- [ ] **Regression testing:** Automated detection of regressions in graph analysis

### Experimental
- [ ] **Symbolic reasoning:** Combine graph analysis + semantic understanding
- [ ] **Test-driven discovery:** Use test failures to infer missing contracts
- [ ] **Blame history:** Use git blame to weight recent vs. historic changes
- [ ] **Community signal:** Weight "popular files" higher in predictions

---

## How to Interpret Your Training Session

When you run a training batch, you'll see output like:

```
Training Session: 2024-07-05-train-001
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📊 Results by Repository:
  rust-lang/rust:    42 predictions, F1 = 0.76 ↑
  kubernetes/k8s:    38 predictions, F1 = 0.72 ↔
  golang/go:         35 predictions, F1 = 0.81 ↑
  torvalds/linux:    29 predictions, F1 = 0.68 ↓
  
📈 Aggregated Metrics:
  Total Predictions: 1,247
  Avg Precision: 0.74
  Avg Recall: 0.71
  Avg F1: 0.72

📍 Percentiles (F1 Distribution):
  p50: 0.74  (median)
  p75: 0.82  (75th percentile)
  p90: 0.89  (90th percentile)
  p95: 0.92  (95th percentile)

📊 Language Breakdown:
  Rust:   15 repos, avg F1 = 0.77 ✓
  Python: 12 repos, avg F1 = 0.68
  Go:     8 repos, avg F1 = 0.79 ✓
  C:      5 repos, avg F1 = 0.62

🔍 Notable Findings:
  • Rust repos trending up (+0.04 vs. last week)
  • Python has high false negatives (recall = 0.63)
  • kubernetes/k8s plateauing at 0.72 for 2 weeks
  • new repo: llama2-cpp/llama2.cpp has promising F1 = 0.84

⚠️  Regressions:
  • torvalds/linux dropped from 0.72 → 0.68
    (Investigate: File structure changed recently?)
```

**How to act on this:**

| Signal | Action |
|--------|--------|
| F1 trending up | ✓ Model is improving; document what changed |
| Language-specific gaps | Review graph extraction for that language |
| Repository regression | Check if repo structure changed; may need re-ingest |
| High false negatives | Graph traversal too conservative; expand search radius |
| High false positives | Graph traversal too aggressive; tighten thresholds |

---

## Integration Points

idud's training system integrates with:

1. **GitHub Discovery** → Finds training candidates
2. **Repository Ingestion** → Builds dependency graph
3. **LLM Predictor** → Generates file predictions from issues
4. **Validation Engine** → Calculates metrics
5. **Training Datalake** → Persists results
6. **Visualization UI** → Shows training trends
7. **API Server** → Exposes results to external tools

This creates a closed feedback loop: **Code → Analysis → Prediction → Validation → Improvement → Better Analysis**

---

## See Also

- **[TRAINING_RESULTS.md](TRAINING_RESULTS.md)** — Public results log and publishing template
- **[CONTRIBUTING_TO_TRAINING.md](CONTRIBUTING_TO_TRAINING.md)** — How to add repos and interpret failures
- **[TRAINING_VALIDATION.md](TRAINING_VALIDATION.md)** — Deep dive on metrics calculations
- **[TRAINING_DISCOVERY.md](TRAINING_DISCOVERY.md)** — Repository discovery mechanics
