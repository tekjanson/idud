# idud Link Tree Visualization Guide

The idud project now includes a **full-featured interactive visualization** for exploring code dependency graphs.

## What You Can See

### 🎨 Visual Elements

The visualization displays:

- **Nodes** (circles): Each node represents a signatory (code unit) such as:
  - Functions
  - Classes
  - Files
  - Tests
  - Workflows
  - API Endpoints
  - Documentation sections
  - Decision records

- **Edges** (lines): Each edge represents a contract (relationship) between two signatories
  - Different relationships like "calls", "requires", "uses", "audits", etc.

- **Colors**: Nodes are color-coded by type:
  - Red: Files
  - Teal: Functions
  - Blue: Classes
  - Green: Tests
  - Yellow: Workflows
  - Purple: API Endpoints
  - Pink: Documentation
  - And more...

### 📊 Sidebar Statistics

- **Signatory Count**: Total number of code units discovered
- **Contract Count**: Total number of dependency relationships
- **Search Box**: Filter signatories by name
- **Signatory List**: Browse all discovered code units with type indicators

### 🔍 Interaction Features

- **Zoom**: Scroll to zoom in/out on the graph
- **Pan**: Click and drag the background to pan around
- **Drag Nodes**: Click and drag individual nodes to reposition them
- **Highlight Selection**: Click a signatory in the list to highlight it in the graph
- **Hover Information**: Hover over nodes and edges for details
- **Legend**: Bottom-right corner shows node types and contract types

## How to Start the Visualization

### Quick Start

```bash
# Build and start the visualization server
npm run ui

# Or directly with cargo
cargo run --release -- serve
```

The server will start at **http://127.0.0.1:3000** (or whatever port you specify).

### Custom Port and Host

```bash
# Run on a different port
cargo run --release -- serve --port 8080 --host 0.0.0.0

# Or using npm
npm run ui:dev
```

### CLI Command Reference

```bash
# Start visualization server (default: http://127.0.0.1:3000)
cargo run --release -- serve

# Specify custom port
cargo run --release -- serve --port 8080 --host 127.0.0.1

# Ingest a repository first to populate the graph
cargo run --release -- ingest-repo --url https://github.com/user/repo --branch main
```

## Architecture

### Frontend
- **Framework**: D3.js v7 (force-directed graph layout)
- **Styling**: Modern CSS with dark theme
- **Features**: Real-time updates, search, filtering, zoom/pan

### Backend
- **Framework**: Actix-web (Rust HTTP server)
- **API Endpoints**:
  - `GET /api/graph` - Get full graph (nodes and edges)
  - `GET /api/signatories` - Get all signatories
  - `GET /api/contracts` - Get all contracts
  - `GET /api/chain/{id}` - Get chain of obligations for a signatory
  - `GET /` - Serves the HTML visualization UI

### Data Flow

```
idud CLI (Rust) 
    ↓
ContractLedger (in-memory graph)
    ↓
Actix-web server
    ↓
JSON API endpoints
    ↓
D3.js visualization (client-side)
```

## Tips & Tricks

### For Large Graphs
- Use the search box to find specific signatories
- Zoom in on regions of interest
- Click a node to highlight it and see its connections
- Use "force directed" layout to automatically organize nodes

### Understanding Dependencies
- **Thick edges** = stronger connections
- **Node proximity** = related code units (calculated by D3 force simulation)
- **Color coding** = quickly identify types of code units
- **Chains** = trace dependency paths using the CLI with `trace` command

### Performance Notes
- Graphs with 100+ nodes may load slower initially
- D3.js simulation adapts responsively
- Browser dev tools (F12) show network requests to API
- Refresh data every 5 seconds (configurable in JavaScript)

## File Structure

```
idud/
├── ui/
│   └── dist/
│       └── index.html          # Interactive visualization UI
├── src/
│   ├── web_server.rs           # Actix-web HTTP server
│   ├── ui/
│   │   └── translator.rs       # Leptos UI components (for future SSR)
│   └── lib.rs                  # Exports web_server
├── Cargo.toml                  # Added actix-web dependencies
├── package.json                # npm scripts for UI
└── README.md                   # Updated with visualization docs
```

## Next Steps

To populate the visualization with real data:

```bash
# 1. Build the project
npm run build

# 2. Ingest a repository
npm run ingest:repo -- --url https://github.com/tokio-rs/tokio --branch master

# 3. Start the visualization
npm run ui

# 4. Open http://127.0.0.1:3000 in your browser
```

## Troubleshooting

### Server won't start
```bash
# Check if port is in use
lsof -i :3000

# Kill existing process if needed
pkill -f "idud serve"

# Try a different port
cargo run --release -- serve --port 8080
```

### Visualization is empty
- No contracts have been ingested yet
- Visit the repository ingestion docs: `CONTRIBUTING.md`
- Run `cargo run --release -- ingest-repo --url <github-url>`

### API returns empty data
- The ledger hasn't been populated
- Make sure to ingest a repository first
- Check server logs for errors

### Performance issues
- Large graphs (1000+ nodes) may be slow
- Try zooming into specific areas
- Use the search filter to focus on subsets

## Architecture Diagram

```
┌─────────────────────────────────┐
│    Browser / D3.js Viz          │
│  (ui/dist/index.html)           │
└────────────┬──────────────────┘
             │
             │ HTTP
             │
┌────────────▼──────────────────┐
│  Actix-web Server             │
│  (src/web_server.rs)          │
│  • /api/graph                 │
│  • /api/signatories           │
│  • /api/contracts             │
│  • /api/chain/{id}            │
└────────────┬──────────────────┘
             │
             │ In-memory
             │
┌────────────▼──────────────────┐
│  ContractLedger               │
│  (src/contract_ledger.rs)     │
│  • Signatories (nodes)        │
│  • Contracts (edges)          │
│  • Topological Graph          │
└───────────────────────────────┘
```
