// src/training_datalake.rs
//! Training Data Lake Infrastructure
//!
//! This module provides robust I/O operations for idud's self-validation training system.
//! It handles serialization/deserialization of training runs, repository metadata, and
//! aggregated metrics with type-safe JSON support via serde.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Repository metadata for training dataset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoMetadata {
    pub url: String,
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub stars: u32,
    #[serde(default)]
    pub forks: u32,
    pub language: String,
    #[serde(default)]
    pub last_activity: Option<DateTime<Utc>>,
    #[serde(default)]
    pub issue_count: u32,
    #[serde(default)]
    pub pr_count: u32,
    pub added_at: DateTime<Utc>,
    #[serde(default)]
    pub ingestion_status: IngestionStatus,
}

/// Status of repository ingestion into the training datalake.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IngestionStatus {
    Pending,
    Ingested,
    Failed,
}

impl Default for IngestionStatus {
    fn default() -> Self {
        IngestionStatus::Pending
    }
}

/// A single training run validation result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrainingRun {
    pub run_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub repo_url: String,
    #[serde(default)]
    pub batch_id: Option<String>,
    pub issue_id: String,
    pub issue_text: String,
    pub predicted_files: Vec<String>,
    pub actual_files: Vec<String>,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    #[serde(default)]
    pub true_positives: u32,
    #[serde(default)]
    pub false_positives: u32,
    #[serde(default)]
    pub false_negatives: u32,
}

impl TrainingRun {
    /// Create a new training run with calculated metrics.
    pub fn new(
        repo_url: String,
        issue_id: String,
        issue_text: String,
        predicted_files: Vec<String>,
        actual_files: Vec<String>,
    ) -> Self {
        let tp = predicted_files
            .iter()
            .filter(|p| actual_files.contains(p))
            .count() as u32;
        let fp = (predicted_files.len() as u32) - tp;
        let fn_count = (actual_files.len() as u32) - tp;

        let precision = if predicted_files.is_empty() {
            1.0
        } else {
            tp as f64 / predicted_files.len() as f64
        };

        let recall = if actual_files.is_empty() {
            1.0
        } else {
            tp as f64 / actual_files.len() as f64
        };

        let f1 = if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * (precision * recall) / (precision + recall)
        };

        TrainingRun {
            run_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            repo_url,
            batch_id: None,
            issue_id,
            issue_text,
            predicted_files,
            actual_files,
            precision,
            recall,
            f1,
            true_positives: tp,
            false_positives: fp,
            false_negatives: fn_count,
        }
    }
}

/// Aggregated metrics summarizing training run performance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregatedMetrics {
    pub metric_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub period: String,
    pub time_window: TimeWindow,
    pub total_repos: u32,
    pub total_predictions: u32,
    pub avg_precision: f64,
    pub avg_recall: f64,
    pub avg_f1: f64,
    #[serde(default)]
    pub improvement_over_time: Vec<Checkpoint>,
    #[serde(default)]
    pub percentiles: Option<PercentileMetrics>,
}

/// Time window for metrics aggregation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Historical F1 score checkpoint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Checkpoint {
    pub checkpoint: DateTime<Utc>,
    pub avg_f1: f64,
}

/// F1 percentile distribution metrics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PercentileMetrics {
    #[serde(default)]
    pub p50_f1: f64,
    #[serde(default)]
    pub p75_f1: f64,
    #[serde(default)]
    pub p90_f1: f64,
    #[serde(default)]
    pub p95_f1: f64,
}

/// Training Data Lake manager for file I/O operations.
pub struct TrainingDataLake {
    base_path: PathBuf,
}

impl TrainingDataLake {
    /// Create a new training datalake with a base path.
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        // Ensure all subdirectories exist
        fs::create_dir_all(base_path.join("repos")).context("Failed to create repos directory")?;
        fs::create_dir_all(base_path.join("runs")).context("Failed to create runs directory")?;
        fs::create_dir_all(base_path.join("metrics"))
            .context("Failed to create metrics directory")?;

        Ok(TrainingDataLake { base_path })
    }

    /// Write repository metadata to datalake.
    pub fn write_repo_metadata(&self, metadata: &RepoMetadata) -> Result<PathBuf> {
        let repos_dir = self.base_path.join("repos");
        let filename = format!("{}.repo_metadata.json", metadata.name);
        let filepath = repos_dir.join(&filename);

        let json =
            serde_json::to_string_pretty(metadata).context("Failed to serialize repo metadata")?;
        fs::write(&filepath, json).context("Failed to write repo metadata file")?;

        Ok(filepath)
    }

    /// Read repository metadata from datalake.
    pub fn read_repo_metadata<P: AsRef<Path>>(&self, path: P) -> Result<RepoMetadata> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read repo metadata from {:?}", path))?;
        serde_json::from_str(&content).context("Failed to deserialize repo metadata")
    }

    /// List all repository metadata files in datalake.
    pub fn list_repo_metadata(&self) -> Result<Vec<RepoMetadata>> {
        let repos_dir = self.base_path.join("repos");
        if !repos_dir.exists() {
            return Ok(Vec::new());
        }

        let mut metadata_list = Vec::new();
        for entry in fs::read_dir(&repos_dir).context("Failed to read repos directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                match self.read_repo_metadata(&path) {
                    Ok(metadata) => metadata_list.push(metadata),
                    Err(e) => eprintln!("Warning: Could not read {:?}: {}", path, e),
                }
            }
        }

        Ok(metadata_list)
    }

    /// Write training run to datalake.
    pub fn write_training_run(&self, run: &TrainingRun) -> Result<PathBuf> {
        let runs_dir = self.base_path.join("runs");
        let filename = format!("{}.training_run.json", run.run_id);
        let filepath = runs_dir.join(&filename);

        let json = serde_json::to_string_pretty(run).context("Failed to serialize training run")?;
        fs::write(&filepath, json).context("Failed to write training run file")?;

        Ok(filepath)
    }

    /// Read training run from datalake.
    pub fn read_training_run<P: AsRef<Path>>(&self, path: P) -> Result<TrainingRun> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read training run from {:?}", path))?;
        serde_json::from_str(&content).context("Failed to deserialize training run")
    }

    /// List all training runs in datalake.
    pub fn list_training_runs(&self) -> Result<Vec<TrainingRun>> {
        let runs_dir = self.base_path.join("runs");
        if !runs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut runs = Vec::new();
        for entry in fs::read_dir(&runs_dir).context("Failed to read runs directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                match self.read_training_run(&path) {
                    Ok(run) => runs.push(run),
                    Err(e) => eprintln!("Warning: Could not read {:?}: {}", path, e),
                }
            }
        }

        Ok(runs)
    }

    /// Write aggregated metrics to datalake.
    pub fn write_aggregated_metrics(&self, metrics: &AggregatedMetrics) -> Result<PathBuf> {
        let metrics_dir = self.base_path.join("metrics");
        let filename = format!("{}.aggregated_metrics.json", metrics.metric_id);
        let filepath = metrics_dir.join(&filename);

        let json = serde_json::to_string_pretty(metrics)
            .context("Failed to serialize aggregated metrics")?;
        fs::write(&filepath, json).context("Failed to write aggregated metrics file")?;

        Ok(filepath)
    }

    /// Read aggregated metrics from datalake.
    pub fn read_aggregated_metrics<P: AsRef<Path>>(&self, path: P) -> Result<AggregatedMetrics> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read aggregated metrics from {:?}", path))?;
        serde_json::from_str(&content).context("Failed to deserialize aggregated metrics")
    }

    /// List all aggregated metrics files in datalake.
    pub fn list_aggregated_metrics(&self) -> Result<Vec<AggregatedMetrics>> {
        let metrics_dir = self.base_path.join("metrics");
        if !metrics_dir.exists() {
            return Ok(Vec::new());
        }

        let mut metrics_list = Vec::new();
        for entry in fs::read_dir(&metrics_dir).context("Failed to read metrics directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                match self.read_aggregated_metrics(&path) {
                    Ok(metrics) => metrics_list.push(metrics),
                    Err(e) => eprintln!("Warning: Could not read {:?}: {}", path, e),
                }
            }
        }

        Ok(metrics_list)
    }

    /// Get the base path of the datalake.
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_training_run_metrics_calculation() {
        let predicted = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "src/utils.rs".to_string(),
        ];
        let actual = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];

        let run = TrainingRun::new(
            "https://github.com/test/repo".to_string(),
            "issue-1".to_string(),
            "Test issue".to_string(),
            predicted,
            actual,
        );

        assert_eq!(run.true_positives, 2);
        assert_eq!(run.false_positives, 1);
        assert_eq!(run.false_negatives, 0);
        assert!((run.precision - (2.0 / 3.0)).abs() < 0.001);
        assert!((run.recall - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_datalake_write_read_repo_metadata() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let datalake = TrainingDataLake::new(temp_dir.path())?;

        let metadata = RepoMetadata {
            url: "https://github.com/test/repo".to_string(),
            owner: "test".to_string(),
            name: "repo".to_string(),
            stars: 100,
            forks: 20,
            language: "Rust".to_string(),
            last_activity: Some(Utc::now()),
            issue_count: 5,
            pr_count: 2,
            added_at: Utc::now(),
            ingestion_status: IngestionStatus::Ingested,
        };

        let path = datalake.write_repo_metadata(&metadata)?;
        let read_metadata = datalake.read_repo_metadata(&path)?;

        assert_eq!(metadata.url, read_metadata.url);
        assert_eq!(metadata.name, read_metadata.name);
        assert_eq!(metadata.stars, read_metadata.stars);

        Ok(())
    }

    #[test]
    fn test_datalake_write_read_training_run() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let datalake = TrainingDataLake::new(temp_dir.path())?;

        let run = TrainingRun::new(
            "https://github.com/test/repo".to_string(),
            "issue-1".to_string(),
            "Test issue".to_string(),
            vec!["src/main.rs".to_string()],
            vec!["src/main.rs".to_string()],
        );

        let path = datalake.write_training_run(&run)?;
        let read_run = datalake.read_training_run(&path)?;

        assert_eq!(run.run_id, read_run.run_id);
        assert_eq!(run.repo_url, read_run.repo_url);
        assert_eq!(run.issue_id, read_run.issue_id);

        Ok(())
    }

    #[test]
    fn test_datalake_list_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let datalake = TrainingDataLake::new(temp_dir.path())?;

        let metadata1 = RepoMetadata {
            url: "https://github.com/test/repo1".to_string(),
            owner: "test".to_string(),
            name: "repo1".to_string(),
            stars: 100,
            forks: 20,
            language: "Rust".to_string(),
            last_activity: None,
            issue_count: 5,
            pr_count: 2,
            added_at: Utc::now(),
            ingestion_status: IngestionStatus::Ingested,
        };

        let metadata2 = RepoMetadata {
            url: "https://github.com/test/repo2".to_string(),
            owner: "test".to_string(),
            name: "repo2".to_string(),
            stars: 50,
            forks: 10,
            language: "Python".to_string(),
            last_activity: None,
            issue_count: 3,
            pr_count: 1,
            added_at: Utc::now(),
            ingestion_status: IngestionStatus::Pending,
        };

        datalake.write_repo_metadata(&metadata1)?;
        datalake.write_repo_metadata(&metadata2)?;

        let all_metadata = datalake.list_repo_metadata()?;
        assert_eq!(all_metadata.len(), 2);

        Ok(())
    }
}
