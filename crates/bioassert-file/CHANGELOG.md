# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/PeterKneale/bioassert/compare/bioassert-file-v0.1.0...bioassert-file-v0.1.1) - 2026-06-18

### Other

- Update Cargo.toml files to use workspace settings and add descriptions for bioassert packages
- Add executors for file properties: line count, column count, size, existence, and emptiness
- Refactor assertion handling by introducing AssertionRequest struct and updating executors to use it
- Refactor module structure to integrate bioassert dependencies and update assertion parsing logic
