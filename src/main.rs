//! BioAssert CLI - Command-line interface for the BioAssert library.

/// bioassert bam a.bam reads eq 10
use anyhow::Result;
use bioassert::bam::assertions::handle;
use clap::{Parser, Subcommand};
use std::path::Path;

/// BioAssert - A bioinformatics assertion and validation tool.
#[derive(Parser)]
#[command(
    name = "bioassert",
    about = "BioAssert - A bioinformatics assertion and validation tool",
    version,
    author
)]

struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Bam {
        file: String,
        metric: String,
        comparator: String,
        expected: String,
    },
    Fasta {
        filepath: String,
        metric: String,
        comparator: String,
        expected: String,
    },
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Bam {
            file,
            metric,
            comparator,
            expected,
        } => {
            let file = Path::new(&file);

            handle(file, metric, comparator, expected)?;
            println!("OK");
        }
        _ => {}
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {}
