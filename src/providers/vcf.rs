//! VCF metric provider (noodles-backed), with transparent gzip/bgzf support.
//!
//! Spec: `docs/spec.md` → "VcfProvider / BcfProvider" and the VCF/BCF metrics row.
//! Metrics: `variant_count`, `snp_count`, `indel_count`, `sample_count`, `contigs`,
//! `info_fields`, `format_fields`, `filter_fields`.
//!
//! (BCF — the binary encoding — is deferred to a later increment.)

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::{MetricProvider, open_reader};
use crate::model::Value;

/// Recognized VCF filename suffixes (case-insensitive), including bgzipped variants.
const SUFFIXES: &[&str] = &[".vcf", ".vcf.gz"];

/// Cached statistics from one pass over the VCF file.
#[derive(Debug, Clone)]
struct VcfStats {
    variant_count: u64,
    snp_count: u64,
    indel_count: u64,
    sample_count: u64,
    contigs: Vec<String>,
    info_fields: Vec<String>,
    format_fields: Vec<String>,
    filter_fields: Vec<String>,
}

/// Provider for VCF files (plain or bgzipped).
#[derive(Debug)]
pub struct VcfProvider {
    path: PathBuf,
    stats: Option<VcfStats>,
}

impl VcfProvider {
    fn stats(&mut self) -> Result<&VcfStats> {
        if self.stats.is_none() {
            self.stats = Some(self.compute()?);
        }
        Ok(self.stats.as_ref().expect("just computed"))
    }

    fn compute(&self) -> Result<VcfStats> {
        let inner = open_reader(&self.path)?;
        let mut reader = noodles_vcf::io::Reader::new(inner);
        let header = reader
            .read_header()
            .with_context(|| format!("reading VCF header from {}", self.path.display()))?;

        let sample_count = header.sample_names().len() as u64;
        let contigs = header.contigs().keys().cloned().collect();
        let info_fields = header.infos().keys().cloned().collect();
        let format_fields = header.formats().keys().cloned().collect();
        let filter_fields = header.filters().keys().cloned().collect();

        let mut variant_count = 0u64;
        let mut snp_count = 0u64;
        let mut indel_count = 0u64;

        for result in reader.records() {
            let record = result
                .with_context(|| format!("reading VCF records from {}", self.path.display()))?;
            variant_count += 1;

            let reference_bases = record.reference_bases();
            let ref_len = reference_bases.len();

            let alternate_bases = record.alternate_bases();
            let alts: &str = alternate_bases.as_ref();
            let mut all_alts_len_one = true;
            let mut any_len_differs = false;
            let mut saw_alt = false;
            for alt in alts.split(',') {
                if alt.is_empty() || alt == "." {
                    continue;
                }
                saw_alt = true;
                if alt.len() != 1 {
                    all_alts_len_one = false;
                }
                if alt.len() != ref_len {
                    any_len_differs = true;
                }
            }

            if saw_alt {
                if ref_len == 1 && all_alts_len_one {
                    snp_count += 1;
                } else if any_len_differs {
                    indel_count += 1;
                }
            }
        }

        Ok(VcfStats {
            variant_count,
            snp_count,
            indel_count,
            sample_count,
            contigs,
            info_fields,
            format_fields,
            filter_fields,
        })
    }
}

impl MetricProvider for VcfProvider {
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
            "variant_count"
                | "snp_count"
                | "indel_count"
                | "sample_count"
                | "contigs"
                | "info_fields"
                | "format_fields"
                | "filter_fields"
        )
    }

    fn new(path: &Path) -> Result<Self> {
        Ok(VcfProvider {
            path: path.to_path_buf(),
            stats: None,
        })
    }

    fn get(&mut self, metric: &str) -> Result<Value> {
        match metric {
            "variant_count" => Ok(Value::Integer(self.stats()?.variant_count)),
            "snp_count" => Ok(Value::Integer(self.stats()?.snp_count)),
            "indel_count" => Ok(Value::Integer(self.stats()?.indel_count)),
            "sample_count" => Ok(Value::Integer(self.stats()?.sample_count)),
            "contigs" => Ok(string_list(&self.stats()?.contigs)),
            "info_fields" => Ok(string_list(&self.stats()?.info_fields)),
            "format_fields" => Ok(string_list(&self.stats()?.format_fields)),
            "filter_fields" => Ok(string_list(&self.stats()?.filter_fields)),
            other => bail!("unknown metric `{other}` for VCF provider"),
        }
    }
}

/// Wrap a list of strings as a `Value::List`.
fn string_list(items: &[String]) -> Value {
    Value::List(items.iter().map(|s| Value::String(s.clone())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::Builder;

    fn vcf_file(contents: &str) -> tempfile::NamedTempFile {
        let mut file = Builder::new().suffix(".vcf").tempfile().expect("temp vcf");
        file.write_all(contents.as_bytes()).expect("write");
        file.flush().expect("flush");
        file
    }

    const SAMPLE: &str = "\
##fileformat=VCFv4.3
##contig=<ID=chr1>
##contig=<ID=chr2>
##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Depth\">
##FILTER=<ID=q10,Description=\"Quality below 10\">
##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">
#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1
chr1\t100\t.\tA\tT\t50\tPASS\tDP=10\tGT\t0/1
chr1\t200\t.\tAC\tA\t50\tPASS\tDP=10\tGT\t0/1
chr2\t300\t.\tG\tGTT\t50\tPASS\tDP=10\tGT\t1/1
";

    #[test]
    fn supports_vcf_suffixes() {
        assert!(VcfProvider::supports(Path::new("variants.vcf")));
        assert!(VcfProvider::supports(Path::new("variants.VCF.GZ")));
        assert!(!VcfProvider::supports(Path::new("variants.bcf")));
        assert!(!VcfProvider::supports(Path::new("variants.txt")));
    }

    #[test]
    fn counts_variants_snps_indels() {
        let file = vcf_file(SAMPLE);
        let mut p = VcfProvider::new(file.path()).unwrap();
        assert_eq!(p.get("variant_count").unwrap(), Value::Integer(3));
        assert_eq!(p.get("snp_count").unwrap(), Value::Integer(1));
        assert_eq!(p.get("indel_count").unwrap(), Value::Integer(2));
    }

    #[test]
    fn reads_header_metrics() {
        let file = vcf_file(SAMPLE);
        let mut p = VcfProvider::new(file.path()).unwrap();
        assert_eq!(p.get("sample_count").unwrap(), Value::Integer(1));
        assert_eq!(
            p.get("contigs").unwrap(),
            Value::List(vec![
                Value::String("chr1".into()),
                Value::String("chr2".into())
            ])
        );
        assert_eq!(
            p.get("info_fields").unwrap(),
            Value::List(vec![Value::String("DP".into())])
        );
        assert_eq!(
            p.get("format_fields").unwrap(),
            Value::List(vec![Value::String("GT".into())])
        );
        assert_eq!(
            p.get("filter_fields").unwrap(),
            Value::List(vec![Value::String("q10".into())])
        );
    }

    #[test]
    fn unknown_metric_errors() {
        let file = vcf_file(SAMPLE);
        let mut p = VcfProvider::new(file.path()).unwrap();
        assert!(
            p.get("read_count")
                .unwrap_err()
                .to_string()
                .contains("unknown metric")
        );
    }
}
