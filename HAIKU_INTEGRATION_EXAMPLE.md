# Haiku Prediction Engine - Integration Examples

## Quick Start

### 1. Basic Prediction via HTTP

```bash
# Set your Anthropic API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Make prediction request
curl -X POST http://localhost:3000/api/training/predict \
  -H "Content-Type: application/json" \
  -d '{
    "issue_text": "Fix: contract validation is not checking for circular dependencies"
  }'
```

### 2. Response Example

```json
{
  "success": true,
  "predicted_files": [
    "src/contract_ledger.rs",
    "src/schemas.rs",
    "src/types.rs"
  ],
  "model_used": "claude-3-5-haiku-20241022",
  "tokens_used": {
    "input_tokens": 1256,
    "output_tokens": 287
  },
  "reasoning": "Based on the dependency graph..."
}
```

## How It Works

### Step 1: Dependency Graph Context
The engine receives the complete code dependency graph:
```
Files and their dependencies:

File: src/contract_ledger.rs
  - src/types.rs (Requires)
  - src/schemas.rs (Calls)
  - src/web_server.rs (CalledBy)

File: src/schemas.rs
  - src/types.rs (Requires)

File: src/types.rs
  - No dependencies
```

### Step 2: Issue Analysis
The system prompt tells Haiku to analyze the issue:
- "Fix: contract validation is not checking for circular dependencies"
- Identify the problem area: validation logic
- Use the graph to find related files

### Step 3: Impact Radius Calculation
Haiku traces through the dependency graph:
- src/schemas.rs implements validation
- src/contract_ledger.rs uses the validator
- src/types.rs defines Signatory/Contract structures

### Step 4: Prediction
Returns only file paths that would likely need changes.

## Use Cases

### Bug Fixes
**Issue**: "Signatory IDs become corrupted when special characters are used"

**Predicted Files**:
- `src/types.rs` - Signatory ID definition
- `src/contract_ledger.rs` - ID handling in ledger
- `src/validation/signatory.rs` - ID validation logic

### Features
**Issue**: "Add support for marking contracts as 'on hold'"

**Predicted Files**:
- `src/types.rs` - Add contract status enum
- `src/contract_ledger.rs` - Handle status changes
- `src/web_server.rs` - New API endpoints for status update

### Refactoring
**Issue**: "Extract contract discovery into separate module"

**Predicted Files**:
- `src/contract_ledger.rs` - Move discovery logic
- `src/pipelines/discovery.rs` - New discovery module
- `src/lib.rs` - Update module exports

## Advanced: Manual Rust API

```rust
use idud::{predict_files_from_issue, PredictionRequest};
use idud::{ContractLedger, Contract, Signatory};

#[tokio::main]
async fn main() {
    let ledger = ContractLedger::new();
    
    let prediction = predict_files_from_issue(
        PredictionRequest {
            issue_text: "Fix validation of circular dependencies".to_string(),
            dependency_graph: ledger.get_all_contracts(),
            signatories: ledger.get_all_signatories(),
        },
        &std::env::var("ANTHROPIC_API_KEY").unwrap()
    ).await.unwrap();

    println!("Predicted files: {:?}", prediction.predicted_files);
    println!("Tokens used: {} input, {} output",
        prediction.tokens_used.input_tokens,
        prediction.tokens_used.output_tokens
    );
}
```

## Performance Tips

1. **Sparse Graphs**: Performance improves with focused dependency graphs
2. **Clear Issues**: More detailed issue descriptions lead to better predictions
3. **Batch Mode**: Consider batching multiple predictions in a single request
4. **Caching**: The dependency graph doesn't change frequently - cache it

## Limitations

1. **Graph Quality**: Predictions are only as good as the dependency graph
2. **Context Window**: Very large graphs (1000+ files) may exceed token limits
3. **Ambiguity**: Issues that span multiple unrelated areas may be harder to predict
4. **False Positives**: May include files that don't actually need changes
5. **False Negatives**: May miss files that indirectly need changes

## Next Steps

1. Monitor prediction accuracy against actual PR changes
2. Collect feedback to fine-tune the system prompt
3. Consider training a custom model on historical PR data
4. Add confidence scores to each prediction
5. Implement user feedback loop for model improvement

