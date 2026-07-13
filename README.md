# idud: I Don't Understand Databases

**A token-efficient, ultra-fast local graph engine for mapping codebase dependencies.**

idud deterministically maps the hidden dependencies between code concepts. It runs locally, builds an in-memory topological graph, and exports an AI-queryable JSON cheat sheet to prevent token-wasting re-analysis. 

## The Core Philosophy
1. **Local-First & Lean:** No servers, no P2P sync, no external databases. Just a fast Rust CLI.
2. **The "Pirate Bay" Data Model:** We store immutable URI pointers (links to repository code) instead of copying raw logic, keeping the graph footprint tiny.
3. **Zero-Token Traversal:** Spend compute upfront during ingestion. Dependency tracing is computationally free.

## Getting Started

### Installation
```bash
# Clone the repository
git clone https://github.com/tekjanson/idud.git
cd idud

# Build the optimized release
cargo build --release
```

### Local Development
The `data/` directory and any `.db`, `.sqlite`, or `.sqlite3` files are ignored by Git so local ingestion artifacts, training dumps, and databases stay off the repository history. On a fresh clone, generate or fetch this data locally before running repository analysis or training commands, for example with the ingestion/training workflow or by restoring the appropriate data bundle from your team’s shared storage.

### Usage

#### CLI Commands
Run idud as a local CLI tool to ingest repositories and export the mapped ledger:

```bash
# Ingest a repository and build the in-memory graph
cargo run --release -- ingest-repo --url https://github.com/org/repo --branch main

# Trace a chain of obligation (dependency path)
cargo run --release -- trace --start "signatory-uuid" --depth 3

# Export the mapped topology
cargo run --release -- brief --entity "core-auth" --output idud_brief.json
```

#### Visual Graph Rendering
View the contract dependency graph in an interactive web visualization:

```bash
# Start the visualization server (runs on http://127.0.0.1:3000)
cargo run --release -- serve --port 3000 --host 127.0.0.1
```

Then open http://127.0.0.1:3000 in your browser. The visualization features:
- **Interactive D3.js graph** showing signatories (nodes) and contracts (edges)
- **Real-time statistics** displaying signatory and contract counts
- **Searchable signatory list** in the left sidebar
- **Node color coding** by type (Function, File, Class, Test, etc.)
- **Zoom and pan** controls for exploring large graphs
- **Drag-to-reposition** nodes for custom layout

---

## Self-Improving via Training: "Code as Customer"

idud improves through an automated training loop that learns from real GitHub issues and pull requests. The core insight: **every merged PR is a training signal**. When developers fix issues, the files they changed represent ground truth for dependency prediction.

### The Training Pipeline

```
Discover Repos → Ingest Graph → Predict Files → Validate → Measure F1 → Improve
```

1. **Discovery:** Find active public repositories (50+ stars, recent updates)
2. **Ingestion:** Build in-memory dependency graph from code
3. **Prediction:** Given an issue description, predict which files will change
4. **Validation:** Compare predictions to actual PR file changes
5. **Measurement:** Calculate precision, recall, F1 score
6. **Improvement:** Identify patterns, refine graph analysis

### Current Training Results

Latest training session: **2024-07-05**
- **Repositories analyzed:** 47
- **Predictions validated:** 1,847
- **Average F1 Score:** 0.727 ↑ +0.028
- **By Language:**
  - Go: 0.815 ✓ (strongest)
  - Rust: 0.790 ✓
  - C: 0.787 ✓
  - JavaScript: 0.620 (improving)

📊 **[View Full Results →](docs/reference/TRAINING_RESULTS.md)**

### Why This Matters

Traditional dependency analysis is static: parse once, analyze forever. idud's training approach is **dynamic**: the more real code we analyze, the better the predictions become.

- **Measure Progress:** Track F1 scores over time to verify improvements work
- **Identify Blindspots:** Failures on specific languages/patterns surface where graph analysis needs work
- **Validate Hypotheses:** Before deploying a graph optimization, measure F1 improvement in training

### Getting Started with Training

**Run a full training session (idempotent - safe to run repeatedly):**
```bash
# Run once
make idud-grow REPOS=100 CONCURRENT=10

# Run again a week later - skips already-processed issues, adds new ones
make idud-grow REPOS=100 CONCURRENT=10

# Check what's been processed so far
make cache-status
```

The training system is **fully idempotent**: 
- Already-processed (repo, issue) pairs are cached and skipped
- Safe to run through crashes and code updates
- Perfect for long-running training over weeks
- See [TRAINING_IDEMPOTENCY.md](docs/reference/TRAINING_IDEMPOTENCY.md) for scaling strategies

**View aggregated metrics:**
```bash
./target/release/idud cache-status --datalake ./data/training_datalake
```

**Contribute to training:**
See [CONTRIBUTING_TO_TRAINING.md](docs/reference/CONTRIBUTING_TO_TRAINING.md) for how to:
- Add a new repository for training validation
- Interpret false positives vs. false negatives
- Suggest improvements based on prediction failures

### Training Documentation

- **[TRAINING_METHODOLOGY.md](docs/reference/TRAINING_METHODOLOGY.md)** — Deep dive into the self-validation architecture
- **[TRAINING_RESULTS.md](docs/reference/TRAINING_RESULTS.md)** — Latest results, metrics by language, trends
- **[TRAINING_IDEMPOTENCY.md](docs/reference/TRAINING_IDEMPOTENCY.md)** — How to safely scale training over weeks
- **[TRAINING_VALIDATION.md](docs/reference/TRAINING_VALIDATION.md)** — How precision/recall/F1 are calculated
- **[TRAINING_DISCOVERY.md](docs/reference/TRAINING_DISCOVERY.md)** — Repository discovery mechanics
- **[CONTRIBUTING_TO_TRAINING.md](docs/reference/CONTRIBUTING_TO_TRAINING.md)** — How to contribute training data
- **[training/README.md](training/README.md)** — Developer guide for training module internals

### The Vision: "Code as Customer"

idud treats codebases as customers. Each merged PR is feedback: "did you understand how this code works?" By validating against millions of real code changes, idud learns to predict developer intent accurately.

This inverts the traditional ML pipeline: instead of collecting labeled data, we let the codebase teach us.
