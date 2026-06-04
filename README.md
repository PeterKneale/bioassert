# BioAssert

[![CI](https://github.com/PeterKneale/bioassert/actions/workflows/ci.yml/badge.svg)](https://github.com/PeterKneale/bioassert/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/bioassert.svg)](https://crates.io/crates/bioassert)

BioAssert is a bioinformatics assertion and validation tool written in Rust. It provides both a reusable **library** (`lib.rs`) and a **CLI binary** (`main.rs`).

## Features

- Validate DNA, RNA, and protein sequences
- Calculate and assert GC content
- Assert sequence length constraints
- Built on the [noodles](https://crates.io/crates/noodles) bioinformatics library

## Installation

```sh
cargo install bioassert
```

## CLI Usage

```sh
# Show help
bioassert --help

# Show version
bioassert --version

# Validate a DNA sequence
bioassert validate-dna ATGCNATGC

# Validate an RNA sequence
bioassert validate-rna AUGCNAUG

# Validate a protein sequence
bioassert validate-protein MSTVX

# Check GC content (with optional range)
bioassert gc-content ATGCATGC --min 0.4 --max 0.6

# Check sequence length (with optional bounds)
bioassert check-length ATGCATGC --min 1 --max 20
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
bioassert = "0.1"
```

```rust
use bioassert::{assert_valid_dna, assert_gc_content, gc_content};

fn main() -> anyhow::Result<()> {
    assert_valid_dna("ATGCN")?;
    assert_gc_content("ATGC", 0.4, 0.6)?;
    println!("GC content: {:.2}%", gc_content("ATGC") * 100.0);
    Ok(())
}
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

## Publishing

A GitHub Actions CD workflow publishes the crate to [crates.io](https://crates.io) automatically when a GitHub Release is created. The workflow requires a `CARGO_REGISTRY_TOKEN` secret to be configured in the repository settings.
