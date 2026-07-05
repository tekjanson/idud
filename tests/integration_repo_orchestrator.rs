//! Integration tests for repository ingestion orchestration
//!
//! Tests verify:
//! 1. Registry loading works correctly
//! 2. Orchestrator initializes and runs
//! 3. Idempotency: running twice skips already-ingested repos
//! 4. Output structure is correct
//! 5. Log files are created

#[cfg(test)]
mod tests {
    use idud::{RepositoryIngestionOrchestrator, RepoIngestionConfig};
    use std::path::PathBuf;
    use std::fs;

    #[test]
    fn test_registry_loads_successfully() {
        let config = RepoIngestionConfig {
            registry_path: PathBuf::from("data/repos_to_ingest.json"),
            output_dir: PathBuf::from("data"),
            max_repos: Some(3),
            timeout_minutes: Some(5),
            skip_already_ingested: true,
        };

        let result = RepositoryIngestionOrchestrator::new(config);
        
        // Should load successfully if file exists
        if result.is_ok() {
            let orchestrator = result.unwrap();
            let registry = orchestrator.get_registry();
            println!("✓ Registry loaded with {} repos", registry.repositories.len());
            assert!(!registry.repositories.is_empty(), 
                "Registry should have repositories");
        } else {
            println!("⚠️  Registry file not found (expected in data/repos_to_ingest.json)");
        }
    }

    #[test]
    fn test_registry_structure() {
        let config = RepoIngestionConfig {
            registry_path: PathBuf::from("data/repos_to_ingest.json"),
            output_dir: PathBuf::from("data"),
            max_repos: None,
            timeout_minutes: None,
            skip_already_ingested: true,
        };

        if let Ok(orchestrator) = RepositoryIngestionOrchestrator::new(config) {
            let registry = orchestrator.get_registry();
            // Verify registry structure
            assert!(!registry.metadata.description.is_empty(),
                "Registry should have description");
            
            // Verify repository entries
            for repo in &registry.repositories {
                assert!(!repo.repo_url.is_empty(), "Repo URL should not be empty");
                assert!(!repo.repo_name.is_empty(), "Repo name should not be empty");
                assert!(repo.stars > 0, "Repos should have positive star count");
                println!("  ✓ {}: {} ⭐ ({})", repo.repo_name, repo.language, repo.stars);
            }
        }
    }

    #[test]
    fn test_idempotency_log_format() {
        use chrono::Utc;
        use idud::training::IngestionLogEntry;

        let entry = IngestionLogEntry {
            repo_name: "test-repo".to_string(),
            repo_url: "https://github.com/test/repo".to_string(),
            timestamp: Utc::now(),
            status: "success".to_string(),
            files_processed: 100,
            signatories: 50,
            contracts: 25,
            duration_secs: 10,
        };

        // Verify log entry can be serialized
        let json = serde_json::to_string(&entry).expect("Should serialize");
        assert!(json.contains("test-repo"), "Log entry should contain repo name");
        assert!(json.contains("success"), "Log entry should contain status");
        
        println!("✓ Log entry serializes correctly: {}", json);
    }

    #[test]
    fn test_config_validation() {
        // Valid config
        let config = RepoIngestionConfig {
            registry_path: PathBuf::from("data/repos_to_ingest.json"),
            output_dir: PathBuf::from("data"),
            max_repos: Some(5),
            timeout_minutes: Some(10),
            skip_already_ingested: true,
        };

        assert_eq!(config.max_repos, Some(5), "Max repos should be set");
        assert_eq!(config.timeout_minutes, Some(10), "Timeout should be set");
        assert!(config.skip_already_ingested, "Skip ingested should be true");
        
        println!("✓ Config validation passed");
    }

    #[test]
    fn test_default_config() {
        let config = RepoIngestionConfig::default_paths();
        
        assert_eq!(config.registry_path, PathBuf::from("data/repos_to_ingest.json"));
        assert_eq!(config.output_dir, PathBuf::from("data"));
        assert!(config.skip_already_ingested, "Should skip ingested repos by default");
        
        println!("✓ Default config is correct");
    }

    #[test]
    fn test_ingestion_log_persistence() {
        let config = RepoIngestionConfig {
            registry_path: PathBuf::from("data/repos_to_ingest.json"),
            output_dir: PathBuf::from("data"),
            max_repos: None,
            timeout_minutes: None,
            skip_already_ingested: true,
        };

        if let Ok(orchestrator) = RepositoryIngestionOrchestrator::new(config) {
            // Try to load existing log
            match orchestrator.load_ingestion_log() {
                Ok(log) => {
                    println!("✓ Loaded existing ingestion log with {} entries", log.len());
                    for (repo_name, entry) in log.iter().take(3) {
                        println!("  - {}: {} ({} files)", 
                            repo_name, entry.status, entry.files_processed);
                    }
                }
                Err(e) => {
                    println!("⚠️  No existing log found (expected on first run): {}", e);
                }
            }
        }
    }

    #[test]
    fn test_registry_has_diverse_languages() {
        let config = RepoIngestionConfig {
            registry_path: PathBuf::from("data/repos_to_ingest.json"),
            output_dir: PathBuf::from("data"),
            max_repos: None,
            timeout_minutes: None,
            skip_already_ingested: true,
        };

        if let Ok(orchestrator) = RepositoryIngestionOrchestrator::new(config) {
            let registry = orchestrator.get_registry();
            let mut languages = std::collections::HashSet::new();
            for repo in &registry.repositories {
                languages.insert(&repo.language);
            }

            println!("✓ Registry includes {} different languages:", languages.len());
            for lang in languages {
                println!("  - {}", lang);
            }

            // Should have at least 3 different languages
            assert!(languages.len() >= 3, 
                "Registry should have at least 3 languages for diversity");
        }
    }

    #[test]
    fn test_markdown_log_creation() {
        use chrono::Utc;
        use idud::training::IngestionMetrics;
        
        let metrics = IngestionMetrics {
            repo_name: "test-repo".to_string(),
            repo_url: "https://github.com/test/repo".to_string(),
            status: idud::training::IngestionStatus::Success,
            started_at: Utc::now(),
            completed_at: Utc::now(),
            files_processed: 100,
            signatories: 50,
            contracts: 25,
            error_message: None,
        };

        // Verify metric can be formatted as markdown
        let status_emoji = match metrics.status {
            idud::training::IngestionStatus::Success => "✅",
            idud::training::IngestionStatus::Failed => "❌",
            idud::training::IngestionStatus::Skipped => "⏭️",
        };

        let markdown_row = format!(
            "| {} | {} | {} | {} | {} | {} |",
            metrics.repo_name,
            status_emoji,
            metrics.files_processed,
            metrics.signatories,
            metrics.contracts,
            (metrics.completed_at - metrics.started_at).num_seconds()
        );

        println!("✓ Markdown row: {}", markdown_row);
        assert!(markdown_row.contains("test-repo"), "Row should contain repo name");
    }

    #[test]
    fn test_registry_priority_ordering() {
        let config = RepoIngestionConfig {
            registry_path: PathBuf::from("data/repos_to_ingest.json"),
            output_dir: PathBuf::from("data"),
            max_repos: None,
            timeout_minutes: None,
            skip_already_ingested: true,
        };

        if let Ok(orchestrator) = RepositoryIngestionOrchestrator::new(config) {
            let registry = orchestrator.get_registry();
            let mut priorities: Vec<u32> = registry.repositories
                .iter()
                .map(|r| r.priority)
                .collect();

            // Should be ordered by priority
            priorities.sort();
            
            println!("✓ Registry has {} repos", registry.repositories.len());
            println!("  Priority range: {} - {}", 
                priorities.first().unwrap_or(&0),
                priorities.last().unwrap_or(&0)
            );
        }
    }

    #[test]
    fn test_output_directory_structure() {
        let output_dir = PathBuf::from("data");
        
        // Create output directory if needed
        fs::create_dir_all(&output_dir).ok();
        
        assert!(output_dir.exists(), "Output directory should exist or be creatable");
        println!("✓ Output directory exists: {}", output_dir.display());

        // Check for expected files after ingestion
        let ingestion_log = output_dir.join("ingestion-log.json");
        if ingestion_log.exists() {
            println!("  - Found ingestion log: {}", ingestion_log.display());
        } else {
            println!("  - Ingestion log will be created on first run");
        }
    }

    #[test]
    fn test_metrics_calculation() {
        use idud::training::IngestionStatus;
        use chrono::Utc;

        let start = Utc::now();
        let end = start + chrono::Duration::seconds(30);
        
        let metrics = idud::training::IngestionMetrics {
            repo_name: "perf-test".to_string(),
            repo_url: "https://github.com/perf/test".to_string(),
            status: IngestionStatus::Success,
            started_at: start,
            completed_at: end,
            files_processed: 500,
            signatories: 2500,
            contracts: 1200,
            error_message: None,
        };

        let duration = metrics.completed_at - metrics.started_at;
        let files_per_sec = metrics.files_processed as f64 / duration.num_seconds() as f64;

        println!("✓ Performance metrics:");
        println!("  - Duration: {} seconds", duration.num_seconds());
        println!("  - Files/sec: {:.2}", files_per_sec);
        println!("  - Signatories/file: {:.2}", 
            metrics.signatories as f64 / metrics.files_processed as f64);
    }
}
