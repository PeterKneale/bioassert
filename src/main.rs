mod cli;

use std::error::Error;
use clap::Parser;
use cli::{Cli, Commands};
use std::fs;
use bioassert::parser::Assertion;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Assert { assertion } => {
            let assertion = bioassert::parser::parse_raw_assertion(&assertion)?;
            execute(assertion);
            Ok(())
        }

        Commands::Run { file } => {
            println!("Running assertions in {}", file.as_path().display());
            let contents = fs::read_to_string(file)?;
            match bioassert::parser::parse_file(&contents) {
                Ok(assertions) => {
                    for assertion in assertions {
                        execute(assertion);
                    }
                }
                Err(err) => {
                    eprintln!("💥{}", err);
                    return Err(err);
                }
            }
            Ok(())
        }
    }
}

fn execute(assertion: Assertion) {
    if let Err(error) = bioassert::executor::execute(assertion) {
        eprintln!("ERROR. {}", error);
    }
}
