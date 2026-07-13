//! AI-augmented dependency linking for idud.
//!
//! This module uses the Copilot CLI to infer semantic dependencies that AST analysis misses.
//! It batches signatories for efficient prompting, applies timeouts to avoid hanging
//! requests, and defends against malformed LLM output.

use crate::types::{ClauseType, Contract, ContractSource, Signatory, SignatoryType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};
use tracing::{debug, info, warn};

/// Configuration for AI linking behavior.
#[derive(Debug, Clone)]
pub struct AILinkerConfig {
    /// Batch size: how many signatories per Copilot request.
    pub batch_size: usize,
    /// Timeout per batch in seconds.
    pub batch_timeout_secs: u64,
    /// Minimum confidence for semantic dependencies.
    pub min_confidence: f32,
    /// Maximum confidence for AI-inferred dependencies.
    pub max_confidence: f32,
    /// Enable verbose logging for prompts and responses.
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

/// Metrics for AI linking operations.
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

/// Errors returned by the AI linker.
#[derive(Debug, Error)]
pub enum AiLinkerError {
    #[error("copilot CLI is not installed")]
    CliNotFound,
    #[error("failed to invoke copilot CLI: {0}")]
    InvocationFailed(String),
    #[error("copilot CLI timed out after {timeout:?}")]
    Timeout { timeout: Duration },
    #[error("copilot CLI returned an error: {0}")]
    CommandFailed(String),
    #[error("copilot response did not contain a JSON array")]
    NoJsonArray,
    #[error("failed to parse copilot response: {0}")]
    ParseFailed(String),
}

/// Abstraction over the Copilot CLI process used by the linker.
#[async_trait]
pub trait CopilotClient: Send + Sync {
    /// Checks whether the backend can be invoked.
    async fn is_available(&self) -> Result<(), AiLinkerError>;

    /// Invokes the backend with a prompt and timeout.
    async fn invoke(&self, prompt: &str, timeout: Duration) -> Result<String, AiLinkerError>;
}

type DynCopilotClient = Arc<dyn CopilotClient + Send + Sync>;

/// Tokio-backed implementation of the Copilot client.
#[derive(Debug, Default)]
pub struct TokioCopilotClient;

#[async_trait]
impl CopilotClient for TokioCopilotClient {
    async fn is_available(&self) -> Result<(), AiLinkerError> {
        let output = Command::new("which")
            .arg("copilot")
            .output()
            .await
            .map_err(|err| {
                AiLinkerError::InvocationFailed(format!("failed to check copilot CLI: {err}"))
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(AiLinkerError::CliNotFound)
        }
    }

    async fn invoke(
        &self,
        prompt: &str,
        timeout_duration: Duration,
    ) -> Result<String, AiLinkerError> {
        let mut child = Command::new("copilot")
            .arg("-p")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|err| {
                AiLinkerError::InvocationFailed(format!("failed to spawn copilot process: {err}"))
            })?;

        let stdin = child.stdin.as_mut().ok_or_else(|| {
            AiLinkerError::InvocationFailed("copilot stdin pipe was unavailable".to_string())
        })?;
        stdin.write_all(prompt.as_bytes()).await.map_err(|err| {
            AiLinkerError::InvocationFailed(format!(
                "failed to write prompt to copilot stdin: {err}"
            ))
        })?;
        stdin.shutdown().await.map_err(|err| {
            AiLinkerError::InvocationFailed(format!("failed to close copilot stdin: {err}"))
        })?;

        match timeout(timeout_duration, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    String::from_utf8(output.stdout).map_err(|err| {
                        AiLinkerError::InvocationFailed(format!(
                            "invalid utf-8 from copilot stdout: {err}"
                        ))
                    })
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(AiLinkerError::CommandFailed(stderr.into_owned()))
                }
            }
            Ok(Err(err)) => Err(AiLinkerError::InvocationFailed(format!(
                "copilot process exited with an error: {err}"
            ))),
            Err(_) => Err(AiLinkerError::Timeout {
                timeout: timeout_duration,
            }),
        }
    }
}

/// AI Linker: uses the Copilot CLI to infer semantic dependencies.
pub struct AILinker {
    config: AILinkerConfig,
    maybe_tokens_used: u64,
    metrics: AILinkerMetrics,
    copilot_client: DynCopilotClient,
}

impl AILinker {
    /// Creates a linker with the default Tokio-backed client.
    pub fn new(config: AILinkerConfig) -> Self {
        Self::with_copilot_client(config, Arc::new(TokioCopilotClient))
    }

    /// Creates a linker with a custom client for unit tests and dependency injection.
    pub fn with_copilot_client(config: AILinkerConfig, client: DynCopilotClient) -> Self {
        Self {
            config: config.clone(),
            maybe_tokens_used: 0,
            metrics: AILinkerMetrics {
                batches_processed: 0,
                batches_succeeded: 0,
                batches_failed: 0,
                batches_timed_out: 0,
                contracts_discovered: 0,
                tokens_estimated: 0,
                total_time_ms: 0,
            },
            copilot_client: client,
        }
    }

    /// Links signatories in a codebase using AI analysis with per-batch timeouts.
    pub async fn link_files(
        &mut self,
        signatories: &[Signatory],
        existing_contracts: &[Contract],
    ) -> Result<Vec<Contract>, AiLinkerError> {
        let overall_start = Instant::now();

        self.copilot_client.is_available().await?;

        let file_signatories = collect_file_signatories(signatories);
        if file_signatories.is_empty() {
            info!("No files to analyze for semantic dependencies");
            return Ok(Vec::new());
        }

        info!(
            files = file_signatories.len(),
            batch_size = self.config.batch_size,
            timeout_secs = self.config.batch_timeout_secs,
            "Starting AI linking pass"
        );

        let mut inferred_contracts = Vec::new();
        for (batch_idx, batch) in file_signatories.chunks(self.config.batch_size).enumerate() {
            self.metrics.batches_processed += 1;
            let batch_start = Instant::now();

            debug!(
                batch = batch_idx + 1,
                files = batch.len(),
                "Processing AI linking batch"
            );

            match self
                .link_batch_with_timeout(batch, signatories, existing_contracts)
                .await
            {
                Ok(contracts) => {
                    let batch_time = batch_start.elapsed();
                    self.metrics.batches_succeeded += 1;
                    self.metrics.contracts_discovered += contracts.len();
                    self.metrics.tokens_estimated += 400;
                    inferred_contracts.extend(contracts);

                    info!(
                        batch = batch_idx + 1,
                        contracts = inferred_contracts.len(),
                        duration_ms = batch_time.as_millis(),
                        "AI batch completed"
                    );
                }
                Err(err) => {
                    if err.to_string().contains("timed out") {
                        self.metrics.batches_timed_out += 1;
                        warn!(
                            batch = batch_idx + 1,
                            error = %err,
                            "AI batch timed out"
                        );
                    } else {
                        self.metrics.batches_failed += 1;
                        warn!(
                            batch = batch_idx + 1,
                            error = %err,
                            "AI batch failed"
                        );
                    }
                }
            }
        }

        self.metrics.total_time_ms = overall_start.elapsed().as_millis();

        info!(
            batches_succeeded = self.metrics.batches_succeeded,
            batches_failed = self.metrics.batches_failed,
            batches_timed_out = self.metrics.batches_timed_out,
            contracts = inferred_contracts.len(),
            elapsed_ms = self.metrics.total_time_ms,
            "AI linking pass completed"
        );

        Ok(inferred_contracts)
    }

    /// Returns metrics from the last linking pass.
    pub fn metrics(&self) -> AILinkerMetrics {
        self.metrics.clone()
    }

    /// Links a single batch of signatories with a timeout.
    async fn link_batch_with_timeout(
        &mut self,
        batch: &[&Signatory],
        all_signatories: &[Signatory],
        existing_contracts: &[Contract],
    ) -> Result<Vec<Contract>, AiLinkerError> {
        let timeout_duration = Duration::from_secs(self.config.batch_timeout_secs);
        let batch_text = format_batch_for_analysis(batch);
        let prompt = build_linking_prompt(&batch_text);

        if self.config.verbose {
            debug!(batch = batch.len(), prompt = %prompt, "Sending AI linking prompt");
        }

        let response = self
            .copilot_client
            .invoke(&prompt, timeout_duration)
            .await?;

        if self.config.verbose {
            debug!(response = %response, "Received AI linking response");
        }

        let inferred_pairs = parse_linking_response(&response, batch)?;
        build_contracts_from_pairs(
            inferred_pairs,
            all_signatories,
            existing_contracts,
            self.config.clone(),
        )
    }

    /// Returns the total number of tokens used in this linking pass.
    pub fn tokens_used(&self) -> u64 {
        self.maybe_tokens_used
    }
}

fn collect_file_signatories(signatories: &[Signatory]) -> Vec<&Signatory> {
    signatories
        .iter()
        .filter(|signatory| signatory.signatory_type == SignatoryType::File)
        .collect()
}

/// Formats a batch of signatories for semantic analysis.
fn format_batch_for_analysis(batch: &[&Signatory]) -> String {
    batch
        .iter()
        .map(|signatory| signatory.label.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Builds a focused prompt for Copilot.
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

fn build_contracts_from_pairs(
    inferred_pairs: Vec<(String, String, String)>,
    all_signatories: &[Signatory],
    existing_contracts: &[Contract],
    config: AILinkerConfig,
) -> Result<Vec<Contract>, AiLinkerError> {
    let mut contracts = Vec::new();
    let existing_set = build_contract_set(existing_contracts);

    for (from_label, to_label, reasoning) in inferred_pairs {
        let from_sig = all_signatories
            .iter()
            .find(|signatory| signatory.label == from_label);
        let to_sig = all_signatories
            .iter()
            .find(|signatory| signatory.label == to_label);

        if let (Some(from), Some(to)) = (from_sig, to_sig) {
            let contract_key = (from.id.clone(), to.id.clone());
            if existing_set.contains(&contract_key) {
                continue;
            }

            let confidence =
                config.min_confidence + (config.max_confidence - config.min_confidence) / 2.0;

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

/// Response structure from Copilot.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkingPair {
    from: String,
    to: String,
    reason: String,
}

/// Parses a Copilot response into dependency pairs.
fn parse_linking_response(
    response: &str,
    batch: &[&Signatory],
) -> Result<Vec<(String, String, String)>, AiLinkerError> {
    let candidates = response
        .match_indices('[')
        .filter_map(|(open_idx, _)| {
            find_matching_bracket(response, open_idx).map(|close_idx| (open_idx, close_idx))
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err(AiLinkerError::NoJsonArray);
    }

    let batch_files: HashSet<&str> = batch
        .iter()
        .map(|signatory| signatory.label.as_str())
        .collect();

    for (json_start, json_end) in candidates {
        let json_str = &response[json_start..=json_end];
        match serde_json::from_str::<Vec<LinkingPair>>(json_str) {
            Ok(pairs) => {
                let mut result = Vec::new();
                for pair in pairs {
                    if batch_files.contains(pair.from.as_str())
                        && batch_files.contains(pair.to.as_str())
                    {
                        result.push((pair.from, pair.to, pair.reason));
                    }
                }
                return Ok(result);
            }
            Err(err) => {
                debug!(error = %err, "Failed to parse a candidate JSON array from Copilot response");
            }
        }
    }

    Err(AiLinkerError::ParseFailed(
        "failed to parse any candidate JSON array from Copilot response".to_string(),
    ))
}

fn find_matching_bracket(response: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in response[open_idx..].char_indices() {
        let absolute_idx = open_idx + idx;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(absolute_idx);
                }
            }
            _ => {}
        }
    }

    None
}

/// Builds a set of existing contract pairs for deduplication.
fn build_contract_set(contracts: &[Contract]) -> HashSet<(String, String)> {
    contracts
        .iter()
        .map(|contract| (contract.principal_id.clone(), contract.guarantor_id.clone()))
        .collect()
}
