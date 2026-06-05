use anyhow::{bail, Result};

#[derive(Clone, Copy)]
pub enum BamMetric {
    ReadCount,
}

pub fn parse_bam_metric(s: &str) -> Result<BamMetric> {
    match s {
        "read_count" => Ok(BamMetric::ReadCount),
        _ => bail!("cannot parse metric '{}'", s),
    }
}