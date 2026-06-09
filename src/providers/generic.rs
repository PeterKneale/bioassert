//! Generic file provider — works for any file regardless of format.
//!
//! Spec: `docs/spec.md` → "GenericFileProvider" and the "Generic (any)" metrics row.
//! Metrics: `exists`, `size`, `lines`, `md5`, `sha256`, `modified_time`.
//!
//! All metrics stream the file (constant memory) and cache their result for reuse, honoring the
//! performance guidance in `AGENTS.md`.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};
use md5::Md5;
use sha2::{Digest, Sha256};

use super::MetricProvider;
use crate::model::Value;

/// Read buffer size for streaming metrics.
const CHUNK: usize = 64 * 1024;

/// Provider for generic file checks. Recognized as a fallback for any path.
#[derive(Debug)]
pub struct GenericFileProvider {
    path: PathBuf,
    size: Option<u64>,
    lines: Option<u64>,
    md5: Option<String>,
    sha256: Option<String>,
    modified_time: Option<u64>,
}

impl GenericFileProvider {
    fn cached_size(&mut self) -> Result<u64> {
        if let Some(size) = self.size {
            return Ok(size);
        }
        let meta = std::fs::metadata(&self.path)
            .with_context(|| format!("reading metadata of {}", self.path.display()))?;
        let size = meta.len();
        self.size = Some(size);
        Ok(size)
    }

    fn cached_lines(&mut self) -> Result<u64> {
        if let Some(lines) = self.lines {
            return Ok(lines);
        }
        let mut reader = BufReader::new(self.open()?);
        let mut buf = [0u8; CHUNK];
        let mut newlines: u64 = 0;
        let mut last_byte: Option<u8> = None;
        loop {
            let n = reader
                .read(&mut buf)
                .with_context(|| format!("reading {}", self.path.display()))?;
            if n == 0 {
                break;
            }
            newlines += buf[..n].iter().filter(|&&b| b == b'\n').count() as u64;
            last_byte = Some(buf[n - 1]);
        }
        // Count a final line that is not terminated by a newline.
        let lines = match last_byte {
            None => 0,               // empty file
            Some(b'\n') => newlines, // ends with newline
            Some(_) => newlines + 1, // trailing partial line
        };
        self.lines = Some(lines);
        Ok(lines)
    }

    fn cached_md5(&mut self) -> Result<String> {
        if let Some(ref digest) = self.md5 {
            return Ok(digest.clone());
        }
        let mut hasher = Md5::new();
        self.hash_into(&mut hasher)?;
        let digest = to_hex(&hasher.finalize());
        self.md5 = Some(digest.clone());
        Ok(digest)
    }

    fn cached_sha256(&mut self) -> Result<String> {
        if let Some(ref digest) = self.sha256 {
            return Ok(digest.clone());
        }
        let mut hasher = Sha256::new();
        self.hash_into(&mut hasher)?;
        let digest = to_hex(&hasher.finalize());
        self.sha256 = Some(digest.clone());
        Ok(digest)
    }

    fn cached_modified_time(&mut self) -> Result<u64> {
        if let Some(t) = self.modified_time {
            return Ok(t);
        }
        let meta = std::fs::metadata(&self.path)
            .with_context(|| format!("reading metadata of {}", self.path.display()))?;
        let modified = meta
            .modified()
            .with_context(|| format!("reading mtime of {}", self.path.display()))?;
        let secs = modified
            .duration_since(UNIX_EPOCH)
            .context("file modified time is before the Unix epoch")?
            .as_secs();
        self.modified_time = Some(secs);
        Ok(secs)
    }

    fn open(&self) -> Result<File> {
        File::open(&self.path).with_context(|| format!("opening {}", self.path.display()))
    }

    fn hash_into<D: Digest>(&self, hasher: &mut D) -> Result<()> {
        let mut reader = BufReader::new(self.open()?);
        let mut buf = [0u8; CHUNK];
        loop {
            let n = reader
                .read(&mut buf)
                .with_context(|| format!("reading {}", self.path.display()))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(())
    }
}

impl MetricProvider for GenericFileProvider {
    /// The generic provider is the fallback and supports any path.
    fn supports(_path: &Path) -> bool {
        true
    }

    /// Generic metrics available for any file.
    fn handles(metric: &str) -> bool {
        matches!(
            metric,
            "exists" | "size" | "lines" | "md5" | "sha256" | "modified_time"
        )
    }

    /// Stores the path; does not open the file so that `exists` works on missing files.
    fn new(path: &Path) -> Result<Self> {
        Ok(GenericFileProvider {
            path: path.to_path_buf(),
            size: None,
            lines: None,
            md5: None,
            sha256: None,
            modified_time: None,
        })
    }

    fn get(&mut self, metric: &str) -> Result<Value> {
        match metric {
            "exists" => Ok(Value::Bool(self.path.exists())),
            "size" => Ok(Value::Integer(self.cached_size()?)),
            "lines" => Ok(Value::Integer(self.cached_lines()?)),
            "md5" => Ok(Value::String(self.cached_md5()?)),
            "sha256" => Ok(Value::String(self.cached_sha256()?)),
            "modified_time" => Ok(Value::Integer(self.cached_modified_time()?)),
            other => bail!("unknown metric `{other}` for generic file provider"),
        }
    }
}

/// Lowercase hex-encode a byte slice.
fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn provider_with(contents: &[u8]) -> (NamedTempFile, GenericFileProvider) {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(contents).expect("write");
        file.flush().expect("flush");
        let provider = GenericFileProvider::new(file.path()).expect("new");
        (file, provider)
    }

    #[test]
    fn exists_true_for_present_file() {
        let (_f, mut p) = provider_with(b"hello\n");
        assert_eq!(p.get("exists").unwrap(), Value::Bool(true));
    }

    #[test]
    fn exists_false_for_missing_file_without_error() {
        let mut p = GenericFileProvider::new(Path::new("/no/such/bioassert/file")).unwrap();
        assert_eq!(p.get("exists").unwrap(), Value::Bool(false));
    }

    #[test]
    fn size_counts_bytes() {
        let (_f, mut p) = provider_with(b"hello\n");
        assert_eq!(p.get("size").unwrap(), Value::Integer(6));
    }

    #[test]
    fn lines_counts_terminated_and_partial_lines() {
        let (_f, mut p) = provider_with(b"hello\n");
        assert_eq!(p.get("lines").unwrap(), Value::Integer(1));

        let (_f2, mut p2) = provider_with(b"a\nb");
        assert_eq!(p2.get("lines").unwrap(), Value::Integer(2));

        let (_f3, mut p3) = provider_with(b"");
        assert_eq!(p3.get("lines").unwrap(), Value::Integer(0));
    }

    #[test]
    fn md5_matches_known_golden() {
        // `printf 'hello\n' | md5sum`
        let (_f, mut p) = provider_with(b"hello\n");
        assert_eq!(
            p.get("md5").unwrap(),
            Value::String("b1946ac92492d2347c6235b4d2611184".into())
        );
    }

    #[test]
    fn sha256_matches_known_golden() {
        // `printf 'hello\n' | sha256sum`
        let (_f, mut p) = provider_with(b"hello\n");
        assert_eq!(
            p.get("sha256").unwrap(),
            Value::String(
                "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03".into()
            )
        );
    }

    #[test]
    fn modified_time_is_returned() {
        let (_f, mut p) = provider_with(b"hello\n");
        match p.get("modified_time").unwrap() {
            Value::Integer(t) => assert!(t > 0),
            other => panic!("expected integer, got {other}"),
        }
    }

    #[test]
    fn unknown_metric_errors() {
        let (_f, mut p) = provider_with(b"hello\n");
        let err = p.get("read_count").unwrap_err();
        assert!(err.to_string().contains("unknown metric"), "got: {err}");
    }

    #[test]
    fn size_on_missing_file_errors() {
        let mut p = GenericFileProvider::new(Path::new("/no/such/bioassert/file")).unwrap();
        assert!(p.get("size").is_err());
    }

    #[test]
    fn metrics_are_cached() {
        let (_f, mut p) = provider_with(b"hello\n");
        let first = p.get("sha256").unwrap();
        let second = p.get("sha256").unwrap();
        assert_eq!(first, second);
        assert!(p.sha256.is_some());
    }
}
