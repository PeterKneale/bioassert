use crate::core::{FileError, Value};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// A compression or container format recognised from a file's leading magic bytes.
///
/// `Bgzf` is the block-gzip variant used by samtools and tabix. Every bgzf file is also a
/// valid gzip file, so detection reports the more specific `Bgzf` in preference to `Gzip`
/// when the block markers are present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
    Bgzf,
    Bzip2,
    Xz,
    Zstd,
    Zip,
}

impl Compression {
    /// The lowercase label used as the `file.compression` metric value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Gzip => "gzip",
            Self::Bgzf => "bgzf",
            Self::Bzip2 => "bzip2",
            Self::Xz => "xz",
            Self::Zstd => "zstd",
            Self::Zip => "zip",
        }
    }

    /// True for any recognised compression or archive format (everything but `None`).
    pub fn is_compressed(self) -> bool {
        self != Self::None
    }
}

impl Display for Compression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// The fixed gzip header is 10 bytes; bgzf adds a 6-byte `BC` extra field, so 18 leading
// bytes are enough to recognise every format below and to spot a standard bgzf block.
const HEADER_LEN: usize = 18;

/// Classifies the compression or archive format of `file` from its leading magic bytes.
///
/// Reads at most [`HEADER_LEN`] bytes and never decompresses, so it stays cheap on
/// multi-gigabyte genomes. A file shorter than a signature, or one with no recognised
/// magic, classifies as [`Compression::None`]. Errors only if the file cannot be opened
/// or read, mirroring the other `file.*` metrics.
pub fn detect_compression(file: &Path) -> Result<Compression, FileError> {
    let mut header = [0u8; HEADER_LEN];
    let read = read_header(file, &mut header)?;
    Ok(classify(&header[..read]))
}

/// As the [`Value`] returned by the executor: the format label as a string.
pub fn compression(file: &Path) -> Result<Value, FileError> {
    Ok(Value::StringValue(detect_compression(file)?.as_str().to_string()))
}

/// As the [`Value`] returned by the executor: whether the file is compressed at all.
pub fn compressed(file: &Path) -> Result<Value, FileError> {
    Ok(Value::BooleanValue(detect_compression(file)?.is_compressed()))
}

/// Fills `buf` with the file's leading bytes, returning how many were read. Unlike
/// `read_exact`, a file shorter than `buf` is not an error: it fills what it can.
fn read_header(file: &Path, buf: &mut [u8]) -> Result<usize, FileError> {
    let mut f = File::open(file).map_err(|e| FileError::new(file, e))?;
    let mut filled = 0;
    while filled < buf.len() {
        match f.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) => return Err(FileError::new(file, e)),
        }
    }
    Ok(filled)
}

fn classify(bytes: &[u8]) -> Compression {
    if bytes.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        Compression::Zstd
    } else if bytes.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        Compression::Xz
    } else if bytes.starts_with(b"BZh") {
        Compression::Bzip2
    } else if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") || bytes.starts_with(b"PK\x07\x08") {
        Compression::Zip
    } else if bytes.starts_with(&[0x1f, 0x8b]) {
        if is_bgzf(bytes) {
            Compression::Bgzf
        } else {
            Compression::Gzip
        }
    } else {
        Compression::None
    }
}

/// A bgzf member is a gzip member (`1f 8b`) using deflate (`CM = 8`) with the FEXTRA flag
/// (`FLG & 0x04`) set and a `BC` extra subfield (`SI1 = 'B' = 0x42`, `SI2 = 'C' = 0x43`)
/// carrying the block size. This scans the extra field within the bytes we read for that
/// marker; a non-standard header whose `BC` field lies beyond [`HEADER_LEN`] falls back to
/// plain gzip rather than erroring.
fn is_bgzf(bytes: &[u8]) -> bool {
    // need the fixed 12-byte gzip header (through XLEN) before the extra field begins
    if bytes.len() < 12 || bytes[2] != 0x08 || bytes[3] & 0x04 == 0 {
        return false;
    }
    let xlen = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;
    let extra = &bytes[12..];
    let end = xlen.min(extra.len());
    let mut i = 0;
    // each subfield is SI1 SI2 SLEN(2 bytes, little-endian) followed by SLEN data bytes
    while i + 4 <= end {
        if extra[i] == 0x42 && extra[i + 1] == 0x43 {
            return true;
        }
        let slen = u16::from_le_bytes([extra[i + 2], extra[i + 3]]) as usize;
        i += 4 + slen;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn classifies_plain_gzip() {
        // 1f 8b 08 00 — deflate, no FEXTRA flag, so not bgzf
        assert_eq!(classify(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0, 3]), Compression::Gzip);
    }

    #[test]
    fn classifies_bgzf() {
        // 1f 8b 08 04 ... XLEN=6 ... BC 02 00 (block-size subfield) — the samtools/tabix variant
        let bgzf = [
            0x1f, 0x8b, 0x08, 0x04, 0, 0, 0, 0, 0, 0xff, 0x06, 0x00, 0x42, 0x43, 0x02, 0x00, 0x1b, 0x00,
        ];
        assert_eq!(classify(&bgzf), Compression::Bgzf);
    }

    #[test]
    fn gzip_with_fextra_but_no_bc_is_plain_gzip() {
        // FEXTRA set, XLEN=4, but the subfield is `AB`, not `BC`
        let gz = [0x1f, 0x8b, 0x08, 0x04, 0, 0, 0, 0, 0, 0xff, 0x04, 0x00, 0x41, 0x42, 0x00, 0x00];
        assert_eq!(classify(&gz), Compression::Gzip);
    }

    #[test]
    fn classifies_zstd() {
        assert_eq!(classify(&[0x28, 0xb5, 0x2f, 0xfd, 0x00]), Compression::Zstd);
    }

    #[test]
    fn classifies_xz() {
        assert_eq!(classify(&[0xfd, b'7', b'z', b'X', b'Z', 0x00, 0x00]), Compression::Xz);
    }

    #[test]
    fn classifies_bzip2() {
        assert_eq!(classify(b"BZh91AY"), Compression::Bzip2);
    }

    #[test]
    fn classifies_zip() {
        assert_eq!(classify(b"PK\x03\x04rest"), Compression::Zip);
        assert_eq!(classify(b"PK\x05\x06rest"), Compression::Zip);
        assert_eq!(classify(b"PK\x07\x08rest"), Compression::Zip);
    }

    #[test]
    fn classifies_uncompressed_as_none() {
        assert_eq!(classify(b"hello, world"), Compression::None);
    }

    #[test]
    fn classifies_empty_as_none() {
        assert_eq!(classify(&[]), Compression::None);
    }

    #[test]
    fn is_compressed_is_false_only_for_none() {
        assert!(!Compression::None.is_compressed());
        assert!(Compression::Gzip.is_compressed());
        assert!(Compression::Bgzf.is_compressed());
        assert!(Compression::Zip.is_compressed());
    }

    #[test]
    fn detects_format_from_a_real_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("x.gz");
        let mut f = File::create(&path).unwrap();
        f.write_all(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0, 3, b'r', b'e', b's', b't']).unwrap();
        assert_eq!(detect_compression(&path).unwrap(), Compression::Gzip);
        assert_eq!(compression(&path).unwrap(), Value::StringValue("gzip".to_string()));
        assert_eq!(compressed(&path).unwrap(), Value::BooleanValue(true));
    }

    #[test]
    fn uncompressed_file_reports_none_and_false() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("plain.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"just some text").unwrap();
        assert_eq!(detect_compression(&path).unwrap(), Compression::None);
        assert_eq!(compression(&path).unwrap(), Value::StringValue("none".to_string()));
        assert_eq!(compressed(&path).unwrap(), Value::BooleanValue(false));
    }

    #[test]
    fn short_file_does_not_error() {
        // a 1-byte file is shorter than any signature; it classifies as none, not an error
        let dir = tempdir().unwrap();
        let path = dir.path().join("tiny");
        File::create(&path).unwrap().write_all(&[0x1f]).unwrap();
        assert_eq!(detect_compression(&path).unwrap(), Compression::None);
    }

    #[test]
    fn missing_file_errors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.gz");
        assert!(detect_compression(&path).is_err());
    }
}
