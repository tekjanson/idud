# idud: I Don't Understand Databases

**A token-efficient knowledge mapping tool that reveals how system complexity emerges from concept interdependence.**

idud maps the hidden dependencies between concepts—showing when they lose independence and "enslave" each other. When enough concepts are enslaved, they form workflows. When workflows cluster, they form products. This gives product owners unprecedented visibility into system complexity and gives AI systems a queryable cheat sheet instead of token-wasting re-analysis.

Designed for scale: 150+ repos, product docs, internal knowledge stores. One unified concept graph. One cheat sheet. No token waste.

## The Problem

Understanding a complex system—a software company, a product, an ecosystem—requires analyzing massive amounts of distributed knowledge: repos, docs, configurations, decision records, contracts. Most tools either:
- **Waste tokens**: LLM agents re-analyze the same data repeatedly
- **Miss dependencies**: Treat each document in isolation instead of building connected understanding
- **Blind product owners**: Engineering leaders can't see which features are coupled to which, making changes expensive and risky
- **Don't scale**: Can't handle 150+ repos without exploding costs

## The Solution

idud reveals how complexity emerges: from **concept enslavement**.

### The Core Insight

Concepts start independent. But when they relate to each other, they lose independence—they become "enslaved" to each other. The more enslaved they become:
- Individual changes affect multiple workflows
- Hidden dependencies break assumptions
- Risk balloons exponentially
- Product owners fly blind

**idud visualizes this enslavement**, showing product owners and AI systems the true shape of their product.

### How It Works

```
Concepts (independent ideas)
   ↓ (concepts relate to each other)
Concept Dependencies ("enslavement")
   ↓ (clusters of enslaved concepts)
Workflows (repeatable patterns)
   ↓ (workflows combine)
Products (coherent customer experiences)
```

**Example**:
- Concept: "User Password Validation"
- Enslavement: Depends on "Password Hashing", "Salt Generation", "Rate Limiting"
- Workflow: "User Authentication" (6 enslaved concepts)
- Product: "SaaS Platform" (Authentication + Billing + API Access + Data Sync workflows)

## Key Features

### Concept Enslavement Model
- **Visualize Dependencies**: See which concepts are enslaved to which
- **Measure Coupling**: Identify fragile systems with high coupling
- **Auto-Detect Workflows**: Concepts with high mutual dependency form workflows
- **Compose Products**: Workflows cluster into coherent products
- **Change Impact**: Predict blast radius before you break things

### Product Owner Dashboard
- **Dependency Network**: Visual graph of concept relationships and enslavement
- **Workflow Inventory**: Auto-detected repeatable patterns
- **Product Composition**: How workflows combine into customer experiences
- **Coupling Metrics**: Is this product well-designed or fragile?
- **Change Impact Analysis**: "If I modify X, what breaks?"
- **Roadmap Integration**: Link features to dependencies before committing timelines

### AI-First Architecture
- **Bulk extraction pipelines**: Scripts for repos, docs, APIs (not LLM calls for every file)
- **Cached relationships**: Computed once, queried many times
- **Structured output**: All data is machine-readable JSON, not raw text
- **AI Cheat Sheet**: Auto-generated knowledge base from the concept graph
- **LLM as analyst, not executor**: Use agents to find gaps or synthesize—not to parse files

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

### Why "Concept Enslavement"?

Independence is fragile. When concepts relate to each other, they lose freedom to change independently. That's enslavement. Most systems hide this; idud makes it visible.

**Real example**: You can't change your password hashing algorithm without affecting login rate limiting, session management, password reset, and 2FA. These concepts are enslaved to each other. If one fails, they all fail.

Product owners need to see this enslavement before committing to timelines or architecture changes.

### Why "I Don't Understand Databases"?

The name reflects a philosophical shift: instead of asking "How do I query this?", we ask "What am I trying to understand about this thing?" The database is a tool for making understanding explicit and queryable—not a thing to master for its own sake.

Also: It's humbling. Databases are designed to scale queries, but understanding complex systems requires understanding *relationships and enslavement*, not just data efficiency.

### How is this different from a dependency graph tool?

Most tools (Maven, npm, cargo) show package dependencies. idud shows **concept dependencies** at the business/feature level:
- Package dependencies: "npm A depends on npm B"
- Concept dependencies: "User authentication depends on password validation, rate limiting, and session management"

idud is for PMs, architects, and AI systems trying to understand *what a system does*. Dependency graph tools are for compilers/build systems.

### How is this different from a wiki or knowledge base?

Wikis are human-written and manually connected. idud is machine-generated from sources and automatically detects enslavement. It's designed for:
- **Scale** (150+ repos) without manual effort
- **AI consumption** (cheat sheets, not human browsing)
- **Product management** (change impact analysis, roadmap planning)

### What about schema migrations?

The schema is versioned. Migrations are backward-compatible. When you add a new field to `Concept`, old concepts still work—they just don't have that field populated until re-ingested.

### Can I query across entities?

No. One database = one entity. To understand relationships between entities, you'd maintain separate idud instances and run cross-instance analysis (which is a good use case for an LLM agent).

### How much does this cost to run?

Depends on your data sources and ingestion frequency:
- **One-time ingestion** of 150 repos: ~$50-200 (mostly embeddings)
- **Weekly updates**: ~$10-50 (incremental ingestion)
- **Database + hosting**: ~$20-100/month depending on scale
- **AI Cheat Sheet queries**: Free (no LLM tokens, just database lookups)

This is dramatically cheaper than continuously re-running LLM agents to understand the same codebase.

### How do I use this for product management?

1. **Ingest** your product: repos, docs, contracts, decision records
2. **View the dependency dashboard**: See which features are coupled
3. **Plan changes**: Query "What breaks if I remove feature X?"
4. **Set timelines**: Understand coupling before committing to delivery dates
5. **Monitor**: Watch coupling metrics as the product evolves

## License

MIT

## Support

- 📖 [Documentation](./docs)
- 💬 [Discussions](https://github.com/yourusername/idud/discussions)
- 🐛 [Issues](https://github.com/yourusername/idud/issues)
