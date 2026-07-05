# idud: I Don't Understand Databases

**A token-efficient knowledge mapping tool for understanding complex systems through concept graphs.**

idud maps large knowledge spaces (150+ repos, product docs, internal knowledge stores) into queryable concept graphs. It's designed to reduce AI token burn by using hard, durable, repeatable data extraction paths instead of wasteful agentic re-analysis.

## The Problem

Understanding a complex system—a software company, a product, an ecosystem—requires analyzing massive amounts of distributed knowledge: repos, docs, configurations, decision records, contracts. Most tools either:
- **Waste tokens**: LLM agents re-analyze the same data repeatedly
- **Lose context**: Treat each document in isolation instead of building connected understanding
- **Don't scale**: Can't handle 150+ repos without exploding costs

## The Solution

idud builds a **concept graph**—a queryable map of what is known about an entity and the proof supporting each claim.

```
Entity (e.g., "Company X")
├── Concept: "Uses microservices architecture"
│   ├── Proof: kubernetes.yaml in repo A
│   └── Proof: Architecture doc (link + hash)
├── Concept: "Supports 3 payment methods"
│   ├── Proof: API docs (section reference)
│   └── Proof: Product roadmap
└── Concept: "Built in TypeScript"
    ├── Proof: tsconfig.json in repos B, C
    └── Proof: Contributing guide
```

Once built, this graph becomes a source of truth:
- **AI agents query it instead of re-analyzing sources**
- **Relationships are machine-readable and reusable**
- **Token burn happens once during ingestion, then queries are cheap**

## Key Features

### AI-First Architecture
- **Bulk extraction pipelines**: Scripts for repos, docs, APIs (not LLM calls for every file)
- **Cached relationships**: Computed once, queried many times
- **Structured output**: All data is machine-readable JSON, not raw text
- **LLM as analyst, not executor**: Use agents to find gaps or synthesize—not to parse files

### Scalability
- Designed for 150+ repositories
- Handles multiple data sources: GitHub, documentation sites, internal wikis, knowledge stores
- Efficient indexing and semantic search (embeddings pre-computed)
- Versioned schema for backward compatibility

### Complete Provenance
Every concept is linked to its proofs:
- Source URL + hash (immutable reference)
- Extraction timestamp and method
- Version history (how knowledge evolved)
- Audit trail (what triggered re-analysis)

### User-Friendly
- React frontend for browsing concept graphs
- Full-text search + semantic search
- Visual relationship explorer
- Workflow-based UI (not database-centric)

## Getting Started

### Prerequisites
- Node.js 18+
- Git
- (Database choice TBD—PostgreSQL/Neo4j/etc.)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/idud.git
cd idud

# Install dependencies
npm install

# Set up environment
cp .env.example .env
# Edit .env with your database connection and API keys
```

### Development

```bash
# Start development server
npm run dev

# Run tests (100% coverage required)
npm run test

# Run linter
npm run lint

# Build for production
npm run build
```

### Ingestion Pipeline

```bash
# Import from a GitHub repository
npm run ingest:repo -- --url https://github.com/org/repo --entity "Company Name"

# Import documentation (markdown/HTML crawl)
npm run ingest:docs -- --url https://docs.example.com --entity "Company Name"

# Import knowledge store (Notion, Obsidian export, etc.)
npm run ingest:knowledge -- --path ./exports/knowledge.json --entity "Company Name"

# Validate and index all data
npm run validate
npm run index
```

## Architecture

### Data Model

**Concepts** (what is known)
- Name, description, category
- Version history
- Relationships to other concepts (dependencies, associations, hierarchies)

**Proofs** (evidence supporting concepts)
- Source: URL, file path, hash
- Type: API doc, README section, code comment, configuration file, etc.
- Extracted: timestamp, method (script, manual, LLM-assisted)
- Content metadata: line ranges, query used to find it

**Entities** (things being understood)
- One entity = one database (one company, one product, one ecosystem)
- Contains concepts and proofs
- Versioned schema

### Technology Stack

**Frontend**
- React 18+ with TypeScript
- State management: Redux/Zustand (TBD)
- Component library: Chakra UI / Headless UI (TBD)

**Backend / Database**
- Node.js + Express / Fastify (TBD)
- Database: PostgreSQL / Neo4j (TBD—graph database preferred for concept relationships)
- Full-text search: Built-in or Elasticsearch (TBD)
- Embeddings: OpenAI API or Ollama (TBD)

**Extraction Pipeline**
- CLI tools for bulk ingestion
- Git/GitHub API for repo analysis
- Web crawlers for documentation
- JSON parsers for structured knowledge stores

## Development Philosophy

### 100% UAT Coverage
All code must pass User Acceptance Tests from a user perspective:
- Every user workflow tested
- Every data path through the system tested
- No untested branches

This is a requirement, not a best practice.

### Token Efficiency Matters
Prefer:
- ✅ 100 deterministic queries → extract all API endpoints from 50 repos
- ✅ 1 batch embedding job → create embeddings for all concepts once
- ❌ 50 LLM calls → "summarize this repo"
- ❌ Agents re-analyzing the same data weekly

### When to Use Agents

**Good use cases** (10% of work):
- Anomaly detection: "Which concepts lack proofs?"
- Gap analysis: "What's documented in one repo but not another?"
- Synthesis: "How do these concepts relate across domains?"
- Novel extraction: "Are there new patterns in this data I haven't seen before?"

**Bad use cases** (the 90%):
- Parsing standard formats (JSON, YAML, Markdown)
- Extracting known patterns (API signatures, config files)
- Building indexes or embeddings
- Validating data

For the 90%, write scripts. For the 10%, use agents.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

### Key Principles
- **Test first**: Write UAT tests before code
- **Document schema changes**: Concept/proof model changes must be versioned
- **Batch over streams**: Prefer bulk operations for efficiency
- **Durable paths**: Create repeatable extraction pipelines, not one-off scripts

## FAQ

### Why "I Don't Understand Databases"?

The name reflects a philosophical shift: instead of asking "How do I query this?", we ask "What am I trying to understand about this thing?" The database is a tool for making understanding explicit and queryable—not a thing to master for its own sake.

### How is this different from a wiki or knowledge base?

Wikis are human-written and manually connected. idud is machine-generated from sources and automatically indexed. It's designed for scale (150+ repos) and for feeding AI systems, not for human browsing primarily (though that's a nice side effect).

### What about schema migrations?

The schema is versioned. Migrations are backward-compatible. When you add a new field to `Concept`, old concepts still work—they just don't have that field populated until re-ingested.

### Can I query across entities?

No. One database = one entity. To understand relationships between entities, you'd maintain separate idud instances and run cross-instance analysis (which is a good use case for an LLM agent).

### How much does this cost to run?

Depends on your data sources and ingestion frequency:
- **One-time ingestion** of 150 repos: ~$50-200 (mostly embeddings)
- **Weekly updates**: ~$10-50 (incremental ingestion)
- **Database + hosting**: ~$20-100/month depending on scale

This is dramatically cheaper than continuously re-running LLM agents to understand the same codebase.

## License

MIT

## Support

- 📖 [Documentation](./docs)
- 💬 [Discussions](https://github.com/yourusername/idud/discussions)
- 🐛 [Issues](https://github.com/yourusername/idud/issues)
