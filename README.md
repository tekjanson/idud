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
