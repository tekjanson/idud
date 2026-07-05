# idud: Copilot Instructions

## Project Vision

**idud** ("I Don't Understand Databases") is an **AI-first** concept and knowledge mapping database tool designed to reduce token burn and agentic overhead by creating hard, durable, repeatable paths for mapping large knowledge spaces.

### Core Purpose
- Map complex knowledge spaces (~150+ repos, product docs, internal knowledge stores) efficiently
- Store **concepts** and **proof of concepts** (evidence/sources) with machine-readable structure
- Build knowledge networks: links, hashes, URLs, and other data sources form queryable graphs
- Enable AI systems to extract and synthesize knowledge without wasteful re-analysis
- Define entities through their contracts and agreements
- Grow the database as knowledge grows—one database per entity

---

## AI-First Architecture

### Token Efficiency Philosophy
The tool is designed to **minimize token burn** by replacing expensive agentic work with structured data paths:
- **Bulk Operations**: Use scripts, indexing, and batch queries to extract knowledge at scale (90% of work)
- **Agentic Use Only**: Reserve LLM agents for synthesis, anomaly detection, and novel analysis (10% of work)
- **Caching & Memoization**: Store computed relationships, classifications, and embeddings to avoid re-analysis
- **Structured Output**: All data operations produce machine-readable formats (JSON, structured fields)

### Knowledge Extraction Pipeline
1. **Source Ingestion**: Automated scripts parse repos, docs, APIs
2. **Schema Normalization**: Convert diverse sources to common concept/proof structure
3. **Relationship Discovery**: Build concept graph through deterministic rules (not LLM calls)
4. **Embedding & Indexing**: Pre-compute embeddings for semantic search (done once, reused many times)
5. **LLM Analysis Only**: Use agents for interpretation, context-building, or finding gaps in coverage

### Data Sources (Scalable Ingestion)
- GitHub repos (clone, parse README/CONTRIBUTING, extract API signatures)
- Product documentation (crawl, extract structured sections)
- Knowledge stores (wiki, notion, obsidian exports)
- Internal docs (markdown, diagrams, decision records)
- Configuration files (.github/copilot-instructions.md patterns establish conventions across repos)

---

## Architecture & Design Principles

### Knowledge as Contracts

The database models entities through explicit **contractual bindings**:

- **Signatories**: Code entities (files, functions, tests, endpoints) that enter into obligations
- **Contracts**: Immutable bindings between signatories specifying obligation types
- **Clauses**: Specific obligation types (Requires, Audits, Calls, Uses, Enslaves, Documents)
- **Confidence**: How certain the AI is that a contract exists (0.0-1.0)

This contract-based model ensures clarity: if something is written as a binding, it's actionable and queryable.

### Concept Mapping Structure: From Concepts to Products

The database models how complex systems emerge from interdependency:

**Concepts** (independent ideas)
- Core ideas or facts about an entity (machine-readable, searchable)
- Initially independent knowledge units

**Concept Dependencies** ("Enslavement")
- When concepts relate to each other, they lose independence
- Enslaved concepts: ones that require other concepts to function
- Track which concepts depend on which (direct dependencies)
- Track coupling strength (tight vs loose)

**Workflows** (enslaved concept clusters)
- Emerge when enough concepts become interdependent
- A repeatable sequence or pattern of enslaved concepts
- Example: "User Authentication Workflow" enslaves: password validation, session management, token generation, permission checking

**Products** (workflow compositions)
- Emerge when enough workflows come together
- A coherent set of enslaved workflows serving a purpose
- Example: "SaaS Platform" = authentication workflow + data persistence workflow + API workflow + analytics workflow

**Proofs**: Evidence supporting concepts (documents, URLs, hashes, sources with metadata)
**Links**: Deterministic relationships between concepts (hierarchies, dependencies, associations)
**Versioning**: Track how knowledge evolves; maintain proof chains across versions

### One Entity, One Database
Each idud database represents a single entity (one product, one company, one ecosystem) with a cohesive data model.

### Product Owner Layer
A dedicated UI for product owners to visualize and manage:
- **Dependency Graph**: Which concepts are enslaved to which (visual network)
- **Coupling Analysis**: Which concepts have high coupling (fragile if changed)
- **Workflow Inventory**: All identified workflows and their concepts
- **Product Composition**: How workflows combine into the product
- **AI Cheat Sheet**: Auto-generated knowledge base for AI systems (no token re-parsing)
- **Change Impact**: When a concept changes, see which workflows/products are affected

### LLM as Analyst, Not Executor
- LLMs synthesize and reason over data paths (not generate them)
- Use structured queries to answer questions instead of re-running analysis
- Store all conclusions with provenance (which proofs led to which conclusions)
- **AI Cheat Sheet Generation**: LLMs create summaries from the concept graph, not from raw docs

---

## Product Owner Layer & AI Cheat Sheet

### Why This Matters
The key insight: **All system complexity emerges from concept enslavement.** Product owners need to see and manage this.

**Before idud**: Product owners manage features independently, blind to dependencies and coupling.
**With idud**: Product owners see which concepts are enslaved to which, understand why changes are expensive, and prevent fragile architectures.

**For AI systems**: Instead of re-reading 150 repos, AI queries a pre-built dependency graph and cheat sheet. Massive token savings.

### Product Owner Dashboard
A dedicated view for product/engineering leadership showing:

1. **Dependency Network Visualization**
   - Nodes: Concepts (color-coded by coupling strength)
   - Edges: Dependencies (thickness = coupling strength)
   - Clusters: Workflows (auto-detected as concept groups)
   - Composition: Products (workflow groups)

2. **Coupling Metrics**
   - Which concepts are most enslaved (highest in-degree)
   - Which concepts enslave the most others (highest out-degree)
   - Coupling density: Is this product well-designed or spaghetti?
   - Critical path: Which concepts, if broken, break everything?

3. **Workflow Inventory**
   - Auto-detected workflows (concept clusters with high internal coupling)
   - Workflows not yet named/documented
   - Workflows with weak boundaries (over-coupled to other workflows)

4. **Change Impact Analysis**
   - "If we modify [concept], which workflows/products are affected?"
   - Risk assessment: Low, Medium, High impact
   - Blast radius: Number of dependent concepts

5. **Product Roadmap Integration**
   - Link planned features to concepts
   - See dependencies before committing to timelines
   - Identify "enslaved features" that can't be de-coupled without major work

### AI Contract Brief Generation

**What it is**: A machine-readable ledger snapshot auto-generated from the Contract Ledger.

**Format**:
```json
{
  "ledger": "workspace-name",
  "generated_at": "2024-01-15T10:30:00Z",
  "signatories": [
    {
      "id": "auth-validate",
      "type": "Function",
      "label": "validateUser",
      "source_uri": "https://github.com/org/repo/blob/main/src/auth.ts#L42",
      "signatory_type": "FUNCTION"
    }
  ],
  "contracts": [
    {
      "principal_id": "auth-validate",
      "guarantor_id": "password-hash",
      "clause_type": "Requires",
      "confidence": 0.95
    }
  ],
  "chains_of_obligation": [
    {
      "start": "auth-validate",
      "path": ["auth-validate", "password-hash", "crypto-lib"],
      "depth": 2
    }
  ]
}
```

**Usage**: AI systems load this once instead of re-analyzing 150+ repos:
- "What does this function require?" → Query contracts by principal_id
- "Will changing this break anything?" → Trace chains of obligation
- "What tests cover this?" → Find Audits clauses

**Cost**: One-time generation (~minutes for 150+ repos), then queries are free (no LLM tokens).

---

## Tech Stack & Implementation

### Current Status
**Phase 1-3 Complete**: Rust implementation with core ingestion pipeline, type system, and pipelines.
- ✅ Contract Ledger schema (Signatories, Contracts, Clauses)
- ✅ Repository ingestion (broad sweep extraction)
- ✅ AI contract discovery (deep link with mock LLM)
- ✅ Semantic embeddings (embedding layer)
- ⏳ Phase 4: Query layer & contract brief export (in progress)

### Core Implementation (Rust)

**Backend**: Rust with tokio async runtime
- **Concurrency**: DashMap for thread-safe concurrent ledger storage
- **Graph Algorithms**: petgraph for obligation tracing and topological traversal
- **Repository Extraction**: git2 for cloning, regex for code parsing, walkdir for traversal
- **LLM Integration**: Mock adapter ready for OpenAI/Ollama integration
- **Embeddings**: Deterministic mock embeddings for testing, integration point for real models

### Build & Test Commands

```bash
# Build
cargo build --release

# Test (100% UAT coverage for core types)
cargo test --lib

# Run CLI
cargo run -- ingest-repo --url <repo> --branch main
cargo run -- audit
cargo run -- trace --start <id> --depth 3
cargo run -- brief --entity <name> --output out.json

# Format & lint
cargo fmt
cargo clippy --all-targets
```

### Ingestion Pipeline (3 Phases)

**Phase 1: Broad Sweep** (90% of work, deterministic)
- Clone repository
- Parse code (functions, classes, types)
- Extract tests (test() and it() calls)
- Extract documentation (markdown headers + content)
- Register all as Signatories

**Phase 2: Deep Link** (10% of work, LLM-assisted)
- For each Signatory, infer Contracts to other Signatories
- System prompt: "What obligations exist from this code element to others?"
- Mock LLM for testing, real API for production
- Batch processing for efficiency
- Confidence scoring for each Contract

**Phase 3: Embedding Layer**
- Generate 384-dim semantic vectors for each Signatory
- Build vector index for nearest-neighbor search
- Enable organic clustering without explicit ontology
- Deterministic seeding for reproducible tests

### Frontend (To Be Implemented)
- **Framework**: React (to be finalized)
- **Dashboard**: Dependency network visualization, coupling metrics, contract browser
- **Build**: `cargo run --release` for backend API, separate frontend build

### Testing & Quality
- **Primary Focus**: UAT (User Acceptance Testing) from a user perspective
- **Test Coverage**: 100% coverage required—all user workflows and data paths must be tested
- **Test Command**: `npm run test` (to be finalized)
- **UAT Scope**: Every user action, data transformation, edge case, and ingestion path must be covered
- **Lint Command**: `npm run lint` (to be finalized)

---

## Key Conventions

### Self-Updating & Reactivity (AI-Aware)
1. **Event-Driven Updates**: When new concepts or proofs arrive, trigger indexing/embedding pipeline
2. **Incremental Analysis**: Only re-analyze changed data; cache stable relationships
3. **Component Philosophy**: Build modular components that react to data changes without re-fetching

### Data Integrity & Provenance
1. **Immutability**: Once a proof is recorded, maintain its history; updates create new versions
2. **Full Traceability**: Every piece of data tracks its source (URL, hash, timestamp, extraction date)
3. **Versioning**: Concept definitions versioned; know what changed and when
4. **Audit Trail**: Store why data was ingested (which LLM call, which repo scan, manual entry)
5. **Enslavement Tracking**: Record when concepts become dependent on others; track coupling evolution

### Concept Dependency Analysis
1. **Direct Dependencies**: Concept A depends on Concept B (explicit requirement)
2. **Transitive Dependencies**: Concept A → B → C (indirect coupling chains)
3. **Coupling Detection**: When enough concepts are interdependent, they form a workflow
4. **Product Composition**: When enough workflows cluster, they form a product
5. **Change Impact Analysis**: Predict blast radius of changes based on dependency graph

### Ingestion & Extraction Best Practices
1. **Batch Over Streaming**: Process sources in bulk to amortize LLM costs across many concepts
2. **Schema Validation**: All ingested data must conform to concept/proof structure before storage
3. **Deduplication**: Use hashes/URLs to detect and merge duplicate proofs
4. **Structured Extraction**: Parse README, API docs, code comments into fields—don't store raw text

### Testing Conventions
1. **UAT-First Mindset**: Write tests from a user's perspective—what workflows matter?
2. **Full Coverage**: No untested code paths. Every branch, every user action, every ingestion pipeline must have a test
3. **Test Structure**: Organize tests by user workflows and data paths, not by implementation details
4. **Batch Integration Tests**: Test bulk ingestion and relationship discovery end-to-end

### Code Readability
- Use clear, descriptive naming—anyone reading the code should understand intent
- Document why, not what—code should be self-evident; comments explain trade-offs
- Keep functions small and focused on one concept or pipeline step

---

## Development Workflow

### Starting Development
1. **Understand the Data Model**: Review concept/proof schema and current database state
2. **Write UAT Tests First**: Define user workflows or ingestion pipelines as tests
3. **Implement Feature or Pipeline**: Write code to pass tests
4. **Ensure 100% Test Coverage**: Every branch, every edge case must be covered before merging
5. **Validate Locally**: Test in dev server with real data

### Adding New Concepts or Proofs
1. Update the data model documentation
2. Write tests covering ingestion, deduplication, and relationship discovery
3. Implement extraction pipeline (script or LLM-assisted analysis)
4. Validate data quality and schema conformance
5. Index and prepare for semantic search

### When to Use Agents vs Scripts
**Use Deterministic Scripts** (90% of work):
- Parsing repos, docs, config files
- Building indexes and embeddings (batch)
- Relationship discovery via rules
- Data validation and consistency checks

**Use LLM Agents** (10% of work):
- Classifying ambiguous concepts
- Finding gaps in coverage (e.g., "which repos are missing API docs?")
- Synthesizing relationships across domains
- Generating summaries or explanations
- Novelty detection (concepts that don't fit existing patterns)

---

## Helpful Context

- **Token Efficiency is a Feature**: Prefer 100 determistic queries over 1 agentic brainstorm
- **Durable Paths Win**: Repeatable extraction pipelines beat one-off analyses
- **Scale Matters**: Designed for 150+ repos; architecture must support bulk operations
- **LLM as Analyst**: Use agents to understand data, not to generate it
- **Living Database**: Expect the schema and extraction patterns to evolve as use cases emerge
