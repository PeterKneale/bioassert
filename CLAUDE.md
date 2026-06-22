# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

`bioassert` is a Rust CLI tool for asserting properties of files using a simple declarative syntax, designed to validate
pipeline outputs in bioinformatics workflows. It is a single crate that builds both a library (`src/lib.rs`) and a
binary (`src/main.rs`); only the `bioassert` crate is published.

## Commands

```bash
# Build
cargo build --release          # binary at target/release/bioassert

# Test
cargo test                     # all unit + integration tests
cargo test <test_name>         # single test by name

# Run
cargo run -- assert "<file> <metric> <comparator> <value>"
cargo run -- run tests/data/assertions.txt
```

## Architecture

The crate is one package organised into four library modules (under `src/`) plus the binary front end. The modules were
previously separate workspace crates and were merged back into one; their boundaries survive as module boundaries.

1. **`src/core/`** — shared types and traits with no dependency on the other modules:
    - `assertion_request.rs` — `AssertionRequest` (resolved file path, parsed `Comparator`, raw expected string)
    - `comparisons/` — `Comparator` enum and its parsing; `compare` is generic over `PartialOrd`. String
      comparison goes through `Comparator::string_matcher`, which returns a reusable `StringMatcher` (compiling any
      `matches` regex once); `compare_string` is a one-shot convenience wrapper over it
    - `values/` — `Value` enum (and `BytesValue`); size units are normalised to bytes
    - `executor.rs` — the `AssertionExecutor` trait (`try_parse` + `execute`) and `AssertionExecutionResult`
    - `errors.rs` / `file_error.rs` — `BioAssertError` and `FileError`
    - `strings.rs` — `strip_quotes`, shared by executors whose expected value may be quoted

2. **`src/file/`** — file-level metric executors, one submodule each (`exists`, `size`, `empty`, `lines`). Each exposes a
   `File*Executor` that implements `core::AssertionExecutor`, split into `executor.rs` (parsing + dispatch) and
   `functions.rs` (the actual filesystem work).

3. **`src/delimited/`** — CSV/TSV/PSV metric executors (`column_count`, `line_count`, `cell`, `column_all`), same
   `*Executor` + `functions.rs` shape, with shared helpers in `functions.rs`. `column_all` handles the
   `<prefix>.column.<n>.all` (every line) and `<prefix>.column.<n>.data.all` (skip header) whole-column metrics; it
   streams the file and short-circuits on the first failing row, using `core::comparisons::StringMatcher` so a
   `matches` regex is compiled once rather than per cell.

4. **`src/bam/`** — BAM SAM-header metric executors built on the `noodles` crate, all under the `bam.header.*`
   namespace: `count` (`bam.header.rg.count`, `bam.header.sq.count`, `bam.header.pg.count`), `read_group`
   (`bam.header.rg.<n>.<tag>` and the `.present` variants), and `header` (`bam.header.hd.vn`, `bam.header.hd.so`).
   `functions.rs` is the only place that touches `noodles`; its `read_header` caches the parsed `sam::Header` per
   path in a thread-local so a `run` over many `bam.header.*` assertions parses each file once. The `bam.header.*`
   prefix leaves room for future record/body metrics under a sibling namespace (e.g. `bam.records.*`).

5. **`src/fasta/`** — FASTA sequence metric executors built on the `noodles` crate, under the `fasta.seq.*`
   namespace for per-record metrics plus the `fasta.length` whole-file aggregate: `count` (`fasta.seq.count`,
   `fasta.length`) and `sequence` (`fasta.seq.<n>.name`, `.description`, `.length`, and the `.present` variants).
   `functions.rs` is the only place that touches `noodles`; its `read_records` scans each file once and caches a
   per-record digest `{ name, description, length }` (never the sequence bytes, so memory stays bounded for
   multi-gigabyte genomes) keyed by path in a thread-local, so a `run` over many `fasta.*` assertions reads each
   file once. The `fasta.seq.*` prefix leaves room for future index metrics (e.g. `fasta.index.*`).

6. **`src/engine/`** — ties it together:
    - `cli.pest` — PEG grammar defining the assertion syntax (referenced as `#[grammar = "engine/cli.pest"]`)
    - `parser.rs` — wraps the PEG parser into `Assertion` structs; `parse_assertion` handles a single assertion,
      `parse_file` skips blank lines and `#` comments
    - `executor.rs` — `execute_all` / `execute`, which try each `*Executor` in turn and build an `AssertionReport`
    - `report.rs` — `AssertionReport`, `AssertionResult`, `Outcome`

7. **Binary** (`src/main.rs`, `src/cli.rs`, `src/report.rs`) — `clap` derive API with two subcommands, `assert` (single
   string) and `run` (path to assertions file). `src/report.rs` is the console/file presentation layer; the binary
   reaches the library modules via the `bioassert::` crate path.

## Adding a new metric

1. Add a submodule under `src/file/`, `src/delimited/`, `src/bam/`, or `src/fasta/` with a `*Executor` implementing
   `core::AssertionExecutor` (`executor.rs` for parsing/dispatch, `functions.rs` for the work).
2. Re-export the new `*Executor` from that module's `mod.rs`.
3. Add a `try_parse` dispatch line in `src/engine/executor.rs`.
4. Extend `src/engine/assertions.pest` if new metric or value syntax is needed. The `metric` rule already accepts any
   dot-separated chain of identifiers/numbers (e.g. `bam.header.rg.0.sm`), so most new metrics need no grammar change.

Expected values are matched by the grammar as a bare alphanumeric string, a quoted string, a number (with optional
size/count unit), or a boolean. Values containing dots, dashes, colons, or spaces (e.g. `'H0164.2'`, `'1.6'`,
`'Solexa-272222'`, `'NC_000001.11'`, `'Homo sapiens chromosome 1'`) must be single- or double-quoted; executors
call `core::strip_quotes` to unwrap them.

## Conditional assertions (guards)

An assertion may carry an optional guard so it is evaluated only when a condition holds:

```
<file> <metric> <comparator> <value> if <condition>
<file> <metric> <comparator> <value> unless <condition>
```

- `if` runs the assertion when the condition is satisfied. `unless` runs it when the condition is not satisfied.
- The condition has two forms. The shorthand is a bare metric on the assertion's own file with an implicit `eq true`
  (`if file.exists`), intended for boolean metrics (`file.exists`, `file.empty`, the `*.present` metrics). The full
  form is a complete `<file> <metric> <comparator> <value>` and may target a different file
  (`if other.bam bam.header.rg.count gt 0`).
- A guard has three outcomes. Satisfied: the assertion runs and reports PASS or FAIL. Not satisfied: the assertion is
  reported as SKIP, a neutral outcome that does not affect the exit code. Cannot be evaluated (for example a full-form
  guard whose file is missing): reported as ERROR. `file.exists` is the safe guard because it returns `false` rather
  than erroring on an absent file.
- The grammar (`src/engine/assertions.pest`) adds an optional `(guard_keyword ~ condition)?` suffix to the `assertion`
  rule. The parser fills the shorthand defaults, and `src/engine/executor.rs` evaluates the guard (through the same
  metric dispatch as a normal assertion) before the assertion itself. SKIP is an `Outcome` variant in
  `src/engine/report.rs`.
- Boolean composition (`and`, `or`, `not`) is not yet supported. To use `if` or `unless` as a literal expected value,
  quote it. The full design is recorded in `specs/conditional-assertions.md`.

## BAM test fixture

`tests/data/sample.bam` is a small header-only BAM regenerated with `cargo run --example gen_sample_bam`. The SAM
header it encodes must stay in sync between `examples/gen_sample_bam.rs`, `src/bam/functions.rs`
(`test_support::SAMPLE_SAM`), and `specs/bam.md`.

## Integration tests

The integration tests in `tests/` run the compiled binary (resolved via `CARGO_BIN_EXE_bioassert`, so a prior
`cargo build` is needed and is handled automatically by `cargo test`) against fixtures in `tests/data/`. Console output
is verified with `insta` snapshots stored in `tests/snapshots/`; review snapshot changes with `cargo insta review` (or
update them with `INSTA_UPDATE=always cargo test`).

## Docker

The Dockerfile uses a two-stage build: a `rust:1.96-slim` builder layer that caches dependency compilation (dummy
`src/main.rs` + `src/lib.rs` trick), then copies the release binary into `debian:bookworm-slim`.
The entrypoint is `bioassert` with no default subcommand.