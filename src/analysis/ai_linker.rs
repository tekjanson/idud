//! AI-augmented dependency linking for idud
//! 
//! This module uses Copilot CLI to infer semantic dependencies that AST analysis misses.
//! It processes signatories in batches with per-batch timeouts for reliability.
//!
//! TOKEN OPTIMIZATION:
//! - Batch process 10-20 signatories per Copilot call
//! - Compact format: "file.rs"
//! - Per-batch timeout: 30 seconds (fail fast on Copilot delays)
//! - Graceful degradation: continue with next batch if one fails
//! - Token tracking per batch for cost monitoring
//! - Each batch: ~100 tokens for file list + ~300 tokens for inference = ~400 tokens
//! - Estimated budget for 926 files: 1,800-2,400 tokens

use crate::types::{Contract, ClauseType, ContractSource, Signatory, SignatoryType};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Configuration for AI linking behavior
#[derive(Debug, Clone)]
pub struct AILinkerConfig {
    /// Batch size: how many signatories per Copilot call (10-20 recommended)
    pub batch_size: usize,
    /// Timeout per batch in seconds (30s gives Copilot time to respond)
    pub batch_timeout_secs: u64,
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
            batch_size: 15,
            batch_timeout_secs: 30,
            min_confidence: 0.40,
            max_confidence: 0.65,
            verbose: false,
        }
    }
}

/// Metrics for AI linking performance tracking
#[derive(Debug, Clone)]
pub struct AILinkerMetrics {
    pub batches_processed: usize,
    pub batches_succeeded: usize,
    pub batches_failed: usize,
    pub batches_timed_out: usize,
    pub contracts_discovered: usize,
    pub tokens_estimated: u64,
    pub total_time_ms: u128,
}

/// AI Linker: uses Copilot CLI to infer semantic dependencies
pub struct AILinker {
    config: AILinkerConfig,
    tokens_used: u64,
    metrics: AILinkerMetrics,
}

impl AILinker {
    pub fn new(config: AILinkerConfig) -> Self {
        Self {
            config,
            tokens_used: 0,
            metrics: AILinkerMetrics {
                batches_processed: 0,
                batches_succeeded: 0,
                batches_failed: 0,
                batches_timed_out: 0,
                contracts_discovered: 0,
                tokens_estimated: 0,
                total_time_ms: 0,
            },
        }
    }

    /// Link signatories in a codebase using AI analysis with per-batch timeouts
    /// 
    /// Takes signatories and existing contracts, groups in batches,
    /// and uses Copilot to infer semantic dependencies.
    /// 
    /// Returns new Contract objects with confidence scores.
    pub fn link_files(
        &mut self,
        signatories: &[Signatory],
        existing_contracts: &[Contract],
    ) -> Result<Vec<Contract>> {
        let overall_start = Instant::now();
        
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
            "Starting AI linking for {} files in batches of {} with {}s timeout per batch",
            file_signatories.len(),
            self.config.batch_size,
            self.config.batch_timeout_secs
        );

        let mut inferred_contracts = Vec::new();

        // Process files in batches with timeout
        for (batch_idx, batch) in file_signatories.chunks(self.config.batch_size).enumerate() {
            self.metrics.batches_processed += 1;
            let batch_start = Instant::now();
            
            debug!("Processing batch {} of {} files", batch_idx + 1, batch.len());

            match self.link_batch_with_timeout(batch, signatories, existing_contracts) {
                Ok(contracts) => {
                    let batch_time = batch_start.elapsed();
                    self.metrics.batches_succeeded += 1;
                    let contract_count = contracts.len();
                    self.metrics.contracts_discovered += contract_count;
                    self.metrics.tokens_estimated += 400; // Estimate per batch
                    
                    info!(
                        "Batch {} OK: {} contracts in {:.1}s (tokens: ~400)",
                        batch_idx + 1,
                        contract_count,
                        batch_time.as_secs_f64()
                    );
                    inferred_contracts.extend(contracts);
                }
                Err(e) => {
                    if e.to_string().contains("timeout") {
                        self.metrics.batches_timed_out += 1;
                        warn!(
                            "Batch {} TIMEOUT: {} (skipping, will continue)",
                            batch_idx + 1,
                            e
                        );
                    } else {
                        self.metrics.batches_failed += 1;
                        warn!("Batch {} ERROR: {} (skipping, will continue)", batch_idx + 1, e);
                    }
                }
            }
        }

        let total_time = overall_start.elapsed();
        self.metrics.total_time_ms = total_time.as_millis();
        
        info!(
            "AI linking complete: {} semantic dependencies from {} batches ({}s, ~{} tokens)",
            inferred_contracts.len(),
            self.metrics.batches_succeeded,
            total_time.as_secs_f64(),
            self.metrics.tokens_estimated
        );
        
        info!(
            "Batch summary: {} succeeded, {} failed, {} timed out",
            self.metrics.batches_succeeded,
            self.metrics.batches_failed,
            self.metrics.batches_timed_out
        );

        Ok(inferred_contracts)
    }
    
    /// Get metrics from the last linking pass
    pub fn metrics(&self) -> AILinkerMetrics {
        self.metrics.clone()
    }

    /// Link a batch of signatories with a timeout
    /// Uses a separate process with timeout to prevent hanging on slow Copilot responses
    fn link_batch_with_timeout(
        &mut self,
        batch: &[&Signatory],
        all_signatories: &[Signatory],
        existing_contracts: &[Contract],
    ) -> Result<Vec<Contract>> {
        let timeout = Duration::from_secs(self.config.batch_timeout_secs);
        
        // Format batch for Copilot
        let batch_text = format_batch_for_analysis(batch);
        let prompt = build_linking_prompt(&batch_text);

        if self.config.verbose {
            debug!("Linking prompt:\n{}", prompt);
        }

        // Call Copilot CLI with timeout
        let response = match invoke_copilot_cli_with_timeout(&prompt, timeout) {
            Ok(resp) => resp,
            Err(e) if e.to_string().contains("timeout") => {
                return Err(anyhow!("timeout: batch processing exceeded {}s", timeout.as_secs()));
            }
            Err(e) => return Err(e),
        };

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
                    ClauseType::Uses,
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

/// Invoke Copilot CLI with a prompt and timeout
fn invoke_copilot_cli_with_timeout(prompt: &str, timeout: Duration) -> Result<String> {
    use std::sync::mpsc;
    use std::thread;

    let prompt_copy = prompt.to_string();
    let (tx, rx) = mpsc::channel();

    // Spawn copilot call in separate thread
    let _handle = thread::spawn(move || {
        let output = Command::new("copilot")
            .arg("-p")
            .arg(&prompt_copy)
            .output();
        
        let _ = tx.send(output);
    });

    // Wait for response with timeout
    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow!("Copilot CLI error: {}", stderr))
            } else {
                String::from_utf8(output.stdout).map_err(|e| anyhow!(e))
            }
        }
        Ok(Err(e)) => Err(anyhow!("Failed to invoke copilot: {}", e)),
        Err(_) => {
            // Timeout occurred
            Err(anyhow!("timeout: copilot did not respond within {:?}", timeout))
        }
    }
}

/// Invoke Copilot CLI with a prompt (legacy, no timeout)
fn invoke_copilot_cli(prompt: &str) -> Result<String> {
    invoke_copilot_cli_with_timeout(prompt, Duration::from_secs(30))
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

        assert_eq!(config.batch_size, 15);
        assert_eq!(config.batch_timeout_secs, 30);
        assert_eq!(config.min_confidence, 0.40);
        assert_eq!(config.max_confidence, 0.65);
        assert!(!config.verbose);
    }
}
