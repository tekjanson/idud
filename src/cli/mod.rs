use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// A thin CLI surface for graph inspection and topology queries.
#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Parse a Rust source file into a structural topology.
    Parse { path: PathBuf },
    /// Show the stored raw source for a pointer hash.
    Show { hash: String },
}

/// Execute the graph-oriented CLI facade.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CliCommand::Parse { path } => {
            println!("Parsing {}", path.display());
        }
        CliCommand::Show { hash } => {
            println!("Show raw node for {}", hash);
        }
    }
    Ok(())
}
