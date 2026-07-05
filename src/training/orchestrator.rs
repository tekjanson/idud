//! Training loop orchestrator: coordinates the entire validation pipeline
//! 
//! Orchestrates a multi-stage training system that:
//! 1. INTAKE: Discovers candidate repositories
//! 2. BATCH: Splits repos into parallel groups
//! 3. PROCESS: For each repo, ingests dependency graph and validates predictions
//! 4. AGGREGATE: Collects metrics and computes trends

use crate::training::{
    discover_training_repos, fetch_issue_and_linked_pr, RepoCandidate, IssueWithPR,
    predict_files_from_issue, PredictionRequest,
};
use crate::training_datalake::{
    AggregatedMetrics, IngestionStatus, PercentileMetrics, RepoMetadata, TimeWindow,
    TrainingDataLake, TrainingRun, Checkpoint,
};
use crate::{RepositoryIngestionConfig, RepositoryTraverser};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

/// Training batch containing a subset of repos to process in parallel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingBatch {
    pub batch_id: String,
    pub repos: Vec<RepoCandidate>,
    pub batch_size: usize,
    pub created_at: DateTime<Utc>,
}

/// Metrics for a single repo processed through the training pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoTrainingMetrics {
    pub repo_url: String,
    pub repo_name: String,
    pub issues_processed: usize,
    pub predictions_made: usize,
    pub avg_precision: f64,
    pub avg_recall: f64,
    pub avg_f1: f64,
    pub total_true_positives: u32,
    pub total_false_positives: u32,
    pub total_false_negatives: u32,
}

/// Overall training results from a complete run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingResults {
    pub run_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_repos_processed: usize,
    pub total_predictions: usize,
    pub repo_metrics: Vec<RepoTrainingMetrics>,
    pub aggregated_metrics: Option<AggregatedMetrics>,
    pub status: TrainingStatus,
}

/// Current status of a training run
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrainingStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl Default for TrainingStatus {
    fn default() -> Self {
        TrainingStatus::Pending
    }
}

/// Configuration for the training orchestrator
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    pub batch_size: usize,
    pub max_concurrent_agents: usize,
    pub anthropic_api_key: String,
    pub datalake_path: String,
}

/// Main orchestrator that drives the training pipeline
pub struct TrainingOrchestrator {
    config: TrainingConfig,
    datalake: Arc<TrainingDataLake>,
}

impl TrainingOrchestrator {
    pub fn new(config: TrainingConfig) -> Result<Self> {
        let datalake = Arc::new(TrainingDataLake::new(&config.datalake_path)?);
        Ok(Self { config, datalake })
    }

    /// Run the complete training loop
    pub async fn run_training_loop(&self, repos: Vec<RepoCandidate>) -> Result<TrainingResults> {
        let run_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        tracing::info!("🚀 Starting training run: {}", run_id);
        tracing::info!("Processing {} repos with {} concurrent agents", 
                      repos.len(), self.config.max_concurrent_agents);

        // Step 1: BATCH repos into parallel groups
        let batches = batch_training_jobs(repos, self.config.batch_size)?;
        tracing::info!("📦 Created {} batches", batches.len());

        let mut all_repo_metrics = Vec::new();
        let mut all_training_runs = Vec::new();

        // Step 2: Process each batch
        for (batch_idx, batch) in batches.iter().enumerate() {
            tracing::info!("Processing batch {}/{} with {} repos", 
                          batch_idx + 1, batches.len(), batch.repos.len());

            let batch_results = self.process_batch(batch, &run_id).await?;
            all_repo_metrics.extend(batch_results.0);
            all_training_runs.extend(batch_results.1);
        }

        // Step 3: Aggregate results and compute metrics
        let aggregated = self.aggregate_metrics(&all_training_runs)?;

        let completed_at = Utc::now();
        let results = TrainingResults {
            run_id,
            started_at,
            completed_at,
            total_repos_processed: all_repo_metrics.len(),
            total_predictions: all_training_runs.len(),
            repo_metrics: all_repo_metrics,
            aggregated_metrics: Some(aggregated),
            status: TrainingStatus::Completed,
        };

        // Persist results to datalake
        self.datalake.write_training_runs(&all_training_runs)?;

        tracing::info!("✅ Training run completed: {}", results.run_id);
        tracing::info!("📊 Processed {} repos, {} predictions", 
                      results.total_repos_processed, results.total_predictions);

        Ok(results)
    }

    /// Process a single batch of repos
    async fn process_batch(
        &self,
        batch: &TrainingBatch,
        run_id: &str,
    ) -> Result<(Vec<RepoTrainingMetrics>, Vec<TrainingRun>)> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_agents));
        let mut tasks = vec![];

        for repo in &batch.repos {
            let semaphore = semaphore.clone();
            let repo = repo.clone();
            let batch_id = batch.batch_id.clone();
            let api_key = self.config.anthropic_api_key.clone();
            let run_id = run_id.to_string();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.ok();
                Self::process_repo(&repo, &batch_id, &run_id, &api_key).await
            });

            tasks.push(task);
        }

        let mut all_metrics = Vec::new();
        let mut all_runs = Vec::new();

        for task in tasks {
            match task.await {
                Ok(Ok((metrics, runs))) => {
                    all_metrics.push(metrics);
                    all_runs.extend(runs);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Failed to process repo: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Task join error: {}", e);
                }
            }
        }

        Ok((all_metrics, all_runs))
    }

    /// Process a single repository through the full pipeline
    async fn process_repo(
        repo: &RepoCandidate,
        batch_id: &str,
        run_id: &str,
        api_key: &str,
    ) -> Result<(RepoTrainingMetrics, Vec<TrainingRun>)> {
        tracing::info!("Processing repo: {}", repo.url);

        // Step 1: Ingest repository and build dependency graph
        let (signatories, contracts) = Self::ingest_repo(&repo.url).await?;
        tracing::debug!("Ingested {} signatories, {} contracts", 
                       signatories.len(), contracts.len());

        // Step 2: Select 1-3 recent issues for validation
        let issues = Self::select_recent_issues(&repo, 1..=3).await?;
        tracing::info!("Selected {} issues for repo {}", issues.len(), repo.name);

        let mut repo_metrics = RepoTrainingMetrics {
            repo_url: repo.url.clone(),
            repo_name: repo.name.clone(),
            issues_processed: issues.len(),
            predictions_made: 0,
            avg_precision: 0.0,
            avg_recall: 0.0,
            avg_f1: 0.0,
            total_true_positives: 0,
            total_false_positives: 0,
            total_false_negatives: 0,
        };

        let mut training_runs = Vec::new();
        let mut precisions = Vec::new();
        let mut recalls = Vec::new();
        let mut f1s = Vec::new();

        // Step 3: For each issue, predict and validate
        for issue in issues {
            // Call predictor
            let prediction_req = PredictionRequest {
                issue_text: issue.issue_body.clone(),
                dependency_graph: contracts.clone(),
                signatories: signatories.clone(),
            };

            match crate::training::predict_files_from_issue(prediction_req, api_key).await {
                Ok(prediction) => {
                    let mut training_run = TrainingRun::new(
                        repo.url.clone(),
                        issue.issue_number.to_string(),
                        issue.issue_body.clone(),
                        prediction.predicted_files.clone(),
                        issue.pr_files.clone(),
                    );
                    training_run.batch_id = Some(batch_id.to_string());

                    precisions.push(training_run.precision);
                    recalls.push(training_run.recall);
                    f1s.push(training_run.f1);

                    repo_metrics.total_true_positives += training_run.true_positives;
                    repo_metrics.total_false_positives += training_run.false_positives;
                    repo_metrics.total_false_negatives += training_run.false_negatives;

                    training_runs.push(training_run);
                    repo_metrics.predictions_made += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to get prediction for issue {}: {}", 
                                  issue.issue_number, e);
                }
            }
        }

        // Calculate repo-level aggregates
        if !precisions.is_empty() {
            repo_metrics.avg_precision = precisions.iter().sum::<f64>() / precisions.len() as f64;
            repo_metrics.avg_recall = recalls.iter().sum::<f64>() / recalls.len() as f64;
            repo_metrics.avg_f1 = f1s.iter().sum::<f64>() / f1s.len() as f64;
        }

        tracing::info!("✅ Completed repo {}: {} predictions, F1={:.3}", 
                      repo.name, training_runs.len(), repo_metrics.avg_f1);

        Ok((repo_metrics, training_runs))
    }

    /// Ingest a repository and extract signatories and contracts
    async fn ingest_repo(url: &str) -> Result<(Vec<crate::Signatory>, Vec<crate::Contract>)> {
        let config = RepositoryIngestionConfig {
            url: url.to_string(),
            branch: "main".to_string(),
            work_dir: None,
        };

        let traverser = RepositoryTraverser::new(config)?;
        let (signatories, contracts) = traverser.extract_signatories_and_contracts()?;

        Ok((signatories, contracts))
    }

    /// Select 1-3 recent issues from a repo with linked PRs
    async fn select_recent_issues(
        repo: &RepoCandidate,
        count_range: std::ops::RangeInclusive<usize>,
    ) -> Result<Vec<IssueWithPR>> {
        let mut issues = Vec::new();
        let max_count = *count_range.end();

        for i in 0..max_count {
            match fetch_issue_and_linked_pr(&repo.owner, &repo.name, i).await {
                Ok(issue) => issues.push(issue),
                Err(_) => break,
            }
            if issues.len() >= *count_range.end() {
                break;
            }
        }

        Ok(issues)
    }

    /// Aggregate training runs into comprehensive metrics
    fn aggregate_metrics(&self, training_runs: &[TrainingRun]) -> Result<AggregatedMetrics> {
        if training_runs.is_empty() {
            return Err(anyhow::anyhow!("No training runs to aggregate"));
        }

        let total_precision: f64 = training_runs.iter().map(|r| r.precision).sum();
        let total_recall: f64 = training_runs.iter().map(|r| r.recall).sum();
        let total_f1: f64 = training_runs.iter().map(|r| r.f1).sum();
        let count = training_runs.len() as f64;

        let avg_precision = total_precision / count;
        let avg_recall = total_recall / count;
        let avg_f1 = total_f1 / count;

        // Calculate percentiles
        let mut precisions: Vec<f64> = training_runs.iter().map(|r| r.precision).collect();
        let mut recalls: Vec<f64> = training_runs.iter().map(|r| r.recall).collect();
        let mut f1s: Vec<f64> = training_runs.iter().map(|r| r.f1).collect();

        precisions.sort_by(|a, b| a.partial_cmp(b).unwrap());
        recalls.sort_by(|a, b| a.partial_cmp(b).unwrap());
        f1s.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = precisions.len();
        let p25_idx = len / 4;
        let p50_idx = len / 2;
        let p75_idx = (len * 3) / 4;

        let percentiles = PercentileMetrics {
            p25_precision: precisions.get(p25_idx).copied().unwrap_or(0.0),
            p50_precision: precisions.get(p50_idx).copied().unwrap_or(0.0),
            p75_precision: precisions.get(p75_idx).copied().unwrap_or(0.0),
            p25_recall: recalls.get(p25_idx).copied().unwrap_or(0.0),
            p50_recall: recalls.get(p50_idx).copied().unwrap_or(0.0),
            p75_recall: recalls.get(p75_idx).copied().unwrap_or(0.0),
            p25_f1: f1s.get(p25_idx).copied().unwrap_or(0.0),
            p50_f1: f1s.get(p50_idx).copied().unwrap_or(0.0),
            p75_f1: f1s.get(p75_idx).copied().unwrap_or(0.0),
        };

        let now = Utc::now();
        Ok(AggregatedMetrics {
            metric_id: Uuid::new_v4(),
            generated_at: now,
            period: format!("Training run at {}", now.format("%Y-%m-%d %H:%M:%S")),
            time_window: TimeWindow {
                start: now - Duration::hours(24),
                end: now,
            },
            total_repos: training_runs
                .iter()
                .map(|r| r.repo_url.clone())
                .collect::<std::collections::HashSet<_>>()
                .len() as u32,
            total_predictions: training_runs.len() as u32,
            avg_precision,
            avg_recall,
            avg_f1,
            improvement_over_time: vec![Checkpoint {
                timestamp: now,
                avg_f1,
                sample_size: training_runs.len() as u32,
            }],
            percentiles: Some(percentiles),
        })
    }
}

/// Split repos into batches for parallel processing
pub fn batch_training_jobs(
    repos: Vec<RepoCandidate>,
    batch_size: usize,
) -> Result<Vec<TrainingBatch>> {
    if batch_size == 0 {
        return Err(anyhow::anyhow!("Batch size must be greater than 0"));
    }

    let mut batches = Vec::new();
    let now = Utc::now();

    for (idx, chunk) in repos.chunks(batch_size).enumerate() {
        let batch = TrainingBatch {
            batch_id: format!("batch-{}-{}", Uuid::new_v4(), idx),
            repos: chunk.to_vec(),
            batch_size: chunk.len(),
            created_at: now,
        };
        batches.push(batch);
    }

    Ok(batches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_training_jobs() {
        let repos = vec![
            RepoCandidate {
                url: "https://github.com/a/a".to_string(),
                name: "a".to_string(),
                owner: "a".to_string(),
                stars: 100,
                language: None,
                issue_count: 5,
                pr_count: 3,
                last_issue_id: None,
                last_pr_id: None,
                updated_at: "2025-01-01".to_string(),
            },
            RepoCandidate {
                url: "https://github.com/b/b".to_string(),
                name: "b".to_string(),
                owner: "b".to_string(),
                stars: 200,
                language: None,
                issue_count: 10,
                pr_count: 5,
                last_issue_id: None,
                last_pr_id: None,
                updated_at: "2025-01-02".to_string(),
            },
            RepoCandidate {
                url: "https://github.com/c/c".to_string(),
                name: "c".to_string(),
                owner: "c".to_string(),
                stars: 300,
                language: None,
                issue_count: 15,
                pr_count: 8,
                last_issue_id: None,
                last_pr_id: None,
                updated_at: "2025-01-03".to_string(),
            },
        ];

        let batches = batch_training_jobs(repos.clone(), 2).unwrap();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].batch_size, 2);
        assert_eq!(batches[1].batch_size, 1);
    }
}
