# BioAssert

[![CI](https://github.com/PeterKneale/bioassert/actions/workflows/ci.yml/badge.svg)](https://github.com/PeterKneale/bioassert/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/bioassert.svg)](https://crates.io/crates/bioassert)

BioAssert is a bioinformatics assertion and validation tool written in Rust. It provides both a reusable **library** (`lib.rs`) and a **CLI binary** (`main.rs`).

## Features

- Assert 
  - BAM
    - Read Counts ✅
    - Sorting ☑️
    - Headers ☑️
    - Read Lengths ☑️
  - FASTA
  - FASTQ
  - VCF
- Built on the [noodles](https://crates.io/crates/noodles) bioinformatics library

## Installation

```sh
cargo install bioassert
```

## CLI Usage

- Successful assertions print `OK` to stdout and exit with a zero status code.
```sh
bioassert bam example.bam read_count eq 53
OK
bioassert bam example.bam read_count gt 10
OK
bioassert bam example.bam read_count lt 200
OK
```

- Failed assertions print an error message to stderr and exit with a non-zero status code.
```sh
bioassert bam example.bam read_count eq 1   
Error: Assertion failed. Expected: read count == 1, actual: 53
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
bioassert = "0.1"
```

```rust
use bioassert;

```

## Development

```sh
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy --all-targets --all-features -- -D warnings
```

## Releasing

This project uses a **manual bump PR** workflow. Versioning lives in `Cargo.toml`
and is enforced against the git tag in CD.

1. Create a release PR:
   - Bump `version` in `Cargo.toml` (semver; pre-1.0 uses `0.MINOR.PATCH`).
   - Update `CHANGELOG.md` (move `Unreleased` items into a new version section).
   - Run `cargo build` so `Cargo.lock` is updated.
   - Commit, e.g. `chore: release v0.1.2`.
2. Merge the PR to `main`.
3. Create a **GitHub Release** with tag `vX.Y.Z` (matching `Cargo.toml`).
4. CD (`.github/workflows/cd.yml`) will:
   - Verify the tag matches `Cargo.toml`.
   - Run tests.
   - `cargo publish --locked --dry-run`, then publish to crates.io.

> Published versions on crates.io are immutable. Always bump before releasing.


