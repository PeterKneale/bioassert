# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/PeterKneale/bioassert/compare/bioassert-v0.1.0...bioassert-v0.1.1) - 2026-06-18

### Other

- Update Cargo.toml files to use workspace settings and add descriptions for bioassert packages
- Refactor assertion handling by introducing AssertionRequest struct and updating executors to use it
- Refactor module structure to integrate bioassert dependencies and update assertion parsing logic
- Refactor metric executors to use a unified module structure and rename MetricExecutor to AssertionExecutor
- Refactor metric executors to use a unified module structure and rename MetricExecutor to AssertionExecutor
- Refactor metric executors to use specific error types and streamline execution logic
- Refactor metric execution by introducing Comparator enum and updating executors to use it
- Add metric executors for delimited cell, column count, line count, file existence, and size checks
- Refactor error handling by introducing BioAssertError and FileError types, updating execute functions to use specific error types
- Refactor metric handling by removing unused MetricError and adding utility functions for delimiter parsing
- Add execution logic for assertions with metric handling and result announcement
- Add insta for snapshot testing and update integration tests for output validation
- Refactor exit codes for assertions and improve error handling in main function
- Refactor exit codes for assertions and improve error handling in main function
- Move test data files into tests/data/ subdirectory
- Exit with code 1 when any assertion fails
- Update README to focus on usage, quick start and examples
- Refactor imports and formatting across multiple files for consistency
- Refactor imports and formatting across multiple files for consistency
- Add metric executors for delimited cell, column count, line count, file existence, emptiness, lines, and size
- Add comprehensive tests for comparator and metric parsing functionality
- Add TSV support with new assertions and integration tests
- Add TSV support with new assertions and integration tests
- Add CSV support with new metrics and comparators for assertions
- Add CLAUDE.md for project guidance and architecture overview
- Update Dockerfile to use Rust 1.96 and improve README formatting
- Update Dockerfile to use Rust 1.96 and improve README formatting
- Expose crate as a library alongside the CLI binary
- Add Dockerfile
- add integration test
- add integration test
- Fix lt to lte
- Add initial implementation of bioassert CLI tool with file assertions
