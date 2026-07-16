//! Regenerates the committed BAM test fixture used by the integration tests.
//!
//! Run with: `cargo run --example gen_sample_bam`
//!
//! The header below must stay in sync with `specs/bam.md` and
//! `src/bam/functions.rs` (`test_support::SAMPLE_SAM`). It is parsed from SAM text and
//! written as a header-only BAM, so the fixture is reproducible without external tools.

use noodles::{bam, sam};
use std::fs::File;

const SAMPLE_SAM: &str = "\
@HD\tVN:1.6\tSO:coordinate
@SQ\tSN:chr1\tLN:248956422
@RG\tID:H0164.1\tSM:NA12878\tLB:Solexa-272222\tPL:ILLUMINA\tPU:H0164ALXX140820.1
@RG\tID:H0164.2\tSM:NA12878\tLB:Solexa-272222\tPL:ILLUMINA\tPU:H0164ALXX140820.2
@PG\tID:bwa\tPN:bwa\tVN:0.7.17\tCL:bwa mem ref.fa reads.fq
@PG\tID:samtools\tPN:samtools\tPP:bwa\tVN:1.17\tCL:samtools sort
";

const OUTPUT: &str = "tests/data/sample.bam";

fn main() -> std::io::Result<()> {
    let mut reader = sam::io::Reader::new(SAMPLE_SAM.as_bytes());
    let header = reader.read_header()?;
    let mut writer = bam::io::Writer::new(File::create(OUTPUT)?);
    writer.write_header(&header)?;
    writer.try_finish()?;
    println!("wrote {OUTPUT}");
    Ok(())
}
