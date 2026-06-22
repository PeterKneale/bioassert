# Specification: FASTA sequence assertions

## Context

`bioassert` validates pipeline outputs with declarative assertions (`<file> <metric> <comparator> <value>`).
Today it covers generic files (`file.*`), delimited text (`csv/tsv/psv.*`), and BAM headers (`bam.header.*`).
Bioinformatics pipelines also produce and consume FASTA files: reference genomes, assembled contigs, extracted
transcripts, and protein sets. Pipelines silently emit the wrong thing — an empty assembly, a reference with the
wrong number of contigs, a sequence truncated to zero length, a renamed or mis-described record — which breaks or
silently corrupts everything downstream. This feature adds assertions over a FASTA file's records: how many there
are, and each record's name, description, and length, using the
[`noodles`](https://docs.rs/noodles-fasta/latest/noodles_fasta/) crate to parse FASTA.

The intended outcome: a user can write checks like `ref.fasta fasta.seq.count eq 3` or
`ref.fasta fasta.seq.0.length gte 1000` in an assertions file and have them pass/fail like any other metric.

## Constraints and key facts

- **No pest grammar change is needed.** `src/engine/cli.pest` defines
  `metric = @{ identifier ~ ("." ~ metric_segment)+ }`,
  which already accepts any dot-separated chain of identifiers/numbers (e.g. `fasta.seq.0.length`). Executors do all
  real
  parsing in `try_parse` via `metric.split('.')` + slice matching — the same pattern `DelimitedCellExecutor`
  (`src/delimited/cell/executor.rs`) and the BAM executors use.
- **Values with special characters must be quoted.** The grammar's bare string is `ASCII_ALPHANUMERIC+` only, so
  `chr1` works unquoted but `NC_000001.11` (dots), `Homo sapiens chromosome 1` (spaces), and descriptions need
  single/double quotes. The grammar already supports quoted strings, so this is a documentation note, not a code change.
- **Executor contract** (`src/core/executor.rs`): `try_parse(metric: &str) -> Option<Self>` then
  `execute(self, &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError>` returning
  `{ success, actual: Value }`. Dispatch is first-match-wins in `src/engine/executor.rs::evaluate`.
- **Value/Comparator** (`src/core/values/`, `src/core/comparisons/`): `Value` is `Boolean/Bytes/Integer/String`.
  `Comparator::compare` is numeric (used by counts and lengths); `compare_string` handles
  `eq/ne/starts/ends/contains/matches` (used by names and descriptions).
- **noodles API** (`noodles-fasta`, pulled by the `noodles` meta-crate's `fasta` feature):
  `let mut r = File::open(path).map(BufReader::new).map(fasta::io::Reader::new)?;` then `r.records()` yields
  `io::Result<fasta::Record>`. From a `Record`: `name()` → record name as bytes, `description()` →
  `Option<&BStr>`, `sequence()` → `&Sequence` with `len() -> usize`. Names/descriptions are converted to `String`
  with `String::from_utf8_lossy` for comparison. (Pin the exact `noodles-fasta` version at implementation time and
  re-verify these signatures; `name()`'s return type has shifted between `&[u8]` and `&BStr` across versions.)

## Design decisions

- **Records addressed by index** (`fasta.seq.0.name`), following file order — robust given that FASTA names contain
  dots (`NC_000001.11`) and spaces, which collide with the dot-delimited metric grammar. This mirrors BAM read
  groups (`bam.header.rg.0.sm`).
- **Presence via count + boolean `.present` suffix** — a missing record or a record with no description yields
  `false`, not an error.
- **Scope**: record count, whole-file total length, and per-record name / description / length. Sequence *content*
  checks (alphabet/IUPAC validation, GC content, ambiguous-base counts) are explicitly out of scope here and are
  the obvious next additions.
- **Cache a digest, not the sequences.** A `run` over an assertions file typically issues many `fasta.*`
  assertions against the same file. Re-reading the file for each would be wasteful, but caching the parsed
  sequences would be ruinous — a reference genome is gigabytes. So `src/fasta/functions.rs` reads each file once
  and caches only a per-record digest `{ name: String, description: Option<String>, length: u64 }`, never the
  sequence bytes. See "Record caching" below.
- **Namespaced under `fasta.seq.*`** for per-record metrics, with `fasta.length` as a whole-file aggregate. A FASTA
  file is a list of sequence records, optionally accompanied by an index (`.fai`). Putting record metrics under
  `fasta.seq.*` reserves `fasta.index.*` / `fasta.fai.*` for future index checks (index present, line width,
  offsets) and leaves the top-level `fasta.*` namespace for file-integrity metrics — mirroring how `bam.header.*`
  reserves `bam.records.*`.

## Metric format

| Metric                              | Value type | Meaning                                                                           |
|-------------------------------------|------------|-----------------------------------------------------------------------------------|
| `fasta.seq.count`                   | integer    | number of sequence records                                                        |
| `fasta.length`                      | integer    | total bases summed across all records                                             |
| `fasta.seq.<n>.name`                | string     | name (ID) of the nth record — the first whitespace-delimited token of the header  |
| `fasta.seq.<n>.description`         | string     | description of the nth record — header text after the name (empty/absent if none) |
| `fasta.seq.<n>.length`              | integer    | length in bases of the nth record's sequence                                      |
| `fasta.seq.<n>.present`             | bool       | whether a record exists at index n                                                |
| `fasta.seq.<n>.description.present` | bool       | whether the nth record has a (non-empty) description                              |

`<n>` is a 0-based index following file order. `name` is always present for a record; `description` is optional.

### Examples

#### FASTA FILE

```
>chr1 Homo sapiens chromosome 1
ACGTACGTACGTACGTACGT
ACGTACGT
>chr2
ACGTACGTAC
>NC_000001.11 alternate assembly
ACGT
```

All examples below are evaluated against the sample file above. Record lengths are: `chr1` = 28 bases
(20 + 8 across two lines), `chr2` = 10 bases, `NC_000001.11` = 4 bases; total = 42. The trailing comment is
the expected outcome.

#### Counts and total length

```
ref.fasta fasta.seq.count          eq  3        # PASS
ref.fasta fasta.seq.count          gte 1        # PASS
ref.fasta fasta.seq.count          lt  10       # PASS
ref.fasta fasta.length             eq  42       # PASS
ref.fasta fasta.length             gte 40       # PASS
ref.fasta fasta.seq.count          eq  5        # FAIL (actual 3)
```

#### Names and descriptions

Values containing a dot, dash, colon, or space must be quoted; bare values must be alphanumeric.

```
ref.fasta fasta.seq.0.name         eq  chr1                          # PASS
ref.fasta fasta.seq.1.name         eq  chr2                          # PASS
ref.fasta fasta.seq.2.name         eq  'NC_000001.11'                # PASS (quote: dot)
ref.fasta fasta.seq.0.description  eq  'Homo sapiens chromosome 1'   # PASS (quote: spaces)
ref.fasta fasta.seq.0.name         contains chr                      # PASS
ref.fasta fasta.seq.0.name         starts ch                         # PASS
ref.fasta fasta.seq.2.name         matches '^NC_[0-9]+\.[0-9]+$'     # PASS (regex)
ref.fasta fasta.seq.0.name         ne  chr2                          # PASS
ref.fasta fasta.seq.0.name         eq  scaffold1                     # FAIL (actual chr1)
```

#### Per-record length

```
ref.fasta fasta.seq.0.length       eq  28       # PASS
ref.fasta fasta.seq.1.length       eq  10       # PASS
ref.fasta fasta.seq.2.length       eq  4        # PASS
ref.fasta fasta.seq.0.length       gte 20       # PASS
ref.fasta fasta.seq.2.length       gt  100      # FAIL (actual 4)
```

#### Presence (never errors on absence)

```
ref.fasta fasta.seq.0.present              eq  true     # PASS (record 0 exists)
ref.fasta fasta.seq.2.present              eq  true     # PASS
ref.fasta fasta.seq.3.present              eq  false    # PASS (only 3 records)
ref.fasta fasta.seq.0.description.present  eq  true     # PASS (chr1 has a description)
ref.fasta fasta.seq.1.description.present  eq  false    # PASS (chr2 has no description)
```

#### Errors (exit code 2)

```
ref.fasta   fasta.seq.3.name        eq  X       # ERROR (record index out of range)
ref.fasta   fasta.seq.1.description eq  X       # ERROR (no description — use .present to test softly)
missing.fasta fasta.seq.count       eq  3       # ERROR (file cannot be opened)
notafasta.bin fasta.seq.count       eq  3       # ERROR (not valid FASTA)
```

### Semantics

Value metrics (`fasta.seq.<n>.name`, `fasta.seq.<n>.description`, `fasta.seq.<n>.length`) **error** if the record
index is out of range, or (for `description`) if the record has no description — consistent with
`DelimitedCellExecutor` erroring on a missing cell and the BAM tag executors erroring on a missing tag. The
`.present` metrics **never error** on absence — they return `false`. This is how a user safely tests "is this set"
versus "what is its value". (`name` and `length` are always present for an existing record, so they have no
`.present` form; record existence is `fasta.seq.<n>.present`.)

## Implementation

### 1. Dependency (`Cargo.toml`)

Add the `fasta` feature to the existing `noodles` dependency:

```toml
noodles = { version = "0.111", features = ["bam", "sam", "fasta"] }
```

### 2. New module `src/fasta/` (mirrors `src/bam/`)

- `src/fasta/mod.rs` — declares submodules and re-exports the executors.
- `src/fasta/functions.rs` — shared noodles work, the single place that touches the crate:
    - A digest type, e.g.
      `pub struct FastaRecord { pub name: String, pub description: Option<String>, pub length: u64 }`.
    - `read_records(file: &Path) -> Result<Rc<Vec<FastaRecord>>, FileError>` — **cached** (see "Record caching");
      on a miss it opens the file, iterates `reader.records()`, and builds the digest. Maps `io::Error` via
      `FileError::new`.
    - helpers take `&[FastaRecord]` (so they work straight off the cached `Rc`): `record_count(&[FastaRecord]) -> u64`,
      `total_length(&[FastaRecord]) -> u64`, `record_present(records, n) -> bool`,
      `record_name(records, n) -> Option<String>`, `record_description(records, n) -> Option<String>`,
      `record_length(records, n) -> Option<u64>`.
    - every `fasta.*` executor's `execute` calls `read_records` and then a helper; none open the file directly.
- `src/fasta/count/executor.rs` — `FastaCountExecutor`: `try_parse` matches `[ "fasta", "seq", "count" ]` and
  `[ "fasta", "length" ]`; `execute` returns `Value::from_integer(expected)` numeric-compared against the count /
  total length.
- `src/fasta/sequence/executor.rs` — two executors (exact-shape `try_parse`, so dispatch order is irrelevant):
    - `FastaSequenceFieldExecutor`: `[ "fasta", "seq", n, field ]` for `field ∈ {name, description}` →
      `StringValue` + `compare_string`, errors if absent; and `field == "length"` → `IntegerValue` +
      numeric `compare`, errors if the index is out of range. (One executor branching on `field`, since all three
      are per-record lookups at index n.)
    - `FastaSequencePresentExecutor`: `[ "fasta", "seq", n, "present" ]` and
      `[ "fasta", "seq", n, "description", "present" ]` → `Value::from_boolean(expected)` compared against the boolean.

Each `executor.rs` carries a `#[cfg(test)] mod tests` of `try_parse` cases (accept valid shapes; reject bad
prefix / unknown field / non-numeric index), matching `src/delimited/cell/executor.rs` and the BAM executor tests.

### 3. Record caching

Mirror `src/bam/functions.rs::read_header`, but cache the **digest** rather than the sequences so memory stays
bounded regardless of genome size:

```rust
thread_local! {
    static RECORD_CACHE: RefCell<HashMap<PathBuf, Rc<Vec<FastaRecord>>>> = RefCell::new(HashMap::new());
}

pub fn read_records(file: &Path) -> Result<Rc<Vec<FastaRecord>>, FileError> {
    if let Some(r) = RECORD_CACHE.with(|c| c.borrow().get(file).cloned()) {
        return Ok(r);
    }
    let mut reader = File::open(file)
        .map(BufReader::new)
        .map(fasta::io::Reader::new)
        .map_err(|e| FileError::new(file, e))?;
    let mut records = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| FileError::new(file, e))?;
        records.push(FastaRecord {
            name: String::from_utf8_lossy(record.name()).into_owned(),
            description: record.description().map(|d| d.to_string()),
            length: record.sequence().len() as u64,
        });
    }
    let records = Rc::new(records);
    RECORD_CACHE.with(|c| c.borrow_mut().insert(file.to_path_buf(), Rc::clone(&records)));
    Ok(records)
}
```

Rationale and choices — identical to BAM's, plus the digest decision:

- **`thread_local!` over threading a cache through `execute`** — keeps the change contained to the fasta module;
  the execution loop is single-threaded and sequential.
- **Digest only, never sequence bytes** — a FASTA may be many GB; we only need name/description/length, so we
  discard the sequence as we go. This is the deliberate departure from BAM (which caches the whole header).
- **Cache successes only.** Errors (missing file, not FASTA) are cheap, rare, and `FileError` is not `Clone`.
- **Key by the path as given** (`PathBuf` from `assertion.file`), like BAM — dedupes without a `canonicalize` syscall.
- **Expose `pub(crate) fn clear_cache()`** for unit tests so fixtures in different tests cannot observe each other.

A unit test asserts caching: `clear_cache()`, `read_records` once, delete/rename the temp file, then
`read_records` again — the second call still succeeds from cache and returns an `Rc` that is `Rc::ptr_eq` to the first.

### 4. Reuse `core::strip_quotes`

The FASTA name/description executors quote-strip their expected value via `core::strip_quotes` (already shared in
`src/core/`, used by the delimited and BAM string executors). No move needed.

### 5. Wire dispatch (`src/engine/executor.rs::evaluate`)

Add `try_parse` lines after the BAM executors (before the final `Err`):

```rust
if let Some(e) = FastaCountExecutor::try_parse( & assertion.metric) { return dispatch(e, assertion, request); }
if let Some(e) = FastaSequenceFieldExecutor::try_parse( & assertion.metric) { return dispatch(e, assertion, request); }
if let Some(e) = FastaSequencePresentExecutor::try_parse( & assertion.metric) { return dispatch(e, assertion, request); }
```

Add the corresponding `use crate::fasta::{...}` import, and `pub mod fasta;` in `src/lib.rs` alongside the existing
module declarations.

### 6. Tests

**The canonical fixture is the sample file shown above**, committed verbatim as `tests/data/sample.fasta` (plain
text — unlike the binary `sample.bam`, no generator binary is needed). It must encode exactly: 3 records;
`chr1` (description `Homo sapiens chromosome 1`, length 28), `chr2` (no description, length 10),
`NC_000001.11` (description `alternate assembly`, length 4); total length 42 — so the assertions and snapshot are
deterministic.

- **Unit tests** (`src/fasta/functions.rs`): a helper writes this exact FASTA to a `NamedTempFile`, then asserts
  each function against the known values — e.g. `record_count == 3`, `total_length == 42`,
  `record_name(r, 0) == Some("chr1")`, `record_name(r, 2) == Some("NC_000001.11")`,
  `record_description(r, 0) == Some("Homo sapiens chromosome 1")`, `record_description(r, 1) == None`,
  `record_length(r, 0) == Some(28)`, `record_present(r, 3) == false`. Each executor's `#[cfg(test)] mod tests`
  covers `try_parse` shapes.
- **Caching test** (`src/fasta/functions.rs`): as described in "Record caching".
- **Integration test + snapshot** (`tests/fasta_subcommand_test.rs`): mirror `tests/bam_subcommand_test.rs` /
  `tests/run_subcommand_test.rs` — invoke the binary via
  `CARGO_BIN_EXE_bioassert --color=never run tests/data/fasta_assertions.txt` and capture stdout with
  `insta::assert_snapshot!`. `tests/data/fasta_assertions.txt` holds a mix of PASS, FAIL, and ERROR lines from the
  examples above so the snapshot exercises all three outcomes and verifies the actual values are reported. Add
  focused `assert`-subcommand cases (exit 0 / 1 / 2) like `tests/assert_subcommand_test.rs`, including the
  out-of-range-index and missing-description ERROR cases.

### 7. Docs (`CLAUDE.md`)

Add a `src/fasta/` bullet to the architecture section (a `fasta.seq.*` / `fasta.length` namespace built on
`noodles`, with `functions.rs` the only place touching `noodles` and a path-keyed digest cache), extend "Adding a
new metric" to mention the fasta module, and reuse the existing note that expected values containing
non-alphanumeric characters (dots, dashes, colons, spaces) must be quoted.

## Test plan

This consolidates the testing work implied by section 6 into a concrete inventory: the fixture files to create,
the unit/integration tests to write, and which snapshots to accept. It follows the existing conventions in `tests/`
(see `tests/bam_subcommand_test.rs` and `tests/run_subcommand_test.rs`): the per-feature run snapshot is driven by
an **all-passing** assertions file (the integration test asserts exit 0), while FAIL and ERROR paths are covered by
focused `assert`-subcommand cases checking exit codes 1 and 2.

### Test files to create

| File                                         | Kind             | Tracked? | Contents                                                                                                                                                                                                                  | How generated                                                                                          |
|----------------------------------------------|------------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| `tests/data/sample.fasta`                    | fixture          | yes      | The canonical 3-record file from the examples above, verbatim: `chr1` (description `Homo sapiens chromosome 1`, 28 bases over two lines), `chr2` (no description, 10 bases), `NC_000001.11` (description `alternate assembly`, 4 bases). Total 42. | Hand-authored plain text. Unlike `sample.bam`, **no generator binary** — committed as-is.       |
| `tests/data/empty.fasta`                     | fixture          | yes      | A file with 0 records (empty, or only blank/comment lines). Exercises `fasta.seq.count eq 0` and `fasta.length eq 0`.                                                                                                       | Hand-authored (e.g. an empty file).                                                                    |
| `tests/data/fasta_assertions.txt`            | fixture          | yes      | An **all-passing** batch of `fasta.*` assertions against `tests/data/sample.fasta`, grouped with `#` comment headers (counts, total length, names/descriptions incl. quoted values and `contains`/`starts`/`matches`, per-record lengths, presence). Mirrors `tests/data/bam_assertions.txt`. | Hand-authored, with a header comment naming the fixture it targets.                  |
| `tests/fasta_subcommand_test.rs`             | integration test | yes      | The test module below (run snapshot + focused `assert` cases). Mirrors `tests/bam_subcommand_test.rs`.                                                                                                                     | Hand-authored Rust.                                                                                    |
| `tests/snapshots/fasta_subcommand_test__run_fasta_passing_stdout.snap` | snapshot | yes      | Captured stdout of `run tests/data/fasta_assertions.txt`.                                                                                                                                                                  | **Generated** by `cargo insta review` / `INSTA_UPDATE=always cargo test`, then committed.              |

No invalid-FASTA fixture is added: the "not valid FASTA" ERROR path reuses an existing non-FASTA fixture
(`tests/data/empty_file.txt` or `tests/data/sample.bam`), exactly as `bam_subcommand_test.rs` reuses
`tests/data/empty_file.txt` for its non-BAM error case. The missing-file ERROR path uses a path that does not exist
(`tests/data/missing.fasta`), which is **not** created.

### Tests to write

**Unit — `src/fasta/functions.rs` (`#[cfg(test)] mod tests`)**

A helper writes the canonical FASTA to a `NamedTempFile`; each test calls `clear_cache()` first so fixtures cannot
observe each other. Assert the helpers against the known values:

- `record_count(&r) == 3`; on 0-record content, `== 0`.
- `total_length(&r) == 42`.
- `record_name(&r, 0) == Some("chr1")`, `record_name(&r, 2) == Some("NC_000001.11")`, `record_name(&r, 3) == None`.
- `record_description(&r, 0) == Some("Homo sapiens chromosome 1")`, `record_description(&r, 1) == None` (chr2),
  `record_description(&r, 2) == Some("alternate assembly")`.
- `record_length(&r, 0) == Some(28)`, `record_length(&r, 1) == Some(10)`, `record_length(&r, 2) == Some(4)`,
  `record_length(&r, 3) == None`.
- `record_present(&r, 2) == true`, `record_present(&r, 3) == false`.
- Error path: `read_records` on a path that is not valid FASTA / cannot be opened returns `Err(FileError)`.

**Unit — caching (`src/fasta/functions.rs`)**

`clear_cache()`, call `read_records(path)` once, then delete or rename the temp file, then call `read_records(path)`
again: the second call still succeeds (served from cache) and returns an `Rc` that is `Rc::ptr_eq` to the first.
Also verify that a `clear_cache()` between the two calls makes the second call fail (proving it really re-reads).

**Unit — `try_parse` shapes (each executor's `#[cfg(test)] mod tests`)**

- `FastaCountExecutor`: accepts `fasta.seq.count` and `fasta.length`; rejects `fasta.seq.counts`, `fasta.count`,
  `fasta.seq.0.count`, and unrelated prefixes (`bam.header.rg.count`).
- `FastaSequenceFieldExecutor`: accepts `fasta.seq.0.name`, `fasta.seq.12.description`, `fasta.seq.3.length`;
  rejects non-numeric index (`fasta.seq.x.name`), unknown field (`fasta.seq.0.gc`), the `.present` shapes
  (those belong to the present executor), and wrong arity (`fasta.seq.0`).
- `FastaSequencePresentExecutor`: accepts `fasta.seq.0.present` and `fasta.seq.0.description.present`; rejects
  `fasta.seq.0.name.present`, non-numeric index, and the bare-field shapes.

**Integration — `tests/fasta_subcommand_test.rs`**

Reuse the `exec(&[..])` helper from the BAM/run tests (invoke `CARGO_BIN_EXE_bioassert --color=never`):

- `run_all_passing_fasta_assertions`: `run tests/data/fasta_assertions.txt` exits 0;
  `assert_snapshot!("run_fasta_passing_stdout", stdout)`.
- `assert_count_passes`: `assert "tests/data/sample.fasta fasta.seq.count eq 3"` → exit 0, stdout contains `PASS.`.
- `assert_total_length_passes`: `... fasta.length eq 42` → exit 0.
- `assert_quoted_name_passes`: `... fasta.seq.2.name eq 'NC_000001.11'` → exit 0 (quoted-value path).
- `assert_value_fails`: `... fasta.seq.0.name eq scaffold1` → exit 1, stdout contains `FAIL.`.
- `assert_errors_on_out_of_range_index`: `... fasta.seq.3.name eq X` → exit 2, stderr contains `ERROR.` and the
  out-of-range message.
- `assert_errors_on_missing_description`: `... fasta.seq.1.description eq X` (chr2 has none) → exit 2, `ERROR.`.
- `assert_errors_on_non_fasta_file`: `... tests/data/empty_file.txt fasta.seq.count eq 1` → exit 2 (or whatever the
  noodles reader yields for non-FASTA — re-verify at implementation time).
- `present_is_false_for_missing_description_without_error`: `... fasta.seq.1.description.present eq false` → exit 0,
  `PASS.` (the soft counterpart to the erroring value check above).
- `present_is_false_for_out_of_range_record`: `... fasta.seq.3.present eq false` → exit 0, `PASS.`.

### Snapshot handling

Only `run_fasta_passing_stdout` is a snapshot. On first run `cargo test` creates a `.snap.new`; accept it with
`cargo insta review` (or `INSTA_UPDATE=always cargo test`) and commit the resulting `.snap`. The FAIL/ERROR cases
above assert exit codes and substrings rather than snapshots, so they need no `.snap` files — matching how
`bam_subcommand_test.rs` snapshots only its passing run.

## Critical files

- `Cargo.toml` — add `fasta` to the noodles features.
- `src/fasta/{mod.rs,functions.rs,count/executor.rs,sequence/executor.rs}` — new.
- `src/engine/executor.rs` — dispatch lines + import.
- `src/lib.rs` — `pub mod fasta;`.
- `tests/fasta_subcommand_test.rs`, `tests/data/sample.fasta`, `tests/data/fasta_assertions.txt`,
  `tests/snapshots/` — new.
- `CLAUDE.md` — docs.

## Verification

1. `cargo build --release` — confirms the noodles `fasta` feature integrates and the crate compiles.
2. `cargo test` — runs new unit tests (temp-file FASTA helpers) and integration tests; first run builds the binary.
3. `cargo insta review` (or `INSTA_UPDATE=always cargo test`) — accept the new FASTA snapshot.
4. Manual end-to-end against the fixture:
   ```bash
   cargo run -- assert "tests/data/sample.fasta fasta.seq.count eq 3"                 # PASS, exit 0
   cargo run -- assert "tests/data/sample.fasta fasta.length eq 42"                   # PASS
   cargo run -- assert "tests/data/sample.fasta fasta.seq.0.name eq chr1"             # PASS
   cargo run -- assert "tests/data/sample.fasta fasta.seq.2.name eq 'NC_000001.11'"   # PASS
   cargo run -- assert "tests/data/sample.fasta fasta.seq.0.length gte 20"            # PASS
   cargo run -- assert "tests/data/sample.fasta fasta.seq.1.description.present eq false" # PASS
   cargo run -- assert "tests/data/sample.fasta fasta.seq.0.name eq scaffold1"        # FAIL, exit 1
   cargo run -- assert "tests/data/sample.fasta fasta.seq.3.name eq X"                # ERROR (out of range), exit 2
   cargo run -- run tests/data/fasta_assertions.txt                                   # batch report
   ```
5. Confirm a value-assert on a missing description exits with ERROR while the matching `.present` check returns
   `false` (PASS against `eq false`).
6. Confirm caching: the caching unit test passes, and a `run` over an assertions file with many `fasta.*` lines
   against one FASTA parses the file only once.
