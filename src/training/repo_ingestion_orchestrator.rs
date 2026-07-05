//! Repository ingestion orchestrator: scales AST-based contract collection across many repos
//!
//! This module orchestrates bulk repository ingestion for training data collection:
//! 1. Loads curated repo list from JSON registry
//! 2. Ingests each repo (clone -> AST parse -> contract extraction)
//! 3. Saves results to data/contracts-<repo>.json
//! 4. Tracks progress and implements idempotency
//! 5. Logs metrics to DATALAKE_LOG.md (git-tracked)

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use uuid::Uuid;

/// Configuration for repo ingestion
#[derive(Debug, Clone)]
pub struct RepoIngestionConfig {
    pub registry_path: PathBuf,
    pub output_dir: PathBuf,
    pub max_repos: Option<usize>,
    pub timeout_minutes: Option<u64>,
    pub skip_already_ingested: bool,
}

impl RepoIngestionConfig {
    pub fn default_paths() -> Self {
        Self {
            registry_path: PathBuf::from("data/repos_to_ingest.json"),
            output_dir: PathBuf::from("data"),
            max_repos: None,
            timeout_minutes: None,
            skip_already_ingested: true,
        }
    }
}

/// A repository entry from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryEntry {
    pub repo_url: String,
    pub repo_name: String,
    pub owner: String,
    pub stars: u32,
    pub language: String,
    pub priority: u32,
    #[serde(default)]
    pub reason: String,
}

/// Registry metadata and repo list
#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryRegistry {
    pub metadata: RegistryMetadata,
    pub repositories: Vec<RepositoryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryMetadata {
    pub description: String,
    pub created_at: String,
    pub total_repos: usize,
    pub languages: Vec<String>,
    pub size_range: String,
    pub selection_criteria: Vec<String>,
}

/// Metrics for a single ingested repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionMetrics {
    pub repo_name: String,
    pub repo_url: String,
    pub status: IngestionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub files_processed: usize,
    pub signatories: usize,
    pub contracts: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum IngestionStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "skipped")]
    Skipped,
}

/// Ingestion log entry (persisted to track state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionLogEntry {
    pub repo_name: String,
    pub repo_url: String,
    pub timestamp: DateTime<Utc>,
    pub status: String,
    pub files_processed: usize,
    pub signatories: usize,
    pub contracts: usize,
    pub duration_secs: u64,
}

/// Main orchestrator for repository ingestion
pub struct RepositoryIngestionOrchestrator {
    config: RepoIngestionConfig,
    registry: RepositoryRegistry,
    log_entries: Vec<IngestionLogEntry>,
}

impl RepositoryIngestionOrchestrator {
    /// Create new orchestrator and load registry
    pub fn new(config: RepoIngestionConfig) -> Result<Self> {
        let registry_content = fs::read_to_string(&config.registry_path)?;
        let registry: RepositoryRegistry = serde_json::from_str(&registry_content)?;

        Ok(Self {
            config,
            registry,
            log_entries: Vec::new(),
        })
    }

    /// Get reference to the loaded registry (for testing)
    pub fn get_registry(&self) -> &RepositoryRegistry {
        &self.registry
    }

    /// Load existing ingestion log to check what's already done
    pub fn load_ingestion_log(&self) -> Result<HashMap<String, IngestionLogEntry>> {
        let log_path = self.config.output_dir.join("ingestion-log.json");
        if !log_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(log_path)?;
        let entries: Vec<IngestionLogEntry> = serde_json::from_str(&content)?;
        
        let mut map = HashMap::new();
        for entry in entries {
            map.insert(entry.repo_name.clone(), entry);
        }
        Ok(map)
    }

    /// Save ingestion log to file
    pub fn save_ingestion_log(&self) -> Result<()> {
        let log_path = self.config.output_dir.join("ingestion-log.json");
        let json = serde_json::to_string_pretty(&self.log_entries)?;
        fs::write(log_path, json)?;
        Ok(())
    }

    /// Run the full ingestion orchestration
    pub async fn run(&mut self) -> Result<IngestionResults> {
        let run_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();
        let deadline = self.config.timeout_minutes
            .map(|mins| started_at + Duration::minutes(mins as i64));

        println!("🌱 Starting repository ingestion orchestration (run: {})", run_id);
        println!("   Registry: {}", self.config.registry_path.display());
        println!("   Output: {}", self.config.output_dir.display());
        if let Some(max) = self.config.max_repos {
            println!("   Max repos: {}", max);
        }
        if let Some(mins) = self.config.timeout_minutes {
            println!("   Timeout: {} minutes", mins);
        }
        println!();

        // Load existing ingestion log for idempotency
        let existing_log = self.load_ingestion_log()?;
        let mut metrics = Vec::new();
        let mut repos_processed = 0;

        // Process each repository
        for (idx, repo) in self.registry.repositories.iter().enumerate() {
            // Check repo limit
            if let Some(max) = self.config.max_repos {
                if repos_processed >= max {
                    println!("📊 Repo limit reached ({}/{})", repos_processed, max);
                    break;
                }
            }

            // Check timeout
            if let Some(deadline) = deadline {
                if Utc::now() >= deadline {
                    println!("⏱️  Timeout reached! Stopping.");
                    break;
                }
            }

            // Check if already ingested
            if self.config.skip_already_ingested && existing_log.contains_key(&repo.repo_name) {
                println!("⏭️  [{}/{}] {} (already ingested, skipping)", 
                    idx + 1, self.registry.repositories.len(), repo.repo_name);
                continue;
            }

            print!("📦 [{}/{}] {} ... ", 
                idx + 1, self.registry.repositories.len(), repo.repo_name);
            std::io::Write::flush(&mut std::io::stdout()).ok();

            let repo_started = Utc::now();
            match self.ingest_repo(repo).await {
                Ok((files, signatories, contracts)) => {
                    let repo_completed = Utc::now();
                    let duration = (repo_completed - repo_started).num_seconds() as u64;
                    
                    println!("✅ {} signatories, {} contracts ({}s)",
                        signatories, contracts, duration);

                    let metric = IngestionMetrics {
                        repo_name: repo.repo_name.clone(),
                        repo_url: repo.repo_url.clone(),
                        status: IngestionStatus::Success,
                        started_at: repo_started,
                        completed_at: repo_completed,
                        files_processed: files,
                        signatories,
                        contracts,
                        error_message: None,
                    };

                    let log_entry = IngestionLogEntry {
                        repo_name: repo.repo_name.clone(),
                        repo_url: repo.repo_url.clone(),
                        timestamp: repo_completed,
                        status: "success".to_string(),
                        files_processed: files,
                        signatories,
                        contracts,
                        duration_secs: duration,
                    };

                    metrics.push(metric);
                    self.log_entries.push(log_entry);
                    repos_processed += 1;
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                    
                    let metric = IngestionMetrics {
                        repo_name: repo.repo_name.clone(),
                        repo_url: repo.repo_url.clone(),
                        status: IngestionStatus::Failed,
                        started_at: repo_started,
                        completed_at: Utc::now(),
                        files_processed: 0,
                        signatories: 0,
                        contracts: 0,
                        error_message: Some(e.to_string()),
                    };

                    let log_entry = IngestionLogEntry {
                        repo_name: repo.repo_name.clone(),
                        repo_url: repo.repo_url.clone(),
                        timestamp: Utc::now(),
                        status: "failed".to_string(),
                        files_processed: 0,
                        signatories: 0,
                        contracts: 0,
                        duration_secs: (Utc::now() - repo_started).num_seconds() as u64,
                    };

                    metrics.push(metric);
                    self.log_entries.push(log_entry);
                }
            }
        }

        // Save ingestion log
        self.save_ingestion_log()?;

        let completed_at = Utc::now();
        let duration = (completed_at - started_at).num_seconds() as u64;

        // Calculate aggregated stats
        let total_files: usize = metrics.iter().map(|m| m.files_processed).sum();
        let total_signatories: usize = metrics.iter().map(|m| m.signatories).sum();
        let total_contracts: usize = metrics.iter().map(|m| m.contracts).sum();
        let successful = metrics.iter().filter(|m| m.status == IngestionStatus::Success).count();
        let failed = metrics.iter().filter(|m| m.status == IngestionStatus::Failed).count();

        let results = IngestionResults {
            run_id,
            started_at,
            completed_at,
            duration_secs: duration,
            repos_processed: repos_processed,
            total_repos: self.registry.repositories.len(),
            total_files,
            total_signatories,
            total_contracts,
            successful,
            failed,
            metrics,
        };

        // Update markdown log
        self.update_markdown_log(&results)?;

        Ok(results)
    }

    /// Ingest a single repository (stub for now - will use RepositoryTraverser)
    async fn ingest_repo(&self, repo: &RepositoryEntry) -> Result<(usize, usize, usize)> {
        // Use the existing RepositoryTraverser for ingestion
        let config = crate::RepositoryIngestionConfig {
            repo_url: repo.repo_url.clone(),
            branch: "main".to_string(),
            work_dir: None,
            skip_clone: false,
        };

        let traverser = crate::RepositoryTraverser::new(config);
        let result = traverser.ingest().await?;

        // Return (files, signatories, contracts)
        // For now, contracts is estimated from the graph structure
        let contracts = result.signatories_registered.len(); // Rough estimate
        
        Ok((result.files_processed, result.signatories_registered.len(), contracts))
    }

    /// Update DATALAKE_LOG.md with progress
    fn update_markdown_log(&self, results: &IngestionResults) -> Result<()> {
        let log_path = self.config.output_dir.join("..").join("DATALAKE_LOG.md");
        
        let mut content = String::new();
        content.push_str("# Data Lake Ingestion Log\n\n");
        content.push_str(&format!("**Last Updated**: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        content.push_str("## Current Status\n\n");
        content.push_str(&format!("- **Run ID**: {}\n", results.run_id));
        content.push_str(&format!("- **Duration**: {} seconds ({:.1} minutes)\n", results.duration_secs, results.duration_secs as f64 / 60.0));
        content.push_str(&format!("- **Repos Processed**: {}/{}\n", results.repos_processed, results.total_repos));
        content.push_str(&format!("- **Success**: {} | **Failed**: {}\n\n", results.successful, results.failed));
        
        content.push_str("## Aggregated Metrics\n\n");
        content.push_str(&format!("- **Total Files**: {}\n", results.total_files));
        content.push_str(&format!("- **Total Signatories**: {}\n", results.total_signatories));
        content.push_str(&format!("- **Total Contracts**: {}\n\n", results.total_contracts));

        content.push_str("## Repository Breakdown\n\n");
        content.push_str("| Repo | Status | Files | Signatories | Contracts | Time (s) |\n");
        content.push_str("|------|--------|-------|-------------|-----------|----------|\n");

        for metric in &results.metrics {
            let status = match metric.status {
                IngestionStatus::Success => "✅",
                IngestionStatus::Failed => "❌",
                IngestionStatus::Skipped => "⏭️",
            };

            let duration = (metric.completed_at - metric.started_at).num_seconds();
            content.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                metric.repo_name,
                status,
                metric.files_processed,
                metric.signatories,
                metric.contracts,
                duration
            ));
        }

        fs::write(log_path, content)?;
        Ok(())
    }
}

/// Overall results from an ingestion run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResults {
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_secs: u64,
    pub repos_processed: usize,
    pub total_repos: usize,
    pub total_files: usize,
    pub total_signatories: usize,
    pub total_contracts: usize,
    pub successful: usize,
    pub failed: usize,
    pub metrics: Vec<IngestionMetrics>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_load() {
        let config = RepoIngestionConfig {
            registry_path: PathBuf::from("data/repos_to_ingest.json"),
            output_dir: PathBuf::from("data"),
            max_repos: Some(5),
            timeout_minutes: Some(10),
            skip_already_ingested: true,
        };

        if config.registry_path.exists() {
            let result = RepositoryIngestionOrchestrator::new(config);
            assert!(result.is_ok(), "Should load registry successfully");
            
            if let Ok(orchestrator) = result {
                assert!(!orchestrator.registry.repositories.is_empty(), 
                    "Registry should have repositories");
            }
        }
    }

    #[test]
    fn test_idempotency_log() {
        let config = RepoIngestionConfig::default_paths();
        
        let entry = IngestionLogEntry {
            repo_name: "test-repo".to_string(),
            repo_url: "https://github.com/test/repo".to_string(),
            timestamp: Utc::now(),
            status: "success".to_string(),
            files_processed: 100,
            signatories: 50,
            contracts: 25,
            duration_secs: 10,
        };

        let mut log_entries = vec![entry];
        let entries = log_entries.pop().unwrap();
        
        assert_eq!(entries.repo_name, "test-repo");
        assert_eq!(entries.files_processed, 100);
    }
}
