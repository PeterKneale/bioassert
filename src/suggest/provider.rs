use crate::core::BioAssertError;
use crate::suggest::Suggestion;
use crate::suggest::providers::{BamProvider, DelimitedProvider, FastaProvider, FileProvider};
use std::path::Path;

/// A source of suggested assertions for one resource family. There is one provider per family
/// (not per executor): the provider reads the file once and emits the family's whole default
/// set, tagging each suggestion with its full metric string so the writer can group by prefix.
pub trait SuggestionProvider {
    /// A short stable name for the family, used in warning messages.
    fn name(&self) -> &'static str;

    /// Whether this provider applies to `path`. The format providers decide purely from the
    /// extension; [`FileProvider`] handles any path.
    fn handles(&self, path: &Path) -> bool;

    /// Computes the family's suggestions for `path`. Called only when [`handles`] is true. An
    /// `Err` means the file matched the family but its properties could not be read (e.g. a
    /// `.bam` that does not parse); the orchestrator records it as a warning and continues.
    ///
    /// [`handles`]: SuggestionProvider::handles
    fn suggest(&self, path: &Path) -> Result<Vec<Suggestion>, BioAssertError>;
}

/// The ordered provider registry, broadest first. The order mirrors the dispatch order in
/// `engine::executor`, so suggested files and run-time dispatch agree on precedence.
pub fn providers() -> Vec<Box<dyn SuggestionProvider>> {
    vec![
        Box::new(FileProvider),
        Box::new(DelimitedProvider),
        Box::new(BamProvider),
        Box::new(FastaProvider),
    ]
}
