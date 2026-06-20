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
    - `comparisons/` — `Comparator` enum and its parsing; `compare` is generic over `PartialOrd`
    - `values/` — `Value` enum (and `BytesValue`); size units are normalised to bytes
    - `executor.rs` — the `AssertionExecutor` trait (`try_parse` + `execute`) and `AssertionExecutionResult`
    - `errors.rs` / `file_error.rs` — `BioAssertError` and `FileError`

2. **`src/file/`** — file-level metric executors, one submodule each (`exists`, `size`, `empty`, `lines`). Each exposes a
   `File*Executor` that implements `core::AssertionExecutor`, split into `executor.rs` (parsing + dispatch) and
   `functions.rs` (the actual filesystem work).

3. **`src/delimited/`** — CSV/TSV/PSV metric executors (`column_count`, `line_count`, `cell`), same `*Executor` +
   `functions.rs` shape, with shared helpers in `functions.rs`.

4. **`src/engine/`** — ties it together:
    - `cli.pest` — PEG grammar defining the assertion syntax (referenced as `#[grammar = "engine/cli.pest"]`)
    - `parser.rs` — wraps the PEG parser into `Assertion` structs; `parse_assertion` handles a single assertion,
      `parse_file` skips blank lines and `#` comments
    - `executor.rs` — `execute_all` / `execute`, which try each `*Executor` in turn and build an `AssertionReport`
    - `report.rs` — `AssertionReport`, `AssertionResult`, `Outcome`

5. **Binary** (`src/main.rs`, `src/cli.rs`, `src/report.rs`) — `clap` derive API with two subcommands, `assert` (single
   string) and `run` (path to assertions file). `src/report.rs` is the console/file presentation layer; the binary
   reaches the library modules via the `bioassert::` crate path.

## Adding a new metric

1. Add a submodule under `src/file/` or `src/delimited/` with a `*Executor` implementing `core::AssertionExecutor`
   (`executor.rs` for parsing/dispatch, `functions.rs` for the work).
2. Re-export the new `*Executor` from that module's `mod.rs`.
3. Add a `try_parse` dispatch line in `src/engine/executor.rs`.
4. Extend `src/engine/cli.pest` if new metric or value syntax is needed.

## Integration tests

The integration tests in `tests/` run the compiled binary (resolved via `CARGO_BIN_EXE_bioassert`, so a prior
`cargo build` is needed and is handled automatically by `cargo test`) against fixtures in `tests/data/`. Console output
is verified with `insta` snapshots stored in `tests/snapshots/`; review snapshot changes with `cargo insta review` (or
update them with `INSTA_UPDATE=always cargo test`).

## Docker

The Dockerfile uses a two-stage build: a `rust:1.96-slim` builder layer that caches dependency compilation (dummy
`src/main.rs` + `src/lib.rs` trick), then copies the release binary into `debian:bookworm-slim`.
The entrypoint is `bioassert` with no default subcommand.