//! FASTQ metric provider (noodles-backed), with transparent gzip support.
//!
//! Spec: `docs/spec.md` → "FastqProvider" and the FASTQ metrics row.
//! Metrics: `read_count`, `average_read_length`, `quality_encoding`, `paired_with`.
//!
//! A single streaming pass computes all statistics, which are cached for reuse.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use super::{MetricProvider, open_reader};
use crate::model::Value;

/// Recognized FASTQ filename suffixes (case-insensitive), including gzip variants.
const SUFFIXES: &[&str] = &[".fastq", ".fq", ".fastq.gz", ".fq.gz"];

/// Cached statistics from one pass over the FASTQ file.
#[derive(Debug, Clone)]
struct FastqStats {
    read_count: u64,
    total_length: u64,
    min_qual: Option<u8>,
    max_qual: Option<u8>,
    /// SHA-256 (hex) of the ordered, mate-suffix-normalized read names. Equal for paired files.
    pairing_key: String,
}

/// Provider for FASTQ files (plain or gzipped).
#[derive(Debug)]
pub struct FastqProvider {
    path: PathBuf,
    stats: Option<FastqStats>,
}

impl FastqProvider {
    fn stats(&mut self) -> Result<&FastqStats> {
        if self.stats.is_none() {
            self.stats = Some(self.compute()?);
        }
        Ok(self.stats.as_ref().expect("just computed"))
    }

    fn compute(&self) -> Result<FastqStats> {
        let reader = open_reader(&self.path)?;
        let mut reader = noodles_fastq::io::Reader::new(reader);

        let mut read_count = 0u64;
        let mut total_length = 0u64;
        let mut min_qual: Option<u8> = None;
        let mut max_qual: Option<u8> = None;
        let mut hasher = Sha256::new();

        for result in reader.records() {
            let record = result
                .with_context(|| format!("reading FASTQ records from {}", self.path.display()))?;
            read_count += 1;
            total_length += record.sequence().len() as u64;

            for &q in record.quality_scores() {
                min_qual = Some(min_qual.map_or(q, |m| m.min(q)));
                max_qual = Some(max_qual.map_or(q, |m| m.max(q)));
            }

            hasher.update(normalize_name(record.name()));
            hasher.update(b"\n");
        }

        Ok(FastqStats {
            read_count,
            total_length,
            min_qual,
            max_qual,
            pairing_key: to_hex(&hasher.finalize()),
        })
    }
}

impl MetricProvider for FastqProvider {
    fn supports(path: &Path) -> bool {
        match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => {
                let lower = name.to_ascii_lowercase();
                SUFFIXES.iter().any(|s| lower.ends_with(s))
            }
            None => false,
        }
    }

    fn handles(metric: &str) -> bool {
        matches!(
            metric,
            "read_count" | "average_read_length" | "quality_encoding" | "paired_with"
        )
    }

    fn new(path: &Path) -> Result<Self> {
        Ok(FastqProvider {
            path: path.to_path_buf(),
            stats: None,
        })
    }

    fn get(&mut self, metric: &str) -> Result<Value> {
        match metric {
            "read_count" => Ok(Value::Integer(self.stats()?.read_count)),
            "average_read_length" => {
                let stats = self.stats()?;
                let avg = if stats.read_count == 0 {
                    0.0
                } else {
                    stats.total_length as f64 / stats.read_count as f64
                };
                Ok(Value::Float(avg))
            }
            "quality_encoding" => Ok(Value::String(self.stats()?.quality_encoding().to_string())),
            "paired_with" => Ok(Value::String(self.stats()?.pairing_key.clone())),
            other => bail!("unknown metric `{other}` for FASTQ provider"),
        }
    }
}

impl FastqStats {
    /// Heuristic Phred offset detection from observed quality-score ASCII range.
    fn quality_encoding(&self) -> &'static str {
        match (self.min_qual, self.max_qual) {
            (Some(min), Some(max)) => {
                if min < 64 {
                    "phred+33"
                } else if max > 74 {
                    "phred+64"
                } else {
                    // Ambiguous range; assume modern Phred+33.
                    "phred+33"
                }
            }
            _ => "unknown",
        }
    }
}

/// Strip a trailing `/1` or `/2` mate suffix from a read name.
fn normalize_name(name: &[u8]) -> &[u8] {
    if name.ends_with(b"/1") || name.ends_with(b"/2") {
        &name[..name.len() - 2]
    } else {
        name
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
    use tempfile::Builder;

    fn fastq_file(contents: &str) -> tempfile::NamedTempFile {
        let mut file = Builder::new()
            .suffix(".fastq")
            .tempfile()
            .expect("temp fastq");
        file.write_all(contents.as_bytes()).expect("write");
        file.flush().expect("flush");
        file
    }

    // Two reads, lengths 4 and 8 → avg 6; low quality (ASCII 33) → phred+33.
    const SAMPLE: &str = "@r1/1\nACGT\n+\n!!!!\n@r2/1\nACGTACGT\n+\n!!!!!!!!\n";

    #[test]
    fn supports_fastq_suffixes() {
        assert!(FastqProvider::supports(Path::new("reads.fastq")));
        assert!(FastqProvider::supports(Path::new("reads.fq")));
        assert!(FastqProvider::supports(Path::new("reads_R1.FASTQ.GZ")));
        assert!(FastqProvider::supports(Path::new("reads.fq.gz")));
        assert!(!FastqProvider::supports(Path::new("reads.fa")));
        assert!(!FastqProvider::supports(Path::new("reads.txt")));
    }

    #[test]
    fn counts_reads_and_average_length() {
        let file = fastq_file(SAMPLE);
        let mut p = FastqProvider::new(file.path()).unwrap();
        assert_eq!(p.get("read_count").unwrap(), Value::Integer(2));
        assert_eq!(p.get("average_read_length").unwrap(), Value::Float(6.0));
    }

    #[test]
    fn detects_phred33() {
        let file = fastq_file(SAMPLE);
        let mut p = FastqProvider::new(file.path()).unwrap();
        assert_eq!(
            p.get("quality_encoding").unwrap(),
            Value::String("phred+33".into())
        );
    }

    #[test]
    fn detects_phred64() {
        // Quality chars 'A'(65)..'h'(104): min>=64 and max>74 → phred+64.
        let file = fastq_file("@r1\nACGT\n+\nAAhh\n");
        let mut p = FastqProvider::new(file.path()).unwrap();
        assert_eq!(
            p.get("quality_encoding").unwrap(),
            Value::String("phred+64".into())
        );
    }

    #[test]
    fn paired_files_share_pairing_key() {
        let r1 = fastq_file("@r1/1\nACGT\n+\n!!!!\n@r2/1\nACGT\n+\n!!!!\n");
        let r2 = fastq_file("@r1/2\nTTTT\n+\n!!!!\n@r2/2\nTTTT\n+\n!!!!\n");
        let mut p1 = FastqProvider::new(r1.path()).unwrap();
        let mut p2 = FastqProvider::new(r2.path()).unwrap();
        assert_eq!(
            p1.get("paired_with").unwrap(),
            p2.get("paired_with").unwrap()
        );

        let mismatched = fastq_file("@x1/2\nTTTT\n+\n!!!!\n");
        let mut p3 = FastqProvider::new(mismatched.path()).unwrap();
        assert_ne!(
            p1.get("paired_with").unwrap(),
            p3.get("paired_with").unwrap()
        );
    }

    #[test]
    fn unknown_metric_errors() {
        let file = fastq_file(SAMPLE);
        let mut p = FastqProvider::new(file.path()).unwrap();
        assert!(
            p.get("sequence_count")
                .unwrap_err()
                .to_string()
                .contains("unknown metric")
        );
    }
}
