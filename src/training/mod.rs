//! Training module: discovering and analyzing repositories for training validation.

pub mod discovery;
pub mod predictor;
pub mod orchestrator;
pub mod validator;

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
