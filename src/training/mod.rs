//! Training module: discovering and analyzing repositories for training validation.

pub mod cache;
pub mod discovery;
pub mod predictor;
pub mod orchestrator;
pub mod validator;
pub mod token_meter;
pub mod pr_predictor;
pub mod repo_ingestion_orchestrator;
pub mod waymark_validator;

pub use cache::{TrainingCache, CacheEntry, CacheStats};
pub use discovery::{
    discover_training_repos, fetch_issue_and_linked_pr, RepoCandidate, IssueWithPR,
    RateLimitStatus,
};
pub use predictor::{predict_files_from_issue, PredictionRequest, PredictionResponse, TokenUsage};
pub use orchestrator::{
    TrainingOrchestrator, TrainingConfig, TrainingResults, TrainingBatch, TrainingStatus,
    RepoTrainingMetrics, batch_training_jobs,
};
pub use validator::{
    validate_prediction, write_training_result, calculate_aggregate_metrics,
    calculate_metrics_by_language, ValidationMetrics, LanguageMetrics,
};
pub use token_meter::{TokenMeter, TokenStats};
pub use pr_predictor::{CoDependencyGraph, PRPredictor, FilePrediction};
pub use repo_ingestion_orchestrator::{
    RepositoryIngestionOrchestrator, RepoIngestionConfig, RepositoryRegistry,
    RepositoryEntry, IngestionMetrics, IngestionStatus, IngestionResults, IngestionLogEntry,
};
pub use waymark_validator::{
    load_waymark_contracts, ValidationEngine, PredictionTestCase, PredictionTestResult,
    ValidationSummary, WaymarkData,
};
