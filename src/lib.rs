// src/lib.rs
pub mod contract_ledger;
pub mod pipelines;
pub mod schemas;
pub mod training;
pub mod training_datalake;
pub mod types;
pub mod ui;
pub mod web_server;

pub use contract_ledger::ContractLedger;
pub use pipelines::{RepositoryIngestionConfig, RepositoryTraverser};
pub use schemas::{ContractFactory, ContractValidator, SignatoryFactory};
pub use training::{
    discover_training_repos, fetch_issue_and_linked_pr, RepoCandidate, IssueWithPR,
    predict_files_from_issue, PredictionRequest, PredictionResponse, TrainingOrchestrator,
    TrainingConfig, TrainingResults, batch_training_jobs, TrainingStatus,
    validate_prediction, write_training_result, calculate_aggregate_metrics,
    calculate_metrics_by_language, ValidationMetrics, LanguageMetrics, TrainingCache, CacheStats,
};
pub use training_datalake::{
    AggregatedMetrics, Checkpoint, IngestionStatus, PercentileMetrics, RepoMetadata,
    TimeWindow, TrainingDataLake, TrainingRun,
};
pub use types::*;
pub use ui::translator::*;
pub use web_server::{serve, WebServerConfig};
