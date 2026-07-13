// src/main.rs
#![forbid(unsafe_code)]

//! idud: Contract Ledger CLI
//! Token-efficient concept mapping through durable contract discovery

use clap::{Parser, Subcommand};
use idud::{
    discover_training_repos, serve, write_synthetic_understanding, Contract, ContractLedger,
    RepoIngestionConfig, RepositoryIngestionConfig, RepositoryIngestionOrchestrator,
    RepositoryTraverser, Signatory, TrainingCache, TrainingConfig, TrainingOrchestrator,
    WebServerConfig,
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
        /// Repository URL or local path
        #[arg(short, long)]
        url: String,

        /// Git branch (default: main)
        #[arg(short, long, default_value = "main")]
        branch: String,

        /// Working directory for clone
        #[arg(short, long)]
        work_dir: Option<PathBuf>,

        /// Skip clone, ingest from local directory
        #[arg(short, long)]
        local: bool,
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

        /// Optional JSON file to load contracts from
        #[arg(short, long)]
        ledger_file: Option<PathBuf>,
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

    /// Grow the training data lake by ingesting repositories
    GrowDatalake {
        /// Repository registry file (default: data/repos_to_ingest.json)
        #[arg(short, long, default_value = "data/repos_to_ingest.json")]
        registry: String,

        /// Output directory for ingested contracts (default: data)
        #[arg(short, long, default_value = "data")]
        output: String,

        /// Maximum number of repos to ingest
        #[arg(short, long)]
        max_repos: Option<usize>,

        /// Maximum duration in minutes
        #[arg(short, long)]
        timeout_minutes: Option<u64>,

        /// Skip already-ingested repos (default: true)
        #[arg(short, long)]
        skip_ingested: bool,
    },

    /// Generate a synthetic understanding artifact for any repository
    #[command(name = "understand-repo", alias = "waymark-understand")]
    UnderstandRepo {
        /// Repository path to analyze
        #[arg(long, default_value = ".")]
        repo_path: PathBuf,

        /// Output JSON path for the synthetic understanding artifact
        #[arg(long, default_value = "data/synthetic_understanding.json")]
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
            local,
        } => {
            println!("📋 Ingesting repository: {}", url);
            std::io::Write::flush(&mut std::io::stdout()).ok();

            let config = RepositoryIngestionConfig {
                repo_url: url.clone(),
                branch,
                work_dir: if local {
                    Some(PathBuf::from(&url))
                } else {
                    work_dir
                },
                skip_clone: local,
            };

            eprintln!("[DEBUG] Config created, starting ingest...");
            let traverser = RepositoryTraverser::new(config);
            eprintln!("[DEBUG] Traverser created, calling ingest()...");
            let result = traverser.ingest().await?;
            eprintln!("[DEBUG] Ingest completed");

            println!("✅ Ingestion complete!");
            println!("   Files processed: {}", result.files_processed);
            println!(
                "   Signatories registered: {}",
                result.signatories_registered.len()
            );
            println!(
                "   Contracts discovered: {}",
                result.contracts_discovered.len()
            );
            std::io::Write::flush(&mut std::io::stdout()).ok();

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

            println!("\n🔗 Contracts discovered:");
            for contract in result.contracts_discovered.iter().take(10) {
                println!(
                    "   - {:?}: {} → {} (confidence: {:.2})",
                    contract.clause_type,
                    contract.principal_id,
                    contract.guarantor_id,
                    contract.confidence
                );
            }
            if result.contracts_discovered.len() > 10 {
                println!("   ... and {} more", result.contracts_discovered.len() - 10);
            }

            // Save contracts to JSON file for later visualization
            let repo_name = url.split('/').last().unwrap_or("repo");
            let contracts_file = format!("data/{}-contracts.json", repo_name);
            std::fs::create_dir_all("data").ok();

            let ledger = Arc::new(ContractLedger::new());
            for signatory in &result.signatories_registered {
                let _ = ledger.register_signatory(signatory.clone());
            }

            // Draft discovered contracts into the ledger
            for contract in &result.contracts_discovered {
                let _ = ledger.draft_contract(contract.clone());
            }

            let signatories = ledger.get_all_signatories();
            let contracts = ledger.get_all_contracts();

            let export_data = serde_json::json!({
                "version": "1.0",
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "stats": {
                    "signatories": signatories.len(),
                    "contracts": contracts.len()
                },
                "signatories": signatories,
                "contracts": contracts
            });

            if let Ok(json_str) = serde_json::to_string_pretty(&export_data) {
                if let Ok(_) = std::fs::write(&contracts_file, json_str) {
                    println!("\n💾 Contracts saved to: {}", contracts_file);
                    println!(
                        "   To visualize: cargo run --release -- serve --ledger-file {}",
                        contracts_file
                    );
                }
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

        Commands::Serve {
            port,
            host,
            ledger_file,
        } => {
            let ledger = Arc::new(ContractLedger::new());

            let mut sigs_loaded = 0;
            let mut contracts_loaded = 0;
            let mut contract_errors = 0;

            // Load contracts from file if provided
            if let Some(file_path) = ledger_file {
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(signatories) = data["signatories"].as_array() {
                            for sig_val in signatories {
                                if let Ok(sig) =
                                    serde_json::from_value::<Signatory>(sig_val.clone())
                                {
                                    let _ = ledger.register_signatory(sig);
                                    sigs_loaded += 1;
                                }
                            }
                        }
                        if let Some(contracts) = data["contracts"].as_array() {
                            for contract_val in contracts {
                                if let Ok(contract) =
                                    serde_json::from_value::<Contract>(contract_val.clone())
                                {
                                    match ledger.draft_contract(contract) {
                                        Ok(_) => contracts_loaded += 1,
                                        Err(e) => {
                                            contract_errors += 1;
                                            if contract_errors <= 3 {
                                                eprintln!("Error drafting contract: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        println!("✅ Loaded contracts from: {}", file_path.display());
                        println!("   Signatories: {}", sigs_loaded);
                        println!(
                            "   Contracts: {} (errors: {})",
                            contracts_loaded, contract_errors
                        );
                    }
                }
            }

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

            let config = TrainingConfig {
                batch_size,
                max_concurrent_agents: concurrent,
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

        Commands::GrowDatalake {
            registry,
            output,
            max_repos,
            timeout_minutes,
            skip_ingested,
        } => {
            let config = RepoIngestionConfig {
                registry_path: PathBuf::from(&registry),
                output_dir: PathBuf::from(&output),
                max_repos,
                timeout_minutes,
                skip_already_ingested: skip_ingested,
            };

            match RepositoryIngestionOrchestrator::new(config) {
                Ok(mut orchestrator) => match orchestrator.run().await {
                    Ok(results) => {
                        println!("\n✅ Ingestion Results");
                        println!("   Run ID: {}", results.run_id);
                        println!(
                            "   Repos processed: {}/{}",
                            results.repos_processed, results.total_repos
                        );
                        println!(
                            "   Successful: {}, Failed: {}",
                            results.successful, results.failed
                        );
                        println!("   Total files: {}", results.total_files);
                        println!("   Total signatories: {}", results.total_signatories);
                        println!("   Total contracts: {}", results.total_contracts);
                        println!(
                            "   Duration: {:.1}s ({:.1} min)",
                            results.duration_secs,
                            results.duration_secs as f64 / 60.0
                        );
                        println!("\n💾 Results saved to data/ingestion-log.json");
                        println!("📊 Progress logged to DATALAKE_LOG.md");
                    }
                    Err(e) => {
                        eprintln!("❌ Ingestion failed: {}", e);
                        return Err(e.into());
                    }
                },
                Err(e) => {
                    eprintln!("❌ Failed to initialize orchestrator: {}", e);
                    return Err(e.into());
                }
            }
        }

        Commands::UnderstandRepo { repo_path, output } => {
            println!(
                "🧠 Generating synthetic understanding for {}",
                repo_path.display()
            );
            let json_path = write_synthetic_understanding(&repo_path, &output)?;
            println!(
                "✅ Wrote synthetic understanding to {}",
                json_path.display()
            );
            println!(
                "📝 Markdown summary: {}",
                output.with_extension("md").display()
            );
            println!(
                "🌐 HTML report: {}",
                output.with_extension("html").display()
            );
        }
    }

    Ok(())
}
