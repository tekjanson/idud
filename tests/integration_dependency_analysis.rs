//! Integration test for AST + AI dependency analysis and contract merging
//!
//! Tests the full pipeline:
//! 1. Repository ingestion with signatory registration
//! 2. AST analysis to extract dependencies
//! 3. Contract merger to deduplicate and assign confidence
//! 4. Validation that graph has edges (contracts)

use idud::analysis::ASTAnalyzer;
use tempfile::TempDir;
use walkdir::WalkDir;

#[tokio::test]
async fn test_dependency_analysis_pipeline() {
    use idud::analysis::ASTAnalyzer;
    use idud::types::{Signatory, SignatoryType};

    // Create a temporary directory with test files
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp directory");
    let repo_path = temp_dir.path();

    // Create test files
    create_test_rust_file(repo_path, "src/main.rs");
    create_test_rust_file(repo_path, "src/lib.rs");

    // Manually analyze the files (since we can't use the traverser with local paths)
    let mut signatories = vec![];
    let mut all_ast_deps = vec![];

    for entry in walkdir::WalkDir::new(repo_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        if let Ok(content) = std::fs::read_to_string(path) {
            // Register signatory
            let rel_path = path.strip_prefix(repo_path).unwrap();
            let rel_str = rel_path.to_string_lossy().to_string();

            let sig = Signatory::new(
                SignatoryType::File,
                format!("file://{}", rel_str),
                rel_str.clone(),
                content.clone(),
            );
            signatories.push(sig);

            // Analyze for dependencies
            if rel_str.ends_with(".rs") {
                let deps = ASTAnalyzer::analyze_rust_file(&format!("file://{}", rel_str), &content);
                all_ast_deps.extend(deps);
            }
        }
    }

    // Validate signatories were registered
    assert!(!signatories.is_empty(), "Should register signatories");
    println!("✓ Registered {} signatories", signatories.len());

    // Merge dependencies into contracts
    let contracts = idud::analysis::ContractMerger::merge_dependencies(
        all_ast_deps,
        vec![],
        &signatories,
    )
    .expect("Merge should succeed");

    println!("✓ Discovered {} contracts", contracts.len());

    // Validate contract integrity
    for contract in &contracts {
        assert!(contract.confidence > 0.0, "Contract confidence should be > 0");
        assert!(contract.confidence <= 1.0, "Contract confidence should be <= 1");
        assert!(
            !contract.principal_id.is_empty(),
            "Principal ID should not be empty"
        );
        assert!(
            !contract.guarantor_id.is_empty(),
            "Guarantor ID should not be empty"
        );
    }
    println!("✓ All contracts have valid confidence scores");

    // Calculate coverage
    let signatory_ids: std::collections::HashSet<_> =
        signatories.iter().map(|s| s.id.clone()).collect();

    let dependent_signatories: std::collections::HashSet<_> = contracts
        .iter()
        .flat_map(|c| vec![c.principal_id.clone(), c.guarantor_id.clone()])
        .filter(|id| signatory_ids.contains(id))
        .collect();

    let coverage_pct =
        (dependent_signatories.len() as f32 / signatory_ids.len() as f32) * 100.0;
    println!(
        "✓ Contract coverage: {:.1}% of signatories have dependencies",
        coverage_pct
    );

    // Print summary
    println!(
        "\n📊 Integration Test Summary:\n   {} signatories\n   {} contracts extracted\n   {:.1}% coverage",
        signatories.len(),
        contracts.len(),
        coverage_pct
    );
}

#[test]
fn test_contract_merger_deduplication() {
    use idud::analysis::ContractMerger;
    use idud::types::{Signatory, SignatoryType, Contract, ContractSource, ClauseType};

    // Create test dependencies from AST
    let ast_deps = vec![
        idud::analysis::Dependency::new(
            "file://a.rs".to_string(),
            "file://b.rs".to_string(),
            "import".to_string(),
            0.95,
        ),
    ];

    // Create test contracts from AI (duplicate + unique)
    let ai_contracts = vec![
        // This one is a duplicate of AST dep - should be skipped
        Contract::new(
            "sig_a".to_string(),
            "sig_b".to_string(),
            ClauseType::Uses,
            0.6,
            ContractSource::AiInferred,
        ),
        // This one is unique - should be included
        Contract::new(
            "sig_b".to_string(),
            "sig_c".to_string(),
            ClauseType::Uses,
            0.7,
            ContractSource::AiInferred,
        ),
    ];

    // Create signatories
    let mut signatories = vec![
        Signatory::new(
            SignatoryType::File,
            "file://a.rs".to_string(),
            "a.rs".to_string(),
            "content".to_string(),
        ),
        Signatory::new(
            SignatoryType::File,
            "file://b.rs".to_string(),
            "b.rs".to_string(),
            "content".to_string(),
        ),
        Signatory::new(
            SignatoryType::File,
            "file://c.rs".to_string(),
            "c.rs".to_string(),
            "content".to_string(),
        ),
    ];

    // Manually set IDs to match contracts
    signatories[0].id = "sig_a".to_string();
    signatories[1].id = "sig_b".to_string();
    signatories[2].id = "sig_c".to_string();

    let merged = ContractMerger::merge_dependencies(ast_deps, ai_contracts, &signatories)
        .expect("Merge should succeed");

    // Should have 2 contracts: 1 AST (replaces AI duplicate) + 1 unique AI
    assert_eq!(merged.len(), 2, "Should merge to 2 contracts");

    // Verify AST version took precedence
    let ast_contract = merged
        .iter()
        .find(|c| c.principal_id == "sig_a" && c.guarantor_id == "sig_b")
        .expect("Should have AST contract");
    assert_eq!(ast_contract.discovered_by, ContractSource::Deterministic);
    assert!(ast_contract.confidence >= 0.90);

    // Verify unique AI contract is included
    let ai_contract = merged
        .iter()
        .find(|c| c.principal_id == "sig_b" && c.guarantor_id == "sig_c")
        .expect("Should have AI contract");
    assert_eq!(ai_contract.discovered_by, ContractSource::AiInferred);
    assert!(ai_contract.confidence <= 0.70);

    println!("✓ Contract merger correctly deduplicates and prioritizes AST");
}

#[test]
fn test_ast_analyzer_confidence_scores() {
    use idud::analysis::ASTAnalyzer;

    // Test Rust import detection
    let rust_code = r#"
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};
    "#;

    let deps = ASTAnalyzer::analyze_rust_file("test.rs", rust_code);
    assert!(!deps.is_empty());

    // Imports should have high confidence
    let import_deps: Vec<_> = deps
        .iter()
        .filter(|d| d.dep_type == "import")
        .collect();
    assert!(!import_deps.is_empty());

    for dep in import_deps {
        assert!(
            dep.confidence >= 0.90,
            "Import confidence should be >= 0.90, got {}",
            dep.confidence
        );
    }

    println!("✓ AST analyzer assigns correct confidence to imports");
}

#[test]
fn test_graph_has_edges_not_isolated_nodes() {
    use idud::{ContractLedger, Signatory, SignatoryType, Contract, ClauseType, ContractSource};
    use std::sync::Arc;

    let ledger = Arc::new(ContractLedger::new());

    // Register signatories
    let sig1 = Signatory::new(
        SignatoryType::File,
        "file://a.rs".to_string(),
        "a.rs".to_string(),
        "content".to_string(),
    );
    let sig2 = Signatory::new(
        SignatoryType::File,
        "file://b.rs".to_string(),
        "b.rs".to_string(),
        "content".to_string(),
    );

    let id1 = ledger.register_signatory(sig1).expect("Should register sig1");
    let id2 = ledger.register_signatory(sig2).expect("Should register sig2");

    // Create contract
    let contract = Contract::new(
        id1.clone(),
        id2.clone(),
        ClauseType::Requires,
        0.95,
        ContractSource::Deterministic,
    );

    ledger
        .draft_contract(contract)
        .expect("Should draft contract");

    // Verify graph has edges
    let all_contracts = ledger.get_all_contracts();
    assert!(
        !all_contracts.is_empty(),
        "Graph should have at least one contract (edge)"
    );
    assert_eq!(all_contracts.len(), 1);

    println!("✓ Graph contains edges, not just isolated nodes");
}

// Helper: Create a test Rust file with imports
fn create_test_rust_file(repo_path: &std::path::Path, rel_path: &str) {
    let file_path = repo_path.join(rel_path);
    std::fs::create_dir_all(file_path.parent().unwrap())
        .expect("Failed to create directory");

    let content = match rel_path {
        "src/main.rs" => {
            r#"
use crate::lib::helper;

fn main() {
    helper();
}
"#
        }
        "src/lib.rs" => {
            r#"
use std::collections::HashMap;

pub fn helper() {
    let _map = HashMap::new();
}
"#
        }
        _ => "// Empty file",
    };

    std::fs::write(&file_path, content).expect("Failed to write test file");
}
