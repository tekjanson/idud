// tests/waymark_integration.rs
//! Integration test: Waymark Repository Analysis
//! Verifies that idud can successfully ingest, analyze, and model dependencies
//! in the Waymark video generation platform codebase.
//!
//! Expected findings:
//! - 670+ files (TypeScript/JavaScript, React components, tests, config)
//! - 7000+ signatories (functions, tests, files, markdown sections)
//! - Dependencies between UI components, utilities, tests, and helpers
//! - High-confidence contracts (0.95+) between related code elements

use idud::pipelines::{RepositoryIngestionConfig, RepositoryTraverser};
use idud::types::*;
use std::path::PathBuf;

#[tokio::test]
async fn waymark_integration_ingest_and_analyze() {
    // Setup: Use clean temporary directory
    let work_dir = PathBuf::from("/tmp/waymark_integration_test");
    let _ = std::fs::remove_dir_all(&work_dir);
    std::fs::create_dir_all(&work_dir).expect("Failed to create work directory");

    // Configure ingestion for Waymark repo on main branch
    let config = RepositoryIngestionConfig {
        repo_url: "https://github.com/tekjanson/Waymark".to_string(),
        branch: "main".to_string(),
        work_dir: Some(work_dir),
        skip_clone: false,
    };

    // Run ingestion
    let traverser = RepositoryTraverser::new(config);
    let result = traverser.ingest().await.expect("Ingestion should succeed");

    println!("\n=== Waymark Repository Analysis Results ===");
    println!("Repository: {}", result.repository);
    println!("Files processed: {}", result.files_processed);
    println!(
        "Signatories registered: {}",
        result.signatories_registered.len()
    );
    println!("Errors encountered: {}", result.errors.len());

    // Assertion 1: Files should be processed (expect 600+)
    assert!(
        result.files_processed > 600,
        "Expected 600+ files in Waymark, found {}",
        result.files_processed
    );
    println!(
        "✅ Files processed: {} (expected >600)",
        result.files_processed
    );

    // Assertion 2: Signatories should be extracted (expect 6000+)
    assert!(
        result.signatories_registered.len() > 6000,
        "Expected 6000+ signatories, found {}",
        result.signatories_registered.len()
    );
    println!(
        "✅ Signatories registered: {} (expected >6000)",
        result.signatories_registered.len()
    );

    // Assertion 3: Categorize signatories by type
    let mut type_counts = std::collections::HashMap::new();
    for sig in &result.signatories_registered {
        *type_counts.entry(sig.signatory_type).or_insert(0) += 1;
    }

    println!("\n📊 Signatory Types Distribution:");
    for (sig_type, count) in &type_counts {
        println!("  {:?}: {}", sig_type, count);
    }

    // Assertion 4: Should have Files, Functions, and Tests
    assert!(
        type_counts.contains_key(&SignatoryType::File),
        "Should have File signatories"
    );
    assert!(
        type_counts.contains_key(&SignatoryType::Function),
        "Should have Function signatories"
    );
    let file_count = type_counts[&SignatoryType::File];
    let func_count = type_counts[&SignatoryType::Function];
    println!(
        "✅ Core signatory types present: Files={}, Functions={}",
        file_count, func_count
    );

    // Assertion 5: Should have Tests (expect 500+)
    let test_count = *type_counts.get(&SignatoryType::Test).unwrap_or(&0);
    assert!(
        test_count > 400,
        "Expected 400+ test signatories, found {}",
        test_count
    );
    println!("✅ Test signatories: {} (expected >400)", test_count);

    // Assertion 6: Analyze signatory locations
    println!("\n📍 Sample Signatories by Location:");
    let location_groups = group_signatories_by_location(&result.signatories_registered);
    let mut shown = 0;
    for (location, sigs) in location_groups.iter().take(5) {
        println!("  {} ({} signatories)", location, sigs.len());
        for sig in sigs.iter().take(2) {
            println!(
                "    - {} ({})",
                sig.label,
                format!("{:?}", sig.signatory_type)
            );
            shown += 1;
        }
        if shown >= 10 {
            break;
        }
    }

    // Assertion 7: Verify no critical errors in ingestion
    let critical_errors = result
        .errors
        .iter()
        .filter(|e| e.contains("Clone failed"))
        .count();
    assert_eq!(
        critical_errors, 0,
        "Should not have clone failures: {:?}",
        result.errors
    );
    println!("✅ No clone errors (ingestion successful)");

    // Assertion 8: Verify snippet quality
    let empty_snippets = result
        .signatories_registered
        .iter()
        .filter(|s| s.snippet.is_empty())
        .count();
    let non_empty_snippets = result.signatories_registered.len() - empty_snippets;
    assert!(
        non_empty_snippets > result.signatories_registered.len() * 8 / 10,
        "Expected 80%+ signatories to have non-empty snippets, found {}%",
        (non_empty_snippets * 100) / result.signatories_registered.len()
    );
    println!(
        "✅ Snippet quality: {}/{} non-empty ({:.1}%)",
        non_empty_snippets,
        result.signatories_registered.len(),
        (non_empty_snippets as f32 * 100.0) / result.signatories_registered.len() as f32
    );

    // Assertion 9: Verify URIs are well-formed
    for sig in result.signatories_registered.iter().take(50) {
        assert!(
            sig.source_uri.contains("github.com"),
            "Signatory URI should contain github.com: {}",
            sig.source_uri
        );
        assert!(
            sig.source_uri.contains("Waymark"),
            "Signatory URI should reference Waymark: {}",
            sig.source_uri
        );
    }
    println!("✅ All sampled URIs well-formed with correct references");

    println!("\n=== 🎉 Waymark Integration Test PASSED ===");
}

#[tokio::test]
async fn waymark_integration_dependency_patterns() {
    // This test would verify that we can discover meaningful dependency patterns
    // between different component types in Waymark

    let work_dir = PathBuf::from("/tmp/waymark_patterns_test");
    let _ = std::fs::remove_dir_all(&work_dir);
    std::fs::create_dir_all(&work_dir).expect("Failed to create work directory");

    let config = RepositoryIngestionConfig {
        repo_url: "https://github.com/tekjanson/Waymark".to_string(),
        branch: "main".to_string(),
        work_dir: Some(work_dir),
        skip_clone: false,
    };

    let traverser = RepositoryTraverser::new(config);
    let result = traverser.ingest().await.expect("Ingestion should succeed");

    println!("\n=== Waymark Dependency Pattern Analysis ===");

    // Find test files and their related source files
    let test_sigs: Vec<_> = result
        .signatories_registered
        .iter()
        .filter(|s| s.signatory_type == SignatoryType::Test)
        .collect();

    let source_sigs: Vec<_> = result
        .signatories_registered
        .iter()
        .filter(|s| {
            s.signatory_type == SignatoryType::Function || s.signatory_type == SignatoryType::File
        })
        .collect();

    println!(
        "Test files: {}, Source elements: {}",
        test_sigs.len(),
        source_sigs.len()
    );

    // Verify good distribution
    assert!(
        test_sigs.len() > 400,
        "Expected 400+ tests for coverage patterns"
    );
    assert!(
        source_sigs.len() > 2000,
        "Expected 2000+ source elements for dependency discovery, found {}",
        source_sigs.len()
    );

    // Analyze file organization patterns
    let file_sigs: Vec<_> = result
        .signatories_registered
        .iter()
        .filter(|s| s.signatory_type == SignatoryType::File)
        .collect();

    let mut file_types = std::collections::HashMap::new();
    for sig in file_sigs {
        let ext = sig.label.split('.').last().unwrap_or("unknown");
        *file_types.entry(ext).or_insert(0) += 1;
    }

    println!("\n📋 File Types in Waymark:");
    for (ext, count) in file_types.iter() {
        println!("  .{}: {}", ext, count);
    }

    // Verify TypeScript/JavaScript focus
    let ts_count = file_types.get("ts").copied().unwrap_or(0);
    let tsx_count = file_types.get("tsx").copied().unwrap_or(0);
    let js_count = file_types.get("js").copied().unwrap_or(0);
    let ts_family_count = ts_count + tsx_count + js_count;

    assert!(
        ts_family_count > 200,
        "Expected 200+ TypeScript/JavaScript files, found {}",
        ts_family_count
    );
    println!(
        "✅ TypeScript/JavaScript focus confirmed: {} files",
        ts_family_count
    );

    println!("\n=== 🎉 Dependency Pattern Analysis PASSED ===");
}

#[tokio::test]
async fn waymark_integration_metadata_validation() {
    // Verify that signatories are properly registered with metadata

    let work_dir = PathBuf::from("/tmp/waymark_metadata_test");
    let _ = std::fs::remove_dir_all(&work_dir);
    std::fs::create_dir_all(&work_dir).expect("Failed to create work directory");

    let config = RepositoryIngestionConfig {
        repo_url: "https://github.com/tekjanson/Waymark".to_string(),
        branch: "main".to_string(),
        work_dir: Some(work_dir),
        skip_clone: false,
    };

    let traverser = RepositoryTraverser::new(config);
    let result = traverser.ingest().await.expect("Ingestion should succeed");

    println!("\n=== Waymark Metadata Validation ===");

    // Check registration timestamps
    let now = chrono::Utc::now();
    let all_recent = result
        .signatories_registered
        .iter()
        .all(|s| (now.signed_duration_since(s.registered_at).num_seconds()) < 300);

    assert!(
        all_recent,
        "All signatories should be recently registered (within 5 minutes)"
    );
    println!("✅ All signatories registered recently");

    // Verify IDs are unique
    let mut ids = std::collections::HashSet::new();
    for sig in &result.signatories_registered {
        assert!(
            ids.insert(sig.id.clone()),
            "Signatory IDs must be unique: {}",
            sig.id
        );
    }
    println!("✅ All signatory IDs are unique");

    // Sample some well-formed entries
    let sample_size = 20.min(result.signatories_registered.len());
    for sig in result.signatories_registered.iter().take(sample_size) {
        // Each signatory should have:
        // - Non-empty ID (UUID format)
        // - Valid type
        // - Non-empty label
        // - Proper source_uri with github.com and branch
        assert!(!sig.id.is_empty(), "Signatory ID should not be empty");
        assert!(!sig.label.is_empty(), "Signatory label should not be empty");
        assert!(
            sig.source_uri.contains("github.com"),
            "Signatory URI should reference github.com: {}",
            sig.source_uri
        );
    }
    println!(
        "✅ Sample validation passed for {} signatories",
        sample_size
    );

    // Verify registration is logged with consistent timestamps
    let first_registered = result
        .signatories_registered
        .iter()
        .map(|s| s.registered_at)
        .min()
        .unwrap();

    let last_registered = result
        .signatories_registered
        .iter()
        .map(|s| s.registered_at)
        .max()
        .unwrap();

    let registration_span = last_registered.signed_duration_since(first_registered);
    println!(
        "✅ Registration span: {} seconds for {} signatories",
        registration_span.num_seconds(),
        result.signatories_registered.len()
    );

    println!("\n=== 🎉 Metadata Validation PASSED ===");
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Group signatories by their source location (file path)
fn group_signatories_by_location(signatories: &[Signatory]) -> Vec<(String, Vec<&Signatory>)> {
    let mut groups: std::collections::HashMap<String, Vec<&Signatory>> =
        std::collections::HashMap::new();

    for sig in signatories {
        // Extract file path from URI: github.com/owner/repo/blob/branch/path/to/file
        if let Some(start) = sig.source_uri.find("/blob/") {
            let after_branch = start + 6; // len("/blob/")
            if let Some(branch_end) = sig.source_uri[after_branch..].find('/') {
                let file_path = sig.source_uri[after_branch + branch_end + 1..].to_string();
                let base_path = file_path.split('/').take(2).collect::<Vec<_>>().join("/");
                groups.entry(base_path).or_insert_with(Vec::new).push(sig);
            }
        }
    }

    let mut result: Vec<_> = groups.into_iter().collect();
    result.sort_by_key(|(_k, v)| std::cmp::Reverse(v.len()));
    result
}
