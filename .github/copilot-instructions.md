# idud: Copilot Instructions

## Core Architectural Axioms (STRICTLY ENFORCED)
**idud is a pure, local-first topological graph engine.** It maps codebase dependencies deterministically. 

**BANNED CONCEPTS & SCOPE CREEP:**
Do NOT generate, suggest, or scaffold code for the following:
- Peer-to-peer (P2P) networks, WebRTC, or MQTT brokers.
- Decentralized sync layers or distributed database logic.
- REST APIs, GraphQL endpoints, or web server frameworks.
- External database drivers. The graph lives in-memory using `petgraph` and `DashMap`, and exports to JSON.

The "Pirate Bay" model applies **strictly to the data schema** (storing immutable `source_uri` pointers to code snippets rather than the code itself). It does NOT imply a file-sharing network topology.

## Token Efficiency & Compute Philosophy
- **Upfront Local Compute, Zero-Token Traversal:** idud does heavy AST parsing locally once. Traversal is O(1) and computationally free.
- **No Agentic Slop:** For standard tasks, write deterministic Rust scripts. Only use LLMs for complex conceptual mapping where regex/AST fails.
- **Lean Dependencies:** Keep `Cargo.toml` minimal. Use `DashMap` for concurrency.

## Lexicon Boundary Condition
- **Signatory:** An atomic code unit.
- **Contract:** The directional link joining two signatories.
- **StrictlyBinds / StrictlyBoundBy:** Use these terms to describe high-coupling dependencies. Do NOT use "Enslaves" or "EnslavedBy".
