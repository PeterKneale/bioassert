use crate::core::FileError;
use noodles::{bam, sam};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::Rc;

thread_local! {
    // Workflow-scoped header cache: a `run` over an assertions file issues many `bam.*`
    // assertions against the same file, and each would otherwise re-open, bgzf-decompress,
    // and re-parse the header. The binary runs once per invocation, so a process-scoped
    // thread-local map is workflow-scoped in practice. Keeping it here means the
    // `AssertionExecutor` trait and the rest of the codebase stay untouched.
    static HEADER_CACHE: RefCell<HashMap<PathBuf, Rc<sam::Header>>> = RefCell::new(HashMap::new());
}

/// Reads and parses the SAM header of a BAM file, caching the result so the header for a
/// given path is parsed exactly once per invocation. Only successful reads are cached;
/// errors (missing file, not a BAM) are cheap and rare, so they are re-attempted.
pub fn read_header(file: &Path) -> Result<Rc<sam::Header>, FileError> {
    if let Some(header) = HEADER_CACHE.with(|cache| cache.borrow().get(file).cloned()) {
        return Ok(header);
    }
    let mut reader = File::open(file)
        .map(bam::io::Reader::new)
        .map_err(|e| FileError::new(file, e))?;
    let header = Rc::new(reader.read_header().map_err(|e| FileError::new(file, e))?);
    HEADER_CACHE.with(|cache| cache.borrow_mut().insert(file.to_path_buf(), Rc::clone(&header)));
    Ok(header)
}

/// Clears the header cache. Test-only so on-the-fly fixtures cannot observe each other.
#[cfg(test)]
pub(crate) fn clear_cache() {
    HEADER_CACHE.with(|cache| cache.borrow_mut().clear());
}

/// Number of `@RG` read-group records.
pub fn read_group_count(header: &sam::Header) -> u64 {
    header.read_groups().len() as u64
}

/// Number of `@SQ` reference-sequence records.
pub fn reference_count(header: &sam::Header) -> u64 {
    header.reference_sequences().len() as u64
}

/// Number of `@PG` program records.
pub fn program_count(header: &sam::Header) -> u64 {
    header.programs().as_ref().len() as u64
}

/// Whether a read group exists at the given 0-based index.
pub fn read_group_present(header: &sam::Header, index: usize) -> bool {
    header.read_groups().get_index(index).is_some()
}

/// Value of a read-group tag at the given 0-based index. `id` resolves to the read-group
/// key; all other tags are looked up in the record's other fields. Returns `None` when the
/// index is out of range or the tag is not set.
pub fn read_group_tag(header: &sam::Header, index: usize, tag: &str) -> Option<String> {
    let (id, map) = header.read_groups().get_index(index)?;
    if tag.eq_ignore_ascii_case("id") {
        return Some(id.to_string());
    }
    let want = tag_bytes(tag)?;
    map.other_fields()
        .iter()
        .find(|&(key, _)| key.as_ref() == &want)
        .map(|(_, value)| value.to_string())
}

/// Value of an `@HD` field. `vn` resolves to the typed version; other fields (e.g. `so`)
/// are looked up in the header's other fields. Returns `None` when there is no `@HD` line
/// or the field is not set.
pub fn hd_field(header: &sam::Header, field: &str) -> Option<String> {
    let hd = header.header()?;
    if field.eq_ignore_ascii_case("vn") {
        return Some(hd.version().to_string());
    }
    let want = tag_bytes(field)?;
    hd.other_fields()
        .iter()
        .find(|&(key, _)| key.as_ref() == &want)
        .map(|(_, value)| value.to_string())
}

/// Normalises a 2-letter metric tag to the uppercase byte pair used by SAM tags.
fn tag_bytes(tag: &str) -> Option<[u8; 2]> {
    let bytes = tag.as_bytes();
    (bytes.len() == 2).then(|| [bytes[0].to_ascii_uppercase(), bytes[1].to_ascii_uppercase()])
}

#[cfg(test)]
pub(crate) mod test_support {
    use noodles::{bam, sam};
    use tempfile::NamedTempFile;

    // Mirrors specs/bam.md and the committed tests/data/sample.bam fixture.
    pub const SAMPLE_SAM: &str = "\
@HD\tVN:1.6\tSO:coordinate
@SQ\tSN:chr1\tLN:248956422
@RG\tID:H0164.1\tSM:NA12878\tLB:Solexa-272222\tPL:ILLUMINA\tPU:H0164ALXX140820.1
@RG\tID:H0164.2\tSM:NA12878\tLB:Solexa-272222\tPL:ILLUMINA\tPU:H0164ALXX140820.2
@PG\tID:bwa\tPN:bwa\tVN:0.7.17\tCL:bwa mem ref.fa reads.fq
";

    /// Parses SAM header text and writes it as a (header-only) BAM into a temp file.
    pub fn write_bam(sam_text: &str) -> NamedTempFile {
        let mut sam_reader = sam::io::Reader::new(sam_text.as_bytes());
        let header = sam_reader.read_header().expect("parse SAM header");
        let file = NamedTempFile::new().expect("temp file");
        let mut writer = bam::io::Writer::new(file.reopen().expect("reopen temp file"));
        writer.write_header(&header).expect("write BAM header");
        writer.try_finish().expect("finish BAM");
        file
    }

    /// A temp BAM holding the sample header above.
    pub fn sample_bam() -> NamedTempFile {
        write_bam(SAMPLE_SAM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn counts_records() {
        let bam = test_support::sample_bam();
        let header = read_header(bam.path()).unwrap();
        assert_eq!(read_group_count(&header), 2);
        assert_eq!(reference_count(&header), 1);
        assert_eq!(program_count(&header), 1);
    }

    #[test]
    fn reads_read_group_tags() {
        let bam = test_support::sample_bam();
        let header = read_header(bam.path()).unwrap();
        assert_eq!(read_group_tag(&header, 0, "id").as_deref(), Some("H0164.1"));
        assert_eq!(read_group_tag(&header, 1, "id").as_deref(), Some("H0164.2"));
        assert_eq!(read_group_tag(&header, 0, "sm").as_deref(), Some("NA12878"));
        assert_eq!(read_group_tag(&header, 0, "lb").as_deref(), Some("Solexa-272222"));
        assert_eq!(read_group_tag(&header, 0, "pl").as_deref(), Some("ILLUMINA"));
        assert_eq!(read_group_tag(&header, 0, "pu").as_deref(), Some("H0164ALXX140820.1"));
        assert_eq!(read_group_tag(&header, 1, "pu").as_deref(), Some("H0164ALXX140820.2"));
    }

    #[test]
    fn tag_lookup_is_case_insensitive() {
        let bam = test_support::sample_bam();
        let header = read_header(bam.path()).unwrap();
        assert_eq!(read_group_tag(&header, 0, "SM").as_deref(), Some("NA12878"));
    }

    #[test]
    fn missing_tag_is_none() {
        let bam = test_support::sample_bam();
        let header = read_header(bam.path()).unwrap();
        assert_eq!(read_group_tag(&header, 0, "dt"), None);
        assert_eq!(read_group_tag(&header, 0, "cn"), None);
    }

    #[test]
    fn out_of_range_read_group() {
        let bam = test_support::sample_bam();
        let header = read_header(bam.path()).unwrap();
        assert!(read_group_present(&header, 0));
        assert!(read_group_present(&header, 1));
        assert!(!read_group_present(&header, 2));
        assert_eq!(read_group_tag(&header, 2, "sm"), None);
    }

    #[test]
    fn reads_hd_fields() {
        let bam = test_support::sample_bam();
        let header = read_header(bam.path()).unwrap();
        assert_eq!(hd_field(&header, "vn").as_deref(), Some("1.6"));
        assert_eq!(hd_field(&header, "so").as_deref(), Some("coordinate"));
    }

    #[test]
    fn read_header_errors_on_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.bam");
        assert!(read_header(&missing).is_err());
    }

    #[test]
    fn read_header_caches_and_reuses() {
        clear_cache();
        let bam = test_support::sample_bam();
        let path = bam.path().to_path_buf();
        let first = read_header(&path).unwrap();

        // Remove the underlying file: a second read must still succeed from cache and
        // return the very same Rc, proving the header was not parsed again.
        bam.close().unwrap();
        let second = read_header(&path).unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        clear_cache();
    }

    #[test]
    fn read_header_errors_on_non_bam() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"this is not a bam file").unwrap();
        file.flush().unwrap();
        clear_cache();
        assert!(read_header(file.path()).is_err());
    }
}
