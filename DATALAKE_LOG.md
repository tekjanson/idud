# Data Lake Ingestion Log

**Last Updated**: 2026-07-05T12:31:35Z (initialized)

## Current Status

- **Run ID**: Not yet started
- **Duration**: 0 seconds
- **Repos Processed**: 0/24
- **Success**: 0 | **Failed**: 0

## Aggregated Metrics

- **Total Files**: 0
- **Total Signatories**: 0
- **Total Contracts**: 0

## Repository Breakdown

| Repo | Status | Files | Signatories | Contracts | Time (s) |
|------|--------|-------|-------------|-----------|----------|

---

## Usage

To start ingesting repositories, run:

```bash
make datalake-grow
```

This will:
1. Load the curated repository registry from `data/repos_to_ingest.json`
2. Ingest each repository using AST-based analysis (no AI)
3. Extract signatories and contracts
4. Track progress in this log
5. Store results in `data/ingestion-log.json`

### Quick Examples

```bash
# Test with 3 repos
make datalake-grow MAX_REPOS=3

# Ingest for 30 minutes
make datalake-grow DURATION_MINUTES=30

# Full ingest (all 24 repos)
make datalake-grow
```

### Monitor Progress

```bash
# Show current status
make datalake-status

# Watch progress in real-time
watch -n 5 'tail -30 DATALAKE_LOG.md'
```

## Registry

The curated registry includes 24 high-quality open-source repositories:

- **JavaScript/TypeScript**: lodash, three.js, react, vue, express, webpack, grafana
- **Rust**: actix-web, tokio, serde, rust
- **Python**: django, cpython, numpy, pytorch
- **Go**: go, kubernetes, docker-ce, prometheus, etcd
- **Java**: spring-framework, eclipse.jdt.core
- **C**: linux, cpython

**Total**: 24 repos across 6 languages

## Architecture

See `REPO_ORCHESTRATOR_GUIDE.md` for detailed documentation on:
- How the orchestrator works
- Configuration options
- Performance characteristics
- Troubleshooting
- Integration with training pipeline
