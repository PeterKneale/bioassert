# Specification: BAM header assertions

## Context

`bioassert` validates pipeline outputs with declarative assertions (`<file> <metric> <comparator> <value>`).
Today it covers generic files (`file.*`) and delimited text (`csv/tsv/psv.*`). Bioinformatics pipelines emit
BAM alignment files whose SAM header carries critical provenance: read groups (`@RG`), reference sequences
(`@SQ`), programs (`@PG`), and the file header line (`@HD`). Pipelines silently lose or mis-set these (wrong
sample name, missing platform, no read group at all), which breaks downstream tools like GATK and duplicate
markers. This feature adds assertions that test both the **presence** and the **values** of these header
fields, focused on read groups, using the [`noodles`](https://docs.rs/noodles-bam/latest/noodles_bam/) crate to
parse BAM.

The intended outcome: a user can write checks like `sample.bam bam.header.rg.0.sm eq NA12878` in an assertions file
and have them pass/fail like any other metric.

## Constraints and key facts

- **No pest grammar change is needed.** `src/engine/cli.pest` defines
  `metric = @{ identifier ~ ("." ~ metric_segment)+ }`,
  which already accepts any dot-separated chain of identifiers/numbers (e.g. `bam.header.rg.0.sm`). Executors do all
  real
  parsing in `try_parse` via `metric.split('.')` + slice matching — exactly the pattern `DelimitedCellExecutor`
  uses for `csv.line.2.column.3` (`src/delimited/cell/executor.rs`).
- **Values with special characters must be quoted.** The grammar's bare string is `ASCII_ALPHANUMERIC+` only, so
  `ILLUMINA` works unquoted but `Solexa-272222`, `H0164.2`, and ISO dates need single/double quotes. The grammar
  already supports quoted strings, so this is a documentation note, not a code change.
- **Executor contract** (`src/core/executor.rs`): `try_parse(metric: &str) -> Option<Self>` then
  `execute(self, &AssertionRequest) -> Result<AssertionExecutionResult, BioAssertError>` returning
  `{ success, actual: Value }`. Dispatch is first-match-wins in `src/engine/executor.rs::evaluate`.
- **Value/Comparator** (`src/core/values/`, `src/core/comparisons/`): `Value` is `Boolean/Bytes/Integer/String`.
  `Comparator::compare` is numeric; `compare_string` handles `eq/ne/starts/ends/contains/matches`.
- **noodles API**: `let mut r = File::open(path).map(bam::io::Reader::new)?; let header = r.read_header()?;`
  yields a `sam::Header`. From it: `read_groups()`, `reference_sequences()`, `programs()` (order-preserving
  `IndexMap`s), and `header()` → `Option<&Map<Header>>` for `@HD`. Read-group ID is the map **key**; other tags
  (SM/LB/PL/PU/...) live in `map.other_fields()` keyed by a 2-byte `Tag`.

## Design decisions

- **Read groups addressed by index** (`bam.header.rg.0.sm`), following header order — robust given that read-group IDs
  contain dots (`H0164.2`) which collide with the dot-delimited metric grammar.
- **Presence via count + boolean `.present` suffix** — a missing tag/RG yields `false`, not an error.
- **Scope**: read groups and programs (`@PG`) in full, plus counts for `@SQ` and `@HD` fields.
- **Namespaced under `bam.header.*`.** A BAM has two sections — the header and the alignment records. Every
  metric here is a header metric, so they live under `bam.header.*`. This reserves a sibling namespace for
  future record/body metrics (e.g. `bam.records.count`, mapped/unmapped/duplicate counts, MAPQ thresholds) and
  file-integrity checks (e.g. BGZF EOF / not-truncated, index present), which are the obvious next additions.
- **Header read once, then cached for the whole workflow.** A `run` over an assertions file typically issues
  many `bam.header.*` assertions against the same file. Each would otherwise re-open, bgzf-decompress, and re-parse the
  header. Instead, `src/bam/functions.rs` holds a process-scoped cache keyed by file path, so the header for a
  given BAM is parsed exactly once per invocation. See "Header caching" below.

## Metric format

| Metric                            | Value type | Meaning                                                                                           |
|-----------------------------------|------------|---------------------------------------------------------------------------------------------------|
| `bam.header.rg.count`             | integer    | number of `@RG` lines                                                                             |
| `bam.header.sq.count`             | integer    | number of `@SQ` reference sequences                                                               |
| `bam.header.pg.count`             | integer    | number of `@PG` program records                                                                   |
| `bam.header.rg.<n>.<tag>`         | string     | tag value of the nth read group (`id`, `sm`, `lb`, `pl`, `pu`, `pi`, `dt`, `cn`, `ds`, `pm`, ...) |
| `bam.header.rg.<n>.present`       | bool       | whether a read group exists at index n                                                            |
| `bam.header.rg.<n>.<tag>.present` | bool       | whether that tag is set on read group n                                                           |
| `bam.header.pg.<n>.<tag>`         | string     | tag value of the nth program (`id`, `pn`, `pp`, `vn`, `cl`, `ds`)                                 |
| `bam.header.pg.<n>.present`       | bool       | whether a program exists at index n                                                               |
| `bam.header.pg.<n>.<tag>.present` | bool       | whether that tag is set on program n                                                              |
| `bam.header.hd.vn`                | string     | `@HD` version (VN)                                                                                |
| `bam.header.hd.so`                | string     | `@HD` sort order (SO)                                                                             |

`<n>` is a 0-based index. `<tag>` is the 2-letter SAM tag; `id` resolves to the record key, all others to
`other_fields`. The `pg` segment and its tags are matched **case-insensitively** (`bam.header.PG.count`,
`bam.header.pg.0.CL`), since `@PG` and its tags are uppercase in the SAM header itself. Programs are addressed
by header order, so `@PG` chaining (`pp`, previous-program ID) can be walked by index.

### Examples

#### BAM FILE

```
@HD	VN:1.6	SO:coordinate
@SQ	SN:chr1	LN:248956422
@RG	ID:H0164.1	SM:NA12878	LB:Solexa-272222	PL:ILLUMINA	PU:H0164ALXX140820.1
@RG	ID:H0164.2	SM:NA12878	LB:Solexa-272222	PL:ILLUMINA	PU:H0164ALXX140820.2
@PG	ID:bwa	PN:bwa	VN:0.7.17	CL:bwa mem ref.fa reads.fq
@PG	ID:samtools	PN:samtools	PP:bwa	VN:1.17	CL:samtools sort
```

All examples below are evaluated against the sample header above. The trailing comment is the expected outcome.

#### Counts

```
sample.bam bam.header.rg.count        eq  2            # PASS
sample.bam bam.header.rg.count        gte 1            # PASS
sample.bam bam.header.rg.count        lt  5            # PASS
sample.bam bam.header.sq.count        eq  1            # PASS
sample.bam bam.header.pg.count        eq  2            # PASS
sample.bam bam.header.PG.count        eq  2            # PASS (pg segment is case-insensitive)
sample.bam bam.header.rg.count        eq  3            # FAIL (actual 2)
```

#### Read-group tag values

Values containing a dot, dash, or colon must be quoted; bare values must be alphanumeric.

```
sample.bam bam.header.rg.0.id         eq  'H0164.1'           # PASS (ID is the @RG key)
sample.bam bam.header.rg.1.id         eq  'H0164.2'           # PASS
sample.bam bam.header.rg.0.sm         eq  NA12878             # PASS
sample.bam bam.header.rg.1.sm         eq  NA12878             # PASS
sample.bam bam.header.rg.0.pl         eq  ILLUMINA            # PASS
sample.bam bam.header.rg.0.lb         eq  'Solexa-272222'     # PASS
sample.bam bam.header.rg.0.pu         eq  'H0164ALXX140820.1' # PASS
sample.bam bam.header.rg.1.pu         eq  'H0164ALXX140820.2' # PASS
sample.bam bam.header.rg.0.sm         ne  NA12891             # PASS (not equal)
sample.bam bam.header.rg.0.pl         contains LUM            # PASS
sample.bam bam.header.rg.0.lb         starts Solexa           # PASS
sample.bam bam.header.rg.0.pu         ends   .1               # FAIL (".1" is unquoted, parses as 1; use 'H0164ALXX140820.1')
sample.bam bam.header.rg.0.pl         matches '^ILL.*A$'      # PASS (regex)
sample.bam bam.header.rg.0.sm         eq  WRONG               # FAIL (actual NA12878)
```

#### Presence (never errors on absence)

```
sample.bam bam.header.rg.0.present    eq  true     # PASS (RG 0 exists)
sample.bam bam.header.rg.1.present    eq  true     # PASS (RG 1 exists)
sample.bam bam.header.rg.2.present    eq  false    # PASS (only 2 read groups)
sample.bam bam.header.rg.0.sm.present eq  true     # PASS (SM is set)
sample.bam bam.header.rg.0.pu.present eq  true     # PASS (PU is set)
sample.bam bam.header.rg.0.dt.present eq  false    # PASS (no DT tag in sample)
sample.bam bam.header.rg.0.cn.present eq  false    # PASS (no CN tag in sample)
sample.bam bam.header.rg.0.pi.present eq  false    # PASS (no PI tag in sample)
```

#### Program records (@PG)

Programs are addressed by index in header order. The `pg` segment and its tags are case-insensitive.

```
sample.bam bam.header.pg.0.id         eq  bwa                     # PASS (ID is the @PG key)
sample.bam bam.header.pg.0.pn         eq  bwa                     # PASS (program name)
sample.bam bam.header.pg.0.vn         eq  '0.7.17'                # PASS (quote: dots)
sample.bam bam.header.pg.0.cl         eq  'bwa mem ref.fa reads.fq' # PASS (quote: spaces)
sample.bam bam.header.pg.0.cl         contains 'bwa mem'          # PASS
sample.bam bam.header.pg.1.id         eq  samtools                # PASS
sample.bam bam.header.pg.1.pp         eq  bwa                     # PASS (previous-program chaining)
sample.bam bam.header.pg.1.PP         eq  bwa                     # PASS (tag is case-insensitive)
sample.bam bam.header.pg.0.present    eq  true                    # PASS
sample.bam bam.header.pg.2.present    eq  false                   # PASS (only 2 programs)
sample.bam bam.header.pg.0.pp.present eq  false                   # PASS (the aligner has no PP)
sample.bam bam.header.pg.1.pp.present eq  true                    # PASS
```

#### Header line (@HD)

```
sample.bam bam.header.hd.vn           eq  '1.6'        # PASS (quote: dot)
sample.bam bam.header.hd.so           eq  coordinate   # PASS
sample.bam bam.header.hd.so           ne  queryname    # PASS
```

#### Errors (exit code 2)

```
sample.bam bam.header.rg.2.sm         eq  NA12878   # ERROR (read-group index out of range)
sample.bam bam.header.rg.0.dt         eq  X         # ERROR (tag not present — use .present to test softly)
sample.bam bam.header.pg.5.id         eq  X         # ERROR (program index out of range)
missing.bam bam.header.rg.count       eq  2         # ERROR (file cannot be opened)
notabam.txt  bam.header.rg.count      eq  2         # ERROR (not a valid BAM)
```

### Semantics

Value metrics (`bam.header.rg.<n>.<tag>`, `bam.header.pg.<n>.<tag>`, `bam.header.hd.*`) **error** if the index /
tag / `@HD` is absent (consistent with `DelimitedCellExecutor` erroring on a missing cell). The `.present`
metrics **never error** on absence — they return `false`. This is how a user safely tests "is this set" versus
"what is its value".

## Implementation

### 1. Dependency (`Cargo.toml`)

Add to `[dependencies]` (pin to current latest at implementation time):

```toml
noodles = { version = "<latest>", features = ["bam", "sam"] }
```

`bam` pulls `bgzf` transitively, so compressed BAMs read transparently.

### 2. New module `src/bam/` (mirrors `src/file/` and `src/delimited/`)

- `src/bam/mod.rs` — declares submodules and re-exports the executors.
- `src/bam/functions.rs` — shared noodles work, the single place that touches the crate:
    - `read_header(file: &Path) -> Result<Rc<sam::Header>, FileError>` — **cached** (see "Header caching" below);
      on a miss it opens the file, parses the header, and stores it. Maps `io::Error` via `FileError::new`.
    - helpers take `&sam::Header` (so they work straight off the cached `Rc`): `read_group_count`,
      `reference_count`, `program_count`, `read_group_tag(header, n, tag) -> Option<String>`,
      `read_group_present(header, n) -> bool`, `program_tag(header, n, tag) -> Option<String>`,
      `program_present(header, n) -> bool`, `hd_field(header, field) -> Option<String>`.
    - tag lookup: `id` → nth key of `read_groups()` / `programs().as_ref()`; otherwise build `Tag` from the
      uppercased 2 bytes and read `other_fields()`. (`Programs` is a newtype over `IndexMap`, so it is indexed
      through `as_ref()`.)
    - every `bam.header.*` executor's `execute` calls `read_header` and then a helper; none open the file directly.
- `src/bam/count/executor.rs` — `BamCountExecutor`: `try_parse` matches `[ "bam", kind, "count" ]` for
  `kind ∈ {rg, sq, pg}`; `execute` returns `Value::from_integer(expected)` numeric-compared against the count.
- `src/bam/read_group/executor.rs` — two executors (exact-shape `try_parse`, so dispatch order is irrelevant):
    - `BamReadGroupTagExecutor`: `[ "bam", "rg", n, tag ]` → `StringValue`, `compare_string`, errors if absent.
    - `BamReadGroupPresentExecutor`: `[ "bam", "rg", n, "present" ]` and `[ "bam", "rg", n, tag, "present" ]`
      → `Value::from_boolean(expected)` compared against the boolean.
- `src/bam/program/executor.rs` — `BamProgramTagExecutor` and `BamProgramPresentExecutor`, mirroring the
  read-group pair but for `[ "bam", "header", pg, n, ... ]`, where the `pg` segment is matched with
  `eq_ignore_ascii_case("pg")` so `PG` also parses; they call `functions::program_tag` / `program_present`.
- `src/bam/header/executor.rs` — `BamHeaderFieldExecutor`: `[ "bam", "hd", field ]` for `field ∈ {vn, so}` →
  `StringValue`.

Each `executor.rs` carries a `#[cfg(test)] mod tests` of `try_parse` cases (accept valid shapes, reject bad
prefix/tag/index), matching `src/delimited/cell/executor.rs` tests.

### 3. Header caching

The header for a given BAM must be parsed exactly once per invocation, then reused across every `bam.header.*`
assertion in the workflow. The binary runs once per `run`/`assert` invocation, so a process-scoped cache is
workflow-scoped in practice. Keep it inside `src/bam/functions.rs` so the rest of the codebase (and the
`AssertionExecutor` trait) is untouched:

```rust
thread_local! {
    static HEADER_CACHE: RefCell<HashMap<PathBuf, Rc<sam::Header>>> = RefCell::new(HashMap::new());
}

pub fn read_header(file: &Path) -> Result<Rc<sam::Header>, FileError> {
    if let Some(h) = HEADER_CACHE.with(|c| c.borrow().get(file).cloned()) {
        return Ok(h);
    }
    let mut reader = File::open(file)
        .map(bam::io::Reader::new)
        .map_err(|e| FileError::new(file, e))?;
    let header = Rc::new(reader.read_header().map_err(|e| FileError::new(file, e))?);
    HEADER_CACHE.with(|c| c.borrow_mut().insert(file.to_path_buf(), Rc::clone(&header)));
    Ok(header)
}
```

Rationale and choices:

- **`thread_local!` over a struct threaded through `execute`** — the `AssertionExecutor::execute` signature
  (`self, &AssertionRequest`) is shared by every executor; adding a cache parameter would ripple through all of
  them. The execution loop is single-threaded and sequential, so a thread-local map is sufficient and keeps the
  change contained to the bam module.
- **Cache successes only.** Errors (missing file, not a BAM) are not cached; they are cheap and rare, and
  `FileError` is not required to be `Clone`.
- **Key by the path as given** (`PathBuf` from `assertion.file`). Assertions in one file reference a BAM by the
  same string, so this dedupes correctly without a `canonicalize` syscall. Canonicalizing is a possible later
  refinement if mixed relative/absolute paths to one file become common.
- **Expose `pub(crate) fn clear_cache()`** for unit tests so on-the-fly fixtures in different tests cannot
  observe each other's entries (temp paths are unique, so this is belt-and-suspenders).

A unit test asserts caching: read a header twice and confirm the second call returns the same `Rc` (e.g.
`Rc::ptr_eq`) without touching the filesystem (delete/rename the temp file between calls and confirm the second
read still succeeds from cache).

### 4. Reuse `strip_quotes`

`strip_quotes` currently lives in `src/delimited/cell/functions.rs` (`pub fn`). The BAM string executors need
it too. Promote it to a shared location — e.g. `src/core/` re-exported as `core::strip_quotes` — and update the
one delimited call site. Avoids duplicating quote-stripping logic.

### 5. Wire dispatch (`src/engine/executor.rs::evaluate`)

Add `try_parse` lines after the delimited executors (before the final `Err`):

```rust
if let Some(e) = BamCountExecutor::try_parse( & assertion.metric) { return dispatch(e, assertion, request); }
if let Some(e) = BamHeaderFieldExecutor::try_parse( & assertion.metric) { return dispatch(e, assertion, request); }
if let Some(e) = BamReadGroupPresentExecutor::try_parse( & assertion.metric) { return dispatch(e, assertion, request); }
if let Some(e) = BamReadGroupTagExecutor::try_parse( & assertion.metric) { return dispatch(e, assertion, request); }
```

Add the `mod bam;` declaration and re-exports alongside the existing modules in `src/lib.rs`.

### 6. Tests

**The canonical fixture is the sample header shown above.** Both the on-the-fly unit-test BAM and the committed
`tests/data/sample.bam` must encode exactly this header — 2 `@RG`, 1 `@SQ` (`chr1`, length 248956422), 2 `@PG`
(the second chained from the first via `PP:bwa`), and an `@HD` with `VN:1.6 SO:coordinate` — so the assertions
and snapshot below are deterministic:

```
@HD	VN:1.6	SO:coordinate
@SQ	SN:chr1	LN:248956422
@RG	ID:H0164.1	SM:NA12878	LB:Solexa-272222	PL:ILLUMINA	PU:H0164ALXX140820.1
@RG	ID:H0164.2	SM:NA12878	LB:Solexa-272222	PL:ILLUMINA	PU:H0164ALXX140820.2
@PG	ID:bwa	PN:bwa	VN:0.7.17	CL:bwa mem ref.fa reads.fq
@PG	ID:samtools	PN:samtools	PP:bwa	VN:1.17	CL:samtools sort
```

- **Unit tests** (`src/bam/functions.rs`): a helper builds this exact header with noodles' `sam::Header` builder +
  `bam::io::Writer` into a `NamedTempFile`, then asserts each function against the known values — e.g.
  `read_group_count == 2`, `reference_count == 1`, `program_count == 2`,
  `read_group_tag(h, 0, "id") == Some("H0164.1")`,
  `read_group_tag(h, 0, "sm") == Some("NA12878")`, `read_group_tag(h, 1, "pu") == Some("H0164ALXX140820.2")`,
  `read_group_tag(h, 0, "dt") == None`, `read_group_present(h, 2) == false`,
  `program_tag(h, 0, "id") == Some("bwa")`, `program_tag(h, 1, "pp") == Some("bwa")`,
  `program_tag(h, 0, "pp") == None`, `program_present(h, 2) == false`, `hd_field(h, "vn") == Some("1.6")`,
  `hd_field(h, "so") == Some("coordinate")`. Each executor's `#[cfg(test)] mod tests` covers `try_parse` shapes.
- **Caching test** (`src/bam/functions.rs`): call `clear_cache()`, `read_header` once, delete/rename the temp
  file, then `read_header` again — the second call must still succeed (served from cache) and return an `Rc` that
  is `Rc::ptr_eq` to the first.
- **Integration test + snapshot** (`tests/bam_subcommand_test.rs`): mirror `tests/run_subcommand_test.rs` — invoke
  the binary via `CARGO_BIN_EXE_bioassert --color=never run tests/data/bam_assertions.txt` and capture stdout with
  `insta::assert_snapshot!`. `tests/data/bam_assertions.txt` holds the example assertions above (a mix of PASS,
  FAIL, and ERROR lines) so the snapshot exercises all three outcomes and verifies the actual values are reported.
  Add focused `assert`-subcommand cases (exit 0 / 1 / 2) like `tests/assert_subcommand_test.rs`, including the
  out-of-range-index and missing-tag ERROR cases.
- **Fixture generation**: commit `tests/data/sample.bam` (a few hundred bytes) generated once with noodles to match
  the header above. Add an `#[ignore]`d generator test (or a small `examples/` binary) that writes it, with a
  comment documenting how to regenerate, so the fixture stays reproducible.

### 7. Docs (`CLAUDE.md`)

Add a `src/bam/` bullet to the architecture section, extend "Adding a new metric" to mention the bam module, and
add a short note that expected values containing non-alphanumeric characters (dots, dashes, colons) must be quoted.

## Critical files

- `Cargo.toml` — add noodles.
- `src/bam/{mod.rs,functions.rs,count/executor.rs,read_group/executor.rs,header/executor.rs}` — new.
- `src/engine/executor.rs` — dispatch lines.
- `src/lib.rs` — `mod bam;` + re-exports.
- `src/delimited/cell/functions.rs` + new `src/core/` location — move `strip_quotes`.
- `tests/bam_subcommand_test.rs`, `tests/data/sample.bam`, `tests/data/bam_assertions.txt`, `tests/snapshots/` — new.
- `CLAUDE.md` — docs.

## Verification

1. `cargo build --release` — confirms noodles integrates and the crate compiles.
2. `cargo test` — runs new unit tests (on-the-fly BAM helpers) and integration tests; first run builds the binary.
3. `cargo insta review` (or `INSTA_UPDATE=always cargo test`) — accept the new BAM snapshot.
4. Manual end-to-end against the fixture:
   ```bash
   cargo run -- assert "tests/data/sample.bam bam.header.rg.count eq 2"           # PASS, exit 0
   cargo run -- assert "tests/data/sample.bam bam.header.rg.0.sm eq NA12878"      # PASS
   cargo run -- assert "tests/data/sample.bam bam.header.rg.1.id eq 'H0164.2'"    # PASS
   cargo run -- assert "tests/data/sample.bam bam.header.hd.vn eq '1.6'"          # PASS
   cargo run -- assert "tests/data/sample.bam bam.header.rg.0.dt.present eq false" # PASS (no DT tag)
   cargo run -- assert "tests/data/sample.bam bam.header.rg.0.sm eq WRONG"        # FAIL, exit 1
   cargo run -- assert "tests/data/sample.bam bam.header.rg.2.sm eq X"            # ERROR (index out of range), exit 2
   cargo run -- run tests/data/bam_assertions.txt                          # batch report
   ```
5. Confirm a value-assert on a genuinely missing tag exits with ERROR while the matching `.present` check returns
   `false` (PASS against `eq false`).
6. Confirm caching: the caching unit test passes, and a `run` over an assertions file with many `bam.header.*` lines
   against one BAM parses the header only once (the `read_header` cache test guards this).
