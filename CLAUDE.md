# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

`bioassert` is a Rust CLI tool for asserting properties of files using a simple declarative syntax, designed to validate
pipeline outputs in bioinformatics workflows. It is published as both a binary and a library crate.

## Commands

```bash
# Build
cargo build --release          # binary at target/release/bioassert

# Test
cargo test                     # all unit + integration tests
cargo test <test_name>         # single test by name

# Run
cargo run -- assert "<file> <metric> <comparator> <value>"
cargo run -- run tests/assertions.txt
```

## Architecture

The assertion pipeline flows through four layers:

1. **Grammar** (`src/cli.pest`) — PEG grammar defining the assertion syntax via `pest`. Any change to the assertion
   syntax starts here.

2. **Parser** (`src/parser.rs`) — wraps the PEG parser into `Assertion` structs (`file`, `metric`, `comparator`,
   `expected` as raw strings). `parse_raw_assertion` handles single assertions; `parse_file` skips blank lines and `#`
   comments.

3. **Assertions module** (`src/assertions/`) — converts raw strings into typed values:
    - `metrics.rs` — `Metric` enum (`FileExists`, `FileSize`, `FileEmpty`, `FileLines`) and `parse_metric`
    - `comparator.rs` — `Comparator` enum and `parse_comparator`; `compare` is generic over `PartialOrd`
    - `values.rs` — `Value` enum with `parse_boolean`, `parse_bytes`, `parse_integer`; size units are normalised to
      bytes

4. **Executor** (`src/executor.rs`) — dispatches on `Metric`, calls the appropriate function from `src/files/`, runs the
   comparison, and prints `PASS.` or `FAIL.` with a human-readable message.

5. **Files module** (`src/files/`) — each metric is a standalone function: `exists`, `size`, `empty` (`size == 0`),
   `count_lines`.

6. **CLI** (`src/cli.rs` + `src/main.rs`) — `clap` derive API with two subcommands: `assert` (single string) and `run` (
   path to assertions file).

## Adding a new metric

1. Add a variant to `Metric` in `src/assertions/metrics.rs` and update `parse_metric`.
2. Add the corresponding file function in `src/files/`.
3. Add a match arm in `src/executor.rs`.
4. Extend `src/cli.pest` if new value syntax is needed.

## Integration test

`tests/integration_test.rs` runs the compiled binary against `tests/assertions.txt` and asserts exact stdout output. The
binary path is resolved via `CARGO_BIN_EXE_bioassert`, so the test requires a prior `cargo build` (handled automatically
by `cargo test`). When adding new assertions to `tests/assertions.txt`, the expected output string in the test must be
updated to match.

## Docker

The Dockerfile uses a two-stage build: a `rust:1.96-slim` builder layer that caches dependency compilation (dummy
`main.rs` trick), then copies the release binary into `debian:bookworm-slim`.
The entrypoint is `bioassert` with no default subcommand.