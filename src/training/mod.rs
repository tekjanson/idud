//! Training module: discovering and analyzing repositories for training validation.

pub mod cache;
pub mod discovery;
pub mod orchestrator;
pub mod pr_predictor;
pub mod predictor;
pub mod repo_ingestion_orchestrator;
pub mod repo_understanding;
pub mod token_meter;
pub mod validator;
pub mod waymark_validator;

pub use cache::{CacheEntry, CacheStats, TrainingCache};
pub use discovery::{
    discover_training_repos, fetch_issue_and_linked_pr, IssueWithPR, RateLimitStatus, RepoCandidate,
};
pub use orchestrator::{
    batch_training_jobs, RepoTrainingMetrics, TrainingBatch, TrainingConfig, TrainingOrchestrator,
    TrainingResults, TrainingStatus,
};
pub use pr_predictor::{CoDependencyGraph, FilePrediction, PRPredictor};
pub use predictor::{predict_files_from_issue, PredictionRequest, PredictionResponse, TokenUsage};
pub use repo_ingestion_orchestrator::{
    IngestionLogEntry, IngestionMetrics, IngestionResults, IngestionStatus, RepoIngestionConfig,
    RepositoryEntry, RepositoryIngestionOrchestrator, RepositoryRegistry,
};
pub use repo_understanding::{
    build_synthetic_understanding, write_synthetic_understanding, DependencyHint, DirectorySummary,
    ExtensionSummary, JourneyCandidate, SyntheticUnderstanding, TestSummary,
};
pub use token_meter::{TokenMeter, TokenStats};
pub use validator::{
    calculate_aggregate_metrics, calculate_metrics_by_language, validate_prediction,
    write_training_result, LanguageMetrics, ValidationMetrics,
};
pub use waymark_validator::{
    load_waymark_contracts, PredictionTestCase, PredictionTestResult, ValidationEngine,
    ValidationSummary, WaymarkData,
};
