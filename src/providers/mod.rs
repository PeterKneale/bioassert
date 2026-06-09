//! Metric providers.
//!
//! Spec: `docs/spec.md` → "Plugin/Metric Provider API". Each file format implements
//! [`MetricProvider`]; the metric registry (Phase 4) selects a provider per file and dispatches
//! metric lookups to it.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};

use crate::model::Value;

pub mod bam;
pub mod fasta;
pub mod fastq;
pub mod generic;
pub mod vcf;

pub use bam::BamProvider;
pub use fasta::FastaProvider;
pub use fastq::FastqProvider;
pub use generic::GenericFileProvider;
pub use vcf::VcfProvider;

/// Open a file as a buffered reader, transparently decompressing gzip (`.gz`) inputs.
///
/// Uses `MultiGzDecoder` so multi-member gzip streams (common for concatenated FASTQ) are read in
/// full.
pub(crate) fn open_reader(path: &Path) -> Result<Box<dyn BufRead>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let is_gzip = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gz"))
        .unwrap_or(false);
    if is_gzip {
        Ok(Box::new(BufReader::new(flate2::read::MultiGzDecoder::new(
            file,
        ))))
    } else {
        Ok(Box::new(BufReader::new(file)))
    }
}

/// A source of metric values for a single file.
pub trait MetricProvider {
    /// Does this provider recognize (support) the given file?
    fn supports(path: &Path) -> bool
    where
        Self: Sized;

    /// Does this provider compute the named metric? Used by the registry to route a metric to
    /// the owning provider without constructing it.
    fn handles(metric: &str) -> bool
    where
        Self: Sized;

    /// Initialize a context for the file (open handles / parse headers as needed).
    fn new(path: &Path) -> Result<Self>
    where
        Self: Sized;

    /// Compute or retrieve a metric value.
    fn get(&mut self, metric: &str) -> Result<Value>;
}
