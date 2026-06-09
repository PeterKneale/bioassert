# BioAssert: Bioinformatics File Validation Toolkit

BioAssert is a **Rust-based** command-line and library tool for declaratively validating bioinformatics files (BAM/CRAM, FASTQ, FASTA, VCF/BCF, etc.) in pipelines. It leverages the [noodles](https://docs.rs/noodles/latest/noodles/) I/O libraries to parse genomic formats and provides a simple assertion DSL (in plain text or YAML) to express expected file properties. This spec outlines BioAssert’s goals, CLI, architecture, file-format plugins, assertion formats, workflow integration, packaging, logging, and governance. The aim is to enable easy insertion into Nextflow/nf-core pipelines (and others) so that developers can verify file existence, format correctness, and numeric metrics (read counts, variant counts, file sizes, etc.) without writing custom code. BioAssert is analogous in spirit to tools like PipeVal, but is implemented in Rust for speed, static linking, and seamless nf-core integration.

**Key highlights:**
- **Declarative assertions:** Write rules like “*bam read_count gt 1000*” or “*read1 paired_with eq read2*”.
- **Extensible format support:** Uses noodles to support BAM/CRAM/SAM, FASTQ, FASTA, VCF/BCF, BED, GTF, indices (BAI/CSI), etc. (noodles currently supports BAM/CRAM, VCF/BCF, FASTA/FASTQ, BED, GTF, CSI, tabix, etc.). BigWig support can be added via crates like *bigtools* or *rustynetics* if needed.
- **Rich comparators:** Numeric (`gt`, `lt`, `eq`, `ne`, `gte`, `lte`), set membership (`in`, `not_in`, `contains`), string matching (`matches`). Boolean metrics (e.g. `exists`, `has_index`) are evaluated with `eq true` / `eq false`. Relational metrics (e.g. `paired_with`) are evaluated with `eq <other_subject>`.
- **Library+CLI:** BioAssert offers a Rust library (for embedding in other programs) and a standalone CLI binary.
- **nf-core integration:** A Nextflow module allows using BioAssert as a process; users supply file channels and an assertion file bundle.

## Quick Start

Get up and running in three steps. Copy-paste any block below.

### 1. Install

```bash
# From crates.io
cargo install bioassert

# Or via Docker
docker pull bioassert/bioassert:latest
```

### 2. Write an assertion file

Save the following as `aligned_bam.assert`:

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
[PASS] bam (sample.bam) size=123456789 gte 100MB
[PASS] bam (sample.bam) read_count=5321 gt 100000
[PASS] bam (sample.bam) sort_order=coordinate eq coordinate
[PASS] bam (sample.bam) read_group_count=1 gte 1
[PASS] bam (sample.bam) has_index=true eq true
```

On failure (exit code `1`):

```
[FAIL] bam (sample.bam) sort_order=unknown, expected 'coordinate'
```

BioAssert also writes a full execution log to `bioassert.log` (override with `--log-file <path>`).

### Multi-format example

Bind multiple named inputs and reference one or more assertion bundles in a single run:

```bash
bioassert \
    --assertions fastq/raw.assert \
    --assertions vcf/variants.assert \
    --input read1=reads_R1.fq.gz \
    --input read2=reads_R2.fq.gz \
    --input vcf=variants.vcf.gz
```

A multi-format assertion file might contain:

```text
bam exists eq true
bam read_count gt 1000000
bam mapped_reads gte 900000
bam sort_order eq coordinate
bam has_index eq true

read1 paired_with eq read2
read1 read_count eq read2
read1 average_read_length eq 150

fasta sequence_count eq 24
fasta total_bases gt 3000000000
fasta longest_sequence gt 1000000

vcf variant_count gt 0
vcf sample_count eq 1
vcf contigs contains chr1
```

See [Supported Formats](#supported-formats-and-example-assertions) for the full metric
catalogue and [Assertion File Formats](#assertion-file-formats-and-examples) for YAML
bundles and parameterized assertion files.

## Goals and Non-Goals

- **Goals:**
    - Provide a *declarative* DSL to assert file properties in bioinformatics pipelines (e.g. SAM/BAM sort order, read counts, file existence, VCF sample count, FASTQ quality encoding, etc.).
    - Leverage existing Rust ecosystems (noodles) for file parsing.
    - Offer both a CLI for pipelines and a library API (so other tools can embed it).
    - Be **extensible**: new file types or metrics can be added via plugins or new metric providers.
    - Integrate cleanly with Nextflow/nf-core; e.g. as an nf-core module with meta-map channels.
    - Provide deterministic pipeline-facing outputs via exit codes, stdout/stderr messages, and a log file.
    - High performance and low overhead on large files (caching and parallel evaluation when possible).

- **Non-Goals:**
    - BioAssert is *not* a full QC suite (it does not compute complex QC plots or replace FastQC, Picard, etc., but it can assert simple metrics like “duplicate_reads < X”).
    - It does not transform or correct files; it only checks them.
    - It does not validate arbitrary non-genomic files (images, spreadsheets, etc.) beyond generic checks (exists, size, checksum).
    - It is **not** a workflow framework; it is a tool to be invoked by pipelines or other tools.

## CLI Design

BioAssert is a single-purpose assertion tool. Assertions are the only CLI function, so the
root command performs validation directly. Key design points:

- **Command:**
  ```
  bioassert [--assertions <file>] [--input name=path]... [--log-file <file>] [--continue] [--quiet|-q] [--verbose|-v] [--help|-h]
  ```
    - `--assertions <file>`: Path to an assertion file (plain text or YAML). Multiple files can be provided.
    - `--input name=path`: Bind a named input to an actual file path. The assertion file refers to that virtual subject name (e.g. `--input bam=sample.bam` is checked by assertions written against `bam`). *Named inputs* allow parameterizing assertion bundles (e.g. `--input bam=sample.bam --input reference=ref.fa`).
    - `--log-file <file>`: Path to the execution log file. If not provided, BioAssert writes `bioassert.log` in the current working directory.
    - `--continue` or `--report-all`: Do not exit on first failure, but evaluate all assertions and report all failures.
    - `--quiet`/`--verbose`: Control logging.
    - `--version`, `--help`: standard info.

- **Examples:**
  ```bash
  # Basic use: one assertion file, bound input name and assertion subject
  bioassert --assertions aligned_bam.assert --input bam=sample.bam

  # With multiple assertion bundles
  bioassert \
      --assertions fastq/raw.assert \
      --assertions vcf/variants.assert \
      --input read1=reads_R1.fq.gz \
      --input read2=reads_R2.fq.gz \
      --input vcf=variants.vcf.gz

  # Write a log file and continue on all failures
  bioassert --assertions bam-checks.assert \
      --input bam=sample.bam --log-file bam-checks.log --continue
  ```

- **Command model:** BioAssert intentionally has no subcommands. The tool's only purpose is
  to evaluate assertions over bound inputs, so the root command is the full user interface.

- **Parser:** Supports both a simple DSL (one assertion per line) and a YAML schema (see below). The parser resolves subject names and metrics, then invokes the core engine.

- **Exit codes:**
    - `0` – All assertions passed.
    - `1` – One or more assertions failed.
    - `2` – CLI or configuration error (e.g. parse error).
    - (Signal termination >128).

  By default, BioAssert fails fast (exit on first failed assertion) unless `--continue` is set. The report (printed summary) always shows which assertions passed/failed.

## Architecture (Rust)

BioAssert is implemented as a **Rust library/crate** (exposed via a CLI binary). Key components:

```mermaid
graph LR
    A[CLI & Config] --> B(Parser)
    B --> C[AssertionRunner]
    C --> D[MetricRegistry]
    D --> BAMProvider
    D --> FASTQProvider
    D --> VCFProvider
    D --> ...OtherProviders
    C --> E[Report]
    E --> F[Output (stdout/stderr/log file)]
```

- **Modules and Crates:**
    - `bioassert` (main crate).
    - **Metric Providers:** Separate modules (or sub-crates) for each file type (e.g. `bam.rs`, `fastq.rs`, `fasta.rs`, `vcf.rs`, etc.), each implementing a common interface.
    - **Parser:** Parses the assertion file (DSL or YAML) into an internal `Assertion` model.
    - **Runner:** Coordinates evaluation: loads or caches files, retrieves metrics, applies comparisons, and records results.
    - **Metric Registry:** A lookup table mapping metric names (strings) to functions that compute values from a `FileContext`.
    - **Cache/Context:** For each subject file, a context object caches expensive data (e.g. BAM header or record counts) so multiple metrics on the same file don’t re-scan.
    - **Report/Output:** Formats results into stdout/stderr messages and writes an execution log file.
    - **Error Handling:** Uses idiomatic Rust error types (e.g. `anyhow::Error`) for failures.

- **Data Models:**
  ```rust
  enum Value { Bool(bool), Integer(u64), Float(f64), String(String), List(Vec<Value>) }
  enum Operator { Eq, Ne, Gt, Lt, Ge, Le, In, NotIn, Contains, Matches }
  struct Assertion {
      subject: String,         // virtual input name, e.g. "bam"
      metric: String,          // e.g. "read_count"
      op: Operator,
      expected: Option<Value>, // Some(value) or None for boolean asserts like 'exists'
  }
  ```  
  The runner evaluates each `Assertion`: it resolves `subject` (using named inputs to map to file paths), calls the metric function to get an actual `Value`, then applies `op` vs. `expected`.

- **Metric Registry and Providers:**  
  We define a trait, e.g.:
  ```rust
  trait MetricProvider {
      /// Returns true if this provider can handle the given file path.
      fn supports(path: &Path) -> bool;
      /// Compute the named metric for the given file context.
      fn get(&mut self, metric: &str) -> Result<Value>;
  }
  ```  
  A `MetricRegistry` holds instances or constructors of providers. For example, `BamProvider::supports(path)` checks file extension or header magic. If supported, runner creates a `BamContext` and uses it for all BAM metrics.

  Internally, a provider (e.g. `BamProvider`) caches state like the parsed header or previous computed read counts. For instance, a first call to `"read_count"` will scan the BAM/CRAM once, count records, and store it for reuse by `"mapped_reads"` or `"duplicate_reads"`. This avoids multiple full reads. See concurrency note below for parallel processing.

- **Concurrency and Performance:**  
  When validating *multiple files* or *multiple independent assertions*, BioAssert can run checks in parallel (multi-thread). For a single large BAM with many assertions, the design ensures one pass covers all needed metrics. PipeVal also highlights parallel validation as important. We can leverage Rust’s threads (or rayon) to validate different files concurrently. For record-scanning metrics, Noodles offers iterators, and we could even split large files by index (if using a CSI/BAI) to parallelize within one file.

  Example: if the user asserts three things on `bam` (bound from `--input bam=sample.bam`), we do one scan to get `{read_count, mapped_reads, unmapped_reads, duplicate_reads, ...}`. If assertions include an index check, we open the .bai/.csi as well. Sorting metrics come from header (no scan).

- **Testing Strategy:**
    - **Unit tests**: For each metric/provider, include small fixture files (e.g. a tiny BAM or VCF) and assert the returned values. E.g. a test BAM with 10 reads should yield `read_count == 10`.
    - **Integration tests**: Use real-ish genomic files (embedded in test data, or downloaded in CI) and run the BioAssert CLI on example assertions to ensure PASS/FAIL as expected.
    - **Golden tests**: For core formats, compare against existing tools (e.g. samtools view or samtools flagstat for BAM counts; bcftools stats for VCF) to validate metrics.
    - **Benchmarking**: Use [Criterion.rs](https://crates.io/crates/criterion) or similar to measure performance on large files. This ensures reading is not a bottleneck. Multi-threaded runs should be measured (PipeVal noted significant speedups).

- **Error Handling:**  
  Errors (bad input, parse failures, unreadable files) return non-zero exit. Assertions that fail are reported but do not cause a crash (unless fatal). The CLI prints user-friendly messages. Under the hood, `anyhow::Context` or similar crates can add context to errors. E.g. if a FASTQ file is malformed, the error might read “FASTQ parse error at byte 1024”.

- **Logging:**  
  Use the Rust `log` crate (with a user option to set log level). By default, BioAssert prints a concise summary to stdout/stderr and writes an execution log to a file. A `-q/--quiet` can suppress non-essential console messages, whereas `-v/--verbose` shows debug info (e.g. “Computed read_count=12345 for bam (sample.bam)”). The log file is always written and contains the full execution trace for troubleshooting.

## Plugin/Metric Provider API

BioAssert defines a plugin-like API so that new file formats can be added. Concretely:

```rust
pub trait MetricProvider {
    /// Does this provider support (recognize) the given file?
    fn supports(path: &Path) -> bool;

    /// Initialize context (open file, parse header).
    fn new(path: &Path) -> Result<Self> where Self: Sized;

    /// Compute or retrieve a metric value.
    fn get(&mut self, metric: &str) -> Result<Value>;
}
```

Each file format has its own provider implementing these. For example:

- **BamProvider**: Supports `.bam`/`.sam`/`.cram` (checking via header magic). In `new()`, it reads the BAM header. `get("read_count")` scans records (once) for count; `get("sort_order")` looks at `@HD` header line; `get("has_index")` checks for a `.bai`/`.crai`; etc.
- **FastqProvider**: Supports `.fastq`/`.fq`/`.fastq.gz`/`.fq.gz`. It may buffer records to compute average length, or sample the first 1000 reads to estimate. `get("quality_encoding")` might examine ASCII ranges.
- **FastaProvider**: Supports FASTA files (`.fa`, `.fasta`, `.fa.gz`). `get("sequence_count")` = number of records; `get("total_bases")` = sum of sequence lengths; `get("sequence_names")` = list of headers.
- **VcfProvider / BcfProvider**: Supports VCF/BCF (and gzipped `.vcf.gz` with tabix). Metrics: `variant_count` (records in file), `sample_count` (from header), `contigs` (header contig lines), `format_fields`, `filter_fields`, `info_fields`, etc.
- **Index Providers**: BAI/CRAI/CSI (`.bai`, `.crai`, `.csi`) – often handled together with their main file. Metrics can include “index_exists” or verifying the index covers the same sequence dictionary. A Tabix index (`.tbi`) for VCF/TSV is similar.
- **GenericFileProvider**: Fallback for any file. Supports: `exists`, `size`, `lines` (if text), `md5`, `sha256`, `modified_time`. This provider ensures basic checks (exists, checksum) for any extension.

Each provider self-documents its metrics. A future plugin system could allow external crates to register, but initially all providers live in the BioAssert crate.

**Extensibility:** Adding a new metric usually means coding one method in the relevant provider (and adding its name to the registry). Adding support for a new format means implementing `MetricProvider` for that type and registering it. Because the assertion DSL is generic (`<subject> <metric> <op> <value>`), the parser doesn’t need changes to support new metrics. This modular approach mirrors how PipeVal’s “library of validation functions” is mapped to file types.

## Supported Formats and Example Assertions

BioAssert natively supports common genomics files. The table below lists formats and representative assertions/metrics (subject to expansion as needed):

| **Format**        | **Example Metrics/Assertions**                                            |
|-------------------|----------------------------------------------------------------------------|
| **Generic (any)** | `exists` (file exists), `size` (bytes or human-readable, e.g. `gt 100MB`),<br>`md5`, `sha256`, `lines` (text files). |
| **BAM/CRAM/SAM**  | `read_count` (total alignments), `mapped_reads`, `unmapped_reads`,<br>`duplicate_reads`, `secondary_reads`, `supplementary_reads`;<br>`sort_order` (coordinate/name/unknown), `read_group_count`, `sample_names` (set of RG:SM values),<br>`has_index` (boolean), `reference_count` (contigs). |
| **FASTQ**        | `read_count` (reads), `average_read_length`, `quality_encoding` (e.g. "illumina phred+33"),<br>`paired` (assert pairwise interleaving or matching R1/R2 counts). |
| **FASTA**        | `sequence_count` (records), `total_bases`, `longest_sequence`,<br>`sequence_names` (list of headers), `no_duplicate_sequence_names`. |
| **VCF/BCF**      | `variant_count` (entries), `snp_count`, `indel_count`, `sample_count` (number of samples),<br>`contigs` (list), `format_fields`, `filter_fields`, `info_fields`. |
| **VCF.GZ/BCF.GZ**| Same as VCF, plus checking for `.tbi` index (if present). |
| **BAI/CSI/CRAI** | `exists` (index file), `index_matches` (verify index references match BAM/CRAM header). |
| **Tabix (TBI)**  | `exists`, `matching_tabix` (index matches parent VCF/BED). |
| **BED/GFF/GTF** | `feature_count` (lines), `sorted_by_coordinate` (boolean), `no_overlap` (boolean), `no_duplicate_ids` (boolean). |
| **BigWig**       | `exists`, `size`, `chromosome_count`, `min_value`/`max_value` (data range), optionally `gc_content_matches_fasta`. |

(*Table*: Formats vs. example metrics. Developers can extend this list.)

See the [Quick Start](#quick-start) for a copy-paste runnable assertion file that exercises
BAM, FASTQ, FASTA, and VCF metrics in a single bundle. Typical pipeline checks include
verifying that a BAM is coordinate-sorted and indexed with the expected number of reads,
that paired FASTQs match in count, that a reference FASTA has the expected chromosome
count, and that a VCF contains at least one variant on a known contig.

## Assertion File Formats and Examples

BioAssert supports two assertion file syntaxes:

1. **Plain Text DSL** (line-oriented):  
   Each line is one assertion in the form:
   ```
   <subject> <metric> <operator> <value>
   ```  
    - *Subject* is a virtual name used in the assertion file and resolved by `--input name=path` bindings.
    - *Operator* is one of `eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `in`, `not_in`, `contains`, `matches`.
    - *Value* is a literal (number, string, boolean, list `[a, b]`, or regex), **or** another bound subject name (e.g. for relational metrics like `paired_with` and for cross-subject comparisons like `read1 read_count eq read2`).
    - **Boolean metrics** (e.g. `exists`, `has_index`) must be written with an explicit comparison: `<subject> <metric> eq true` (or `eq false`). The shorthand `<subject> <metric>` is **not** supported.
    - **Size literals** use SI decimal suffixes: `KB`, `MB`, `GB`, `TB` (e.g. `size gte 100MB`).
    - **DSL→enum mapping:** DSL `gte`/`lte`/`not_in` correspond to the `Operator` enum variants `Ge`/`Le`/`NotIn`.

   **Example (single-file bundle):**
   ```text
   # assertions for an aligned BAM bound as `bam`
   bam exists eq true
   bam size gte 100MB
   bam read_count gt 100000
   bam sort_order eq coordinate
   bam read_group_count gte 1
   bam has_index eq true
   ```

   **Example (multi-input / named):**
   ```text
   # named input: bam
   bam has_index eq true
   bam sort_order eq coordinate
   ```

   **Example (paired FASTQ):**
   ```text
   read1 paired_with eq read2
   read1 read_count eq read2
   read1 average_read_length eq 151
   ```

2. **YAML format:**  
   Useful for grouping, naming, and composition.

   ```yaml
   name: aligned_bam_checks
   inputs:
     bam: sample.bam
   assertions:
     - bam read_count gt 100000
     - bam sort_order eq coordinate
     - bam has_index eq true
   ```

   Multi-file/assertion example:
   ```yaml
   name: paired_fastqs
   inputs:
     read1: reads_R1.fastq.gz
     read2: reads_R2.fastq.gz
   assertions:
     - read1 paired_with eq read2
     - read1 read_count eq read2
   ```

   Alternatively, a more explicit style:
   ```yaml
   assertions:
     - name: tumour_indexed
       expression: bam has_index eq true
       inputs:
         bam: tumour.bam

     - name: fastq_paired
       expression: read1 paired_with eq read2
       inputs:
         read1: sample_R1.fq.gz
         read2: sample_R2.fq.gz
   ```

The YAML can also **include** other assertion files or share input sections (for bundles). Parameterized bundles (e.g. a pipeline specifying `inputs.bam`) let a common `.assert` file be reused for different files.

## Nextflow / nf-core Module Integration

BioAssert is designed to integrate smoothly into Nextflow pipelines, especially nf-core style. We provide an nf-core module `bioassert` that wraps the CLI. Key points:

- **Module API:** The module expects a meta-map channel of inputs and a parameter for the assertion file(s).
- **Example usage:** (in an nf-core pipeline)
  ```nextflow
  include { BIOASSERT } from '../modules/nf-core/bioassert/main'

  workflow {
      // Set up meta-map channel for BAM and reference
      Channel
        .fromPath(params.bam_path)
        .map { bam -> [id: bam.baseName, bam: bam] }
        .set { bam_ch }

      // Invoke BioAssert on the channel
      BIOASSERT(bam_ch, params.assertions_file)
  }
  ```
  This will translate internally to something like:
  ```nextflow
  process BIOASSERT {
      tag "$bam.id"
      input:
        val meta_map from bam_ch
        path assertions_file from params.assertions_file

      output:
        stdout, stderr, path("bioassert.log"), exit status into assert_results

      """
      bioassert \
        --assertions $assertions_file \
        --input bam=${meta_map.bam} \
        --log-file bioassert.log
      """
  }
  ```
  Here `meta_map` is a Nextflow meta-map with key `bam` (the file) and an `id` used for tagging. This follows nf-core convention of using meta-maps. The module adds standard labels, resources, and meta tags automatically.

- **Channel structures:** If a pipeline’s channel is just files (no meta), the developer should map it to `[ [id: sampleName], file ]` before passing to BioAssert, to give an `id` tag. This is standard nf-core practice.

- **Module Configuration:** The module expects:
    - One or more input channels (each a meta-map with file(s)). Example: for paired FASTQs, a channel of `[id: chr, read1: path1, read2: path2]`.
    - A parameter (or `params.assertions`) pointing to the assertion bundle file or directory.

- **Example:**
  ```nextflow
  workflow {
      // Example: paired FASTQs
      Channel.fromFilePairs('samples/*_{1,2}.fastq.gz', fileExt: '.fq.gz', flat: true)
             .map { pair -> [id: pair.key, read1: pair.value[0], read2: pair.value[1]] }
             .set { fastq_pair_ch }

      include { BIOASSERT } from '../modules/nf-core/bioassert/main'

      BIOASSERT(fastq_pair_ch, 'assertions/fastq/paired.assert')
  }
  ```

For further details on writing and using nf-core modules (boilerplate and resources), see the nf-core documentation.

## Packaging and Distribution

- **Rust Crate and CLI:** BioAssert is published on crates.io (e.g. `bioassert = "0.x"`). The repository includes `Cargo.toml` with metadata, and a `src/main.rs` for the CLI. The crate uses semantic versioning.
- **Docker Image:** A Docker container is provided (with a statically linked binary). The
  repository `Dockerfile` uses a multi-stage build on `rust:<version>-alpine` (musl), linking the
  C runtime statically (`-C target-feature=+crt-static`), then copies the binary into a minimal
  `scratch` base. For example (in Dockerfile):
  ```dockerfile
  FROM rust:1.96-alpine AS builder
  RUN apk add --no-cache musl-dev
  WORKDIR /src
  COPY . .
  ENV RUSTFLAGS="-C target-feature=+crt-static"
  RUN cargo build --locked --release

  FROM scratch
  COPY --from=builder /src/target/release/bioassert /usr/local/bin/bioassert
  ENTRYPOINT ["/usr/local/bin/bioassert"]
  ```
  This yields a static binary with no glibc dependencies. The image is published to the **GitHub
  Container Registry (`ghcr.io/<owner>/bioassert`)** with tags matching the crate version
  (`{{version}}`, `{{major}}.{{minor}}`, `{{major}}`) and `latest`. Multi-arch (`linux/amd64`,
  `linux/arm64`) images are built with Docker Buildx + QEMU in GitHub Actions.

- **CI/CD:** Use GitHub Actions to test on each push/PR (run `cargo fmt -- --check`,
  `cargo clippy`, `cargo build`, `cargo test`). On release, automatically publish the crate to
  crates.io and build/push the multi-arch Docker image to `ghcr.io` (authenticated with the
  built-in `GITHUB_TOKEN`, `packages: write`). Ensure reproducible builds by using `--locked` and
  a pinned Rust toolchain.

- **Library API:** In addition to the binary, BioAssert is usable as a library. We document public functions (e.g. `run_assertions(assertions: &str, inputs: HashMap<String, PathBuf>) -> Result<Report>`). Other Rust programs can call these to integrate BioAssert checks.

- **Versioning:** Follows semantic versioning (major.minor.patch). Releases are tagged in git, with CHANGELOG notes.

- **Repositories:** Hosted on GitHub, MIT or Apache-2.0 licensed, with an issue tracker and PR workflow.

- **Distribution:** Users can `cargo install bioassert` or use the Docker image or the nf-core module. We ensure the binary is statically linked so no runtime deps.

## Logging, Exit Codes, and Pipeline Behavior

- **Exit Codes:** As noted, exit code 0 means all checks passed; 1 means one or more failed assertions; 2 (or >1) means a usage error or crash. The module returns success only if all assertions pass (unless `--continue` is used, in which case it still returns 1 but prints all failures).
- **Fail-Fast vs. Report-All:** By default, BioAssert stops at the first failure (fail-fast). The `--continue` flag runs all assertions regardless, reporting each pass/fail. This is configurable so pipelines can choose to either stop early or collect all issues.
- **Outputs:**
    - **Exit code:** `0`, `1`, or `2` as described above.
    - **stdout:** Human-readable pass summaries and final success/failure summary.
    - **stderr:** Usage errors, parse errors, fatal runtime errors, and warning/error diagnostics.
    - **Log file:** A persistent execution log capturing assertion evaluation, metric computation context, and failures for post-run inspection.

  Example console output:
  ```
  [PASS] bam (sample.bam) exists eq true
  [PASS] bam (sample.bam) read_count=5321 gt 1000
  [FAIL] bam (sample.bam) sort_order=unknown, expected 'coordinate'
  ```

  Example log excerpt:
  ```text
  2026-06-09T10:15:01Z INFO starting bioassert
  2026-06-09T10:15:01Z INFO evaluating bam (sample.bam) read_count gt 1000
  2026-06-09T10:15:02Z ERROR assertion failed: bam (sample.bam) sort_order eq coordinate (actual=unknown)
  ```

- **Verbosity:** `-q/--quiet` silences progress messages; `-v/--verbose` (or `--debug`) enables debug logging. Errors always printed to stderr.

- **Performance:** When scanning large files, BioAssert caches metrics. For example, three read-count assertions on a 50GB BAM will use one pass. If parallel threads are used (e.g. multiple files), IO and CPU should scale (with caveats about I/O bandwidth). There is an option to limit threads.

- **Resource Limits:** By default, BioAssert uses streaming reading (so memory ~O(1) per file). It can be run in containers with memory/CPU limits, and will respect Nextflow’s process resource labels.

## Security and Reproducibility

- **Deterministic Builds:** We freeze dependencies (`Cargo.lock`) and build with `--locked`. Use specific Rust and library versions. All build scripts and Dockerfiles pin versions (e.g. `FROM rust:1.77`).
- **Static Linking:** The Rust binary is statically linked (via musl), reducing dependency vulnerabilities.
- **Cargo Audit:** The CI runs `cargo audit` to check for known vulnerabilities in dependencies.
- **Minimal Privileges:** The binary runs unprivileged; it only reads input files and writes output to stdout/stderr plus the configured log file. No network or external calls.
- **Reproducible Docker:** The Dockerfile adds only necessary files (binary + CA certs). We pin base images by digest (or use official Rust stable tags) to ensure reproducibility.

## Examples, Documentation, and Help

- **Man Page / `--help`:** Running `bioassert --help` shows usage. A manual page is included in docs (or can be generated from CLI metadata).
- **Assertion Bundle Examples:** We provide example assertion files in the `examples/` directory, such as `aligned_bam.assert`, `paired_fastq.assert`, etc. For instance, `aligned_bam.assert` might contain:
  ```text
  bam exists eq true
  bam read_count gt 1000
  bam sort_order eq coordinate
  bam has_index eq true
  ```
  These examples are also in documentation.

- **Sample Outputs:**
    - *PASS:* When all checks pass, output like `[PASS] bam (sample.bam) read_count=1500 gt 1000` for each.
    - *FAIL:* Example of a failure:
      ```
      [ERROR] Assertion failed: bam (sample.bam) sort_order = unknown (expected coordinate)
      ```
      And in the log file:
      ```text
      2026-06-09T10:15:02Z ERROR assertion failed: bam (sample.bam) sort_order eq coordinate (actual=unknown)
      ```  
  These examples are documented in README.

- **Help Text (excerpt):**
  ```
  bioassert 0.1.0
  Assert bioinformatics file properties.

  USAGE:
      bioassert [OPTIONS] --assertions <file> --input <name=path>...

  OPTIONS:
      -a, --assertions <file>    Path to assertion file (text or YAML)
      -i, --input <name=path>    Bind input name to file (can repeat)
      -l, --log-file <file>      Write execution logs to the given file (default: bioassert.log)
      -c, --continue             Continue after failures (report all)
      -q, --quiet                Minimal logging
      -v, --verbose              Verbose logging
      -h, --help                 Print help info
      -V, --version              Print version
  ```

## Recommended Tests and Benchmarks

- **Unit Tests:** For each metric, include a small test file. E.g. a FASTA with 3 sequences, a tiny VCF, etc. Use [noodles examples](https://docs.rs/noodles-bam/latest/noodles_bam/) as fixtures.
- **Integration Tests:** Sample pipelines using `bioassert` on real data (e.g. public datasets). Confirm correct PASS/FAIL.
- **Cross-Platform:** Ensure it runs on Linux, macOS, Windows (via Windows Subsystem or similar).
- **Benchmarks:** Use `cargo bench` or Criterion to measure:
    - BAM read_count on a 50M-read file (with and without index).
    - FASTQ average length on a large file.
    - Impact of `--continue` (shouldn't re-scan file).  
      PipeVal’s team found multi-threading crucial for throughput; we aim for similar performance.

## Migration and Extension

- **Adding a Metric:** Update the appropriate provider. For example, to add `supplementary_reads` to BAM: implement it in `BamProvider::get("supplementary_reads")` by counting flag SAM_SUPPLEMENTARY. No parser change needed.
- **Adding a Format:** Create a new `FooProvider` implementing the `MetricProvider` trait, register it in the registry, and document metrics. E.g. a BigWig provider could use [rustynetics](https://crates.io/crates/rustynetics) to read headers.
- **Upgrading Assertion DSL:** The DSL is parsed by a PEG or simple parser; if needed, we could add new operators. That’s backward-compatible.
- **Deprecated Metrics:** Clearly document and bump major version if removing an assertion name.

For complex changes, follow semantic versioning. Contributors should update CHANGELOG and possibly an example file.

## Community and Contribution Guidelines

BioAssert is open-source (MIT/Apache) and welcomes community input:

- **Contributions:** Fork the repo, create PRs. Follow Rust style (rustfmt, clippy). Include tests for new features.
- **Code of Conduct:** Adhere to a friendly, inclusive code of conduct (e.g. the [Contributor Covenant](https://www.contributor-covenant.org)).
- **Governance:** Maintainers will review issues/PRs. Significant changes should be discussed via issues first.
- **License:** We recommend Apache-2.0 (or MIT) to align with nf-core ethos.
- **Versioning:** We use [semver](https://semver.org/) and tag releases on GitHub; CI builds and publishes each release.

## Diagrams

### Architecture Overview

```mermaid
graph LR
    subgraph CLI
      C1[Parse CLI args]
      C2[Load assertions file(s)]
      C1 --> C2
    end
    subgraph Core
      R[Assertion Runner] --> M[MetricRegistry]
      M --> B[BAM/CRAM Provider]
      M --> Q[FASTQ Provider]
      M --> F[FASTA Provider]
      M --> V[VCF/BCF Provider]
      M --> G[GTF/BED Provider]
      M --> U[Generic File Provider]
      R --> O[Reporter / Log Writer]
    end
    CLI -.-> R
    O --> OUT[stdout/stderr/log file output]
```
*Figure: BioAssert core architecture. The CLI parses input, the Runner evaluates each assertion via metrics from providers, and the Reporter formats results.*

### Assertion Evaluation Flow

```mermaid
flowchart TD
    A[Start] --> B[Parse assertion file]
    B --> C[Loop: for each assertion]
    C --> D[Resolve subject files]
    D --> E{MetricProvider found?}
    E -->|No| F[Error: Unsupported file/metric]
    E -->|Yes| G[Compute actual metric value]
    G --> H[Compare using operator]
    H --> I[Record PASS/FAIL]
    I --> C
    C --> J[Aggregation]
    J --> K{Any failures?}
    K -->|Yes| L[Exit code 1 (FAIL)]
    K -->|No| M[Exit code 0 (PASS)]
    L --> N[Print report]
    M --> N
    N --> O[End]
```
*Figure: High-level flow of assertion evaluation. Each assertion is parsed, the appropriate metric computed, and the comparison applied.*

## References

- The [noodles](https://docs.rs/noodles/latest/noodles/) Rust libraries (supporting BAM, CRAM, SAM, FASTA, FASTQ, VCF, BCF, BED, GTF, CSI, tabix, etc.) serve as the parsing backbone.
- PipeVal: A similar validation tool (Python) that validates formats like FASTQ, SAM, BAM, CRAM, VCF, with built-in checks (e.g. “BAM has ≥1 read and an index”). BioAssert aims for analogous coverage.
- nf-core/Nextflow conventions: Modules are included via `include { X } from ...` and take meta-map channels, tags, labels, etc. We follow these patterns.
- Packaging best-practices: static Rust binaries via musl (see musl-build discussion), semantic versioning, continuous integration.

All details above are subject to further refinement as BioAssert evolves. The objective is a rigorous, reproducible tool that improves bioinformatics pipeline robustness by catching file issues early.