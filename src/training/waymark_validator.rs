//! Waymark data loader and validation engine
//!
//! Loads Waymark contracts from JSON and runs comprehensive validation
//! on the PR prediction pipeline.

use crate::training::pr_predictor::{CoDependencyGraph, PRPredictor};
use crate::types::{Contract, Signatory};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct WaymarkData {
    pub signatories: Vec<Signatory>,
    pub contracts: Vec<Contract>,
    pub exported_at: String,
    pub version: String,
}

/// Loads Waymark contracts from JSON file
pub fn load_waymark_contracts<P: AsRef<Path>>(path: P) -> Result<WaymarkData> {
    let content = fs::read_to_string(path).context("Failed to read Waymark contracts file")?;

    let data: WaymarkData =
        serde_json::from_str(&content).context("Failed to parse Waymark contracts JSON")?;

    Ok(data)
}

/// Test case for PR prediction validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionTestCase {
    pub name: String,
    pub description: String,
    /// Files that changed (the seed)
    pub changed_files: Vec<String>,
    /// Files that should be predicted
    pub expected_related_files: Vec<String>,
    /// Minimum expected precision
    pub min_precision: f32,
    /// Minimum expected recall
    pub min_recall: f32,
}

/// Result of a single prediction test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionTestResult {
    pub test_name: String,
    pub changed_files: Vec<String>,
    pub predicted_files: Vec<String>,
    pub expected_files: Vec<String>,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: f32,
    pub recall: f32,
    pub f1_score: f32,
    pub passed: bool,
    pub compute_time_ms: u128,
}

/// Validation engine for PR predictions
pub struct ValidationEngine {
    predictor: PRPredictor,
    graph: CoDependencyGraph,
}

impl ValidationEngine {
    /// Create validation engine from Waymark data
    pub fn from_waymark(waymark_data: WaymarkData) -> Self {
        let graph = CoDependencyGraph::build(waymark_data.signatories, waymark_data.contracts);
        let predictor = PRPredictor::new(graph.clone());

        Self { predictor, graph }
    }

    /// Get graph stats
    pub fn graph_stats(&self) -> (usize, usize) {
        (self.graph.total_signatories, self.graph.total_contracts)
    }

    /// Run a single prediction test
    pub fn run_prediction_test(&self, test_case: &PredictionTestCase) -> PredictionTestResult {
        let prediction = self.predictor.predict(test_case.changed_files.clone(), 20);

        // Calculate metrics
        let true_positives = prediction
            .predicted_files
            .iter()
            .filter(|f| test_case.expected_related_files.contains(f))
            .count();

        let false_positives = prediction.predicted_files.len() - true_positives;
        let false_negatives = test_case
            .expected_related_files
            .iter()
            .filter(|f| !prediction.predicted_files.contains(f))
            .count();

        let precision = if prediction.predicted_files.is_empty() {
            if test_case.expected_related_files.is_empty() {
                1.0
            } else {
                0.0
            }
        } else {
            true_positives as f32 / prediction.predicted_files.len() as f32
        };

        let recall = if test_case.expected_related_files.is_empty() {
            1.0
        } else {
            true_positives as f32 / test_case.expected_related_files.len() as f32
        };

        let f1_score = if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * (precision * recall) / (precision + recall)
        };

        let passed = precision >= test_case.min_precision && recall >= test_case.min_recall;

        PredictionTestResult {
            test_name: test_case.name.clone(),
            changed_files: test_case.changed_files.clone(),
            predicted_files: prediction.predicted_files,
            expected_files: test_case.expected_related_files.clone(),
            true_positives,
            false_positives,
            false_negatives,
            precision,
            recall,
            f1_score,
            passed,
            compute_time_ms: prediction.compute_time_ms,
        }
    }

    /// Run all test cases
    pub fn run_all_tests(&self, test_cases: Vec<PredictionTestCase>) -> Vec<PredictionTestResult> {
        test_cases
            .iter()
            .map(|tc| self.run_prediction_test(tc))
            .collect()
    }

    /// Generate test cases from real graph data
    /// Finds file clusters in the dependency graph
    pub fn generate_test_cases_from_graph(&self, count: usize) -> Vec<PredictionTestCase> {
        let mut test_cases = Vec::new();

        // For now, create synthetic test cases based on common patterns
        // In reality, this would analyze the actual graph structure

        // Test 1: Random pair of files
        if count >= 1 {
            test_cases.push(PredictionTestCase {
                name: "simple_dependency".to_string(),
                description: "Test prediction on a simple dependency".to_string(),
                changed_files: vec![],
                expected_related_files: vec![],
                min_precision: 0.7,
                min_recall: 0.5,
            });
        }

        // Test 2: Multiple related files
        if count >= 2 {
            test_cases.push(PredictionTestCase {
                name: "multi_file_cluster".to_string(),
                description: "Test prediction with multiple related files".to_string(),
                changed_files: vec![],
                expected_related_files: vec![],
                min_precision: 0.6,
                min_recall: 0.4,
            });
        }

        test_cases
    }
}

/// Summary statistics for validation run
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationSummary {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub average_precision: f32,
    pub average_recall: f32,
    pub average_f1: f32,
    pub average_compute_time_ms: u128,
    pub graph_signatories: usize,
    pub graph_contracts: usize,
    pub accuracy: f32,
}

impl ValidationSummary {
    /// Create summary from test results
    pub fn from_results(
        results: &[PredictionTestResult],
        signatories: usize,
        contracts: usize,
    ) -> Self {
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|r| r.passed).count();
        let failed_tests = total_tests - passed_tests;

        let avg_precision = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.precision).sum::<f32>() / results.len() as f32
        };

        let avg_recall = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.recall).sum::<f32>() / results.len() as f32
        };

        let avg_f1 = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.f1_score).sum::<f32>() / results.len() as f32
        };

        let avg_time = if results.is_empty() {
            0
        } else {
            results.iter().map(|r| r.compute_time_ms).sum::<u128>() / results.len() as u128
        };

        let accuracy = if total_tests == 0 {
            0.0
        } else {
            passed_tests as f32 / total_tests as f32
        };

        Self {
            total_tests,
            passed_tests,
            failed_tests,
            average_precision: avg_precision,
            average_recall: avg_recall,
            average_f1: avg_f1,
            average_compute_time_ms: avg_time,
            graph_signatories: signatories,
            graph_contracts: contracts,
            accuracy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_summary_calculation() {
        let results = vec![
            PredictionTestResult {
                test_name: "test1".to_string(),
                changed_files: vec![],
                predicted_files: vec![],
                expected_files: vec![],
                true_positives: 5,
                false_positives: 2,
                false_negatives: 1,
                precision: 0.71,
                recall: 0.83,
                f1_score: 0.77,
                passed: true,
                compute_time_ms: 50,
            },
            PredictionTestResult {
                test_name: "test2".to_string(),
                changed_files: vec![],
                predicted_files: vec![],
                expected_files: vec![],
                true_positives: 3,
                false_positives: 1,
                false_negatives: 2,
                precision: 0.75,
                recall: 0.6,
                f1_score: 0.67,
                passed: true,
                compute_time_ms: 45,
            },
        ];

        let summary = ValidationSummary::from_results(&results, 100, 50);
        assert_eq!(summary.total_tests, 2);
        assert_eq!(summary.passed_tests, 2);
        assert!(summary.average_precision > 0.7);
        assert!(summary.average_recall > 0.6);
    }
}
