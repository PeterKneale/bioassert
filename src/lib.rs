//! `bioassert` library: the assertion engine behind the CLI.
//!
//! The crate is organised into layers, mirroring the assertion pipeline:
//! [`core`] (shared types and traits), the metric-executor families ([`file`],
//! [`delimited`], [`bam`], [`fasta`], [`text`]), and [`engine`] (parsing, dispatch, and
//! reporting). [`suggest`] is the inverse layer: it inspects a file and proposes a default set
//! of assertions, reusing the same property functions the executors evaluate.

pub mod bam;
pub mod core;
pub mod delimited;
pub mod engine;
pub mod fasta;
pub mod file;
pub mod suggest;
pub mod text;
