//! AI prediction engine using Claude Haiku
//! Predicts which files need to change based on issue description and dependency graph

use crate::types::{Contract, Signatory, SignatoryType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRequest {
    pub issue_text: String,
    pub dependency_graph: Vec<Contract>,
    pub signatories: Vec<Signatory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResponse {
    pub predicted_files: Vec<String>,
    pub model_used: String,
    pub tokens_used: TokenUsage,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessage {
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: i32,
    output_tokens: i32,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: i32,
    system: String,
    messages: Vec<AnthropicMessage2>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage2 {
    role: String,
    content: String,
}

/// Predicts which files need to change based on issue and dependency graph
/// Uses Claude Haiku to analyze the issue in context of the code dependency graph
pub async fn predict_files_from_issue(
    request: PredictionRequest,
    api_key: &str,
) -> Result<PredictionResponse, Box<dyn std::error::Error>> {
    let graph_text = format_graph_for_context(&request.signatories, &request.dependency_graph);
    let prompt = build_system_prompt();

    let client = reqwest::Client::new();
    let anthropic_request = AnthropicRequest {
        model: "claude-3-5-haiku-20241022".to_string(),
        max_tokens: 1024,
        system: prompt,
        messages: vec![AnthropicMessage2 {
            role: "user".to_string(),
            content: format!(
                "ISSUE DESCRIPTION:\n{}\n\n---\n\nDEPENDENCY GRAPH:\n{}\n\nAnalyze the issue and graph to predict affected files.",
                request.issue_text, graph_text
            ),
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&anthropic_request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Anthropic API error: {}", error_text).into());
    }

    let message: AnthropicMessage = response.json().await?;

    // Extract JSON from response
    let text = message
        .content
        .first()
        .and_then(|c| c.text.as_ref())
        .ok_or("No text content in response")?;

    let predicted_files = extract_file_list_from_response(text)?;

    Ok(PredictionResponse {
        predicted_files,
        model_used: "claude-3-5-haiku-20241022".to_string(),
        tokens_used: TokenUsage {
            input_tokens: message.usage.input_tokens,
            output_tokens: message.usage.output_tokens,
        },
        reasoning: Some(text.clone()),
    })
}

/// Format the dependency graph into human-readable text
fn format_graph_for_context(signatories: &[Signatory], contracts: &[Contract]) -> String {
    let mut graph_text = String::new();
    graph_text.push_str("Files and their dependencies:\n\n");

    // Build file-to-dependencies mapping
    let mut file_deps: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for sig in signatories {
        if sig.signatory_type == SignatoryType::File {
            file_deps.insert(sig.label.clone(), Vec::new());
        }
    }

    for contract in contracts {
        let principal = signatories
            .iter()
            .find(|s| s.id == contract.principal_id)
            .map(|s| s.label.clone());

        let guarantor = signatories
            .iter()
            .find(|s| s.id == contract.guarantor_id)
            .map(|s| s.label.clone());

        if let (Some(prin), Some(guar)) = (principal, guarantor) {
            if let Some(deps) = file_deps.get_mut(&prin) {
                deps.push((guar.clone(), format!("{:?}", contract.clause_type)));
            }
        }
    }

    // Print file dependencies
    for (file, deps) in &file_deps {
        graph_text.push_str(&format!("File: {}\n", file));
        if deps.is_empty() {
            graph_text.push_str("  - No dependencies\n");
        } else {
            for (dep_file, relation) in deps {
                graph_text.push_str(&format!("  - {} ({})\n", dep_file, relation));
            }
        }
        graph_text.push('\n');
    }

    graph_text
}

/// Build the system prompt for Haiku
fn build_system_prompt() -> String {
    r#"You are an expert code dependency analyzer trained to predict file changes based on issue descriptions and code dependency graphs.

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

Remember: Use dependency relationships to understand which files would be impacted by changes."#.to_string()
}

/// Extract file list from Haiku's response
fn extract_file_list_from_response(text: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Try to find JSON array in the response
    let start = text.find('[').ok_or("No JSON array found in response")?;
    let end = text.rfind(']').ok_or("JSON array not properly closed")?;

    let json_str = &text[start..=end];
    let files: Vec<String> = serde_json::from_str(json_str)?;

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_graph_for_context() {
        let signatories = vec![
            Signatory::new(
                SignatoryType::File,
                "src/main.rs".to_string(),
                "src/main.rs".to_string(),
                "fn main()".to_string(),
            ),
            Signatory::new(
                SignatoryType::File,
                "src/lib.rs".to_string(),
                "src/lib.rs".to_string(),
                "pub mod utils".to_string(),
            ),
        ];

        let graph_text = format_graph_for_context(&signatories, &[]);
        assert!(graph_text.contains("File: src/main.rs"));
        assert!(graph_text.contains("File: src/lib.rs"));
    }

    #[test]
    fn test_extract_file_list() {
        let response = r#"Based on the dependency graph and issue, here are the affected files:
```json
["src/main.rs", "src/lib.rs", "src/utils/helpers.rs"]
```
This analysis shows these three files need changes."#;

        let files = extract_file_list_from_response(response).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0], "src/main.rs");
        assert_eq!(files[1], "src/lib.rs");
        assert_eq!(files[2], "src/utils/helpers.rs");
    }

    #[test]
    fn test_build_system_prompt() {
        let prompt = build_system_prompt();
        assert!(prompt.contains("dependency graph"));
        assert!(prompt.contains("JSON array"));
        assert!(prompt.contains("CRITICAL RULES"));
    }
}
