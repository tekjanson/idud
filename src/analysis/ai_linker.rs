//! AI-augmented dependency linking for idud
//! 
//! This module uses Copilot CLI to infer semantic dependencies that AST analysis misses.
//! It processes files in batches and uses Copilot to identify implicit relationships
//! like duck typing patterns, protocols, and shared concepts.
//!
//! TOKEN OPTIMIZATION:
//! - Batch process 5-10 files per Copilot call
//! - Compact format: "file.rs: [func1, func2]"
//! - Minimal system prompt (~5 tokens)
//! - Each batch: ~100 tokens for file list + ~300 tokens for inference = ~400 tokens total
//! - Budget: 500-5000 tokens for 100 files (~5-50 tokens per file)

use crate::types::{Contract, ClauseType, ContractSource, Signatory, SignatoryType};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info, warn};

/// Configuration for AI linking behavior
#[derive(Debug, Clone)]
pub struct AILinkerConfig {
    /// Batch size: how many files to process per Copilot call (5-10 recommended)
    pub batch_size: usize,
    /// Minimum confidence threshold for semantic dependencies (0.40-0.75)
    pub min_confidence: f32,
    /// Maximum confidence for AI-inferred dependencies (below AST confidence)
    pub max_confidence: f32,
    /// Enable verbose logging of prompts and responses
    pub verbose: bool,
}

impl Default for AILinkerConfig {
    fn default() -> Self {
        Self {
            batch_size: 8,
            min_confidence: 0.40,
            max_confidence: 0.65,
            verbose: false,
        }
    }
}

/// AI Linker: uses Copilot CLI to infer semantic dependencies
pub struct AILinker {
    config: AILinkerConfig,
    tokens_used: u64,
}

impl AILinker {
    pub fn new(config: AILinkerConfig) -> Self {
        Self {
            config,
            tokens_used: 0,
        }
    }

    /// Link files in a codebase using AI analysis
    /// 
    /// Takes signatories and existing contracts, groups files in batches,
    /// and uses Copilot to infer semantic dependencies.
    /// 
    /// Returns new Contract objects with confidence scores.
    pub fn link_files(
        &mut self,
        signatories: &[Signatory],
        existing_contracts: &[Contract],
    ) -> Result<Vec<Contract>> {
        // Validate Copilot CLI is available
        self.validate_copilot_cli()?;

        // Filter file signatories for batching
        let file_signatories: Vec<&Signatory> = signatories
            .iter()
            .filter(|s| s.signatory_type == SignatoryType::File)
            .collect();

        if file_signatories.is_empty() {
            info!("No files to analyze for semantic dependencies");
            return Ok(Vec::new());
        }

        info!(
            "Starting AI linking for {} files in batches of {}",
            file_signatories.len(),
            self.config.batch_size
        );

        let mut inferred_contracts = Vec::new();

        // Process files in batches
        for batch in file_signatories.chunks(self.config.batch_size) {
            debug!("Processing batch of {} files", batch.len());

            match self.link_batch(batch, signatories, existing_contracts) {
                Ok(contracts) => {
                    info!("Batch produced {} inferred dependencies", contracts.len());
                    inferred_contracts.extend(contracts);
                }
                Err(e) => {
                    warn!("Error processing batch: {}", e);
                    // Continue with next batch on error
                }
            }
        }

        info!(
            "AI linking complete: {} semantic dependencies inferred",
            inferred_contracts.len()
        );

        Ok(inferred_contracts)
    }

    /// Link a batch of files together
    fn link_batch(
        &mut self,
        batch: &[&Signatory],
        all_signatories: &[Signatory],
        existing_contracts: &[Contract],
    ) -> Result<Vec<Contract>> {
        // Format batch for Copilot
        let batch_text = format_batch_for_analysis(batch);
        let prompt = build_linking_prompt(&batch_text);

        if self.config.verbose {
            debug!("Linking prompt:\n{}", prompt);
        }

        // Call Copilot CLI
        let response = invoke_copilot_cli(&prompt)?;

        if self.config.verbose {
            debug!("Copilot response:\n{}", response);
        }

        // Parse response to extract inferred pairs
        let inferred_pairs = parse_linking_response(&response, batch)?;

        // Convert pairs to contracts, avoiding duplicates
        let mut contracts = Vec::new();
        let existing_set = build_contract_set(existing_contracts);

        for (from_label, to_label, reasoning) in inferred_pairs {
            // Find signatories by label
            let from_sig = all_signatories
                .iter()
                .find(|s| s.label == from_label);
            let to_sig = all_signatories
                .iter()
                .find(|s| s.label == to_label);

            if let (Some(from), Some(to)) = (from_sig, to_sig) {
                // Skip if this contract already exists
                let contract_key = (from.id.clone(), to.id.clone());
                if existing_set.contains(&contract_key) {
                    continue;
                }

                // Create contract with moderate confidence
                let confidence = self.config.min_confidence
                    + (self.config.max_confidence - self.config.min_confidence) / 2.0;

                let contract = Contract::new(
                    from.id.clone(),
                    to.id.clone(),
                    ClauseType::Uses, // Generic "uses" for semantic dependencies
                    confidence,
                    ContractSource::AiInferred,
                )
                .with_reasoning(reasoning);

                contracts.push(contract);
            }
        }

        Ok(contracts)
    }

    /// Validate that Copilot CLI is available
    fn validate_copilot_cli(&self) -> Result<()> {
        let output = Command::new("which")
            .arg("copilot")
            .output()
            .map_err(|e| anyhow!("Failed to check for copilot CLI: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Copilot CLI not found. Install from: https://github.com/github/gh-copilot"
            ));
        }

        Ok(())
    }

    /// Get total tokens used in this linking pass
    pub fn tokens_used(&self) -> u64 {
        self.tokens_used
    }
}

/// Format a batch of files for semantic analysis
/// 
/// Example output:
///   src/main.rs: [main, init, run]
///   src/lib.rs: [register_handler, process]
///   src/utils.rs: [format, parse]
fn format_batch_for_analysis(batch: &[&Signatory]) -> String {
    let mut text = String::new();

    for sig in batch {
        // Extract file path from label (typically just the path)
        let file_path = &sig.label;
        text.push_str(&format!("{}\n", file_path));
    }

    text
}

/// Build a focused linking prompt for Copilot
fn build_linking_prompt(batch_text: &str) -> String {
    format!(
        r#"You are analyzing source files for implicit semantic dependencies.

FILES TO ANALYZE:
{}

TASK: Identify pairs of files that likely interact or depend on each other through duck typing, shared protocols, or implicit patterns.

RESPOND with ONLY a JSON array like this:
[
  {{"from": "file1", "to": "file2", "reason": "Both implement Stream protocol"}},
  {{"from": "file2", "to": "file3", "reason": "file2 creates instances of file3 types"}}
]

NO EXPLANATION, NO MARKDOWN. ONLY JSON."#,
        batch_text
    )
}

/// Invoke Copilot CLI with a prompt
fn invoke_copilot_cli(prompt: &str) -> Result<String> {
    let output = Command::new("copilot")
        .arg("-p")
        .arg(prompt)
        .output()
        .map_err(|e| anyhow!("Failed to invoke copilot: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Copilot CLI error: {}", stderr));
    }

    let response = String::from_utf8(output.stdout)?;
    Ok(response)
}

/// Response structure from Copilot
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkingPair {
    from: String,
    to: String,
    reason: String,
}

/// Parse Copilot response to extract inferred dependency pairs
/// 
/// Returns a vec of (from_file, to_file, reasoning) tuples
fn parse_linking_response(
    response: &str,
    batch: &[&Signatory],
) -> Result<Vec<(String, String, String)>> {
    // Extract JSON from response
    let json_start = response
        .find('[')
        .ok_or_else(|| anyhow!("No JSON array found in response"))?;
    let json_end = response
        .rfind(']')
        .ok_or_else(|| anyhow!("Malformed JSON array in response"))?;

    let json_str = &response[json_start..=json_end];

    // Parse JSON
    let pairs: Vec<LinkingPair> = serde_json::from_str(json_str)?;

    // Filter pairs to only those referencing files in the batch
    let batch_files: std::collections::HashSet<&str> =
        batch.iter().map(|s| s.label.as_str()).collect();

    let mut result = Vec::new();
    for pair in pairs {
        if batch_files.contains(pair.from.as_str()) && batch_files.contains(pair.to.as_str()) {
            result.push((pair.from, pair.to, pair.reason));
        }
    }

    Ok(result)
}

/// Build a set of existing contract pairs for deduplication
fn build_contract_set(contracts: &[Contract]) -> std::collections::HashSet<(String, String)> {
    contracts
        .iter()
        .map(|c| (c.principal_id.clone(), c.guarantor_id.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_batch_for_analysis() {
        let signatories = vec![
            Signatory::new(
                SignatoryType::File,
                "http://example.com/repo/blob/main/src/main.rs".to_string(),
                "src/main.rs".to_string(),
                "File content".to_string(),
            ),
            Signatory::new(
                SignatoryType::File,
                "http://example.com/repo/blob/main/src/lib.rs".to_string(),
                "src/lib.rs".to_string(),
                "File content".to_string(),
            ),
        ];

        let batch: Vec<_> = signatories.iter().collect();
        let formatted = format_batch_for_analysis(&batch);

        assert!(formatted.contains("src/main.rs"));
        assert!(formatted.contains("src/lib.rs"));
    }

    #[test]
    fn test_build_linking_prompt() {
        let batch_text = "src/main.rs\nsrc/lib.rs\n";
        let prompt = build_linking_prompt(batch_text);

        assert!(prompt.contains("FILES TO ANALYZE"));
        assert!(prompt.contains("TASK"));
        assert!(prompt.contains("JSON"));
        assert!(prompt.contains("["));
    }

    #[test]
    fn test_parse_linking_response_valid() {
        let response = r#"Here are the inferred dependencies:
[
  {"from": "src/main.rs", "to": "src/lib.rs", "reason": "imports Stream"},
  {"from": "src/lib.rs", "to": "src/utils.rs", "reason": "duck typing"}
]
Some explanation text"#;

        let sig1 = Signatory::new(
            SignatoryType::File,
            "http://example.com/repo/blob/main/src/main.rs".to_string(),
            "src/main.rs".to_string(),
            "content".to_string(),
        );
        let sig2 = Signatory::new(
            SignatoryType::File,
            "http://example.com/repo/blob/main/src/lib.rs".to_string(),
            "src/lib.rs".to_string(),
            "content".to_string(),
        );
        let sig3 = Signatory::new(
            SignatoryType::File,
            "http://example.com/repo/blob/main/src/utils.rs".to_string(),
            "src/utils.rs".to_string(),
            "content".to_string(),
        );

        let batch = vec![&sig1, &sig2, &sig3];

        let result = parse_linking_response(response, &batch).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "src/main.rs");
        assert_eq!(result[0].1, "src/lib.rs");
        assert_eq!(result[0].2, "imports Stream");
    }

    #[test]
    fn test_parse_linking_response_filters_out_of_batch() {
        let response = r#"[
  {"from": "src/main.rs", "to": "src/lib.rs", "reason": "interacts"},
  {"from": "src/other.rs", "to": "src/main.rs", "reason": "not in batch"}
]"#;

        let sig1 = Signatory::new(
            SignatoryType::File,
            "http://example.com/repo/blob/main/src/main.rs".to_string(),
            "src/main.rs".to_string(),
            "content".to_string(),
        );
        let sig2 = Signatory::new(
            SignatoryType::File,
            "http://example.com/repo/blob/main/src/lib.rs".to_string(),
            "src/lib.rs".to_string(),
            "content".to_string(),
        );

        let batch = vec![&sig1, &sig2];

        let result = parse_linking_response(response, &batch).unwrap();

        // Only the first pair is in batch
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "src/main.rs");
    }

    #[test]
    fn test_build_contract_set() {
        let contracts = vec![
            Contract::new(
                "sig1".to_string(),
                "sig2".to_string(),
                ClauseType::Uses,
                0.5,
                ContractSource::Deterministic,
            ),
            Contract::new(
                "sig2".to_string(),
                "sig3".to_string(),
                ClauseType::Requires,
                0.6,
                ContractSource::AiInferred,
            ),
        ];

        let set = build_contract_set(&contracts);

        assert_eq!(set.len(), 2);
        assert!(set.contains(&("sig1".to_string(), "sig2".to_string())));
        assert!(set.contains(&("sig2".to_string(), "sig3".to_string())));
    }

    #[test]
    fn test_ai_linker_config_defaults() {
        let config = AILinkerConfig::default();

        assert_eq!(config.batch_size, 8);
        assert_eq!(config.min_confidence, 0.40);
        assert_eq!(config.max_confidence, 0.65);
        assert!(!config.verbose);
    }
}
