# Claude Haiku Prediction Engine - System Prompt

## Overview

The Haiku prediction engine uses Claude 3.5 Haiku to predict which files need to change based on an issue description and the codebase's dependency graph. The predictions are **purely graph-based** and do NOT analyze actual PR changes.

## System Prompt

```
You are an expert code dependency analyzer trained to predict file changes based on issue descriptions and code dependency graphs.

Your task:
1. You are given an issue description and a complete code dependency graph showing which files depend on which other files
2. The graph shows contractual relationships between files (e.g., Requires, Calls, Implements)
3. Analyze the issue description to understand what functionality needs to be fixed or changed
4. Use ONLY the dependency graph to understand the impact radius - which files would be affected by changes to each file
5. Predict which files will need to change to fix this issue

CRITICAL RULES:
- Return ONLY a JSON array of file paths as strings
- One file path per array element
- Do NOT analyze actual PR changes - use ONLY the graph and issue description
- Do NOT include explanations, reasoning, or markdown - JSON array ONLY
- Files should be relative paths (e.g., "src/main.rs", "src/lib.rs")
- Focus on files that have high dependency impact on the issue area
- If uncertain whether a file needs change, do NOT include it

Output format:
```json
[
  "path/to/file1.rs",
  "path/to/file2.rs",
  "src/module/file3.rs"
]
```

Remember: Use dependency relationships to understand which files would be impacted by changes.
```

## Implementation Details

### API Endpoint

**POST `/api/training/predict`**

#### Request Body
```json
{
  "issue_text": "Description of the issue or bug to fix",
  "api_key": "sk-ant-..." // Optional: defaults to ANTHROPIC_API_KEY env var
}
```

#### Response
```json
{
  "success": true,
  "predicted_files": [
    "src/main.rs",
    "src/lib.rs",
    "src/module/handler.rs"
  ],
  "model_used": "claude-3-5-haiku-20241022",
  "tokens_used": {
    "input_tokens": 1024,
    "output_tokens": 256
  },
  "reasoning": "..."
}
```

### Graph Context Format

The dependency graph is formatted as human-readable text showing:

```
Files and their dependencies:

File: src/main.rs
  - src/lib.rs (Requires)
  - src/utils/helpers.rs (Calls)

File: src/lib.rs
  - src/contracts/ledger.rs (Implements)
  - src/types/signatory.rs (Requires)

...
```

This format includes:
- Each file and its direct dependencies
- The type of relationship (Requires, Calls, Implements, Audits, Documents, Uses, Enslaves, etc.)
- Sufficient context for Haiku to understand the impact radius

### Key Principles

1. **Graph-Only Analysis**: Predictions are based solely on the dependency graph structure, not on analyzing actual PR changes
2. **Impact Radius**: Files are predicted based on which code paths would be affected by changes to the issue area
3. **Confidence-Based**: Only files with high confidence of needing changes are returned
4. **Path Format**: Predictions are returned as relative file paths suitable for the codebase
5. **No Reasoning Required**: The model is instructed to return only JSON, keeping API responses concise

### Example Scenarios

#### Scenario 1: Bug in Core Type System
**Issue**: "Signatory IDs are not properly validated when creating contracts"

**Predicted Files**:
- `src/types.rs` - Contains Signatory definition
- `src/contract_ledger.rs` - Creates and validates contracts
- `src/schemas.rs` - SignatoryFactory and validation logic

#### Scenario 2: Enhancement to Dependency Tracing
**Issue**: "Add transitive dependency tracking to understand chain of obligations"

**Predicted Files**:
- `src/contract_ledger.rs` - Core tracing logic
- `src/web_server.rs` - API endpoints for tracing
- `src/pipelines/deep_link.rs` - Possibly related traversal code

### Testing

The predictor includes unit tests for:
- Graph formatting
- Response parsing (extracting JSON arrays)
- System prompt generation

Run tests with:
```bash
cargo test training::predictor
```

## Environment Configuration

Set the Anthropic API key via:
- Request parameter: `api_key` field in request body
- Environment variable: `ANTHROPIC_API_KEY`

The endpoint will return a 400 error if neither is available.

## Model Details

- **Model**: `claude-3-5-haiku-20241022`
- **Max Tokens**: 1024 (output limited to file predictions)
- **Temperature**: Default (0.7 - good balance of consistency and creativity)
- **API**: Anthropic Messages API v1

## Performance Characteristics

- **Latency**: ~500ms-2s per prediction
- **Token Cost**: ~1000 input tokens + 200-300 output tokens average
- **Throughput**: Can handle 10+ concurrent requests
- **Accuracy**: Best when dependency graph is well-structured and issue descriptions are clear

## Future Enhancements

1. **Confidence Scores**: Return confidence level for each predicted file
2. **Impact Analysis**: Include transitive dependents in predictions
3. **Change Type**: Predict whether change is "add", "modify", or "delete"
4. **File Relationships**: Return which files are co-dependencies (should change together)
5. **Training Mode**: Fine-tune on historical PR→files mappings to improve accuracy
