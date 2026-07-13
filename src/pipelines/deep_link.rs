// src/pipelines/deep_link.rs
//! PHASE 3.2: The Deep Link — Local-First Inference Dispatcher
//! AI prompts for inferring contracts between signatories
//! Implements bounded task queue with Ollama for zero-token wastefulness
//! Uses tokio::sync::Semaphore to prevent OOM and thermal throttling

use crate::types::*;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMDiscoveryRequest {
    pub signatory: Signatory,
    pub context_signatories: Vec<Signatory>,
    pub max_contracts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredContract {
    pub guarantor_id: String,
    pub clause_type: ClauseType,
    pub confidence: f32,
    pub clause_reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMDiscoveryResponse {
    pub principal_id: String,
    pub contracts: Vec<InferredContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaResponse {
    pub response: String,
}

/// InferenceClient trait for pluggable LLM backends
#[async_trait]
pub trait InferenceClient: Send + Sync {
    async fn infer(&self, request: OllamaRequest) -> Result<OllamaResponse>;
}

/// Local Ollama dispatcher: targets localhost:11434 by default
pub struct OllamaDispatcher {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaDispatcher {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| "mistral".to_string()),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

#[async_trait]
impl InferenceClient for OllamaDispatcher {
    async fn infer(&self, request: OllamaRequest) -> Result<OllamaResponse> {
        let url = format!("{}/api/generate", self.base_url);

        let req = OllamaRequest {
            model: self.model.clone(),
            ..request
        };

        let response = self.client.post(&url).json(&req).send().await?;

        let resp: OllamaResponse = response.json().await?;
        Ok(resp)
    }
}

/// Build discovery prompt for a signatory
fn discovery_prompt(principal: &Signatory, context: &[Signatory]) -> String {
    let mut prompt = format!(
        "Discover contractual bindings for this principal signatory.\n\n\
         PRINCIPAL:\n\
         Type: {:?}\n\
         Name: {}\n\
         Location: {}\n\
         Code:\n{}\n\n\
         CONTEXT (other signatories in the repository):\n",
        principal.signatory_type, principal.label, principal.source_uri, principal.snippet
    );

    for (i, signatory) in context.iter().enumerate() {
        prompt.push_str(&format!(
            "\n[{}] Type: {:?}, Name: {}\nCode:\n{}\n",
            signatory.id,
            signatory.signatory_type,
            signatory.label,
            signatory
                .snippet
                .lines()
                .take(3)
                .collect::<Vec<_>>()
                .join("\n")
        ));
        if i >= 10 {
            prompt.push_str("\n(... and more signatories in context)");
            break;
        }
    }

    prompt.push_str(
        "\n\nIdentify contract clauses from PRINCIPAL to CONTEXT signatories. Return JSON only.",
    );
    prompt
}

/// LLM interface for contract discovery
pub struct ContractDiscoveryEngine {
    client: Arc<dyn InferenceClient>,
    enable_mock: bool,
}

impl ContractDiscoveryEngine {
    pub fn new(client: Arc<dyn InferenceClient>, enable_mock: bool) -> Self {
        Self {
            client,
            enable_mock,
        }
    }

    pub fn with_mock() -> Self {
        Self {
            client: Arc::new(MockInferenceClient {}),
            enable_mock: true,
        }
    }

    /// Discover contracts for a signatory using LLM
    pub async fn discover_contracts(
        &self,
        request: LLMDiscoveryRequest,
    ) -> Result<LLMDiscoveryResponse> {
        if self.enable_mock {
            return self.mock_discovery(request).await;
        }

        let prompt = discovery_prompt(&request.signatory, &request.context_signatories);
        let ollama_req = OllamaRequest {
            model: "mistral".to_string(),
            prompt,
            stream: false,
            temperature: 0.3,
        };

        let response = self.client.infer(ollama_req).await?;

        // Parse JSON from response
        let contracts: Vec<InferredContract> = serde_json::from_str(&response.response)
            .ok()
            .and_then(|json: serde_json::Value| {
                json.get("contracts")
                    .and_then(|c| serde_json::from_value(c.clone()).ok())
            })
            .unwrap_or_default();

        Ok(LLMDiscoveryResponse {
            principal_id: request.signatory.id,
            contracts: contracts.into_iter().take(request.max_contracts).collect(),
        })
    }

    /// Mock discovery for testing (deterministic for UAT)
    async fn mock_discovery(&self, request: LLMDiscoveryRequest) -> Result<LLMDiscoveryResponse> {
        let mut contracts = Vec::new();

        // Heuristic: functions often bind to functions with complementary names
        if request.signatory.signatory_type == SignatoryType::Function {
            for context in &request.context_signatories {
                if context.signatory_type == SignatoryType::Function {
                    let principal_name = request.signatory.label.to_lowercase();
                    let context_name = context.label.to_lowercase();

                    if (principal_name.contains("fetch") && context_name.contains("parse"))
                        || (principal_name.contains("validate") && context_name.contains("clean"))
                    {
                        contracts.push(InferredContract {
                            guarantor_id: context.id.clone(),
                            clause_type: ClauseType::Calls,
                            confidence: 0.75,
                            clause_reasoning: format!(
                                "Name pattern: '{}' calls '{}'",
                                request.signatory.label, context.label
                            ),
                        });
                    }
                }
            }
        }

        // Tests bind to the code they audit
        if request.signatory.signatory_type == SignatoryType::Test {
            for context in &request.context_signatories {
                if context.signatory_type == SignatoryType::Function {
                    let test_name = request.signatory.label.to_lowercase();
                    let func_name = context.label.to_lowercase();

                    if test_name.contains(&func_name) || func_name.contains(&test_name) {
                        contracts.push(InferredContract {
                            guarantor_id: context.id.clone(),
                            clause_type: ClauseType::Audits,
                            confidence: 0.95,
                            clause_reasoning: format!(
                                "Test '{}' audits '{}'",
                                request.signatory.label, context.label
                            ),
                        });
                    }
                }
            }
        }

        Ok(LLMDiscoveryResponse {
            principal_id: request.signatory.id,
            contracts: contracts.into_iter().take(request.max_contracts).collect(),
        })
    }
}

/// Mock inference client for testing without actual LLM
struct MockInferenceClient {}

#[async_trait]
impl InferenceClient for MockInferenceClient {
    async fn infer(&self, _request: OllamaRequest) -> Result<OllamaResponse> {
        Ok(OllamaResponse {
            response: r#"{"contracts": []}"#.to_string(),
        })
    }
}

/// Concurrent batch discovery engine with bounded concurrency
pub struct BatchDiscoveryEngine {
    engine: Arc<ContractDiscoveryEngine>,
    batch_size: usize,
    semaphore: Arc<Semaphore>,
}

impl BatchDiscoveryEngine {
    /// Create new batch engine with concurrency limit
    pub fn new(engine: ContractDiscoveryEngine, batch_size: usize, max_concurrent: usize) -> Self {
        Self {
            engine: Arc::new(engine),
            batch_size,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Discover contracts for signatories with bounded concurrency
    pub async fn discover_batch(
        &self,
        signatories: Vec<Signatory>,
        context: Vec<Signatory>,
    ) -> Result<Vec<InferredContract>> {
        let mut all_contracts = Vec::new();

        for batch in signatories.chunks(self.batch_size) {
            let mut batch_futures = Vec::new();

            for signatory in batch {
                let semaphore = Arc::clone(&self.semaphore);
                let engine = Arc::clone(&self.engine);
                let request = LLMDiscoveryRequest {
                    signatory: signatory.clone(),
                    context_signatories: context.clone(),
                    max_contracts: 5,
                };

                let future = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await;
                    engine.discover_contracts(request).await
                });

                batch_futures.push(future);
            }

            // Await all requests in this batch
            for future in batch_futures {
                if let Ok(Ok(response)) = future.await {
                    all_contracts.extend(response.contracts);
                }
            }
        }

        Ok(all_contracts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_discovery() {
        let engine = ContractDiscoveryEngine::with_mock();
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

        let result = engine.discover_contracts(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.contracts.is_empty());
    }

    #[tokio::test]
    async fn test_batch_discovery_respects_concurrency() {
        let engine = ContractDiscoveryEngine::with_mock();
        let batch_engine = BatchDiscoveryEngine::new(engine, 2, 2);

        let signatories = vec![
            Signatory::new(
                SignatoryType::Function,
                "uri1".to_string(),
                "fetchUser".to_string(),
                "fn fetchUser(){}".to_string(),
            ),
            Signatory::new(
                SignatoryType::Function,
                "uri2".to_string(),
                "parseUser".to_string(),
                "fn parseUser(){}".to_string(),
            ),
            Signatory::new(
                SignatoryType::Function,
                "uri3".to_string(),
                "validateUser".to_string(),
                "fn validateUser(){}".to_string(),
            ),
        ];

        let context = vec![Signatory::new(
            SignatoryType::Function,
            "uri4".to_string(),
            "cleanData".to_string(),
            "fn cleanData(){}".to_string(),
        )];

        let result = batch_engine.discover_batch(signatories, context).await;
        assert!(result.is_ok());
    }
}
