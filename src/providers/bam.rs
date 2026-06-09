//! BAM metric provider (noodles-backed).
//!
//! Spec: `docs/spec.md` → "BamProvider" and the BAM/CRAM/SAM metrics row.
//! Metrics: `read_count`, `mapped_reads`, `unmapped_reads`, `duplicate_reads`,
//! `secondary_reads`, `supplementary_reads`, `sort_order`, `read_group_count`, `sample_names`,
//! `has_index`, `reference_count`.
//!
//! A single streaming pass over records computes the flag-based counts; header-derived metrics
//! come from the parsed header (no record scan). `has_index` is a filesystem check only.
//!
//! (SAM (text) and CRAM are deferred to later increments.)

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use noodles_sam::{self as sam, alignment::record::Flags};

use super::MetricProvider;
use crate::model::Value;

/// Recognized BAM filename suffixes (case-insensitive).
const SUFFIXES: &[&str] = &[".bam"];

/// Flag-based record counts accumulated in one pass.
#[derive(Debug, Default, Clone, Copy)]
struct Counts {
    total: u64,
    mapped: u64,
    unmapped: u64,
    duplicate: u64,
    secondary: u64,
    supplementary: u64,
}

impl Counts {
    fn account(&mut self, flags: Flags) {
        self.total += 1;
        if flags.is_unmapped() {
            self.unmapped += 1;
        } else {
            self.mapped += 1;
        }
        if flags.is_duplicate() {
            self.duplicate += 1;
        }
        if flags.is_secondary() {
            self.secondary += 1;
        }
        if flags.is_supplementary() {
            self.supplementary += 1;
        }
    }
}

/// Header-derived statistics (no record scan required).
#[derive(Debug, Clone)]
struct HeaderStats {
    sort_order: String,
    read_group_count: u64,
    sample_names: Vec<String>,
    reference_count: u64,
}

/// Cached statistics from one pass over the BAM file.
#[derive(Debug, Clone)]
struct BamStats {
    counts: Counts,
    header: HeaderStats,
}

/// Provider for BAM files.
#[derive(Debug)]
pub struct BamProvider {
    path: PathBuf,
    stats: Option<BamStats>,
}

impl BamProvider {
    fn stats(&mut self) -> Result<&BamStats> {
        if self.stats.is_none() {
            self.stats = Some(self.compute()?);
        }
        Ok(self.stats.as_ref().expect("just computed"))
    }

    fn compute(&self) -> Result<BamStats> {
        let mut reader = noodles_bam::io::reader::Builder
            .build_from_path(&self.path)
            .with_context(|| format!("opening BAM {}", self.path.display()))?;
        let header = reader
            .read_header()
            .with_context(|| format!("reading BAM header from {}", self.path.display()))?;

        let header_stats = header_stats(&header);

        let mut counts = Counts::default();
        for result in reader.records() {
            let record = result
                .with_context(|| format!("reading BAM records from {}", self.path.display()))?;
            let flags = record.flags();
            counts.account(flags);
        }

        Ok(BamStats {
            counts,
            header: header_stats,
        })
    }

    /// Filesystem check for a companion index (`.bai`/`.csi`/`.crai`), no record scan.
    fn has_index(&self) -> bool {
        let candidates = [
            append_ext(&self.path, "bai"),   // sample.bam.bai
            append_ext(&self.path, "csi"),   // sample.bam.csi
            append_ext(&self.path, "crai"),  // sample.bam.crai
            self.path.with_extension("bai"), // sample.bai
        ];
        candidates.iter().any(|c| c.exists())
    }
}

/// Extract header-derived metrics from a parsed SAM/BAM header.
fn header_stats(header: &sam::Header) -> HeaderStats {
    let sort_order = header
        .header()
        .and_then(|hd| hd.other_fields().get(b"SO"))
        .map(|so| so.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let read_group_count = header.read_groups().len() as u64;

    let mut samples = BTreeSet::new();
    for read_group in header.read_groups().values() {
        if let Some(sample) = read_group.other_fields().get(b"SM") {
            samples.insert(sample.to_string());
        }
    }
    let sample_names = samples.into_iter().collect();

    let reference_count = header.reference_sequences().len() as u64;

    HeaderStats {
        sort_order,
        read_group_count,
        sample_names,
        reference_count,
    }
}

/// Append `.ext` to the full path (e.g. `sample.bam` → `sample.bam.bai`).
fn append_ext(path: &Path, ext: &str) -> PathBuf {
    let mut os: OsString = path.as_os_str().to_owned();
    os.push(".");
    os.push(ext);
    PathBuf::from(os)
}

impl MetricProvider for BamProvider {
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
            "read_count"
                | "mapped_reads"
                | "unmapped_reads"
                | "duplicate_reads"
                | "secondary_reads"
                | "supplementary_reads"
                | "sort_order"
                | "read_group_count"
                | "sample_names"
                | "has_index"
                | "reference_count"
        )
    }

    fn new(path: &Path) -> Result<Self> {
        Ok(BamProvider {
            path: path.to_path_buf(),
            stats: None,
        })
    }

    fn get(&mut self, metric: &str) -> Result<Value> {
        // `has_index` is a filesystem check and must not trigger a record scan.
        if metric == "has_index" {
            return Ok(Value::Bool(self.has_index()));
        }

        match metric {
            "read_count" => Ok(Value::Integer(self.stats()?.counts.total)),
            "mapped_reads" => Ok(Value::Integer(self.stats()?.counts.mapped)),
            "unmapped_reads" => Ok(Value::Integer(self.stats()?.counts.unmapped)),
            "duplicate_reads" => Ok(Value::Integer(self.stats()?.counts.duplicate)),
            "secondary_reads" => Ok(Value::Integer(self.stats()?.counts.secondary)),
            "supplementary_reads" => Ok(Value::Integer(self.stats()?.counts.supplementary)),
            "sort_order" => Ok(Value::String(self.stats()?.header.sort_order.clone())),
            "read_group_count" => Ok(Value::Integer(self.stats()?.header.read_group_count)),
            "sample_names" => {
                let names = self
                    .stats()?
                    .header
                    .sample_names
                    .iter()
                    .map(|n| Value::String(n.clone()))
                    .collect();
                Ok(Value::List(names))
            }
            "reference_count" => Ok(Value::Integer(self.stats()?.header.reference_count)),
            other => bail!("unknown metric `{other}` for BAM provider"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noodles_sam::alignment::RecordBuf;
    use noodles_sam::alignment::io::Write as _;
    use noodles_sam::header::record::value::{Map, map::ReferenceSequence};
    use std::num::NonZeroUsize;
    use tempfile::Builder;

    fn len(n: usize) -> NonZeroUsize {
        NonZeroUsize::try_from(n).unwrap()
    }

    /// Write a BAM file with two reference sequences and the given record flags (all unmapped).
    fn write_bam(flag_bits: &[u16]) -> tempfile::NamedTempFile {
        let header = sam::Header::builder()
            .add_reference_sequence("chr1", Map::<ReferenceSequence>::new(len(100)))
            .add_reference_sequence("chr2", Map::<ReferenceSequence>::new(len(200)))
            .build();

        let file = Builder::new().suffix(".bam").tempfile().expect("temp bam");
        {
            let mut writer =
                noodles_bam::io::Writer::new(std::fs::File::create(file.path()).unwrap());
            writer.write_header(&header).unwrap();
            for &bits in flag_bits {
                let record = RecordBuf::builder().set_flags(Flags::from(bits)).build();
                writer.write_alignment_record(&header, &record).unwrap();
            }
        }
        file
    }

    #[test]
    fn counts_account_flag_logic() {
        let mut c = Counts::default();
        c.account(Flags::from(0x0)); // mapped
        c.account(Flags::from(0x4)); // unmapped
        c.account(Flags::from(0x400)); // mapped + duplicate
        c.account(Flags::from(0x100)); // mapped + secondary
        c.account(Flags::from(0x800)); // mapped + supplementary
        assert_eq!(c.total, 5);
        assert_eq!(c.mapped, 4);
        assert_eq!(c.unmapped, 1);
        assert_eq!(c.duplicate, 1);
        assert_eq!(c.secondary, 1);
        assert_eq!(c.supplementary, 1);
    }

    #[test]
    fn supports_and_handles() {
        assert!(BamProvider::supports(Path::new("sample.bam")));
        assert!(BamProvider::supports(Path::new("SAMPLE.BAM")));
        assert!(!BamProvider::supports(Path::new("sample.sam")));
        assert!(BamProvider::handles("read_count"));
        assert!(BamProvider::handles("has_index"));
        assert!(!BamProvider::handles("variant_count"));
    }

    #[test]
    fn reads_counts_and_reference_count_from_real_bam() {
        // 4 unmapped records: plain, +duplicate, +secondary, +supplementary.
        let file = write_bam(&[0x4, 0x4 | 0x400, 0x4 | 0x100, 0x4 | 0x800]);
        let mut p = BamProvider::new(file.path()).unwrap();
        assert_eq!(p.get("read_count").unwrap(), Value::Integer(4));
        assert_eq!(p.get("unmapped_reads").unwrap(), Value::Integer(4));
        assert_eq!(p.get("mapped_reads").unwrap(), Value::Integer(0));
        assert_eq!(p.get("duplicate_reads").unwrap(), Value::Integer(1));
        assert_eq!(p.get("secondary_reads").unwrap(), Value::Integer(1));
        assert_eq!(p.get("supplementary_reads").unwrap(), Value::Integer(1));
        assert_eq!(p.get("reference_count").unwrap(), Value::Integer(2));
        // No @HD/@RG in the header.
        assert_eq!(
            p.get("sort_order").unwrap(),
            Value::String("unknown".into())
        );
        assert_eq!(p.get("read_group_count").unwrap(), Value::Integer(0));
        assert_eq!(p.get("sample_names").unwrap(), Value::List(vec![]));
    }

    #[test]
    fn has_index_detects_companion_bai() {
        let file = write_bam(&[0x4]);
        let mut p = BamProvider::new(file.path()).unwrap();
        assert_eq!(p.get("has_index").unwrap(), Value::Bool(false));

        // Create a companion `.bam.bai`.
        let bai = append_ext(file.path(), "bai");
        std::fs::write(&bai, b"\x00").unwrap();
        assert_eq!(p.get("has_index").unwrap(), Value::Bool(true));
        let _ = std::fs::remove_file(&bai);
    }

    #[test]
    fn unknown_metric_errors() {
        let file = write_bam(&[0x4]);
        let mut p = BamProvider::new(file.path()).unwrap();
        assert!(
            p.get("variant_count")
                .unwrap_err()
                .to_string()
                .contains("unknown metric")
        );
    }
}
