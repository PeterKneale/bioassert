# Specification: FASTQ assertions

## Context

`bioassert` validates pipeline outputs with declarative assertions (`<file> <metric> <comparator> <value>`). It
covers generic files (`file.*`), delimited text (`csv/tsv/psv.*`), BAM headers (`bam.header.*`) and FASTA records
(`fasta.*`). Pipelines also produce and consume FASTQ read sets, and the commonest way they go wrong is silent
structural damage: a transfer or a crashed writer truncates the file mid-record, a trimmer emits a quality string
shorter than its sequence, a concatenation leaves a stray blank line, or a gzip stream ends early. A line count
divisible by four is the traditional smoke test, but it passes a file truncated on a record boundary, cannot be
computed on a gzipped file at all and misses a corrupt gzip stream entirely.

This feature adds the `fastq.*` family. `fastq.valid` is a boolean metric that streams the whole file through a
FASTQ parser and reports whether every record is well formed and the file ends at a record boundary. The same
pass accumulates the aggregates behind `fastq.read.count`, `fastq.length`, `fastq.read.length.min`,
`fastq.read.length.max` and `fastq.quality.encoding`. Per-record metrics (`fastq.read.<n>.*`) are deliberately
not provided: read sets are assessed in aggregate, not row by row. Mean length, a uniform-length flag and Phred
score summaries were considered and left out for now.

## Constraints and key facts

- **No grammar change.** `fastq.valid` is an ordinary two-segment metric.
- **Executor contract** (`src/core/executor.rs`): `try_parse` then `execute` returning `{ success, actual }`.
- **Transparent gzip.** Nearly every real FASTQ is `.fastq.gz`. The executor sniffs the leading magic bytes through
  the detection in `src/file/compression` (never the extension) and wraps the reader in a `flate2`
  `MultiGzDecoder` for `gzip` and `bgzf`. Other compressions (`bzip2`, `xz`, `zstd`, `zip`) are an **error**, since
  the file may well be valid and a `false` would mislead.
- **noodles-fastq behaviour** (`noodles` meta-crate, `fastq` feature). `Reader::read_record` checks the `@` and `+`
  line prefixes and returns an `UnexpectedEof` error when the input ends between the sequence and the `+` line. It
  does **not** check that the quality string is as long as the sequence, and it reads an empty quality string
  without error when the input ends after the `+` line. Both are therefore checked explicitly after each record.

## Design decisions

- **Reports `false`, never errors, on a malformed file.** This is the `file.exists` / `.present` pattern: the
  metric exists to ask "is this file sound", so a malformed file is a plain `FAIL`, and the metric is the safe
  guard for the other `fastq.*` metrics. A file that cannot be opened is still an **error**.
- **Four-line records only.** The format technically allows wrapped sequence and quality lines, but no sequencer
  emits them and most tools reject them. Requiring one line per field keeps "valid" crisp and is what the
  multiple-of-four check was trying to express.
- **An empty file is valid** with zero records. Emptiness is `file.size`'s job.
- **An empty sequence is valid.** Zero-length reads occur after aggressive trimming and some trimmers keep them.
  An empty name (`@` alone) is not valid.
- **Cache a fixed-size summary, not records.** The pass keeps the record count, total bases, shortest and
  longest read, lowest and highest quality character and, if it stopped, why. Memory is constant regardless of
  read count, and a `run` with several `fastq.*` assertions on one file streams it once.
- **Aggregates on an invalid file are an error** carrying the same reason, since a count over a truncated file
  would be a misleading PASS. Guard them with `fastq.valid` to SKIP instead.
- **Encoding is a heuristic, in a fixed order.** A quality character below `@` (64) cannot be phred64, so any
  such character proves `phred33`. Otherwise a character above `J` (74, Q41 in phred33) is taken as `phred64`.
  Everything in between, and an empty file, is `ambiguous`. Long-read files with phred33 scores above Q41 are
  classified correctly in practice because they also carry low-quality bases below `@`.
- **The reason for `false` is reported in brackets.** The scan keeps the offending record index (0-based), its
  starting line and the reason, and the executor returns it as the result's `detail`, which the engine appends to
  the message for both the console and the log: `got false (record 1 (line 5): unexpected end of file)`. The
  detail is attached whenever the file is invalid, so an `eq false` PASS and a skipped guard show it too.

## Metric format

| Metric                    | Value type | Meaning                                                            |
|---------------------------|------------|--------------------------------------------------------------------|
| `fastq.valid`             | bool       | every record is well formed and the file ends at a record boundary |
| `fastq.read.count`        | integer    | number of records                                                  |
| `fastq.length`            | integer    | total bases summed across all records                              |
| `fastq.read.length.min`   | integer    | length of the shortest read (`0` on an empty file)                 |
| `fastq.read.length.max`   | integer    | length of the longest read (`0` on an empty file)                  |
| `fastq.quality.encoding`  | string     | `phred33`, `phred64` or `ambiguous`                                |

A record is well formed when it is exactly four lines: an `@` header with a non-empty name, the sequence, a `+`
separator line and a quality string with exactly as many characters as the sequence. Line endings may be `\n` or
`\r\n`, and the final newline is optional.

### Examples

```
reads_R1.fastq.gz  fastq.valid  eq  true                                              # PASS on a sound file
reads_R1.fastq.gz  fastq.valid  eq  true                                           # FAIL if truncated, with reason
reads_R1.fastq.gz  fastq.valid  ne  false                                             # same check, negated form
reads_R1.fastq.gz  fastq.read.count        gte 1000000                               # at least a million reads
reads_R1.fastq.gz  fastq.length            gt  0                                     # some bases
reads_R1.fastq.gz  fastq.read.length.max   lte 151                                   # nothing longer than the kit
reads_R1.fastq.gz  fastq.read.length.min   gte 20                                    # after trimming
reads_R1.fastq.gz  fastq.quality.encoding  eq  phred33                               # not a legacy phred64 file
reads_R1.fastq.gz  fastq.read.count        gt  0   if reads_R1.fastq.gz fastq.valid eq true   # SKIP if unsound
reads_R1.fastq.gz  fastq.read.count        gt  0                                     # ERROR with the reason if unsound
reads_R1.fastq.gz  file.size    gt  1GB  if reads_R1.fastq.gz fastq.valid eq true     # SKIP when the file is unsound
missing.fastq.gz   fastq.valid  eq  true                                              # ERROR (file cannot be opened)
reads.fastq.bz2    fastq.valid  eq  true                                              # ERROR (bzip2 not supported)
```

Conditions that make a file invalid, each with the record index (0-based) and starting line recorded in the scan:

- a header line that does not start with `@`, including a blank line between records
- a separator line that does not start with `+`
- a header with no name
- a quality string whose length differs from the sequence length, which also covers a file that ends after the
  `+` line
- a file that ends after the header or sequence line, or a truncated gzip stream (`unexpected end of file`)
- a gzip stream that fails its checksum

## Implementation

- `Cargo.toml`: add the `fastq` feature to `noodles` and the `flate2` dependency.
- `src/fastq/mod.rs`, `src/fastq/functions.rs`: `FastqScan { reads, invalid: Option<String> }`, a thread-local
  cache keyed by path (as for `src/fasta/functions.rs`), `scan(path)`, and the private `open` (compression sniff
  plus decoder) and `validate` (the record loop). `functions.rs` is the only place that touches `noodles::fastq`
  and `flate2`.
- `src/fastq/valid/executor.rs`: `FastqValidExecutor`, matching exactly `fastq.valid` and comparing the boolean.
- `src/fastq/count/executor.rs` (`fastq.read.count`, `fastq.length`), `src/fastq/read_length/executor.rs`
  (`fastq.read.length.min`, `.max`) and `src/fastq/quality/executor.rs` (`fastq.quality.encoding`) read the same
  cached summary through `scan_valid`, which turns an invalid scan into an error carrying the reason.
- `src/file/compression` exposes `detect_compression` and `Compression` crate-wide so the sniff is shared.
- `src/engine/executor.rs`: dispatch line after the fasta executors.
- `src/suggest/providers/fastq.rs`: `FastqProvider` handles `.fastq`, `.fq`, `.fastq.gz` and `.fq.gz` and emits
  `fastq.valid eq true` whether or not the file currently passes. When it does, it adds a +/- 50% band on
  `fastq.read.count`, the current shortest and longest read as `gte`/`lte` bounds, and
  `fastq.quality.encoding eq <label>` when the label is not `ambiguous`.

## Testing

- Unit tests in `functions.rs` drive `validate` over in-memory bytes for every invalid condition above plus the
  valid edge cases (no trailing newline, CRLF, empty file, empty sequence), and `scan` over temp files for gzip,
  truncated gzip, unsupported compression and caching.
- Fixtures in `tests/data/`: `sample.fastq` (three Illumina-style reads of 100, 20 and 8 bases, phred33) and its
  `gzip -n` twin, `truncated.fastq` (the first six lines), `truncated.fastq.gz` (the gzip with its last 20 bytes
  removed), `mismatch.fastq`, `empty.fastq`, `phred64.fastq` and `ambiguous.fastq`.
- `tests/fastq_subcommand_test.rs` runs `tests/data/fastq_assertions.txt` end to end (snapshot) and covers the
  exit codes for a FAIL, a missing file and an unsupported compression, and a guard that skips on an invalid file.
- `tests/suggest_subcommand_test.rs` snapshots the suggestion for `sample.fastq.gz`.
