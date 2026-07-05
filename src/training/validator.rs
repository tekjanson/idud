//! Prediction validation engine for training pipeline
//!
//! Validates predictions against ground truth data, calculates metrics,
//! and aggregates performance trends over time.

use crate::training_datalake::{TrainingRun, TrainingDataLake, AggregatedMetrics, TimeWindow, Checkpoint, PercentileMetrics};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Validation metrics for a single prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub precision: f32,
    pub recall: f32,
    pub f1: f32,
    pub true_positives: u32,
    pub false_positives: u32,
    pub false_negatives: u32,
}

/// Validates a single prediction against ground truth.
///
/// Calculates precision, recall, F1 score, and confusion matrix components.
///
/// # Arguments
/// * `predicted_files` - Files predicted by the model
/// * `actual_files` - Files that actually changed (ground truth)
///
/// # Returns
/// Validation metrics including precision, recall, F1, and confusion matrix
pub fn validate_prediction(
    predicted_files: Vec<String>,
    actual_files: Vec<String>,
) -> ValidationMetrics {
    let tp = predicted_files
        .iter()
        .filter(|p| actual_files.contains(p))
        .count() as u32;
    
    let fp = (predicted_files.len() as u32).saturating_sub(tp);
    let fn_count = (actual_files.len() as u32).saturating_sub(tp);
    
    // Precision: TP / (TP + FP) - of files we predicted, how many were right?
    let precision = if predicted_files.is_empty() {
        1.0
    } else {
        tp as f32 / predicted_files.len() as f32
    };
    
    // Recall: TP / (TP + FN) - of actual files that changed, how many did we predict?
    let recall = if actual_files.is_empty() {
        1.0
    } else {
        tp as f32 / actual_files.len() as f32
    };
    
    // F1: 2 * (precision * recall) / (precision + recall) - harmonic mean
    let f1 = if (precision + recall) == 0.0 {
        0.0
    } else {
        2.0 * (precision * recall) / (precision + recall)
    };
    
    ValidationMetrics {
        precision,
        recall,
        f1,
        true_positives: tp,
        false_positives: fp,
        false_negatives: fn_count,
    }
}

/// Writes a training result to the training datalake.
///
/// Persists the prediction, ground truth, and calculated metrics to disk
/// for historical analysis and trend tracking.
///
/// # Arguments
/// * `datalake` - The training data lake instance
/// * `repo_url` - Repository URL
/// * `issue_id` - Issue identifier
/// * `issue_text` - Issue description text
/// * `predicted_files` - Files predicted by model
/// * `actual_files` - Actual files changed
///
/// # Returns
/// UUID of the stored training run
pub fn write_training_result(
    datalake: &TrainingDataLake,
    repo_url: String,
    issue_id: String,
    issue_text: String,
    predicted_files: Vec<String>,
    actual_files: Vec<String>,
) -> Result<Uuid> {
    let training_run = TrainingRun::new(
        repo_url,
        issue_id,
        issue_text,
        predicted_files,
        actual_files,
    );
    
    let run_id = training_run.run_id;
    datalake.write_training_run(&training_run)
        .context("Failed to write training run to datalake")?;
    
    Ok(run_id)
}

/// Calculates aggregated metrics from all training results.
///
/// Analyzes performance trends across all stored training runs,
/// computing averages, percentiles, and improvement trends.
///
/// # Arguments
/// * `datalake` - The training data lake instance
///
/// # Returns
/// Aggregated metrics with trend analysis
pub fn calculate_aggregate_metrics(datalake: &TrainingDataLake) -> Result<AggregatedMetrics> {
    let runs = datalake.list_training_runs()
        .context("Failed to list training runs")?;
    
    if runs.is_empty() {
        return Ok(AggregatedMetrics {
            metric_id: Uuid::new_v4(),
            generated_at: Utc::now(),
            period: "no_data".to_string(),
            time_window: TimeWindow {
                start: Utc::now(),
                end: Utc::now(),
            },
            total_repos: 0,
            total_predictions: 0,
            avg_precision: 0.0,
            avg_recall: 0.0,
            avg_f1: 0.0,
            improvement_over_time: Vec::new(),
            percentiles: None,
        });
    }
    
    let total_predictions = runs.len() as u32;
    let unique_repos = runs
        .iter()
        .map(|r| r.repo_url.clone())
        .collect::<std::collections::HashSet<_>>()
        .len() as u32;
    
    // Calculate averages
    let avg_precision = runs.iter().map(|r| r.precision).sum::<f64>() / runs.len() as f64;
    let avg_recall = runs.iter().map(|r| r.recall).sum::<f64>() / runs.len() as f64;
    let avg_f1 = runs.iter().map(|r| r.f1).sum::<f64>() / runs.len() as f64;
    
    // Calculate percentiles
    let mut f1_scores: Vec<f64> = runs.iter().map(|r| r.f1).collect();
    f1_scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let percentiles = calculate_percentiles(&f1_scores);
    
    // Calculate improvement over time (group by day)
    let improvement_over_time = calculate_improvement_checkpoints(&runs);
    
    // Determine time window
    let start = runs.iter().map(|r| r.timestamp).min().unwrap_or_else(Utc::now);
    let end = runs.iter().map(|r| r.timestamp).max().unwrap_or_else(Utc::now);
    
    Ok(AggregatedMetrics {
        metric_id: Uuid::new_v4(),
        generated_at: Utc::now(),
        period: format!("{}_{}", start.format("%Y%m%d"), end.format("%Y%m%d")),
        time_window: TimeWindow { start, end },
        total_repos: unique_repos,
        total_predictions,
        avg_precision,
        avg_recall,
        avg_f1,
        improvement_over_time,
        percentiles: Some(percentiles),
    })
}

/// Calculates aggregated metrics grouped by language.
///
/// Provides per-language performance analysis for understanding
/// prediction accuracy across different programming languages.
///
/// # Arguments
/// * `datalake` - The training data lake instance
///
/// # Returns
/// HashMap mapping language to aggregated metrics
pub fn calculate_metrics_by_language(
    datalake: &TrainingDataLake,
) -> Result<HashMap<String, LanguageMetrics>> {
    let repos = datalake.list_repo_metadata()
        .context("Failed to list repo metadata")?;
    let runs = datalake.list_training_runs()
        .context("Failed to list training runs")?;
    
    // Build language -> repo_urls map
    let mut language_repos: HashMap<String, Vec<String>> = HashMap::new();
    for repo in repos {
        language_repos
            .entry(repo.language.clone())
            .or_insert_with(Vec::new)
            .push(repo.url.clone());
    }
    
    // Group runs by language
    let mut metrics_by_lang: HashMap<String, LanguageMetrics> = HashMap::new();
    
    for run in runs {
        // Find language for this repo
        for (lang, repo_urls) in &language_repos {
            if repo_urls.contains(&run.repo_url) {
                let metrics = metrics_by_lang
                    .entry(lang.clone())
                    .or_insert_with(|| LanguageMetrics {
                        language: lang.clone(),
                        repo_count: 0,
                        prediction_count: 0,
                        total_precision: 0.0,
                        total_recall: 0.0,
                        total_f1: 0.0,
                        avg_precision: 0.0,
                        avg_recall: 0.0,
                        avg_f1: 0.0,
                    });
                metrics.prediction_count += 1;
                metrics.total_precision += run.precision;
                metrics.total_recall += run.recall;
                metrics.total_f1 += run.f1;
                break;
            }
        }
    }
    
    // Calculate unique repo counts and finalize averages
    for (lang, repo_urls) in &language_repos {
        if let Some(metrics) = metrics_by_lang.get_mut(lang) {
            metrics.repo_count = repo_urls.len() as u32;
            if metrics.prediction_count > 0 {
                metrics.avg_precision = metrics.total_precision / metrics.prediction_count as f64;
                metrics.avg_recall = metrics.total_recall / metrics.prediction_count as f64;
                metrics.avg_f1 = metrics.total_f1 / metrics.prediction_count as f64;
            }
        }
    }
    
    Ok(metrics_by_lang)
}

/// Per-language performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMetrics {
    pub language: String,
    pub repo_count: u32,
    pub prediction_count: u32,
    pub avg_precision: f64,
    pub avg_recall: f64,
    pub avg_f1: f64,
    #[serde(skip)]
    total_precision: f64,
    #[serde(skip)]
    total_recall: f64,
    #[serde(skip)]
    total_f1: f64,
}

/// Calculates F1 score percentiles.
fn calculate_percentiles(sorted_f1_scores: &[f64]) -> PercentileMetrics {
    if sorted_f1_scores.is_empty() {
        return PercentileMetrics {
            p50_f1: 0.0,
            p75_f1: 0.0,
            p90_f1: 0.0,
            p95_f1: 0.0,
        };
    }
    
    let len = sorted_f1_scores.len() as f64;
    let p50_idx = ((len * 0.5) as usize).min(sorted_f1_scores.len() - 1);
    let p75_idx = ((len * 0.75) as usize).min(sorted_f1_scores.len() - 1);
    let p90_idx = ((len * 0.90) as usize).min(sorted_f1_scores.len() - 1);
    let p95_idx = ((len * 0.95) as usize).min(sorted_f1_scores.len() - 1);
    
    PercentileMetrics {
        p50_f1: sorted_f1_scores[p50_idx],
        p75_f1: sorted_f1_scores[p75_idx],
        p90_f1: sorted_f1_scores[p90_idx],
        p95_f1: sorted_f1_scores[p95_idx],
    }
}

/// Calculates improvement checkpoints over time.
fn calculate_improvement_checkpoints(runs: &[TrainingRun]) -> Vec<Checkpoint> {
    if runs.is_empty() {
        return Vec::new();
    }
    
    // Group runs by day
    let mut daily_runs: HashMap<String, Vec<&TrainingRun>> = HashMap::new();
    for run in runs {
        let day_key = run.timestamp.format("%Y-%m-%d").to_string();
        daily_runs.entry(day_key).or_insert_with(Vec::new).push(run);
    }
    
    // Calculate F1 average per day
    let mut checkpoints: Vec<Checkpoint> = daily_runs
        .into_iter()
        .map(|(day_key, day_runs)| {
            let avg_f1 = day_runs.iter().map(|r| r.f1).sum::<f64>() / day_runs.len() as f64;
            let checkpoint_date = chrono::NaiveDate::parse_from_str(&day_key, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(12, 0, 0))
                .map(|dt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or_else(Utc::now);
            Checkpoint {
                checkpoint: checkpoint_date,
                avg_f1,
            }
        })
        .collect();
    
    // Sort by checkpoint date
    checkpoints.sort_by_key(|cp| cp.checkpoint);
    
    checkpoints
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_prediction_perfect_match() {
        let predicted = vec!["a.rs".to_string(), "b.rs".to_string()];
        let actual = vec!["a.rs".to_string(), "b.rs".to_string()];
        
        let metrics = validate_prediction(predicted, actual);
        assert_eq!(metrics.precision, 1.0);
        assert_eq!(metrics.recall, 1.0);
        assert_eq!(metrics.f1, 1.0);
        assert_eq!(metrics.true_positives, 2);
        assert_eq!(metrics.false_positives, 0);
        assert_eq!(metrics.false_negatives, 0);
    }
    
    #[test]
    fn test_validate_prediction_partial_overlap() {
        let predicted = vec!["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()];
        let actual = vec!["a.rs".to_string(), "b.rs".to_string()];
        
        let metrics = validate_prediction(predicted, actual);
        assert_eq!(metrics.true_positives, 2);
        assert_eq!(metrics.false_positives, 1);
        assert_eq!(metrics.false_negatives, 0);
        assert!((metrics.precision - (2.0 / 3.0)).abs() < 0.001);
        assert_eq!(metrics.recall, 1.0);
    }
    
    #[test]
    fn test_validate_prediction_no_overlap() {
        let predicted = vec!["x.rs".to_string(), "y.rs".to_string()];
        let actual = vec!["a.rs".to_string(), "b.rs".to_string()];
        
        let metrics = validate_prediction(predicted, actual);
        assert_eq!(metrics.precision, 0.0);
        assert_eq!(metrics.recall, 0.0);
        assert_eq!(metrics.f1, 0.0);
    }
    
    #[test]
    fn test_validate_prediction_empty_predictions() {
        let predicted = vec![];
        let actual = vec!["a.rs".to_string()];
        
        let metrics = validate_prediction(predicted, actual);
        assert_eq!(metrics.precision, 1.0);
        assert_eq!(metrics.recall, 0.0);
        assert_eq!(metrics.f1, 0.0);
    }
    
    #[test]
    fn test_validate_prediction_empty_actual() {
        let predicted = vec!["a.rs".to_string()];
        let actual = vec![];
        
        let metrics = validate_prediction(predicted, actual);
        assert_eq!(metrics.precision, 0.0);
        assert_eq!(metrics.recall, 1.0);
        assert_eq!(metrics.f1, 0.0);
    }
    
    #[test]
    fn test_calculate_percentiles() {
        let scores = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        let percentiles = calculate_percentiles(&scores);
        assert!(percentiles.p50_f1 > 0.0);
        assert!(percentiles.p75_f1 >= percentiles.p50_f1);
        assert!(percentiles.p90_f1 >= percentiles.p75_f1);
        assert!(percentiles.p95_f1 >= percentiles.p90_f1);
    }
}
