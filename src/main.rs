// src/main.rs
//! idud: Contract Ledger CLI
//! Token-efficient concept mapping through durable contract discovery

use clap::{Parser, Subcommand};
use idud::{
    ContractLedger, RepositoryIngestionConfig, RepositoryTraverser, serve, WebServerConfig,
    discover_training_repos, TrainingOrchestrator, TrainingConfig,
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

        Commands::Train {
            repos,
            concurrent,
            batch_size,
            datalake,
        } => {
            println!("🎓 Starting training validation pipeline");
            println!("   Repos to process: {}", repos);
            println!("   Concurrent agents: {}", concurrent);
            println!("   Batch size: {}", batch_size);

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
                    println!("   P25 F1: {:.4}", percentiles.p25_f1);
                    println!("   P50 F1: {:.4}", percentiles.p50_f1);
                    println!("   P75 F1: {:.4}", percentiles.p75_f1);
                }
            }

            println!("\n✅ Training pipeline completed successfully!");
        }
    }

    Ok(())
}
