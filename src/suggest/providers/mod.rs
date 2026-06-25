mod bam;
mod delimited;
mod fasta;
mod file;

pub use bam::BamProvider;
pub use delimited::DelimitedProvider;
pub use fasta::FastaProvider;
pub use file::FileProvider;

use std::path::Path;

/// The lowercased file extension, e.g. `Some("tsv")` for `reads.TSV`. `None` when the path has
/// no extension. Used by the format providers to decide which family handles a file.
fn extension(path: &Path) -> Option<String> {
    path.extension().map(|e| e.to_string_lossy().to_lowercase())
}
