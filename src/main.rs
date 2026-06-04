//! BioAssert CLI - Command-line interface for the BioAssert library.

use anyhow::Result;
use clap::{Parser, Subcommand};

use bioassert::{
    assert_gc_content, assert_non_empty_sequence, assert_sequence_length, assert_valid_dna,
    assert_valid_protein, assert_valid_rna, gc_content,
};

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
    /// Validate a DNA sequence
    ValidateDna {
        /// The DNA sequence to validate
        sequence: String,
    },
    /// Validate an RNA sequence
    ValidateRna {
        /// The RNA sequence to validate
        sequence: String,
    },
    /// Validate a protein sequence
    ValidateProtein {
        /// The protein sequence to validate
        sequence: String,
    },
    /// Check the GC content of a DNA or RNA sequence
    GcContent {
        /// The sequence to analyse
        sequence: String,
        /// Minimum acceptable GC content (0.0 - 1.0)
        #[arg(long, default_value_t = 0.0)]
        min: f64,
        /// Maximum acceptable GC content (0.0 - 1.0)
        #[arg(long, default_value_t = 1.0)]
        max: f64,
    },
    /// Check the length of a sequence
    CheckLength {
        /// The sequence to check
        sequence: String,
        /// Minimum acceptable length
        #[arg(long, default_value_t = 1)]
        min: usize,
        /// Maximum acceptable length
        #[arg(long, default_value_t = usize::MAX)]
        max: usize,
    },
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::ValidateDna { sequence } => {
            assert_non_empty_sequence(&sequence)?;
            assert_valid_dna(&sequence)?;
            println!("DNA sequence is valid: {}", sequence);
        }
        Commands::ValidateRna { sequence } => {
            assert_non_empty_sequence(&sequence)?;
            assert_valid_rna(&sequence)?;
            println!("RNA sequence is valid: {}", sequence);
        }
        Commands::ValidateProtein { sequence } => {
            assert_non_empty_sequence(&sequence)?;
            assert_valid_protein(&sequence)?;
            println!("Protein sequence is valid: {}", sequence);
        }
        Commands::GcContent { sequence, min, max } => {
            assert_non_empty_sequence(&sequence)?;
            assert_gc_content(&sequence, min, max)?;
            let gc = gc_content(&sequence);
            println!("GC content: {:.4} ({:.2}%)", gc, gc * 100.0);
        }
        Commands::CheckLength { sequence, min, max } => {
            assert_sequence_length(&sequence, min, max)?;
            println!(
                "Sequence length {} is within range [{}, {}]",
                sequence.len(),
                min,
                max
            );
        }
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
mod tests {
    use super::*;

    #[test]
    fn test_run_validate_dna_valid() {
        let cli = Cli {
            command: Commands::ValidateDna {
                sequence: "ATGCN".to_string(),
            },
        };
        assert!(run(cli).is_ok());
    }

    #[test]
    fn test_run_validate_dna_invalid() {
        let cli = Cli {
            command: Commands::ValidateDna {
                sequence: "ATGX".to_string(),
            },
        };
        assert!(run(cli).is_err());
    }

    #[test]
    fn test_run_validate_rna_valid() {
        let cli = Cli {
            command: Commands::ValidateRna {
                sequence: "AUGCN".to_string(),
            },
        };
        assert!(run(cli).is_ok());
    }

    #[test]
    fn test_run_validate_rna_invalid() {
        let cli = Cli {
            command: Commands::ValidateRna {
                sequence: "AUGCT".to_string(),
            },
        };
        assert!(run(cli).is_err());
    }

    #[test]
    fn test_run_validate_protein_valid() {
        let cli = Cli {
            command: Commands::ValidateProtein {
                sequence: "MSTV".to_string(),
            },
        };
        assert!(run(cli).is_ok());
    }

    #[test]
    fn test_run_gc_content_in_range() {
        let cli = Cli {
            command: Commands::GcContent {
                sequence: "ATGC".to_string(),
                min: 0.4,
                max: 0.6,
            },
        };
        assert!(run(cli).is_ok());
    }

    #[test]
    fn test_run_gc_content_out_of_range() {
        let cli = Cli {
            command: Commands::GcContent {
                sequence: "AAAA".to_string(),
                min: 0.4,
                max: 0.6,
            },
        };
        assert!(run(cli).is_err());
    }

    #[test]
    fn test_run_check_length_valid() {
        let cli = Cli {
            command: Commands::CheckLength {
                sequence: "ATGC".to_string(),
                min: 1,
                max: 10,
            },
        };
        assert!(run(cli).is_ok());
    }

    #[test]
    fn test_run_check_length_invalid() {
        let cli = Cli {
            command: Commands::CheckLength {
                sequence: "ATGCATGCATGC".to_string(),
                min: 1,
                max: 5,
            },
        };
        assert!(run(cli).is_err());
    }

    #[test]
    fn test_run_validate_dna_empty() {
        let cli = Cli {
            command: Commands::ValidateDna {
                sequence: "".to_string(),
            },
        };
        assert!(run(cli).is_err());
    }
}
