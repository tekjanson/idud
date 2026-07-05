# Contributing to idud

## Development Philosophy: Small and Lean

idud is designed to be a lightweight, lightning-fast CLI tool. It is an immutable registry, not a bloated web service.

### The "No Network Slop" Rule
This project operates entirely local-first. We do not accept PRs that introduce:
- Distributed syncing or P2P logic.
- Server-client architectures.
- External database requirements.

If a feature requires setting up a server, a message broker, or a database container, it is out of scope. idud builds the graph in memory and exports a static `AIContractBrief` JSON.

### Extraction Pipelines
When adding new data sources, create a deterministic extraction pipeline. Use AST parsers or simple text chunking. 

**When to use LLMs**:
- Only if you cannot deterministically extract the data.
- All LLM interactions must be mockable via `mockall` for local testing. Zero actual HTTP requests during the test suite.
