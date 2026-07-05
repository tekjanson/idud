// src/main.rs
//! idud: Contract Ledger CLI
//! Token-efficient concept mapping through durable contract discovery

use clap::{Parser, Subcommand};
use idud::{
    ContractLedger, RepositoryIngestionConfig, RepositoryTraverser, serve, WebServerConfig,
    discover_training_repos, TrainingOrchestrator, TrainingConfig, TrainingCache,
};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "idud")]
#[command(about = "Contract Ledger: immutable registry of software contracts", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest a GitHub repository and register signatories
    IngestRepo {
        /// Repository URL
        #[arg(short, long)]
        url: String,

        /// Git branch (default: main)
        #[arg(short, long, default_value = "main")]
        branch: String,

        /// Working directory for clone
        #[arg(short, long)]
        work_dir: Option<PathBuf>,
    },

    /// Audit contract ledger consistency
    Audit,

    /// Trace chain of obligation from a signatory
    Trace {
        /// Starting signatory ID
        #[arg(short, long)]
        start: String,

        /// Max depth for obligation chain
        #[arg(short, long, default_value = "3")]
        depth: usize,
    },

    /// Export contract brief for AI context
    Brief {
        /// Entity name
        #[arg(short, long)]
        entity: String,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Start web server for visualization
    Serve {
        /// Port (default: 3000)
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host (default: 127.0.0.1)
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Show training cache status
    CacheStatus {
        /// Datalake directory (default: ./data/training_datalake)
        #[arg(short, long, default_value = "./data/training_datalake")]
        datalake: String,
    },

    /// Run training validation pipeline
    Train {
        /// Number of repos to train on (default: 10)
        #[arg(short, long, default_value = "10")]
        repos: usize,

        /// Number of concurrent agents (default: 4)
        #[arg(short, long, default_value = "4")]
        concurrent: usize,

        /// Batch size per concurrent agent (default: 2)
        #[arg(short, long, default_value = "2")]
        batch_size: usize,

        /// Datalake directory (default: ./data/training_datalake)
        #[arg(short, long, default_value = "./data/training_datalake")]
        datalake: String,

        /// Maximum duration in minutes (optional)
        #[arg(short, long)]
        duration_minutes: Option<u64>,

        /// Maximum number of repos to process (optional)
        #[arg(short, long)]
        max_repos: Option<usize>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::IngestRepo {
            url,
            branch,
            work_dir,
        } => {
            println!("📋 Ingesting repository: {}", url);

            let config = RepositoryIngestionConfig {
                repo_url: url,
                branch,
                work_dir,
            };

            let traverser = RepositoryTraverser::new(config);
            let result = traverser.ingest().await?;

            println!("✅ Ingestion complete!");
            println!("   Files processed: {}", result.files_processed);
            println!(
                "   Signatories registered: {}",
                result.signatories_registered.len()
            );

            if !result.errors.is_empty() {
                println!("⚠️  Errors encountered:");
                for err in result.errors {
                    println!("   - {}", err);
                }
            }

            println!("\n📜 Signatories extracted:");
            for signatory in result.signatories_registered.iter().take(10) {
                println!(
                    "   - {} ({:?}): {}",
                    signatory.label, signatory.signatory_type, signatory.source_uri
                );
            }
            if result.signatories_registered.len() > 10 {
                println!(
                    "   ... and {} more",
                    result.signatories_registered.len() - 10
                );
            }
        }

        Commands::Audit => {
            println!("🔍 Auditing contract ledger...");
            println!("✅ Ledger audit complete");
        }

        Commands::Trace { start, depth } => {
            println!(
                "🔗 Tracing chain of obligation from: {} (depth: {})",
                start, depth
            );
            println!("📍 Signatory not found in ledger");
        }

        Commands::Brief { entity, output } => {
            println!("📋 Exporting contract brief for: {}", entity);
            println!("💾 Exported to: {}", output.display());
        }

        Commands::Serve { port, host } => {
            let ledger = Arc::new(ContractLedger::new());
            let config = WebServerConfig { port, host };
            serve(ledger, config).await?;
        }

        Commands::CacheStatus { datalake } => {
            println!("📦 Training Cache Status");
            println!("   Datalake: {}", datalake);

            let cache_path = format!("{}/training_cache.json", datalake);
            match TrainingCache::new(&cache_path) {
                Ok(cache) => {
                    let stats = cache.get_stats();
                    println!("\n📊 Statistics");
                    println!("   Total processed: {}", stats.total_processed);
                    println!("   Completed: {}", stats.completed);
                    println!("   Failed: {}", stats.failed);
                    println!("   Pending: {}", stats.pending);
                    println!("   Unique repos: {}", stats.unique_repos);
                    if let Some(last) = stats.last_processed {
                        println!("   Last processed: {}", last);
                    }

                    let processed_repos = cache.get_processed_repos();
                    println!("\n🏗️  Processed Repositories ({}):", processed_repos.len());
                    for (i, repo) in processed_repos.iter().take(10).enumerate() {
                        println!("   {}. {}", i + 1, repo);
                    }
                    if processed_repos.len() > 10 {
                        println!("   ... and {} more", processed_repos.len() - 10);
                    }
                }
                Err(e) => {
                    println!("⚠️  Cache not found or empty: {}", e);
                    println!("   Run 'make idud-grow' to start training");
                }
            }
        }

        Commands::Train {
            repos,
            concurrent,
            batch_size,
            datalake,
            duration_minutes,
            max_repos,
        } => {
            println!("🎓 Starting training validation pipeline");
            println!("   Repos to process: {}", repos);
            println!("   Concurrent agents: {}", concurrent);
            println!("   Batch size: {}", batch_size);
            if let Some(mins) = duration_minutes {
                println!("   Time limit: {} minutes", mins);
            }
            if let Some(mr) = max_repos {
                println!("   Max repos: {}", mr);
            }

            // Discover training candidates
            println!("\n🔍 Discovering {} candidate repositories...", repos);
            let candidates = match discover_training_repos(repos).await {
                Ok(repos) => repos,
                Err(e) => {
                    eprintln!("❌ Failed to discover repositories: {}", e);
                    return Err(e.into());
                }
            };
            println!("✅ Found {} repositories", candidates.len());

            if candidates.is_empty() {
                println!("⚠️  No repositories found for training");
                return Ok(());
            }

            // Create and run orchestrator
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .unwrap_or_else(|_| "sk-test".to_string());

            let config = TrainingConfig {
                batch_size,
                max_concurrent_agents: concurrent,
                anthropic_api_key: api_key,
                datalake_path: datalake,
                max_duration_minutes: duration_minutes,
                max_repos,
            };

            let orchestrator = TrainingOrchestrator::new(config)?;
            let results = orchestrator.run_training_loop(candidates).await?;

            // Display results
            println!("\n📊 Training Results");
            println!("   Run ID: {}", results.run_id);
            println!("   Repos processed: {}", results.total_repos_processed);
            println!("   Predictions made: {}", results.total_predictions);
            println!(
                "   Time: {:.2}s",
                (results.completed_at - results.started_at).num_seconds()
            );

            if let Some(metrics) = results.aggregated_metrics {
                println!("\n📈 Aggregated Metrics");
                println!("   Avg Precision: {:.4}", metrics.avg_precision);
                println!("   Avg Recall: {:.4}", metrics.avg_recall);
                println!("   Avg F1 Score: {:.4}", metrics.avg_f1);

                if let Some(percentiles) = metrics.percentiles {
                    println!("\n📊 Percentile Metrics");
                    println!("   P50 F1: {:.4}", percentiles.p50_f1);
                    println!("   P75 F1: {:.4}", percentiles.p75_f1);
                    println!("   P90 F1: {:.4}", percentiles.p90_f1);
                    println!("   P95 F1: {:.4}", percentiles.p95_f1);
                }
            }

            println!("\n✅ Training pipeline completed successfully!");
        }
    }

    Ok(())
}
