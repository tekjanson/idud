// src/pipelines/mod.rs
pub mod bootstrap_eval;
pub mod broad_sweep;
pub mod deep_link;
pub mod embedding;

pub use bootstrap_eval::{BootstrapMetrics, TopologicalAnalyzer};
pub use broad_sweep::{IngestionResult, RepositoryIngestionConfig, RepositoryTraverser};
pub use deep_link::{
    BatchDiscoveryEngine, ContractDiscoveryEngine, InferenceClient, InferredContract,
    LLMDiscoveryRequest, LLMDiscoveryResponse, OllamaDispatcher, OllamaRequest, OllamaResponse,
};
