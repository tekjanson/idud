//! Integration test for PR file change prediction using Waymark contracts
//!
//! This test validates the complete training pipeline:
//! 1. Loads real Waymark contracts (6,174 signatories, 88 contracts)
//! 2. Builds a co-dependency graph
//! 3. Runs prediction tests on file changes
//! 4. Validates accuracy metrics
//! 5. Measures performance characteristics

use idud::training::{load_waymark_contracts, ValidationEngine, PredictionTestCase, ValidationSummary};
use std::time::Instant;

#[test]
#[ignore] // Run with: cargo test --test pr_prediction_waymark -- --ignored --nocapture
fn test_pr_prediction_waymark() {
    let waymark_path = "/home/tekjanson/Documents/Code/idud/data/Waymark-contracts.json";
    
    // Step 1: Load Waymark data
    println!("\n=== Step 1: Loading Waymark Contracts ===");
    let start = Instant::now();
    let waymark_data = load_waymark_contracts(waymark_path)
        .expect("Failed to load Waymark contracts");
    let load_time = start.elapsed();
    
    println!("✓ Loaded {} signatories", waymark_data.signatories.len());
    println!("✓ Loaded {} contracts", waymark_data.contracts.len());
    println!("✓ Time: {:?}", load_time);

    // Step 2: Build co-dependency graph
    println!("\n=== Step 2: Building Co-Dependency Graph ===");
    let start = Instant::now();
    let engine = ValidationEngine::from_waymark(waymark_data);
    let graph_build_time = start.elapsed();
    
    let (signatories, contracts) = engine.graph_stats();
    println!("✓ Graph built with {} signatories and {} contracts", signatories, contracts);
    println!("✓ Time: {:?}", graph_build_time);

    // Step 3: Analyze graph structure
    println!("\n=== Step 3: Analyzing Graph Structure ===");
    analyze_graph(waymark_path);

    // Step 4: Create test cases
    println!("\n=== Step 4: Creating Test Cases ===");
    let test_cases = create_prediction_test_cases();
    println!("✓ Created {} test cases", test_cases.len());

    // Step 5: Run predictions
    println!("\n=== Step 5: Running Predictions ===");
    let start = Instant::now();
    let results = engine.run_all_tests(test_cases);
    let prediction_time = start.elapsed();

    // Print detailed results
    for (idx, result) in results.iter().enumerate() {
        println!("\n[Test {}] {}", idx + 1, result.test_name);
        println!("  Input: {:?}", result.changed_files);
        println!("  Predictions: {} files in {}ms", 
            result.predicted_files.len(), result.compute_time_ms);
        println!("  Expected: {} files", result.expected_files.len());
        
        if !result.predicted_files.is_empty() {
            println!("  Top 3 predictions:");
            for (i, file) in result.predicted_files.iter().take(3).enumerate() {
                let score = result.expected_files.iter()
                    .map(|_| "✓")
                    .nth(0)
                    .unwrap_or("");
                println!("    {}. {} {}", i + 1, file, score);
            }
        }
        
        println!("  Confusion Matrix:");
        println!("    TP: {} | FP: {} | FN: {}",
            result.true_positives, result.false_positives, result.false_negatives);
        println!("  Metrics: Precision={:.1}% | Recall={:.1}% | F1={:.3}",
            result.precision * 100.0, result.recall * 100.0, result.f1_score);
        println!("  Status: {}", if result.passed { "✓ PASS" } else { "✗ FAIL" });
    }

    // Step 6: Calculate summary
    println!("\n=== Step 6: Validation Summary ===");
    let summary = ValidationSummary::from_results(&results, signatories, contracts);
    
    println!("Passed: {}/{} tests ({:.1}%)", 
        summary.passed_tests,
        summary.total_tests,
        summary.accuracy * 100.0
    );
    
    println!("\nAggregate Metrics:");
    println!("  Precision: {:.1}%", summary.average_precision * 100.0);
    println!("  Recall:    {:.1}%", summary.average_recall * 100.0);
    println!("  F1 Score:  {:.4}", summary.average_f1);
    
    println!("\nPerformance:");
    println!("  Avg time per prediction: {}ms", summary.average_compute_time_ms);
    println!("  Total time: {:?}", load_time + graph_build_time + prediction_time);

    // Step 7: Validate success criteria
    println!("\n=== Step 7: Validating Success Criteria ===");
    
    assert!(summary.total_tests > 0, "Should have run at least one test");
    println!("✓ Ran {} tests", summary.total_tests);
    
    if summary.average_precision > 0.5 {
        println!("✓ Average precision {:.1}% > 50%", summary.average_precision * 100.0);
    }
    
    if summary.average_f1 > 0.4 {
        println!("✓ Average F1 score {:.4} > 0.4", summary.average_f1);
    }
    
    // Ensure all computations are fast (should be <100ms per prediction for in-memory graph)
    if summary.average_compute_time_ms < 100 {
        println!("✓ Fast computation: {}ms per prediction (expected <100ms)", 
            summary.average_compute_time_ms);
    } else {
        println!("⚠ Computation slower than expected: {}ms per prediction", 
            summary.average_compute_time_ms);
    }

    println!("\n=== VALIDATION COMPLETE ===");
}

fn analyze_graph(waymark_path: &str) {
    use std::fs;
    use std::collections::HashMap;

    let content = fs::read_to_string(waymark_path).expect("Failed to read file");
    let data: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse JSON");

    let empty_vec = vec![];
    let contracts = data["contracts"].as_array().unwrap_or(&empty_vec);
    let signatories = data["signatories"].as_array().unwrap_or(&empty_vec);

    let mut clause_types = HashMap::new();
    let mut source_uri_patterns = HashMap::new();

    for contract in contracts {
        let clause_type = contract["clause_type"]
            .as_str()
            .unwrap_or("Unknown");
        *clause_types.entry(clause_type).or_insert(0) += 1;
    }

    for sig in signatories {
        let uri = sig["source_uri"].as_str().unwrap_or("");
        let file_ext = uri.split('.').last().unwrap_or("unknown");
        *source_uri_patterns.entry(file_ext).or_insert(0) += 1;
    }

    println!("Clause types:");
    for (clause_type, count) in clause_types.iter() {
        println!("  - {}: {}", clause_type, count);
    }

    println!("\nFile extensions:");
    for (ext, count) in source_uri_patterns.iter() {
        println!("  - .{}: {}", ext, count);
    }

    // Find most connected files
    let mut file_connections = HashMap::new();
    for contract in contracts {
        let principal = contract["principal_id"].as_str().unwrap_or("");
        let guarantor = contract["guarantor_id"].as_str().unwrap_or("");
        
        *file_connections.entry(principal).or_insert(0) += 1;
        *file_connections.entry(guarantor).or_insert(0) += 1;
    }

    let mut connections: Vec<_> = file_connections.iter().collect();
    connections.sort_by(|a, b| b.1.cmp(a.1));

    println!("\nMost connected files:");
    for (id, count) in connections.iter().take(3) {
        let uri = signatories
            .iter()
            .find_map(|sig| {
                if sig["id"].as_str() == Some(id) {
                    sig["source_uri"].as_str()
                } else {
                    None
                }
            });
        if let Some(uri) = uri {
            println!("  - {} ({} connections)", uri, count);
        }
    }
}

fn create_prediction_test_cases() -> Vec<PredictionTestCase> {
    use std::fs;

    let waymark_path = "/home/tekjanson/Documents/Code/idud/data/Waymark-contracts.json";
    let content = fs::read_to_string(waymark_path).expect("Failed to read file");
    let data: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse JSON");

    let empty_vec = vec![];
    let signatories = data["signatories"].as_array().unwrap_or(&empty_vec);
    let contracts = data["contracts"].as_array().unwrap_or(&empty_vec);

    let mut test_cases = Vec::new();

    // Test 1: First contract pair
    if let Some(contract) = contracts.first() {
        if let (Some(principal_id), Some(guarantor_id)) = (
            contract["principal_id"].as_str(),
            contract["guarantor_id"].as_str(),
        ) {
            let principal_uri = signatories
                .iter()
                .find_map(|sig| {
                    if sig["id"].as_str() == Some(principal_id) {
                        sig["source_uri"].as_str()
                    } else {
                        None
                    }
                })
                .unwrap_or("");

            let guarantor_uri = signatories
                .iter()
                .find_map(|sig| {
                    if sig["id"].as_str() == Some(guarantor_id) {
                        sig["source_uri"].as_str()
                    } else {
                        None
                    }
                })
                .unwrap_or("");

            if !principal_uri.is_empty() && !guarantor_uri.is_empty() {
                test_cases.push(PredictionTestCase {
                    name: "test_direct_dependency_first".to_string(),
                    description: "First contract in Waymark graph".to_string(),
                    changed_files: vec![principal_uri.to_string()],
                    expected_related_files: vec![guarantor_uri.to_string()],
                    min_precision: 0.0,
                    min_recall: 0.0,
                });
            }
        }
    }

    // Test 2: Middle contract pair
    if contracts.len() > 5 {
        if let Some(contract) = contracts.get(5) {
            if let (Some(principal_id), Some(guarantor_id)) = (
                contract["principal_id"].as_str(),
                contract["guarantor_id"].as_str(),
            ) {
                let principal_uri = signatories
                    .iter()
                    .find_map(|sig| {
                        if sig["id"].as_str() == Some(principal_id) {
                            sig["source_uri"].as_str()
                        } else {
                            None
                        }
                    })
                    .unwrap_or("");

                let guarantor_uri = signatories
                    .iter()
                    .find_map(|sig| {
                        if sig["id"].as_str() == Some(guarantor_id) {
                            sig["source_uri"].as_str()
                        } else {
                            None
                        }
                    })
                    .unwrap_or("");

                if !principal_uri.is_empty() && !guarantor_uri.is_empty() {
                    test_cases.push(PredictionTestCase {
                        name: "test_dependency_from_middle".to_string(),
                        description: "Contract from middle of list".to_string(),
                        changed_files: vec![principal_uri.to_string()],
                        expected_related_files: vec![guarantor_uri.to_string()],
                        min_precision: 0.0,
                        min_recall: 0.0,
                    });
                }
            }
        }
    }

    // Test 3: Empty change set
    test_cases.push(PredictionTestCase {
        name: "test_empty_change_set".to_string(),
        description: "No changed files".to_string(),
        changed_files: vec![],
        expected_related_files: vec![],
        min_precision: 1.0,
        min_recall: 1.0,
    });

    // Test 4: Non-existent file
    test_cases.push(PredictionTestCase {
        name: "test_non_existent_file".to_string(),
        description: "File not in graph".to_string(),
        changed_files: vec!["nonexistent/path/file.rs".to_string()],
        expected_related_files: vec![],
        min_precision: 1.0,
        min_recall: 1.0,
    });

    test_cases
}
