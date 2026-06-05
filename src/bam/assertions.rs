use std::path::Path;
use crate::bam::metrics::{parse_bam_metric, BamMetric};
use crate::bam::read_count::handle_read_count;
use crate::common::file::assert_file_exists;
use anyhow::{Result};
pub fn handle(file: &Path, metric: String, comparator: String, expected: String) -> Result<()> {
    assert_file_exists(file)?;
    match parse_bam_metric(&metric)? {
        BamMetric::ReadCount => {
            handle_read_count(file, comparator, expected)?;
            Ok(())
        }
    }
}

