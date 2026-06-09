//! BioAssert: declaratively assert bioinformatics file properties.
//!
//! This crate exposes both a library API and a CLI binary. The authoritative behavior is
//! defined in [`docs/spec.md`](../docs/spec.md) and the staged delivery in
//! [`docs/implementation-plan.md`](../docs/implementation-plan.md).
//!
//! # Library API
//!
//! ```no_run
//! use std::collections::HashMap;
//! use std::path::PathBuf;
//!
//! let mut inputs = HashMap::new();
//! inputs.insert("bam".to_string(), PathBuf::from("sample.bam"));
//!
//! let report = bioassert::run_assertions("bam exists eq true", inputs)?;
//! println!("{} passed, {} failed", report.passed(), report.failed());
//! # Ok::<(), anyhow::Error>(())
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

pub mod cli;
pub mod engine;
pub mod exit;
pub mod model;
pub mod parser;
pub mod providers;
pub mod registry;

pub use engine::{MetricResolver, compare, evaluate};
pub use model::{Assertion, AssertionResult, Operator, Report, Status, Value};
pub use parser::{ParseError, ParsedBundle};
pub use providers::{
    BamProvider, FastaProvider, FastqProvider, GenericFileProvider, MetricProvider, VcfProvider,
};
pub use registry::MetricRegistry;

/// Options controlling evaluation behavior.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Evaluate all assertions and report every failure instead of stopping at the first
    /// (`--continue` / `--report-all`).
    pub continue_on_failure: bool,
}

/// Evaluate the given assertions against the bound inputs and return a [`Report`].
///
/// `source` is the raw assertion source (plain-text DSL or YAML). `inputs` maps virtual subject
/// names (e.g. `"bam"`) to physical file paths. Bindings declared inside a YAML bundle are merged
/// with `inputs`, with `inputs` (the caller's `--input` bindings) taking precedence.
///
/// Uses the default fail-fast behavior; see [`run_assertions_with`] for options.
pub fn run_assertions(source: &str, inputs: HashMap<String, PathBuf>) -> Result<Report> {
    run_assertions_with(source, inputs, &Options::default())
}

/// Like [`run_assertions`], but with explicit [`Options`].
pub fn run_assertions_with(
    source: &str,
    inputs: HashMap<String, PathBuf>,
    options: &Options,
) -> Result<Report> {
    let bundle = parser::parse(source)?;

    let mut resolved_inputs = bundle.inputs.clone();
    resolved_inputs.extend(inputs); // CLI `--input` overrides YAML `inputs`.

    log::debug!(
        "parsed {} assertion(s) with {} input binding(s)",
        bundle.assertions.len(),
        resolved_inputs.len()
    );

    let mut registry = registry::MetricRegistry::with_default_providers();
    engine::evaluate(
        &bundle.assertions,
        &resolved_inputs,
        &mut registry,
        options.continue_on_failure,
    )
}
