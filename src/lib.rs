//! `bioassert` library: the assertion engine behind the CLI.
//!
//! The crate is organised into four layers, mirroring the assertion pipeline:
//! [`core`] (shared types and traits), [`file`] and [`delimited`] (metric
//! executors), and [`engine`] (parsing, dispatch, and reporting).

pub mod bam;
pub mod core;
pub mod delimited;
pub mod engine;
pub mod fasta;
pub mod file;
