use crate::core::comparisons::Comparator;
use std::path::Path;

/// A single resolved assertion ready to run against a resource.
///
/// `locator` is the raw first token of the assertion (quotes already stripped). It is an
/// opaque string interpreted by whichever executor the metric selects: a `file.*`
/// executor treats it as a filesystem path (see [`Self::path`]), a `text.*` executor uses
/// it verbatim. The resource type is therefore decided by the metric namespace, not by
/// the locator itself.
pub struct AssertionRequest {
    pub locator: String,
    pub comparator: Comparator,
    pub expected: String,
}

impl AssertionRequest {
    /// Interprets the locator as a filesystem path, for the file-backed metric families
    /// (`file.*`, `tsv.*`, `bam.*`, `fasta.*`).
    pub fn path(&self) -> &Path {
        Path::new(&self.locator)
    }
}
