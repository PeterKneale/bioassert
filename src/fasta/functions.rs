use crate::core::FileError;
use noodles::fasta;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// A per-record digest of a FASTA file: just enough to answer `fasta.*` assertions without
/// holding the sequence bytes. A reference genome may be many gigabytes, so the sequence is
/// read and discarded one record at a time, keeping only name, description, and length.
#[derive(Debug, Clone, PartialEq)]
pub struct FastaRecord {
    pub name: String,
    pub description: Option<String>,
    pub length: u64,
}

thread_local! {
    // Workflow-scoped record cache: a `run` over an assertions file issues many `fasta.*`
    // assertions against the same file, and each would otherwise re-open and re-scan it. The
    // binary runs once per invocation, so a process-scoped thread-local map is workflow-scoped
    // in practice. Unlike the BAM header cache, this stores only the per-record digest (never
    // the sequence bytes), so memory stays bounded regardless of genome size. Keeping it here
    // means the `AssertionExecutor` trait and the rest of the codebase stay untouched.
    static RECORD_CACHE: RefCell<HashMap<PathBuf, Rc<Vec<FastaRecord>>>> = RefCell::new(HashMap::new());
}

/// Reads a FASTA file into a vector of per-record digests, caching the result so a given path
/// is scanned exactly once per invocation. Only successful reads are cached; errors (missing
/// file, not FASTA) are cheap and rare, so they are re-attempted.
pub fn read_records(file: &Path) -> Result<Rc<Vec<FastaRecord>>, FileError> {
    if let Some(records) = RECORD_CACHE.with(|cache| cache.borrow().get(file).cloned()) {
        return Ok(records);
    }
    let mut reader = File::open(file)
        .map(BufReader::new)
        .map(fasta::io::Reader::new)
        .map_err(|e| FileError::new(file, e))?;
    let mut records = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| FileError::new(file, e))?;
        records.push(FastaRecord {
            name: String::from_utf8_lossy(record.name()).into_owned(),
            description: record.description().map(|d| d.to_string()),
            length: record.sequence().len() as u64,
        });
    }
    let records = Rc::new(records);
    RECORD_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(file.to_path_buf(), Rc::clone(&records))
    });
    Ok(records)
}

/// Clears the record cache. Test-only so on-the-fly fixtures cannot observe each other.
#[cfg(test)]
pub(crate) fn clear_cache() {
    RECORD_CACHE.with(|cache| cache.borrow_mut().clear());
}

/// Number of sequence records.
pub fn record_count(records: &[FastaRecord]) -> u64 {
    records.len() as u64
}

/// Total bases summed across every record.
pub fn total_length(records: &[FastaRecord]) -> u64 {
    records.iter().map(|r| r.length).sum()
}

/// Whether a record exists at the given 0-based index.
pub fn record_present(records: &[FastaRecord], index: usize) -> bool {
    index < records.len()
}

/// Name of the record at the given 0-based index. `None` when the index is out of range.
pub fn record_name(records: &[FastaRecord], index: usize) -> Option<String> {
    records.get(index).map(|r| r.name.clone())
}

/// Description of the record at the given 0-based index. `None` when the index is out of range
/// or the record has no description.
pub fn record_description(records: &[FastaRecord], index: usize) -> Option<String> {
    records.get(index).and_then(|r| r.description.clone())
}

/// Length in bases of the record at the given 0-based index. `None` when the index is out of range.
pub fn record_length(records: &[FastaRecord], index: usize) -> Option<u64> {
    records.get(index).map(|r| r.length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Mirrors specs/fasta.md and the committed tests/data/sample.fasta fixture:
    // chr1 (28 bases over two lines), chr2 (no description, 10 bases),
    // NC_000001.11 (4 bases); total 42.
    const SAMPLE_FASTA: &str = "\
>chr1 Homo sapiens chromosome 1
ACGTACGTACGTACGTACGT
ACGTACGT
>chr2
ACGTACGTAC
>NC_000001.11 alternate assembly
ACGT
";

    fn write_fasta(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(content.as_bytes()).expect("write fasta");
        file.flush().expect("flush fasta");
        file
    }

    fn sample_fasta() -> NamedTempFile {
        write_fasta(SAMPLE_FASTA)
    }

    #[test]
    fn counts_and_total_length() {
        clear_cache();
        let fasta = sample_fasta();
        let records = read_records(fasta.path()).unwrap();
        assert_eq!(record_count(&records), 3);
        assert_eq!(total_length(&records), 42);
    }

    #[test]
    fn counts_zero_records_for_empty_file() {
        clear_cache();
        let fasta = write_fasta("");
        let records = read_records(fasta.path()).unwrap();
        assert_eq!(record_count(&records), 0);
        assert_eq!(total_length(&records), 0);
    }

    #[test]
    fn reads_record_names() {
        clear_cache();
        let fasta = sample_fasta();
        let records = read_records(fasta.path()).unwrap();
        assert_eq!(record_name(&records, 0).as_deref(), Some("chr1"));
        assert_eq!(record_name(&records, 1).as_deref(), Some("chr2"));
        assert_eq!(record_name(&records, 2).as_deref(), Some("NC_000001.11"));
        assert_eq!(record_name(&records, 3), None);
    }

    #[test]
    fn reads_record_descriptions() {
        clear_cache();
        let fasta = sample_fasta();
        let records = read_records(fasta.path()).unwrap();
        assert_eq!(
            record_description(&records, 0).as_deref(),
            Some("Homo sapiens chromosome 1")
        );
        assert_eq!(record_description(&records, 1), None);
        assert_eq!(
            record_description(&records, 2).as_deref(),
            Some("alternate assembly")
        );
    }

    #[test]
    fn reads_record_lengths() {
        clear_cache();
        let fasta = sample_fasta();
        let records = read_records(fasta.path()).unwrap();
        assert_eq!(record_length(&records, 0), Some(28));
        assert_eq!(record_length(&records, 1), Some(10));
        assert_eq!(record_length(&records, 2), Some(4));
        assert_eq!(record_length(&records, 3), None);
    }

    #[test]
    fn reports_presence() {
        clear_cache();
        let fasta = sample_fasta();
        let records = read_records(fasta.path()).unwrap();
        assert!(record_present(&records, 0));
        assert!(record_present(&records, 2));
        assert!(!record_present(&records, 3));
    }

    #[test]
    fn read_records_errors_on_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.fasta");
        assert!(read_records(&missing).is_err());
    }

    #[test]
    fn read_records_errors_on_non_fasta() {
        clear_cache();
        let fasta = write_fasta("this is not a fasta file\n");
        assert!(read_records(fasta.path()).is_err());
    }

    #[test]
    fn read_records_caches_and_reuses() {
        clear_cache();
        let fasta = sample_fasta();
        let path = fasta.path().to_path_buf();
        let first = read_records(&path).unwrap();

        // Remove the underlying file: a second read must still succeed from cache and return
        // the very same Rc, proving the file was not scanned again.
        fasta.close().unwrap();
        let second = read_records(&path).unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        clear_cache();
    }

    #[test]
    fn clear_cache_forces_a_reread() {
        clear_cache();
        let fasta = sample_fasta();
        let path = fasta.path().to_path_buf();
        let _first = read_records(&path).unwrap();

        // Clearing the cache and removing the file means the next read must hit the filesystem
        // and fail, proving the first read was really served from cache.
        fasta.close().unwrap();
        clear_cache();
        assert!(read_records(&path).is_err());
    }
}
