# Specification: Generate command (bioassert generate)

## Context

bioassert today only evaluates assertions. Authoring an assertions file by hand is tedious and easy to
get wrong, especially for a new pipeline output where the author does not yet know the column count,
record count or sort order of the file. `bioassert generate <file>` removes that friction: it inspects
a file, composes a set of sensible default assertions from the file extension and the file's current
properties, writes them to a text assertions file (the same format `bioassert run` consumes), and tells
the user where it wrote them. The author then reviews and tightens the thresholds.

Generation is the inverse of evaluation. Today an `AssertionExecutor` takes a parsed metric plus an
expected value and produces a pass or fail. Generation goes the other way: it reads the file's current
properties and produces suggested assertions. There is no existing "suggest" capability, so this is a
purely additive layer. None of the executor or dispatch code changes; the new layer reuses the same
property-computing functions the executors already call (`exists`, `get_file_size`, `column_count`,
`line_count`, `read_header` plus counts, `read_records` plus counts).

This spec covers the design only. Implementation follows as a separate task.

## Constraints and key facts

- `Value::Display` (`src/core/values/value.rs`) already renders the way an assertions file needs:
  `BytesValue` renders like `1.00MB` (binary, 1024-based), `IntegerValue` renders as a plain number
  with no `K`/`M` suffix, `BooleanValue` renders `true`/`false`, `StringValue` renders verbatim. It is
  the single rendering path and is reused so generated values round-trip through the parser.
- `Comparator::Display` (`src/core/comparisons/comparator.rs`) renders SYMBOLS (`>=`, `==`), but an
  assertions file needs the KEYWORD form (`gte`, `eq`). The two must not be confused. Suggestions store
  the comparator as the keyword, not as the `Comparator` enum, so the symbol form can never leak.
- The delimited family maps a prefix to a delimiter (`delimiter_for_prefix("tsv") -> '\t'`) in
  `src/delimited/functions.rs`. Generation needs the reverse direction, extension to prefix. A new
  `prefix_for_extension` helper is required. Extensions happen to equal prefixes today, but the helper
  is the seam for future aliases such as `.tab` mapping to `tsv`.
- The metric's first segment is the provider name (`file`, `tsv`, `bam`, `fasta`). That is the grouping
  and comment key. The group header is derived from `metric.split('.').next()`, never from a provider
  field, so the grouping is data-driven and cannot drift from the metric strings.
- `parse_file` (`src/engine/parser.rs`) skips blank lines and lines starting with `#`, so `#` group
  headers, a provenance banner and blank separators are safe and are ignored on a later `run`.
- The +/- 50% band is expressed as two bounded assertions on the same metric: `gte floor(0.5R)` and
  `lte ceil(1.5R)`. Integer arithmetic: `band(n) = (n / 2, n + (n + 1) / 2)`. For `n = 4` this is
  `(2, 6)`, for `n = 3` it is `(1, 5)`, for `n = 0` it is `(0, 0)` (degenerate but valid).
- An output line is the assertion syntax `resource metric comparator value`, optionally followed by
  `  # comment`. The resource is the file path as given. A path containing whitespace is single-quoted
  so it round-trips through the grammar, the same rule the parser enforces for locators.
- The `text.*` family is inline-literal only and is not file-backed, so it contributes nothing when the
  resource is a file path.

## Design decisions

**The comparator is stored as a keyword, the value pre-rendered.** A suggestion carries
`comparator: &'static str` (one of `eq`, `gte`, `lte`) rather than the `Comparator` enum, because the
enum's `Display` renders symbols and an assertions file needs keywords. The set of comparators a
generator emits is tiny and fixed, so a keyword literal is the simplest correct representation and reads
identically to what lands in the file. A constructor `debug_assert!`s the keyword parses via
`Comparator::from_str` so a typo is caught in tests. The expected value is a pre-rendered `String`:
each provider computes a `Value` from the existing functions and renders it once via `Value::Display`,
and the band bounds are derived integers rendered directly. This keeps `Display` as the single source
of truth and makes the writer a dumb string formatter.

**One provider per resource family, not per executor.** The user described "each metric executor"
contributing, and separately the output being grouped by "metric provider" which is the first segment
of the metric name. These pull in opposite directions. A per-executor provider fragments the file
work (every bam executor would re-read the header) and scatters the "what is a sensible default set for
a TSV" policy across many tiny units. A per-family provider reads the file once, emits the family's
whole default set in broadest-first order, and tags each suggestion with its full metric string. The
group header is then derived from the metric prefix at write time, so "comment by provider equals the
first segment" is satisfied by the data, not by the provider count. This is also faithful to the
existing architecture, where each family's `functions.rs` already centralises the I/O and the
thread-local caches. The per-family provider is the "metric provider" the user referred to.

**The universal file group stays broad; bands are reserved for domain magnitudes.** Every file gets
`file.exists eq true` and `file.size gt 0B`. The +/- 50% band is applied only to domain quantities that
legitimately vary run to run (delimited rows, fasta total length). This matches the user putting
`file.exists` and `file.size` as the broadest checks and the +/- 50% band on rows. Counts that
represent a schema (delimited columns, fasta records, bam references) are pinned with `eq`, because a
change there is a real regression.

**The orchestrator returns data, the binary owns I/O.** `generate` returns the rendered string plus any
warnings; the binary writes the file and prints the message. This mirrors how `executor::execute_all`
returns a report and `main.rs` does the writing, and it keeps the orchestrator unit-testable without
touching the filesystem.

**The output path is derived and never clobbered silently.** The default output is
`<file>.assertions.txt`, mirroring how report files derive `<file>.log`. The command refuses to
overwrite an existing output unless `--force`, because the output is hand-editable and an accidental
re-run should not destroy a tightened assertions file. `--output` overrides the path.

## Syntax

### Output file format

The generated file is ordinary bioassert assertion syntax with `#` comments. It opens with a two-line
provenance banner, then one group per contributing provider. Each group is led by a `# <prefix>` header
and separated from the next by a blank line. Each assertion is `resource metric comparator value`, with
an optional inline `# comment`.

```text
# Generated by bioassert generate from junctions.tsv
# Review and adjust thresholds before use.

# file
junctions.tsv file.exists eq true  # file is present
junctions.tsv file.size gt 0B  # file is not empty

# tsv
junctions.tsv tsv.columns.count eq 12  # expected 12 columns
junctions.tsv tsv.lines.count gte 2  # rows within +/- 50% of 4
junctions.tsv tsv.lines.count lte 6
```

### Structured model

A suggestion is the structured form of one output line, defined in the new `src/generate/` module:

```rust
pub struct Suggestion {
    pub resource: String,         // the file path as passed to generate
    pub metric: String,           // e.g. "file.size", "tsv.lines.count"
    pub comparator: &'static str, // keyword form: "eq", "gt", "gte", "lte"
    pub expected: String,         // rendered expected value, e.g. "0B", "12", "true"
    pub comment: Option<String>,  // optional inline explanation
}
```

## Semantics

- **Provider order.** An ordered registry `providers()` mirrors the dispatch order in
  `src/engine/executor.rs`, broadest first: `FileProvider`, `DelimitedProvider`, `BamProvider`,
  `FastaProvider`. `FileProvider` handles any existing file; each format provider handles only its
  extensions. This single list is the ordering authority, so generated files and run-time dispatch
  agree on precedence.
- **Contribution is optional.** A provider that does not handle the file is skipped silently. A provider
  that handles the file but whose property computation fails records a warning and is skipped; other
  providers still contribute.
- **Grouping.** The accumulated suggestions are grouped by metric prefix (`metric.split('.').next()`),
  preserving first-seen order. A `# <prefix>` header precedes each group and a blank line separates
  groups. A two-line provenance banner precedes the first group.
- **Band arithmetic.** `band(n) = (n / 2, n + (n + 1) / 2)`, giving `floor(0.5n)` and `ceil(1.5n)`. A
  provider emits two suggestions from one banded quantity, a `gte` lower bound and an `lte` upper bound.
- **Default assertion sets.** Comparators are keyword form. Comments are the suggested inline text.

  | Provider | Handles | Suggestions |
  |---|---|---|
  | file | any existing file | `file.exists eq true`; `file.size gt 0B` |
  | delimited | `.tsv`, `.csv`, `.psv` | `{p}.columns.count eq C`; `{p}.lines.count gte floor(0.5R)`; `{p}.lines.count lte ceil(1.5R)` |
  | bam | `.bam` | `bam.header.sq.count eq S`; `bam.header.rg.count gte 1` |
  | fasta | `.fasta`, `.fa`, `.fna` | `fasta.seq.count eq N`; `fasta.length gte floor(0.5L)`; `fasta.length lte ceil(1.5L)` |

  C is the column count of the first line, R is the total line count, S is the `@SQ` count, N is the
  record count, L is the total sequence length. `{p}` is the prefix resolved from the extension.

- **Exit codes.** `0` on success (output written with at least one suggestion). `2` on a fatal error:
  the output exists without `--force`, the write fails, or no suggestions could be produced at all.
  Warnings alone do not change the exit code, a partial generation is still useful and exits `0`.

### Message formats

- On success, stdout gets `Wrote <N> assertions to <path>`.
- Each provider warning goes to stderr, formatted through the existing `format_outcome(Outcome::Error, ...)`
  path so color and icons stay consistent with the rest of the binary.
- Fatal errors print through `fatal(...)` and exit `2`, the same as the other subcommands.

## Examples

```text
# .bam input (sample.bam: 1 reference sequence, 2 read groups)
# Generated by bioassert generate from sample.bam
# Review and adjust thresholds before use.

# file
sample.bam file.exists eq true  # file is present
sample.bam file.size gt 0B  # file is not empty

# bam
sample.bam bam.header.sq.count eq 1  # 1 reference sequence
sample.bam bam.header.rg.count gte 1  # at least one read group
```

```text
# .fasta input (sample.fasta: 3 records, total length 42)
# Generated by bioassert generate from sample.fasta
# Review and adjust thresholds before use.

# file
sample.fasta file.exists eq true  # file is present
sample.fasta file.size gt 0B  # file is not empty

# fasta
sample.fasta fasta.seq.count eq 3  # 3 sequence records
sample.fasta fasta.length gte 21  # total length within +/- 50% of 42
sample.fasta fasta.length lte 63
```

```text
# Unknown extension (notes.txt): only the universal file group
# Generated by bioassert generate from notes.txt
# Review and adjust thresholds before use.

# file
notes.txt file.exists eq true  # file is present
notes.txt file.size gt 0B  # file is not empty
```

## Implementation

### 1. New module (`src/lib.rs`, `src/generate/`)

Add `pub mod generate;` to `src/lib.rs`. Create `src/generate/` with `mod.rs` re-exporting the public
items, plus `suggestion.rs`, `provider.rs`, `orchestrator.rs` and a `providers/` subdirectory.

### 2. Suggestion (`src/generate/suggestion.rs`)

Define the `Suggestion` struct from the Syntax section. Add a constructor that `debug_assert!`s the
comparator keyword parses via `Comparator::from_str`. Add `render(&self) -> String` producing
`resource metric comparator expected`, appending `  # comment` when present, and single-quoting
`resource` when it contains whitespace and is not already quoted. Add the free function
`band(n: u64) -> (u64, u64)`.

### 3. Provider trait and registry (`src/generate/provider.rs`)

```rust
pub trait SuggestionProvider {
    fn handles(&self, path: &Path) -> bool;
    fn suggest(&self, path: &Path) -> Result<Vec<Suggestion>, BioAssertError>;
}

pub fn providers() -> Vec<Box<dyn SuggestionProvider>> {
    vec![
        Box::new(FileProvider),
        Box::new(DelimitedProvider),
        Box::new(BamProvider),
        Box::new(FastaProvider),
    ]
}
```

### 4. Providers (`src/generate/providers/`) and `prefix_for_extension`

One unit struct per family, each reusing the existing property functions:

- `file.rs` `FileProvider`: `handles` returns true for any path. `suggest` always emits
  `file.exists eq true`. When the file exists it also emits `file.size gt 0B` (read via the existing
  size function); when the file is missing it emits only the exists assertion (no current size to base
  anything on) and returns `Ok`.
- `delimited.rs` `DelimitedProvider`: `handles` lowercases the extension and checks
  `prefix_for_extension(ext).is_some()`. `suggest` resolves the prefix, then `delimiter_for_prefix` for
  `column_count`, computes `line_count`, and emits the columns and rows-band suggestions with the
  prefix-qualified metric strings.
- `bam.rs` `BamProvider`: `handles` accepts `.bam`. `suggest` calls `read_header` then `reference_count`
  and `read_group_count`, emitting `bam.header.sq.count eq S` and `bam.header.rg.count gte 1`. A parse
  failure returns `Err`.
- `fasta.rs` `FastaProvider`: `handles` accepts `.fasta`, `.fa`, `.fna`. `suggest` calls `read_records`
  then `record_count` and `total_length`, emitting the record-count and length-band suggestions.

Add `pub(crate) fn prefix_for_extension(ext: &str) -> Option<&'static str>` in
`src/delimited/functions.rs` next to `delimiter_for_prefix` (`"tsv" => "tsv"`, `"csv" => "csv"`,
`"psv" => "psv"`, else `None`). Expose any family property function currently private to `generate` by
making the `fn` and its submodule `pub` in the family `mod.rs`. Convert `FileError` into
`BioAssertError` using the existing `From` impl the executors already rely on.

### 5. Orchestrator (`src/generate/orchestrator.rs`)

```rust
pub struct GenerateResult {
    pub suggestions: Vec<Suggestion>,
    pub rendered: String,
    pub warnings: Vec<String>,
}

pub fn generate(path: &Path) -> GenerateResult
```

Iterate `providers()` in order. Skip when `handles` is false. On `suggest` `Ok`, extend the accumulator;
on `Err`, push `format!("{provider}: {e}")` to warnings and continue. Render by walking the accumulated
suggestions once, emitting a `# <prefix>` header (and a leading blank line for every group after the
first) whenever the metric prefix changes, then each suggestion's `render()`. Prepend the two-line
provenance banner. Return the accumulator, the rendered body and the warnings.

### 6. CLI (`src/cli.rs`)

Add to the `Commands` enum:

```rust
Generate {
    #[arg(help = "Path to the file to inspect")]
    file: PathBuf,
    #[arg(short, long, value_name = "FILE",
          help = "Write generated assertions to FILE instead of <file>.assertions.txt")]
    output: Option<PathBuf>,
    #[arg(long, help = "Overwrite the output file if it already exists")]
    force: bool,
}
```

### 7. Output path and binary wiring (`src/report.rs`, `src/main.rs`)

Add `resolve_output_file(input: &Path, explicit: Option<PathBuf>) -> PathBuf` in `src/report.rs`
alongside `resolve_report_file`: return `explicit` when given, else `<input>.assertions.txt`.

In `src/main.rs`, handle `Commands::Generate` before the assertion-gathering and report pipeline:
resolve the output path; if it exists and `--force` is not set, `fatal(...)`; call
`bioassert::generate::generate(file)`; if there are no suggestions, `fatal(...)`; print each warning to
stderr; write `result.rendered` with `std::fs::write`, `fatal` on error; print
`Wrote <N> assertions to <path>` on stdout; exit `0`.

### 8. Docs (`CLAUDE.md`, `README.md`)

Add a "Generate command" note to `CLAUDE.md`, a bullet for the `src/generate/` module in the module
list, and the new "Adding a new metric" interaction (a new family should also add a provider). Add a
`bioassert generate` usage section to `README.md`.

## Test plan

### Unit tests

- `suggestion.rs`: `render` with and without a comment; resource single-quoted when the path has a
  space; the keyword comparator is preserved with no symbol leakage; `band(n)` for `n` in
  `{0, 1, 2, 3, 4, 100}`.
- `providers/file.rs`: on a temp file with content, asserts `file.exists eq true` and `file.size gt 0B`;
  on a missing path, asserts only `file.exists eq true`.
- `providers/delimited.rs`: on `tests/data/example.tsv` (3 columns, 3 lines) asserts
  `tsv.columns.count eq 3`, `tsv.lines.count gte 1`, `tsv.lines.count lte 5`; on `tests/data/junctions.tsv`
  (12 columns, 4 lines) asserts `eq 12`, `gte 2`, `lte 6`; `handles` accepts `.tsv`/`.csv`/`.psv` and
  rejects `.bam`. Add a `prefix_for_extension` mapping test in `src/delimited/functions.rs`.
- `providers/bam.rs`: on `tests/data/sample.bam` (1 `@SQ`, 2 `@RG`) asserts `bam.header.sq.count eq 1`
  and `bam.header.rg.count gte 1`; a non-BAM input makes `suggest` return `Err`.
- `providers/fasta.rs`: on `tests/data/sample.fasta` (3 records, length 42) asserts `fasta.seq.count eq 3`,
  `fasta.length gte 21`, `fasta.length lte 63`; on `tests/data/empty.fasta` asserts `seq.count eq 0` and
  a `gte 0`/`lte 0` band.
- `orchestrator.rs`: on a `.tsv` fixture, asserts the `file` group precedes the `tsv` group, the
  `# file` and `# tsv` headers and blank separation are present; an unknown extension yields only the
  `file` group; a missing file populates warnings and still emits `file.exists eq true`.
- `report.rs`: `resolve_output_file` derives `<input>.assertions.txt`, and an explicit `--output` wins.

### Integration tests and fixtures

New `tests/generate_subcommand_test.rs`, modelled on `tests/resource_subcommand_test.rs`, running the
compiled binary with `--color=never` for determinism:

- Run `generate` against a committed fixture with `--output` into a `tempfile` directory, read the
  output file and `assert_snapshot!` its body. One snapshot each for a `.tsv` (`junctions.tsv`), a
  `.bam` (`sample.bam`), a `.fasta` (`sample.fasta`) and an unknown extension (`empty_file.txt`).
- A `--force` test: a second run without `--force` exits `2`, with `--force` exits `0`.
- A round-trip test: `generate` a fixture to a temp output, then `run` that output and assert exit `0`,
  proving every generated assertion holds against the file it was generated from.

All fixtures already exist in `tests/data/` (`sample.bam`, `sample.fasta`, `empty.fasta`, `example.tsv`,
`example.csv`, `example.psv`, `junctions.tsv`, `empty_file.txt`). No new fixtures are required.
Snapshots live in `tests/snapshots/` and are reviewed with `cargo insta review`.

## Critical files

- `src/lib.rs` (add `pub mod generate;`)
- `src/generate/suggestion.rs`, `src/generate/provider.rs`, `src/generate/orchestrator.rs`,
  `src/generate/mod.rs`, `src/generate/providers/{file,delimited,bam,fasta}.rs`,
  `src/generate/providers/mod.rs` (new)
- `src/delimited/functions.rs` (add `prefix_for_extension`)
- family `functions.rs`/`mod.rs` as needed to expose reused property functions
- `src/cli.rs` (add the `Generate` subcommand)
- `src/report.rs` (add `resolve_output_file`)
- `src/main.rs` (add the `Generate` arm before the report pipeline)
- `tests/generate_subcommand_test.rs` (new), `tests/snapshots/` (new snapshots)
- `CLAUDE.md`, `README.md` (docs)

## Verification

- `cargo test` passes, including the new unit and integration tests.
- `cargo run -- generate tests/data/junctions.tsv` writes `tests/data/junctions.tsv.assertions.txt`,
  prints `Wrote 5 assertions to ...`, and a second run without `--force` exits `2`.
- `cargo run -- run tests/data/junctions.tsv.assertions.txt` exits `0`, confirming the generated
  assertions hold against their source file.
- `cargo run -- generate tests/data/empty_file.txt` emits only the `file` group.
- `cargo run -- generate tests/data/sample.bam` emits the `file` and `bam` groups with
  `bam.header.sq.count eq 1`.

## Deferred

- Banding `file.size` for a tighter universal size check, rather than `gt 0B`. Deferred because the
  human-readable byte rendering is lossy above 1KB and an exact-byte band reads poorly. The broad
  `gt 0B` always round-trips.
- A stricter `bam.header.rg.count eq G` for pipelines that fix the read-group set, offered as an
  alternative to the `gte 1` presence floor.
- Per-record or per-cell suggestions (for example `fasta.seq.0.name`), which need heuristics about
  which records or cells are stable enough to assert on.
- Suggestions for compression (`file.compression`) when a compressed extension is detected.
