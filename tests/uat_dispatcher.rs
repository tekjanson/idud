// tests/uat_dispatcher.rs
//! UAT Scaffolding: 100% User Acceptance Testing
//! Tests batch discovery engine with concurrency limits and mocked LLM endpoint
//! Zero actual HTTP requests: all inference is mocked with mockall

use idud::contract_ledger::ContractLedger;
use idud::pipelines::deep_link::{
    BatchDiscoveryEngine, ContractDiscoveryEngine, InferenceClient, LLMDiscoveryRequest,
    OllamaRequest, OllamaResponse,
};
use idud::types::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

// Mock inference client for UAT
struct UATMockClient {
    request_count: Arc<AtomicUsize>,
    concurrent_calls: Arc<AtomicUsize>,
    max_concurrent: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<String>>>,
}

impl UATMockClient {
    fn new() -> Self {
        Self {
            request_count: Arc::new(AtomicUsize::new(0)),
            concurrent_calls: Arc::new(AtomicUsize::new(0)),
            max_concurrent: Arc::new(AtomicUsize::new(0)),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn request_count(&self) -> usize {
        self.request_count.load(Ordering::SeqCst)
    }

    fn max_concurrent_observed(&self) -> usize {
        self.max_concurrent.load(Ordering::SeqCst)
    }

    async fn recorded_requests(&self) -> Vec<String> {
        self.requests.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl InferenceClient for UATMockClient {
    async fn infer(&self, request: OllamaRequest) -> anyhow::Result<OllamaResponse> {
        // Track concurrent calls
        let concurrent = self.concurrent_calls.fetch_add(1, Ordering::SeqCst) + 1;

        // Update max concurrent if needed
        let mut max = self.max_concurrent.load(Ordering::SeqCst);
        while concurrent > max {
            match self.max_concurrent.compare_exchange(
                max,
                concurrent,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual_max) => max = actual_max,
            }
        }

        // Record request
        self.requests.lock().await.push(request.prompt.clone());

        // Simulate some work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Decrement concurrent counter
        self.concurrent_calls.fetch_sub(1, Ordering::SeqCst);

        // Increment total request count
        self.request_count.fetch_add(1, Ordering::SeqCst);

        // Return mock response
        Ok(OllamaResponse {
            response: r#"{"contracts": []}"#.to_string(),
        })
    }
}

// ============================================================================
// UAT 1: Graph Layer — Contract Registration and Traversal
// ============================================================================

#[tokio::test]
async fn uat_graph_register_signatory_creates_node() {
    let ledger = ContractLedger::new();

    let sig = Signatory::new(
        SignatoryType::Function,
        "repo/auth.rs".to_string(),
        "validatePassword".to_string(),
        "fn validatePassword(pwd: &str) -> bool { pwd.len() > 8 }".to_string(),
    );
    let sig_id = sig.id.clone();

    let result = ledger.register_signatory(sig);
    assert!(result.is_ok(), "Signatory registration should succeed");

    let retrieved = ledger.get_signatory(&sig_id);
    assert!(
        retrieved.is_some(),
        "Signatory should be retrievable after registration"
    );
    assert_eq!(retrieved.unwrap().label, "validatePassword");
}

#[tokio::test]
async fn uat_graph_draft_contract_creates_edge() {
    let ledger = ContractLedger::new();

    let principal = Signatory::new(
        SignatoryType::Function,
        "repo/auth.rs".to_string(),
        "login".to_string(),
        "fn login() { validatePassword(); }".to_string(),
    );
    let principal_id = principal.id.clone();

    let guarantor = Signatory::new(
        SignatoryType::Function,
        "repo/auth.rs".to_string(),
        "validatePassword".to_string(),
        "fn validatePassword() {}".to_string(),
    );
    let guarantor_id = guarantor.id.clone();

    ledger.register_signatory(principal).unwrap();
    ledger.register_signatory(guarantor).unwrap();

    let contract = Contract::new(
        principal_id.clone(),
        guarantor_id.clone(),
        ClauseType::Calls,
        0.98,
        ContractSource::Deterministic,
    )
    .with_reasoning("login function directly calls validatePassword".to_string());

    let result = ledger.draft_contract(contract);
    assert!(result.is_ok(), "Contract drafting should succeed");

    // Verify obligation was registered
    let obligations = ledger.get_obligations(&principal_id);
    assert!(!obligations.is_empty(), "Principal should have obligations");
    assert_eq!(obligations[0].guarantor_id, guarantor_id);
}

#[tokio::test]
async fn uat_graph_trace_chain_respects_max_depth() {
    let ledger = ContractLedger::new();

    // Create chain: A -> B -> C -> D
    let sigs = vec![
        Signatory::new(
            SignatoryType::Function,
            "uri".to_string(),
            "funcA".to_string(),
            "".to_string(),
        ),
        Signatory::new(
            SignatoryType::Function,
            "uri".to_string(),
            "funcB".to_string(),
            "".to_string(),
        ),
        Signatory::new(
            SignatoryType::Function,
            "uri".to_string(),
            "funcC".to_string(),
            "".to_string(),
        ),
        Signatory::new(
            SignatoryType::Function,
            "uri".to_string(),
            "funcD".to_string(),
            "".to_string(),
        ),
    ];

    let ids: Vec<_> = sigs.iter().map(|s| s.id.clone()).collect();

    for sig in sigs {
        ledger.register_signatory(sig).unwrap();
    }

    // Create contracts: A->B, B->C, C->D
    for i in 0..3 {
        let contract = Contract::new(
            ids[i].clone(),
            ids[i + 1].clone(),
            ClauseType::Requires,
            0.95,
            ContractSource::Deterministic,
        );
        ledger.draft_contract(contract).unwrap();
    }

    // Trace with depth 2: should find A, B, C (not D)
    let chain = ledger.trace_chain_of_obligation(&ids[0], 2).unwrap();
    assert_eq!(
        chain.total_signatories, 3,
        "Max depth 2 should include root + 2 levels"
    );

    // Trace with depth 3: should find A, B, C, D
    let chain = ledger.trace_chain_of_obligation(&ids[0], 3).unwrap();
    assert_eq!(
        chain.total_signatories, 4,
        "Max depth 3 should include all 4 signatories"
    );
}

#[tokio::test]
async fn uat_graph_audit_coverage_detects_unaudited_functions() {
    let ledger = ContractLedger::new();

    let func1 = Signatory::new(
        SignatoryType::Function,
        "uri".to_string(),
        "criticalAuth".to_string(),
        "".to_string(),
    );
    let func1_id = func1.id.clone();

    let func2 = Signatory::new(
        SignatoryType::Function,
        "uri".to_string(),
        "helperFunc".to_string(),
        "".to_string(),
    );
    let func2_id = func2.id.clone();

    let test = Signatory::new(
        SignatoryType::Test,
        "uri".to_string(),
        "test_auth".to_string(),
        "".to_string(),
    );
    let test_id = test.id.clone();

    ledger.register_signatory(func1).unwrap();
    ledger.register_signatory(func2).unwrap();
    ledger.register_signatory(test).unwrap();

    // Only audit func1
    let audit_contract = Contract::new(
        test_id,
        func1_id,
        ClauseType::Audits,
        1.0,
        ContractSource::Deterministic,
    );
    ledger.draft_contract(audit_contract).unwrap();

    // Report should show func2 as unaudited
    let report = ledger.audit_contract_coverage();
    assert_eq!(
        report.unaudited.len(),
        1,
        "Should detect 1 unaudited function"
    );
    assert_eq!(report.unaudited[0].label, "helperFunc");
}

// ============================================================================
// UAT 2: Inference Dispatcher — Concurrency Limits
// ============================================================================

#[tokio::test]
async fn uat_dispatcher_respects_concurrency_limits() {
    let mock_client = Arc::new(UATMockClient::new());
    let engine = ContractDiscoveryEngine::new(mock_client.clone(), false);
    let batch_engine = BatchDiscoveryEngine::new(engine, 3, 2); // max_concurrent=2

    // Create 5 signatories
    let signatories: Vec<_> = (0..5)
        .map(|i| {
            Signatory::new(
                SignatoryType::Function,
                format!("uri{}", i),
                format!("func{}", i),
                "".to_string(),
            )
        })
        .collect();

    let context = vec![Signatory::new(
        SignatoryType::Function,
        "uri_ctx".to_string(),
        "context_func".to_string(),
        "".to_string(),
    )];

    let _result = batch_engine
        .discover_batch(signatories, context)
        .await
        .unwrap();

    // Check that max concurrent never exceeded 2
    let max_concurrent = mock_client.max_concurrent_observed();
    assert!(
        max_concurrent <= 2,
        "Max concurrent should never exceed 2, but was {}",
        max_concurrent
    );
}

#[tokio::test]
async fn uat_dispatcher_batches_all_signatories() {
    let mock_client = Arc::new(UATMockClient::new());
    let engine = ContractDiscoveryEngine::new(mock_client.clone(), false);
    let batch_engine = BatchDiscoveryEngine::new(engine, 2, 3); // batch_size=2

    let signatories: Vec<_> = (0..6)
        .map(|i| {
            Signatory::new(
                SignatoryType::Function,
                format!("uri{}", i),
                format!("func{}", i),
                "".to_string(),
            )
        })
        .collect();

    let context = vec![Signatory::new(
        SignatoryType::Function,
        "uri_ctx".to_string(),
        "context_func".to_string(),
        "".to_string(),
    )];

    let _result = batch_engine
        .discover_batch(signatories, context)
        .await
        .unwrap();

    // With 6 signatories and batch_size=2, should make 3 rounds
    // Each signatory processes independently, so 6 total requests
    let request_count = mock_client.request_count();
    assert_eq!(
        request_count, 6,
        "Should make 6 inference requests for 6 signatories"
    );
}

#[tokio::test]
async fn uat_dispatcher_never_makes_http_requests() {
    let mock_client = Arc::new(UATMockClient::new());
    let engine = ContractDiscoveryEngine::new(mock_client.clone(), false);

    let principal = Signatory::new(
        SignatoryType::Function,
        "uri".to_string(),
        "fetchData".to_string(),
        "fn fetchData(){}".to_string(),
    );

    let context = vec![Signatory::new(
        SignatoryType::Function,
        "uri".to_string(),
        "parseData".to_string(),
        "fn parseData(){}".to_string(),
    )];

    let request = LLMDiscoveryRequest {
        signatory: principal,
        context_signatories: context,
        max_contracts: 5,
    };

    let _result = engine.discover_contracts(request).await;

    // Verify only mock client was called, not actual HTTP
    // This is implicit: if actual HTTP was attempted, it would fail (no real Ollama)
    assert_eq!(
        mock_client.request_count(),
        1,
        "Mock should have been called once"
    );
}

#[tokio::test]
async fn uat_dispatcher_parses_json_response_safely() {
    let mock_client = Arc::new(UATMockClient::new());
    let engine = ContractDiscoveryEngine::new(mock_client, false);

    let principal = Signatory::new(
        SignatoryType::Function,
        "uri".to_string(),
        "main".to_string(),
        "".to_string(),
    );

    let context = vec![];

    let request = LLMDiscoveryRequest {
        signatory: principal.clone(),
        context_signatories: context,
        max_contracts: 5,
    };

    let result = engine.discover_contracts(request).await;

    // Should not panic on empty response
    assert!(result.is_ok(), "Should handle mock response gracefully");
    let response = result.unwrap();
    assert_eq!(response.principal_id, principal.id);
}

// ============================================================================
// UAT 3: UI Translator — Contract to Natural Language
// ============================================================================

#[test]
fn uat_translator_converts_calls_to_sentence() {
    use idud::ui::translator::{contract_to_sentence, ContractRenderer};

    let principal = Signatory::new(
        SignatoryType::Function,
        "uri".to_string(),
        "fetchUser".to_string(),
        "".to_string(),
    );

    let guarantor = Signatory::new(
        SignatoryType::Function,
        "uri".to_string(),
        "parseJSON".to_string(),
        "".to_string(),
    );

    let contract = Contract::new(
        principal.id.clone(),
        guarantor.id.clone(),
        ClauseType::Calls,
        0.92,
        ContractSource::Deterministic,
    );

    let sentence = contract_to_sentence(&principal, &contract, &guarantor);

    // Verify natural language sentence
    assert!(sentence.full_sentence.contains("fetchUser"));
    assert!(sentence.full_sentence.contains("calls"));
    assert!(sentence.full_sentence.contains("parseJSON"));

    // Verify HTML rendering
    let html = ContractRenderer::render_sentence(&sentence, true);
    assert!(html.contains("confidence-high"));
    assert!(html.contains("92% confident"));
}

#[test]
fn uat_translator_renders_audit_relationship() {
    use idud::ui::translator::contract_to_sentence;

    let test = Signatory::new(
        SignatoryType::Test,
        "uri".to_string(),
        "test_validate".to_string(),
        "".to_string(),
    );

    let func = Signatory::new(
        SignatoryType::Function,
        "uri".to_string(),
        "validate".to_string(),
        "".to_string(),
    );

    let contract = Contract::new(
        test.id.clone(),
        func.id.clone(),
        ClauseType::Audits,
        1.0,
        ContractSource::Deterministic,
    );

    let sentence = contract_to_sentence(&test, &contract, &func);
    assert!(sentence.full_sentence.contains("is audited by"));
}

#[test]
fn uat_translator_chain_rendering() {
    use idud::ui::translator::ContractRenderer;

    let s1 = Signatory::new(
        SignatoryType::Function,
        "uri1".to_string(),
        "func1".to_string(),
        "".to_string(),
    );
    let s2 = Signatory::new(
        SignatoryType::Function,
        "uri2".to_string(),
        "func2".to_string(),
        "".to_string(),
    );

    let contract = Contract::new(
        s1.id.clone(),
        s2.id.clone(),
        ClauseType::Requires,
        0.88,
        ContractSource::Deterministic,
    )
    .with_reasoning("func1 depends on func2 for data".to_string());

    let chain = ChainOfObligation {
        root_signatory: s1,
        chain: vec![(s2, Some(contract))],
        max_depth: 3,
        total_signatories: 2,
    };

    let html = ContractRenderer::render_chain(&chain);
    assert!(html.contains("Chain of Obligations"));
    assert!(html.contains("2 signatories involved"));
    assert!(html.contains("88%"));
}

// ============================================================================
// UAT 4: Integration Tests
// ============================================================================

#[tokio::test]
async fn uat_end_to_end_workflow() {
    let ledger = ContractLedger::new();

    // 1. Register signatories
    let auth_func = Signatory::new(
        SignatoryType::Function,
        "src/auth.rs".to_string(),
        "authenticate".to_string(),
        "fn authenticate(user: &User) -> Result { validatePassword(user.password) }".to_string(),
    );
    let auth_id = auth_func.id.clone();

    let validate_func = Signatory::new(
        SignatoryType::Function,
        "src/auth.rs".to_string(),
        "validatePassword".to_string(),
        "fn validatePassword(pwd: &str) -> bool { pwd.len() > 8 }".to_string(),
    );
    let validate_id = validate_func.id.clone();

    let test = Signatory::new(
        SignatoryType::Test,
        "tests/auth_test.rs".to_string(),
        "test_authenticate".to_string(),
        "#[test]\nfn test_authenticate() { assert!(authenticate(...)) }".to_string(),
    );
    let test_id = test.id.clone();

    ledger.register_signatory(auth_func).unwrap();
    ledger.register_signatory(validate_func).unwrap();
    ledger.register_signatory(test).unwrap();

    // 2. Draft contracts
    let contract1 = Contract::new(
        auth_id.clone(),
        validate_id.clone(),
        ClauseType::Calls,
        0.98,
        ContractSource::Deterministic,
    );
    ledger.draft_contract(contract1).unwrap();

    let contract2 = Contract::new(
        test_id.clone(),
        auth_id.clone(),
        ClauseType::Audits,
        1.0,
        ContractSource::Deterministic,
    );
    ledger.draft_contract(contract2).unwrap();

    // 3. Verify the graph is correct
    let (sig_count, contract_count) = ledger.stats();
    assert_eq!(sig_count, 3, "Should have 3 signatories");
    assert_eq!(contract_count, 2, "Should have 2 contracts");

    // 4. Trace obligations
    let chain = ledger.trace_chain_of_obligation(&auth_id, 2).unwrap();
    assert_eq!(
        chain.total_signatories, 2,
        "authenticate should lead to validatePassword"
    );

    // 5. Audit coverage
    let report = ledger.audit_contract_coverage();
    assert_eq!(
        report.audited_signatories, 2,
        "Both functions involved in audit contract"
    );

    // 6. Generate AI brief
    let brief = ledger.generate_contract_brief("test_entity");
    assert_eq!(brief.signatory_count, 3);
    assert_eq!(brief.contract_count, 2);
}
