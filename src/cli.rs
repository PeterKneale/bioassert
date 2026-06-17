use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

}
#[derive(Subcommand)]
pub enum Commands {
    Assert {
        assertion: String,
    },
    Run {
        file: PathBuf,
    },
}