# BioAssert

> Declaratively assert bioinformatics file properties (BAM/CRAM, FASTQ, FASTA, VCF/BCF, …) in
> pipelines — a fast, statically-linked Rust CLI + library, built on the
> [noodles](https://docs.rs/noodles/latest/noodles/) ecosystem.

BioAssert evaluates a small assertion DSL (plain text or YAML) against named file inputs and
reports pass/fail via **exit codes**, **stdout/stderr**, and an always-written **log file** —
ideal for Nextflow/nf-core and other pipelines.

## Quick Start

### 1. Install

```bash
# From crates.io (once published)
cargo install bioassert

# Or build from source
cargo build --release
```

### 2. Write an assertion file

Save as `aligned_bam.assert` (also in [`examples/`](examples/aligned_bam.assert)):

```text
# assertions for an aligned BAM bound as `bam`
bam exists eq true
bam size gte 100MB
bam read_count gt 100000
bam sort_order eq coordinate
bam read_group_count gte 1
bam has_index eq true
```

### 3. Run BioAssert

Bind the virtual subject `bam` to a real file and run:

```bash
bioassert \
    --assertions aligned_bam.assert \
    --input bam=sample.bam
```

### Expected output

On success (exit code `0`):

```
[PASS] bam (sample.bam) exists eq true
[PASS] bam (sample.bam) read_count=5321 gt 100000
...
6 passed, 0 failed
```

On failure (exit code `1`):

```
[FAIL] bam (sample.bam) sort_order=unknown, expected 'coordinate'
```

A full execution log is written to `bioassert.log` (override with `--log-file <path>`).

## Assertion DSL (summary)

Each line is `‹subject› ‹metric› ‹operator› ‹value›`:

- **Subjects** are virtual names bound with `--input name=path` (e.g. `bam`, `read1`, `vcf`).
- **Operators:** `eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `in`, `not_in`, `contains`, `matches`.
- **Boolean metrics** (e.g. `exists`, `has_index`) are explicit: `bam has_index eq true`.
- **Relational/cross-subject:** `read1 paired_with eq read2`, `read1 read_count eq read2`.
- **Sizes** use SI suffixes: `KB`, `MB`, `GB`, `TB`.

YAML bundles are also supported — see the spec.

## CLI

```
bioassert [OPTIONS] --assertions <file> --input <name=path>...

  -a, --assertions <file>    Path to assertion file (text or YAML), repeatable
  -i, --input <name=path>    Bind input name to file, repeatable
  -l, --log-file <file>      Write execution logs (default: bioassert.log)
  -c, --continue             Continue after failures (report all)
  -q, --quiet                Minimal logging
  -v, --verbose              Verbose logging
  -h, --help                 Print help
  -V, --version              Print version
```

**Exit codes:** `0` all passed · `1` one or more failed · `2` usage/config error.

## Development

```bash
cargo fmt
cargo clippy --all-targets
cargo test
```

## License

Licensed under either of Apache-2.0 or MIT at your option.

