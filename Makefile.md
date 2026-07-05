# Makefile Documentation

Complete reference for building, training, and managing the **idud** Contract Ledger system.

## Overview

The Makefile provides a unified interface for all primary operations:
- **Server**: Start the visualization web server
- **Training**: Run the training pipeline on GitHub repositories
- **Development**: Build, test, lint, and format code

All targets provide clear status messages and progress indicators.

---

## Primary Training Targets

### `make idud` - Start the Server

**Purpose**: Build and start the idud web server for visualization and interaction.

**What it does**:
1. Builds the release binary in optimized mode
2. Starts the Actix-web server
3. Displays the server URL and control instructions

**Output**:
```
🚀 Building idud (release mode)...
✓ Build complete

🚀 Starting idud server...
   📡 Running at http://127.0.0.1:3000
   Press Ctrl+C to stop
```

**Usage**:
```bash
# Start the server
make idud

# The server will run on http://127.0.0.1:3000
# Use Ctrl+C in the terminal to stop it gracefully
```

**What you can do with the server**:
- View the contract ledger visualization
- Query registered signatories
- Explore contract relationships
- Import new repositories
- Review training results

---

### `make idud-grow` - Train on Repositories

**Purpose**: Run the distributed training pipeline to discover and process GitHub repositories.

**What it does**:
1. Validates and creates the training datalake directory
2. Discovers GitHub repositories
3. Processes repositories concurrently
4. Registers contracts and signatories
5. Generates training data for model improvement
6. Writes results to the datalake

**Output**:
```
🌱 Starting training pipeline...
   Repos: 100
   Concurrent agents: 10
   Batch size: 2
   Output: ./data/training_datalake

📊 Training in progress...
[Training runs...]

✓ Training complete!
   Results saved to: ./data/training_datalake
```

**Usage**:

```bash
# Train on 100 repos with 10 concurrent agents (defaults)
make idud-grow

# Train on 50 repos with 5 concurrent agents
make idud-grow REPOS=50 CONCURRENT=5

# Train on 200 repos with 20 concurrent agents
make idud-grow REPOS=200 CONCURRENT=20

# Custom batch size and datalake location
make idud-grow REPOS=100 CONCURRENT=10 BATCH_SIZE=5 DATALAKE=/custom/path
```

**Configuration Parameters**:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `REPOS` | `100` | Number of repositories to train on |
| `CONCURRENT` | `10` | Number of concurrent training agents |
| `BATCH_SIZE` | `2` | Batch size per concurrent agent |
| `DATALAKE` | `./data/training_datalake` | Output directory for training results |

**Training Pipeline Details**:

1. **Repository Discovery** - Finds relevant GitHub repos matching criteria
2. **Concurrent Processing** - Distributes work across agents for parallelism
3. **Contract Extraction** - Identifies contracts and signatories
4. **Data Generation** - Creates training datasets for ML models
5. **Aggregation** - Combines results and generates summary statistics

**Output Structure** (`./data/training_datalake/`):
```
training_datalake/
├── repos/                    # Processed repository data
├── contracts/                # Extracted contracts
├── signatories/              # Registered signatories
├── training_data/            # Generated training datasets
└── summary.json              # Training run summary
```

---

## Utility Targets

### `make build` - Build Release Binary

**Purpose**: Compile the project in release mode with optimizations.

**Output**:
```
🔨 Building release binary...
✓ Build complete
```

**Usage**:
```bash
make build
```

**Binary location**: `./target/release/idud`

---

### `make test` - Run All Tests

**Purpose**: Execute the complete test suite to verify code correctness.

**Output**:
```
🧪 Running tests...
[Test output...]
✓ All tests passed
```

**Usage**:
```bash
make test
```

**What's tested**:
- Contract ledger logic
- Repository ingestion
- Signatory registration
- Training pipeline
- Web server endpoints

---

### `make lint` - Run Clippy Linter

**Purpose**: Check code for potential issues, style problems, and anti-patterns.

**Output**:
```
📝 Running clippy...
[Lint output...]
✓ No lint issues found
```

**Usage**:
```bash
make lint
```

**Checks**:
- Unused imports and variables
- Performance issues
- Code clarity problems
- Rust idiom violations

---

### `make fmt` - Format Code

**Purpose**: Automatically format all Rust code to project standards.

**Output**:
```
✨ Formatting code...
✓ Code formatted
```

**Usage**:
```bash
make fmt
```

**Note**: This modifies files in-place. Use `make check-format` to verify without changes.

---

### `make check-format` - Check Code Formatting

**Purpose**: Verify code formatting without making changes.

**Output**:
```
🔍 Checking code format...
✓ Code format is correct
```

**Usage**:
```bash
make check-format
```

**CI/CD Integration**: Use this in pre-commit hooks and CI pipelines.

---

### `make clean` - Remove Build Artifacts

**Purpose**: Delete all compiled binaries and intermediate build files.

**Output**:
```
🧹 Cleaning build artifacts...
✓ Clean complete
```

**Usage**:
```bash
make clean
```

**Use cases**:
- Resolve compilation issues
- Save disk space
- Force a complete rebuild

---

### `make help` - Display Help

**Purpose**: Show all available targets and usage instructions.

**Output**:
```
idud - Contract Ledger Training System

PRIMARY TARGETS:
  idud                 Build and run the server
  idud-grow            Train on repositories

UTILITY TARGETS:
  build                Build release binary
  test                 Run all tests
  ...

ENVIRONMENT VARIABLES:
  REPOS=100            Number of repos for idud-grow (default: 100)
  ...

EXAMPLES:
  make idud
  make idud-grow
  ...
```

**Usage**:
```bash
make help
```

---

## Quick Start Guide

### 1. Development Setup

```bash
# Build the project
make build

# Run tests to verify everything works
make test

# Check code quality
make lint
```

### 2. Start the Server

```bash
# Start the web server
make idud

# Open in browser: http://127.0.0.1:3000
# Press Ctrl+C to stop
```

### 3. Run Training

```bash
# Train on default 100 repos
make idud-grow

# Or customize the training
make idud-grow REPOS=50 CONCURRENT=5
```

### 4. Code Maintenance

```bash
# Format all code
make fmt

# Check formatting (no changes)
make check-format

# Run linter
make lint

# Clean build artifacts
make clean
```

---

## Advanced Usage

### Progressive Training Runs

```bash
# Start with a small pilot
make idud-grow REPOS=10 CONCURRENT=2

# Once validated, scale up
make idud-grow REPOS=100 CONCURRENT=10

# Full production run
make idud-grow REPOS=500 CONCURRENT=20
```

### Custom Output Location

```bash
# Save training data to a specific location
make idud-grow DATALAKE=/mnt/storage/training_data
```

### Parallel Development

Run multiple terminals:

```bash
# Terminal 1: Start the server
make idud

# Terminal 2: Start training
make idud-grow

# Terminal 3: Run tests in watch mode (or manually)
make test
```

---

## Troubleshooting

### Build Failures

```bash
# Clean and rebuild
make clean
make build
```

### Test Failures

```bash
# Run tests with verbose output
cargo test --all -- --nocapture --test-threads=1
```

### Lint Warnings

```bash
# Check what clippy is complaining about
make lint

# Fix formatting first
make fmt
```

### Out of Memory During Training

Reduce concurrency:
```bash
make idud-grow REPOS=100 CONCURRENT=5 BATCH_SIZE=1
```

---

## Integration with CI/CD

### GitHub Actions Example

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: make test
      - run: make check-format
      - run: make lint
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit
make check-format || exit 1
make lint || exit 1
make test || exit 1
```

---

## Performance Tips

### Training Optimization

- **Increase CONCURRENT** for more parallelism (use cautiously - memory intensive)
- **Decrease BATCH_SIZE** to process repos faster but with less thoroughness
- **Increase REPOS** to train on a larger dataset for better model improvement

```bash
# Aggressive training
make idud-grow REPOS=500 CONCURRENT=20 BATCH_SIZE=1

# Conservative training
make idud-grow REPOS=50 CONCURRENT=5 BATCH_SIZE=3
```

### Build Optimization

The Makefile uses `--release` with:
- Level 3 optimizations (`opt-level = 3`)
- Link-time optimization (`lto = true`)
- Single codegen unit (`codegen-units = 1`)

This results in slower builds but much faster execution.

---

## Environment Variables Reference

```bash
# Number of repositories to process during training
export REPOS=100

# Number of concurrent training agents
export CONCURRENT=10

# Batch size for each concurrent agent
export BATCH_SIZE=2

# Directory where training results are saved
export DATALAKE=./data/training_datalake

# Then run with: make idud-grow
```

Or pass them inline:

```bash
make idud-grow REPOS=200 CONCURRENT=15 BATCH_SIZE=3
```

---

## Version Information

- **idud** v0.1.0
- **Rust Edition**: 2021
- **Build Profile**: Release (optimized)

For more information, see [README.md](./README.md) and [SETUP.md](./SETUP.md)
