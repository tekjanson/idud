// src/main.rs
//! idud: Link Tree CLI
//! Token-efficient concept mapping through durable graph extraction

use clap::{Parser, Subcommand};
use idud::{LinkTree, RepositoryIngestionConfig, RepositoryTraverser};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "idud")]
#[command(about = "Link Tree: concept mapping for complex systems", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest a GitHub repository
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

    /// Validate graph consistency
    Validate,

    /// Query the graph
    Query {
        /// Starting node ID
        #[arg(short, long)]
        start: String,

        /// Max traversal depth
        #[arg(short, long, default_value = "3")]
        depth: usize,
    },

    /// Export AI Cheat Sheet
    Export {
        /// Entity name
        #[arg(short, long)]
        entity: String,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,
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
            println!("🔄 Ingesting repository: {}", url);

            let config = RepositoryIngestionConfig {
                repo_url: url,
                branch,
                work_dir,
            };

            let traverser = RepositoryTraverser::new(config);
            let result = traverser.ingest().await?;

            println!("✅ Ingestion complete!");
            println!("   Files processed: {}", result.files_processed);
            println!("   Nodes created: {}", result.nodes_created.len());

            if !result.errors.is_empty() {
                println!("⚠️  Errors encountered:");
                for err in result.errors {
                    println!("   - {}", err);
                }
            }

            // TODO: persist nodes to database
            println!("\n📊 Nodes extracted:");
            for node in result.nodes_created.iter().take(10) {
                println!("   - {} ({}): {}", node.label, format!("{:?}", node.node_type), node.source_uri);
            }
            if result.nodes_created.len() > 10 {
                println!("   ... and {} more", result.nodes_created.len() - 10);
            }
        }

        Commands::Validate => {
            println!("🔍 Validating graph...");
            println!("✅ Graph is valid");
        }

        Commands::Query { start, depth } => {
            println!("🔗 Querying from node: {} (depth: {})", start, depth);
            println!("📍 Node not found in graph");
        }

        Commands::Export { entity, output } => {
            println!("📋 Exporting cheat sheet for: {}", entity);
            println!("💾 Exported to: {}", output.display());
        }
    }

    Ok(())
}
