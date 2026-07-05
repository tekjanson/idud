# Knowledge Base Import Feature

## Overview

The Knowledge Base import functionality allows idud to ingest external documentation and markdown files directly through the UI. This transforms idud into a self-serve tool that can map both repository dependencies AND external documentation as signatories in the contract graph.

## Features

### 1. URL-Based Import
- Import markdown documentation directly from URLs
- Fetches content asynchronously
- Auto-detects markdown structure and headers
- Supports any public markdown file or raw GitHub content URLs

**Example URLs:**
- `https://raw.githubusercontent.com/owner/repo/main/README.md`
- `https://docs.example.com/api.md`
- `https://github.com/owner/repo/blob/main/ARCHITECTURE.md` (GitHub renders raw)

### 2. File Upload Import
- Upload local `.md` or `.txt` files directly
- Files processed client-side, streamed to server
- Preserves file structure and formatting

**Supported formats:**
- `.md` (Markdown)
- `.txt` (Plain text)

## Implementation Details

### Backend (Rust)

#### New API Endpoints

**POST `/api/import-url`**
```json
{
  "url": "https://example.com/docs.md"
}
```

Response:
```json
{
  "success": true,
  "message": "Successfully imported 15 sections from URL",
  "signatories_added": 12,
  "sections_parsed": 15
}
```

**POST `/api/import-file`** (multipart/form-data)
- Field name: `file`
- Accepts: `.md` or `.txt` files

Response:
```json
{
  "success": true,
  "message": "Successfully imported 8 sections from file",
  "signatories_added": 8,
  "sections_parsed": 8
}
```

#### Processing Pipeline

1. **Fetch/Read**: Content is fetched from URL or read from uploaded file
2. **Parse**: Markdown is parsed using `pulldown-cmark` crate
3. **Section Extraction**: Headings (H1-H6) become documentation concepts (signatories)
4. **Link Collection**: Hyperlinks within sections are captured as metadata
5. **Contract Generation**: Hierarchical relationships between sections create `Documents` contracts
   - Parent section documents child section (h2 documents under h1, etc.)
   - High confidence (0.95) - deterministic structure

#### Markdown Section Signatory

Each parsed section becomes a `MarkdownSection` signatory with:

```rust
{
  "id": "uuid",
  "signatory_type": "MarkdownSection",
  "source_uri": "doc://markdown#{index}",
  "label": "Section Title",
  "snippet": "Section content...",
  "registered_at": "ISO8601 timestamp",
  "metadata": {
    "level": 2,
    "links": ["https://...", "https://..."]
  }
}
```

**Metadata fields:**
- `level`: Heading level (1-6)
- `links`: Hyperlinks found in section

#### Contract Generation

Sections at different levels create `Documents` contracts:
- **Clause Type**: `Documents`
- **Confidence**: 0.95 (deterministic)
- **Source**: Deterministic
- **Reasoning**: "Section at level X documents level Y"

**Example:**
```
# API Guide (H1)      ← Section 1
## Getting Started    ← Section 2 (documents Section 1)
### Authentication   ← Section 3 (documents Section 2)
```

Contracts:
- Section 2 → Section 1: Documents
- Section 3 → Section 2: Documents

### Frontend (JavaScript)

#### UI Components

1. **Sidebar Section**: "📚 Knowledge Base"
   - Button: "📄 Import Docs"
   - Status display for import feedback

2. **Modal: Knowledge Base Import**
   - Two tabs: "🌐 From URL" and "📁 From File"
   - Tab switching with visual feedback
   - Form validation
   - Progress tracking
   - Real-time status updates

#### Form Features

**URL Import Form:**
- Markdown URL input with validation
- Placeholder: `https://example.com/docs/readme.md`
- URL format validation
- Error handling with user feedback

**File Import Form:**
- File picker (accepts `.md` and `.txt`)
- File size indication
- Drag-and-drop ready (CSS ready)
- Format validation

#### Status Display

- **Success**: ✅ Green banner with confirmation
- **Error**: ❌ Red banner with error message
- **Info**: ℹ️ Blue banner with progress info
- Progress bar with shimmer animation

#### User Flow

1. Click "📄 Import Docs" button
2. Choose import method (URL or File)
3. Enter/select content
4. Click "Import from URL" or "Import File"
5. View progress bar
6. See success/error status
7. Modal auto-closes on success
8. Graph auto-refreshes with new signatories

## Usage Examples

### Example 1: Import GitHub README

```bash
# In UI:
1. Open Knowledge Base
2. Enter URL: https://raw.githubusercontent.com/kubernetes/kubernetes/master/README.md
3. Click "Import from URL"
4. Wait for import to complete
5. View new documentation sections in graph
```

### Example 2: Upload Local Architecture Doc

```bash
# In UI:
1. Open Knowledge Base
2. Switch to "From File" tab
3. Select local ARCHITECTURE.md
4. Click "Import File"
5. View sections appear in graph
```

## Data Schema

### MarkdownSection Signatory

```rust
pub struct Signatory {
    pub id: String,
    pub signatory_type: SignatoryType::MarkdownSection,
    pub source_uri: String,           // "doc://markdown#{index}"
    pub label: String,                 // Heading text
    pub snippet: String,               // Section content
    pub registered_at: DateTime<Utc>,
    pub metadata: {
        "level": usize,               // H1-H6
        "links": Vec<String>          // URLs in section
    }
}
```

### Documents Contract

```rust
pub struct Contract {
    pub principal_id: String,         // Child section
    pub guarantor_id: String,         // Parent section
    pub clause_type: ClauseType::Documents,
    pub confidence: f32,               // 0.95
    pub discovered_by: Deterministic,
    pub clause_reasoning: String,     // "Section at level X documents level Y"
}
```

## Integration with Contract Graph

The imported documentation becomes first-class citizens in the contract graph:

- **Nodes**: Each section is a node colored `#fd79a8` (pink)
- **Edges**: Documentation contracts link sections hierarchically
- **Search**: Signatories searchable by section title
- **Stats**: Section count included in network stats
- **Traversal**: Can trace documentation chains using `/api/chain/{id}`

## Benefits

1. **Self-Serve Documentation Ingestion**: No CLI needed
2. **Documentation as Code**: Docs are now signatories with contracts
3. **Cross-Reference Discovery**: Links between docs become visible in graph
4. **Hierarchical Analysis**: Section nesting reveals documentation structure
5. **Single Source of Truth**: Combine code + docs in one contract graph

## Dependencies

- **actix-multipart**: Form data handling for file uploads
- **pulldown-cmark**: Markdown parsing
- **futures-util**: Async stream handling
- **reqwest**: HTTP client for URL fetching

## Error Handling

### Common Errors

1. **Invalid URL**: Returns 400 with message
2. **Failed to fetch**: Network timeout or 404 - returns 400
3. **Invalid file format**: Wrong extension - caught client-side
4. **Empty content**: Returns 400 "No file content provided"
5. **Malformed markdown**: Gracefully handles - returns partial sections

### User Feedback

All errors displayed in modal with:
- ❌ Red warning banner
- Clear error message
- Suggestion for resolution (when applicable)

## Future Enhancements

- [ ] Incremental parsing progress with streaming updates
- [ ] Support for `.docx`, `.pdf` document formats
- [ ] Auto-link detection between documentation and code signatories
- [ ] Documentation indexing for full-text search
- [ ] Import history/audit log
- [ ] Batch import from documentation sources
- [ ] Preview mode before confirming import
- [ ] Documentation versioning/snapshots

## API Examples

### Using curl

```bash
# Import from URL
curl -X POST http://localhost:3000/api/import-url \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/docs.md"}'

# Import from file
curl -X POST http://localhost:3000/api/import-file \
  -F "file=@docs.md"
```

### Using JavaScript

```javascript
// URL import
const response = await fetch('/api/import-url', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ url: 'https://example.com/docs.md' })
});

// File import
const formData = new FormData();
formData.append('file', fileInputElement.files[0]);
const response = await fetch('/api/import-file', {
  method: 'POST',
  body: formData
});
```

## Testing

```bash
# Build project
cargo build

# Run server
cargo run -- serve --port 3000

# Navigate to http://localhost:3000
# Test Knowledge Base import via UI
```

## Files Modified

- `src/web_server.rs`: New endpoints and markdown parsing logic
- `ui/dist/index.html`: Knowledge Base UI components and JavaScript handlers
- `Cargo.toml`: Added dependencies (actix-multipart, pulldown-cmark, futures-util)

## Architecture Alignment

This feature maintains idud's core principles:

- **Pure Local-First**: All processing happens locally, no external services
- **Zero-Token Traversal**: Markdown sections stored as in-memory signatories
- **Deterministic Contracts**: Section hierarchy creates high-confidence bindings
- **Pirate Bay Model**: Stores `source_uri` pointers, not content copies
