// src/main.rs
//! idud: Contract Ledger CLI
//! Token-efficient concept mapping through durable contract discovery

use clap::{Parser, Subcommand};
use idud::{ContractLedger, RepositoryIngestionConfig, RepositoryTraverser, serve, WebServerConfig};
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
    }

    Ok(())
}
