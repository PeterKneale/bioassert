use crate::parse::parse_int;
use crate::{parse_comparator, Comparator};
use anyhow::{bail, Result};
use noodles::bam;
use std::fs::File;
use std::io;
use std::path::Path;

pub fn handle_read_count(file: &Path, comparator: String, expected: String) -> Result<()> {
    let comparator = parse_comparator(&comparator)?;
    let expected = parse_int(&expected)?;

    assert_read_count(file, comparator, expected)
}
fn assert_read_count(file: &Path, comparator: Comparator, expected: u64) -> Result<()> {
    let actual = count_reads(file)?;
    let assertion = format!("read count {comparator} {expected}");

    if comparator.compare(actual, expected) {
        println!("Assertion OK. {assertion}");
        Ok(())
    } else {
        bail!("Assertion failed. Expected: {assertion}, actual: {actual}");
    }
}
fn count_reads(file: &Path) -> io::Result<u64> {
    let mut reader = File::open(file).map(bam::io::Reader::new)?;
    reader.read_header()?;

    let mut n = 0_u64;

    for result in reader.records() {
        let _ = result?;
        n += 1;
    }

    Ok(n)
}
