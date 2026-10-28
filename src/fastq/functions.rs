use crate::core::FileError;
use crate::file::compression::functions::{Compression, detect_compression};
use flate2::bufread::MultiGzDecoder;
use noodles::fastq;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// What one streaming pass over a FASTQ file found: how many records were read before the
/// pass stopped and, if the file is not valid FASTQ, why. Never holds record data.
#[derive(Debug, Clone, PartialEq)]
pub struct FastqScan {
    pub reads: u64,
    pub invalid: Option<String>,
}

impl FastqScan {
    pub fn is_valid(&self) -> bool {
        self.invalid.is_none()
    }
}

thread_local! {
    // Workflow-scoped scan cache, as for the fasta record cache: a `run` issues many `fastq.*`
    // assertions against the same file and each would otherwise re-stream it. Only the
    // fixed-size summary is stored, so memory is constant regardless of read count.
    static SCAN_CACHE: RefCell<HashMap<PathBuf, Rc<FastqScan>>> = RefCell::new(HashMap::new());
}

/// Streams a FASTQ file once and caches the summary per path. Errors only when the file cannot
/// be opened or uses an unsupported compression; a file that opens but does not parse is
/// reported as an invalid scan, not an error.
pub fn scan(file: &Path) -> Result<Rc<FastqScan>, FileError> {
    if let Some(scan) = SCAN_CACHE.with(|cache| cache.borrow().get(file).cloned()) {
        return Ok(scan);
    }
    let scan = Rc::new(validate(open(file)?));
    SCAN_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(file.to_path_buf(), Rc::clone(&scan))
    });
    Ok(scan)
}

#[cfg(test)]
pub(crate) fn clear_cache() {
    SCAN_CACHE.with(|cache| cache.borrow_mut().clear());
}

/// Opens the file for reading, transparently decompressing gzip and bgzf (sniffed from the
/// magic bytes, never from the extension).
fn open(file: &Path) -> Result<Box<dyn BufRead>, FileError> {
    let compression = detect_compression(file)?;
    let raw = File::open(file)
        .map(BufReader::new)
        .map_err(|e| FileError::new(file, e))?;
    match compression {
        Compression::None => Ok(Box::new(raw)),
        Compression::Gzip | Compression::Bgzf => {
            Ok(Box::new(BufReader::new(MultiGzDecoder::new(raw))))
        }
        other => Err(FileError::new(
            file,
            io::Error::new(
                io::ErrorKind::Unsupported,
                format!("{other} compression is not supported for fastq.* metrics"),
            ),
        )),
    }
}

/// Reads every record, stopping at the first that is malformed. The noodles reader checks the
/// `@` and `+` prefixes and a truncated record boundary; the quality-length check is ours,
/// since noodles accepts a quality string of any length (including a truncated empty one).
fn validate<R: BufRead>(reader: R) -> FastqScan {
    let mut reader = fastq::io::Reader::new(reader);
    let mut record = fastq::Record::default();
    let mut reads = 0u64;
    loop {
        match reader.read_record(&mut record) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                return invalid(reads, "unexpected end of file");
            }
            Err(e) => return invalid(reads, e),
        }
        if record.name().is_empty() {
            return invalid(reads, "empty name");
        }
        let (sequence, quality) = (record.sequence().len(), record.quality_scores().len());
        if sequence != quality {
            return invalid(
                reads,
                format!("quality length {quality} does not match sequence length {sequence}"),
            );
        }
        reads += 1;
    }
    FastqScan {
        reads,
        invalid: None,
    }
}

fn invalid(reads: u64, reason: impl Display) -> FastqScan {
    FastqScan {
        reads,
        invalid: Some(format!("record {reads} (line {}): {reason}", reads * 4 + 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    const VALID: &[u8] = b"@r1 1:N:0:ACGT\nACGTACGT\n+\nIIIIIIII\n@r2\nACGT\n+r2\nIIII\n";

    fn validate_bytes(bytes: &[u8]) -> FastqScan {
        validate(bytes)
    }

    #[test]
    fn valid_two_records() {
        let scan = validate_bytes(VALID);
        assert!(scan.is_valid());
        assert_eq!(scan.reads, 2);
    }

    #[test]
    fn valid_without_trailing_newline() {
        assert!(validate_bytes(b"@r1\nACGT\n+\nIIII").is_valid());
    }

    #[test]
    fn valid_with_crlf() {
        assert!(validate_bytes(b"@r1\r\nACGT\r\n+\r\nIIII\r\n").is_valid());
    }

    #[test]
    fn empty_file_is_valid_with_zero_reads() {
        let scan = validate_bytes(b"");
        assert!(scan.is_valid());
        assert_eq!(scan.reads, 0);
    }

    #[test]
    fn empty_sequence_is_valid() {
        assert!(validate_bytes(b"@r1\n\n+\n\n").is_valid());
    }

    #[test]
    fn quality_length_mismatch() {
        let scan = validate_bytes(b"@r1\nACGT\n+\nIIII\n@r2\nACGTACGT\n+\nIII\n");
        assert_eq!(
            scan.invalid.as_deref(),
            Some("record 1 (line 5): quality length 3 does not match sequence length 8")
        );
        assert_eq!(scan.reads, 1);
    }

    #[test]
    fn truncated_after_sequence() {
        let scan = validate_bytes(b"@r1\nACGT\n+\nIIII\n@r2\nACGT\n");
        assert_eq!(
            scan.invalid.as_deref(),
            Some("record 1 (line 5): unexpected end of file")
        );
        assert_eq!(scan.reads, 1);
    }

    #[test]
    fn truncated_after_plus() {
        let scan = validate_bytes(b"@r1\nACGT\n+\n");
        assert!(
            scan.invalid
                .unwrap()
                .starts_with("record 0 (line 1): quality length 0")
        );
    }

    #[test]
    fn missing_at_prefix() {
        let scan = validate_bytes(b"r1\nACGT\n+\nIIII\n");
        assert!(scan.invalid.unwrap().contains("invalid name prefix"));
    }

    #[test]
    fn missing_plus_line() {
        let scan = validate_bytes(b"@r1\nACGT\nIIII\n");
        assert!(scan.invalid.unwrap().contains("invalid description prefix"));
    }

    #[test]
    fn empty_name() {
        let scan = validate_bytes(b"@\nACGT\n+\nIIII\n");
        assert_eq!(
            scan.invalid.as_deref(),
            Some("record 0 (line 1): empty name")
        );
    }

    #[test]
    fn blank_line_between_records_is_invalid() {
        assert!(!validate_bytes(b"@r1\nACGT\n+\nIIII\n\n@r2\nACGT\n+\nIIII\n").is_valid());
    }

    #[test]
    fn fasta_is_invalid() {
        assert!(!validate_bytes(b">chr1\nACGT\n").is_valid());
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        use flate2::write::GzEncoder;
        let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        File::create(&path).unwrap().write_all(bytes).unwrap();
        path
    }

    #[test]
    fn scan_reads_gzip_transparently() {
        clear_cache();
        let dir = tempdir().unwrap();
        let path = write(dir.path(), "reads.fastq.gz", &gzip(VALID));
        let scan = scan(&path).unwrap();
        assert!(scan.is_valid());
        assert_eq!(scan.reads, 2);
    }

    #[test]
    fn scan_truncated_gzip_is_invalid() {
        clear_cache();
        let dir = tempdir().unwrap();
        let bytes = gzip(VALID);
        let path = write(dir.path(), "reads.fastq.gz", &bytes[..bytes.len() - 12]);
        let scan = scan(&path).unwrap();
        assert!(!scan.is_valid());
        assert!(
            scan.invalid
                .as_deref()
                .unwrap()
                .ends_with("unexpected end of file"),
            "{:?}",
            scan.invalid
        );
    }

    #[test]
    fn scan_rejects_unsupported_compression() {
        clear_cache();
        let err = scan(Path::new("tests/data/plain.txt.bz2")).unwrap_err();
        assert_eq!(err.source.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn scan_errors_on_missing_file() {
        clear_cache();
        assert!(scan(Path::new("tests/data/missing.fastq")).is_err());
    }

    #[test]
    fn scan_is_cached_per_path() {
        clear_cache();
        let dir = tempdir().unwrap();
        let path = write(dir.path(), "reads.fastq", VALID);
        let first = scan(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let second = scan(&path).unwrap();
        assert!(Rc::ptr_eq(&first, &second));
    }
}
