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

### Usage
Run idud strictly as a local CLI tool to ingest repositories and export the mapped ledger:

```bash
# Ingest a repository and build the in-memory graph
cargo run --release -- ingest-repo --url https://github.com/org/repo --branch main

# Trace a chain of obligation (dependency path)
cargo run --release -- trace --start "signatory-uuid" --depth 3

# Export the mapped topology
cargo run --release -- brief --entity "core-auth" --output idud_brief.json
```
