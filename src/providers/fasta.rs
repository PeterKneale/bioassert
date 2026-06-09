//! FASTA metric provider (noodles-backed).
//!
//! Spec: `docs/spec.md` → "FastaProvider" and the FASTA metrics row.
//! Metrics: `sequence_count`, `total_bases`, `longest_sequence`, `sequence_names`,
//! `no_duplicate_sequence_names`.
//!
//! A single streaming pass computes all record statistics, which are cached for reuse.

use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::MetricProvider;
use crate::model::Value;

/// Recognized plain-text FASTA extensions.
const EXTENSIONS: &[&str] = &["fa", "fasta", "fna"];

/// Cached statistics from one pass over the FASTA file.
#[derive(Debug, Clone)]
struct FastaStats {
    sequence_count: u64,
    total_bases: u64,
    longest_sequence: u64,
    names: Vec<String>,
}

/// Provider for FASTA files.
#[derive(Debug)]
pub struct FastaProvider {
    path: PathBuf,
    stats: Option<FastaStats>,
}

impl FastaProvider {
    fn stats(&mut self) -> Result<&FastaStats> {
        if self.stats.is_none() {
            self.stats = Some(self.compute()?);
        }
        Ok(self.stats.as_ref().expect("just computed"))
    }

    fn compute(&self) -> Result<FastaStats> {
        let file =
            File::open(&self.path).with_context(|| format!("opening {}", self.path.display()))?;
        let mut reader = noodles_fasta::io::Reader::new(BufReader::new(file));

        let mut sequence_count = 0u64;
        let mut total_bases = 0u64;
        let mut longest_sequence = 0u64;
        let mut names = Vec::new();

        for result in reader.records() {
            let record = result
                .with_context(|| format!("reading FASTA records from {}", self.path.display()))?;
            sequence_count += 1;
            let len = record.sequence().len() as u64;
            total_bases += len;
            longest_sequence = longest_sequence.max(len);
            names.push(String::from_utf8_lossy(record.name()).into_owned());
        }

        Ok(FastaStats {
            sequence_count,
            total_bases,
            longest_sequence,
            names,
        })
    }
}

impl MetricProvider for FastaProvider {
    fn supports(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
    }

    fn handles(metric: &str) -> bool {
        matches!(
            metric,
            "sequence_count"
                | "total_bases"
                | "longest_sequence"
                | "sequence_names"
                | "no_duplicate_sequence_names"
        )
    }

    fn new(path: &Path) -> Result<Self> {
        Ok(FastaProvider {
            path: path.to_path_buf(),
            stats: None,
        })
    }

    fn get(&mut self, metric: &str) -> Result<Value> {
        match metric {
            "sequence_count" => Ok(Value::Integer(self.stats()?.sequence_count)),
            "total_bases" => Ok(Value::Integer(self.stats()?.total_bases)),
            "longest_sequence" => Ok(Value::Integer(self.stats()?.longest_sequence)),
            "sequence_names" => {
                let names = self
                    .stats()?
                    .names
                    .iter()
                    .map(|n| Value::String(n.clone()))
                    .collect();
                Ok(Value::List(names))
            }
            "no_duplicate_sequence_names" => {
                let names = &self.stats()?.names;
                let mut seen = HashSet::new();
                let unique = names.iter().all(|n| seen.insert(n.as_str()));
                Ok(Value::Bool(unique))
            }
            other => bail!("unknown metric `{other}` for FASTA provider"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::Builder;

    fn fasta_file(contents: &str) -> tempfile::NamedTempFile {
        let mut file = Builder::new().suffix(".fa").tempfile().expect("temp fasta");
        file.write_all(contents.as_bytes()).expect("write");
        file.flush().expect("flush");
        file
    }

    const SAMPLE: &str = ">seq1 first sequence\nACGTACGT\nACGT\n>seq2\nGGGG\n";

    #[test]
    fn supports_fasta_extensions() {
        assert!(FastaProvider::supports(Path::new("ref.fa")));
        assert!(FastaProvider::supports(Path::new("ref.fasta")));
        assert!(FastaProvider::supports(Path::new("ref.FNA")));
        assert!(!FastaProvider::supports(Path::new("ref.txt")));
        assert!(!FastaProvider::supports(Path::new("ref.bam")));
    }

    #[test]
    fn counts_sequences_and_bases() {
        let file = fasta_file(SAMPLE);
        let mut p = FastaProvider::new(file.path()).unwrap();
        assert_eq!(p.get("sequence_count").unwrap(), Value::Integer(2));
        assert_eq!(p.get("total_bases").unwrap(), Value::Integer(16));
        assert_eq!(p.get("longest_sequence").unwrap(), Value::Integer(12));
    }

    #[test]
    fn sequence_names_strip_description() {
        let file = fasta_file(SAMPLE);
        let mut p = FastaProvider::new(file.path()).unwrap();
        assert_eq!(
            p.get("sequence_names").unwrap(),
            Value::List(vec![
                Value::String("seq1".into()),
                Value::String("seq2".into()),
            ])
        );
    }

    #[test]
    fn detects_duplicate_names() {
        let file = fasta_file(SAMPLE);
        let mut p = FastaProvider::new(file.path()).unwrap();
        assert_eq!(
            p.get("no_duplicate_sequence_names").unwrap(),
            Value::Bool(true)
        );

        let dup = fasta_file(">seq1\nAAAA\n>seq1\nCCCC\n");
        let mut p2 = FastaProvider::new(dup.path()).unwrap();
        assert_eq!(
            p2.get("no_duplicate_sequence_names").unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn unknown_metric_errors() {
        let file = fasta_file(SAMPLE);
        let mut p = FastaProvider::new(file.path()).unwrap();
        assert!(
            p.get("read_count")
                .unwrap_err()
                .to_string()
                .contains("unknown metric")
        );
    }
}
