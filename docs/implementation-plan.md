# BioAssert Implementation Plan

> Status: Draft for review
> Source of truth: [`docs/spec.md`](./spec.md)
> Governance: [`AGENTS.md`](../AGENTS.md) — Specification Driven Development (SDD)

This plan decomposes the work required to implement the functionality described in
`docs/spec.md`. Every phase and deliverable references the specification section(s) it
implements so that work remains traceable (AGENTS.md §3 "Requirements Traceability").

No item in this plan introduces behavior that is not described in the specification.

## Progress

- ✅ **Phase 0 — Foundations & Dependencies** (lib/bin targets, data model, exit codes, CLI
  skeleton, `run_assertions` surface, pinned toolchain, README + example).
- ✅ **Phase 1 — Model & Parser** (plain-text DSL + YAML parser; explicit booleans,
  relational/cross-subject RHS, SI sizes; parse errors surfaced with line numbers → exit 2).
- ✅ **Phase 2 — Engine / Runner + Comparators** (`MetricResolver` trait, full comparator set
  incl. `matches`/`in`/`contains`/numeric coercion, cross-subject resolution, fail-fast vs.
  `--continue`, `Report` construction; tested against a stub resolver).
- ✅ **Phase 3 — Generic Provider** (`exists`, `size`, `lines`, `md5`, `sha256`, `modified_time`;
  streaming + cached; golden checksum tests).
- ✅ **Phase 4 — Registry + Dispatch** (`MetricRegistry` implements `MetricResolver`, routes
  metrics to providers with per-file caching; `run_assertions`/`run_assertions_with` wired so the
  CLI evaluates real files end-to-end with correct exit codes).
- 🔄 **Phase 5 — Format Providers (Noodles)** — in progress:
  - ✅ **FASTA** (`sequence_count`, `total_bases`, `longest_sequence`, `sequence_names`,
    `no_duplicate_sequence_names`; noodles-fasta, single-pass cached stats; mixed generic+FASTA
    routing verified).
  - ✅ **FASTQ** (`read_count`, `average_read_length`, `quality_encoding`, `paired_with`;
    noodles-fastq + flate2 gzip support; relational `paired_with` & cross-subject `read_count`
    verified end-to-end). `paired` (single-file interleaving) deferred.
  - ✅ **VCF** (`variant_count`, `snp_count`, `indel_count`, `sample_count`, `contigs`,
    `info_fields`, `format_fields`, `filter_fields`; noodles-vcf, plain + bgzip; verified
    end-to-end). **BCF** (binary) deferred.
  - ✅ **BAM** (`read_count`, `mapped_reads`, `unmapped_reads`, `duplicate_reads`,
    `secondary_reads`, `supplementary_reads`, `sort_order`, `read_group_count`, `sample_names`,
    `has_index`, `reference_count`; noodles-bam/-sam; single-pass flag counts + header metrics;
    `has_index` is filesystem-only; tested against a real BAM written via noodles). **SAM** (text)
    and **CRAM** deferred.
  - ⏭️ index (BAI/CRAI/CSI/TBI as subject) → BED/GFF/GTF.

---

## 1. Current State

- `Cargo.toml`: package only, `edition = "2024"`, no dependencies.
- `src/lib.rs`: placeholder `add` function + sample test.
- `tests/`: empty.
- No CLI binary, parser, engine, providers, or reporter yet.

Clean scaffold — the plan builds the architecture from the ground up following the module
boundaries defined in the spec.

---

## 2. Architectural Boundaries (must be preserved)

Per spec "Architecture (Rust)" and AGENTS.md "Architecture Rules":

| Layer | Responsibility | Must NOT contain |
|-------|----------------|------------------|
| CLI (`src/main.rs`, `cli/`) | Arg parsing, config, `--input` binding, user-facing output | Format parsing, metric calculation |
| Parser | DSL + YAML → `Assertion` model | Metric calculation, CLI concerns |
| Assertion Engine / Runner | Evaluation, comparator execution, result generation | CLI concerns, output formatting |
| Metric Providers | Format-specific logic, metric calculation, caching | Assertion evaluation, CLI logic |
| Reporting Layer | stdout/stderr summary + log-file writer | Business logic, metric calculation |

Proposed crate layout (single crate, internal modules):

```
src/
  lib.rs                 // public library API (run_assertions, re-exports)
  main.rs                // CLI binary entry point (root command, no subcommands)
  cli/                   // arg parsing, config, --input binding, exit codes
  parser/                // DSL parser + YAML parser -> Assertion model
  model.rs               // Value, Operator, Assertion, Report, Status types
  engine/                // AssertionRunner, comparator logic, subject resolution
  registry.rs            // MetricRegistry: file -> provider, metric dispatch
  providers/
    mod.rs               // MetricProvider trait
    generic.rs           // exists, size, lines, md5, sha256, modified_time
    bam.rs               // BAM/CRAM/SAM
    fastq.rs             // FASTQ
    fasta.rs             // FASTA
    vcf.rs               // VCF/BCF
    index.rs             // BAI/CRAI/CSI/TBI
    bed_gtf.rs           // BED/GFF/GTF
  report/                // stdout/stderr formatter + log-file writer
```

> **Library API contract (spec "Packaging / Library API"):**
> `run_assertions(assertions: &str, inputs: HashMap<String, PathBuf>) -> Result<Report>`

---

## 3. Core Data Model (spec "Architecture → Data Models")

Implement exactly the spec's types:

```rust
enum Value { Bool(bool), Integer(u64), Float(f64), String(String), List(Vec<Value>) }
enum Operator { Eq, Ne, Gt, Lt, Ge, Le, In, NotIn, Contains, Matches }
struct Assertion {
    subject: String,         // virtual input name, e.g. "bam"
    metric: String,          // e.g. "read_count"
    op: Operator,
    expected: Option<Value>, // value literal OR a bound subject reference
}
```

Result/report types for the Reporting Layer:

```rust
enum Status { Pass, Fail }
struct AssertionResult {
    subject: String,         // virtual name
    resolved_path: PathBuf,  // physical file, for "bam (sample.bam)" diagnostics
    metric: String,
    op: Operator,
    expected: Option<Value>,
    actual: Option<Value>,
    status: Status,
    message: String,
}
struct Report { results: Vec<AssertionResult> /* + summary counts */ }
```

`MetricProvider` trait verbatim from spec "Plugin/Metric Provider API":

```rust
pub trait MetricProvider {
    fn supports(path: &Path) -> bool;
    fn new(path: &Path) -> Result<Self> where Self: Sized;
    fn get(&mut self, metric: &str) -> Result<Value>;
}
```

---

## 4. DSL & Comparison Semantics (locked per spec)

The parser and engine must enforce the finalized grammar (spec "Assertion File Formats"):

- Grammar is **subject-first**: `<subject> <metric> <operator> <value>`.
- **No boolean shorthand.** Boolean metrics must be explicit: `bam has_index eq true` / `eq false`.
- **Relational/cross-subject** RHS is a bound subject name, not a path:
  - relational metric: `read1 paired_with eq read2`
  - cross-subject value: `read1 read_count eq read2` (same metric on the other subject)
- **Operators (DSL):** `eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `in`, `not_in`, `contains`, `matches`.
  `exists` / `has_index` are boolean **metrics**, not operators.
- **DSL→enum mapping:** `gte`→`Ge`, `lte`→`Le`, `not_in`→`NotIn`.
- **Size literals:** SI decimal suffixes `KB`, `MB`, `GB`, `TB` (e.g. `size gte 100MB`).
- **Canonical metric names:** index existence is `has_index` (not `indexed`); VCF key sets are
  `format_fields`, `filter_fields`, `info_fields`.

---

## 5. Phased Delivery

Each phase is independently shippable, ships with tests (AGENTS.md "Testing Requirements"),
and preserves exit-code and logging behavior.

### Phase 0 — Foundations & Dependencies
*Implements: spec "Architecture", "Packaging", "Logging", "Security/Reproducibility".*

- Configure `Cargo.toml` with `[lib]` + `[[bin]]` targets.
- Add dependencies (justified per AGENTS.md "Dependency Management", prefer Noodles):
  - `clap` (derive) — CLI parsing.
  - `serde` + `serde_yaml` — YAML assertion files.
  - `anyhow` (+ `thiserror`) — error handling (spec uses `anyhow::Context`).
  - `log` + a logger (e.g. `env_logger`/`fern`) — console + log-file output.
  - `noodles` (features: bam, cram, sam, fasta, fastq, vcf, bcf, bed, gtf, csi, tabix).
  - `sha2` / `md-5` — checksums for the generic provider.
  - `regex` — `matches` operator.
  - `rayon` — parallel evaluation (spec "Concurrency").
  - dev: `criterion` (benchmarks), `assert_cmd` + `predicates` (CLI integration tests).
- Validate selected dependency versions for CVEs before pinning; commit `Cargo.lock`, build `--locked`.
- Pin Rust toolchain (`rust-toolchain.toml`).
- Centralized exit-code module: `0` pass, `1` fail, `2` usage/config error.

### Phase 1 — Model & Parser (DSL + YAML)
*Implements: spec "Assertion File Formats and Examples", "CLI → Parser", §4 above.*

- `model.rs`: `Value`, `Operator`, `Assertion`, `Report`, `Status`.
- Plain-text DSL parser: subject-first grammar, comments (`#`), blank lines, the locked
  operator set, SI size literals, list/regex/bool/number literals, and bound-subject RHS.
- Enforce **no boolean shorthand** (clear parse error pointing at the line).
- YAML parser: shorthand-string assertions, the `name/inputs/assertions` bundle form, and the
  explicit `name/expression/inputs` form — all using subject-first expressions.
- **Tests:** DSL parsing, YAML parsing, boolean-shorthand rejection, relational/cross-subject
  parsing, size-unit parsing, and parse-error cases (exit code 2).

### Phase 2 — Engine / Runner + Comparators
*Implements: spec "Runner", "Rich comparators", "Assertion Evaluation Flow".*

- `AssertionRunner`: resolve subject via `--input` bindings → dispatch to provider →
  compare actual vs expected (value literal OR resolved cross-subject value) → record result.
- Comparator semantics for every `Operator` across `Value` variants (numeric, string, set,
  regex, contains), plus boolean (`eq true/false`) and relational (`paired_with eq <subject>`).
- Fail-fast by default; `--continue` evaluates all (spec "Fail-Fast vs. Report-All").
- Per-file caching/context so multiple metrics on one subject cause a single scan.
- **Tests:** per-comparator unit tests; runner tests with a stub provider; cross-subject and
  relational evaluation; fail-fast vs. continue behavior.

### Phase 3 — Generic Provider
*Implements: spec "GenericFileProvider", "Generic (any)" metrics table.*

- Metrics: `exists`, `size` (SI thresholds), `lines`, `md5`, `sha256`, `modified_time`.
- Streaming reads for size/lines/checksums (AGENTS.md "Performance": avoid full load).
- **Tests:** fixtures + unit tests; golden checksum comparison.

### Phase 4 — Registry + Provider dispatch
*Implements: spec "Metric Registry and Providers".*

- `MetricRegistry` maps file → provider via `supports(path)` (extension + header magic).
- Dispatch metric name to the owning provider; clear error on unsupported file/metric
  (flow node "Error: Unsupported file/metric").
- **Tests:** dispatch unit tests; unsupported-format and unknown-metric error paths.

### Phase 5 — Format Providers (Noodles-backed)
*Implements: spec "Supported Formats and Example Assertions", per-format provider bullets.*
One provider per PR, each with fixtures + golden tests:

1. **BAM/CRAM/SAM** — `read_count`, `mapped_reads`, `unmapped_reads`, `duplicate_reads`,
   `secondary_reads`, `supplementary_reads`, `sort_order`, `read_group_count`, `sample_names`,
   `has_index`, `reference_count`. Single-pass record scan; header-only for sort/RG.
   Golden vs `samtools flagstat`/`view`.
2. **FASTQ** — `read_count`, `average_read_length`, `quality_encoding`, `paired`/`paired_with`.
3. **FASTA** — `sequence_count`, `total_bases`, `longest_sequence`, `sequence_names`,
   `no_duplicate_sequence_names`.
4. **VCF/BCF** — `variant_count`, `snp_count`, `indel_count`, `sample_count`, `contigs`,
   `format_fields`, `filter_fields`, `info_fields`. Golden vs `bcftools stats`.
5. **Index providers** — BAI/CRAI/CSI/TBI: `exists`, `index_matches`/`matching_tabix`.
6. **BED/GFF/GTF** — `feature_count`, `sorted_by_coordinate`, `no_overlap`, `no_duplicate_ids`.

> **BigWig deferred.** Listed in the spec table but optional/add-via-crate. Do not add
> `bigtools`/`rustynetics` until a spec update explicitly approves it (AGENTS.md "File Format
> Support"). This is the only open scope question.

### Phase 6 — Reporting Layer
*Implements: spec "Outputs", "Logging, Exit Codes, and Pipeline Behavior".*

- stdout formatter: `[PASS]/[FAIL]` lines using the `bam (sample.bam)` virtual+physical format,
  with actual vs expected diagnostics and a final summary.
- stderr formatter for usage/parse/fatal errors and warning/error diagnostics.
- Log-file writer: timestamped execution trace; default path `bioassert.log`, overridable by
  `--log-file`; always written.
- **Tests:** snapshot/golden tests for stdout, stderr, and log-file content.

### Phase 7 — CLI Wiring
*Implements: spec "CLI Design", "Examples", "Help Text".*

- Root command `bioassert` (no subcommands) with flags: `--assertions` (repeatable),
  `--input name=path` (repeatable), `--log-file`, `--continue`/`--report-all`,
  `--quiet/-q`, `--verbose/-v`, `--version`, `--help`.
- Map CLI → library `run_assertions`; translate `Report` → stdout/stderr + log file + exit code.
- Logging levels wired through `log` (quiet/verbose); errors always to stderr.
- **Tests:** `assert_cmd` integration tests covering exit codes 0/1/2, console output, and
  log-file creation/override.

### Phase 8 — Concurrency & Performance
*Implements: spec "Concurrency and Performance", "Recommended Tests and Benchmarks".*

- Parallel evaluation across independent files/assertions via `rayon`; thread-limit option.
- Confirm single-pass caching holds under `--continue` (no re-scan).
- Criterion benchmarks: BAM `read_count` (large, with/without index), FASTQ avg length,
  `--continue` no-rescan.

### Phase 9 — Packaging, Distribution, CI/CD
*Implements: spec "Packaging and Distribution", "Security and Reproducibility".*

- Multi-stage `Dockerfile` (musl static build → `scratch`), per spec snippet, pinned base.
- GitHub Actions: `cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo audit`,
  `--locked` builds; release job publishes crate + multi-arch Docker image.
- Crate metadata (license Apache-2.0/MIT, semver), `CHANGELOG.md`.

### Phase 10 — nf-core Module & Docs
*Implements: spec "Nextflow / nf-core Module Integration", "Examples, Documentation, and Help".*

- `modules/nf-core/bioassert/main.nf` wrapping the CLI with meta-map channels; emits
  `bioassert.log` and propagates exit status.
- `examples/` assertion bundles (`aligned_bam.assert`, `paired_fastq.assert`, ...) matching the
  Quick Start.
- `README.md` mirroring the spec Quick Start; generated `--help`/man page; sample PASS/FAIL.

---

## 6. Cross-Cutting Requirements (every phase)

- **Error handling:** `Result<T, E>`; no `unwrap()/expect()` outside tests (AGENTS.md).
- **Exit codes preserved:** 0 / 1 / 2 semantics validated by integration tests.
- **Outputs preserved:** stdout summary, stderr diagnostics, always-written log file; no
  JSON/JUnit/TAP (removed from spec).
- **Streaming IO / minimal allocations** for large genomic files.
- **Determinism:** stable ordering of results in reports.
- **Tests alongside implementation:** unit + integration + regression (for any bug fix).
- **Docs synchronized:** update `docs/spec.md` first if behavior must change.

---

## 7. Traceability Matrix (summary)

| Spec Section | Plan Phase(s) |
|--------------|---------------|
| Quick Start | 7, 10 |
| CLI Design | 1, 7 |
| Architecture / Data Models | 0, 2, 3 |
| Plugin/Metric Provider API | 4, 5 |
| Supported Formats & Assertions | 5 |
| Assertion File Formats (DSL/YAML) | 1, §4 |
| nf-core Integration | 10 |
| Packaging & Distribution | 9 |
| Logging, Exit Codes, Pipeline Behavior | 2, 6, 7 |
| Security & Reproducibility | 0, 9 |
| Tests & Benchmarks | all (esp. 8) |

---

## 8. Open Questions

1. **BigWig** — remains out of scope until a dedicated spec update approves a crate
   (`bigtools`/`rustynetics`) and metric set.

All previously-open grammar, metric-naming, output-format, and size-unit questions are now
resolved in the current spec (subject-first DSL, explicit `eq true/false`, `paired_with eq
<subject>`, bare-subject cross-references, `has_index`, SI decimal sizes, stdout/stderr/log-file
outputs, and the `run_assertions` library API).

