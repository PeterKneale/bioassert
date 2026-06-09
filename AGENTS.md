# AGENTS.md

## Purpose

This repository follows a **Specification Driven Development (SDD)** approach.

All contributors, AI agents, automation systems, and human developers must treat the project specification as the primary source of truth.

The authoritative specification is located at:

```text
./docs/spec.md
```

Before implementing any feature, fixing a bug, modifying architecture, or introducing new dependencies, contributors must review and understand the relevant sections of the specification.

---

## Core Principles

### 1. Specification First

Do not begin implementation from assumptions.

Always:

1. Read the relevant sections of `./docs/spec.md`
2. Understand the requirements
3. Validate that the requested work is covered by the specification
4. Implement only what the specification requires

If a requested change is not covered by the specification:

- Do not invent behavior
- Do not infer requirements
- Do not create undocumented features

Instead:

1. Propose a specification update
2. Obtain approval for the specification change
3. Implement only after the specification has been updated

---

### 2. Specification is the Source of Truth

When there is a conflict between:

- Code
- Documentation
- Comments
- Tests
- Previous discussions

The specification takes precedence.

Priority order:

```text
1. docs/spec.md
2. Architecture Decision Records (if present)
3. Tests
4. Source code
5. Comments
```

---

### 3. Requirements Traceability

All work should be traceable to one or more specification requirements.

When creating:

- Pull Requests
- Design Documents
- Issues
- Test Plans

Reference the relevant section(s) of:

```text
docs/spec.md
```

Example:

```text
Implements:
Section 5.2 - BAM Assertions
Section 7.1 - CLI Validation Command
```

---

## Workflow

### New Feature

For every new feature:

1. Read relevant specification sections
2. Identify requirements
3. Create implementation plan
4. Implement feature
5. Create tests
6. Verify implementation satisfies specification
7. Update documentation if required

### Bug Fix

For every bug:

1. Determine expected behavior from specification
2. Determine actual behavior
3. Fix implementation
4. Add regression tests
5. Verify behavior matches specification

If specification is ambiguous:

- Stop implementation
- Raise a clarification issue
- Update specification first

---

## Architecture Rules

BioAssert architecture is defined in:

```text
docs/spec.md
```

Agents must preserve architectural boundaries.

Examples:

### CLI Layer

Responsible for:

- Argument parsing
- Configuration loading
- User-facing output

Must not contain:

- File format parsing logic
- Metric calculation logic

### Assertion Engine

Responsible for:

- Assertion evaluation
- Comparator execution
- Result generation

Must not contain:

- CLI concerns
- Output formatting concerns

### Metric Providers

Responsible for:

- File format specific logic
- Metric calculation
- Data extraction

Must not contain:

- Assertion evaluation
- CLI logic

### Reporting Layer

Responsible for:

- Console output
- JSON output
- JUnit output
- TAP output

Must not contain:

- Business logic
- Metric calculation

---

## Implementation Expectations

### Before Coding

Review:

```text
docs/spec.md
```

Specifically:

- Goals
- Non-goals
- Architecture
- CLI behavior
- Exit codes
- Logging requirements
- Supported file formats
- Assertion DSL

### During Coding

Ensure implementation remains:

- Consistent with specification
- Backward compatible where required
- Testable
- Deterministic

Avoid introducing:

- Undocumented behavior
- Hidden configuration
- Implicit assumptions

---

## Testing Requirements

Every implementation should include tests.

Minimum expectations:

### Unit Tests

Validate:

- Metric providers
- Comparators
- DSL parsing
- Assertion evaluation

### Integration Tests

Validate:

- CLI execution
- End-to-end assertion evaluation
- Exit codes
- Output formats

### Regression Tests

Required for bug fixes.

A bug fix is not complete without a test demonstrating the previous failure mode.

---

## Documentation Requirements

When behavior changes:

1. Update `docs/spec.md`
2. Update user documentation
3. Update examples if necessary

Code and specification must remain aligned.

---

## Rust Development Guidelines

### Preferred Principles

- Strong typing
- Explicit errors
- Zero-cost abstractions
- Streaming IO where possible
- Minimal allocations
- Deterministic behavior

### Error Handling

Prefer:

```rust
Result<T, E>
```

Avoid:

```rust
unwrap()
expect()
```

outside of tests.

### Performance

Large genomic files are expected.

Implementations should:

- Stream records
- Avoid loading entire files into memory
- Cache expensive computations
- Reuse computed metrics where possible

---

## File Format Support

Supported formats and assertions are defined in:

```text
docs/spec.md
```

Do not add new formats or assertions without updating the specification.

Examples include:

- BAM
- CRAM
- SAM
- FASTQ
- FASTA
- VCF
- BCF
- BED
- GTF

and future formats explicitly approved in the specification.

---

## Dependency Management

Before adding dependencies:

1. Verify they are required
2. Verify they align with project goals
3. Verify they do not duplicate existing functionality
4. Update specification if architectural impact exists

Prefer:

- Mature crates
- Actively maintained crates
- Pure Rust implementations

When possible, leverage the existing Noodles ecosystem before introducing alternative libraries.

---

## AI Agent Rules

AI agents operating in this repository must:

1. Read `docs/spec.md` before making changes
2. Follow specification-driven development
3. Avoid inventing requirements
4. Preserve architectural boundaries
5. Produce tests alongside implementation
6. Keep documentation synchronized with code

AI agents must not:

- Implement speculative features
- Introduce architectural changes without specification updates
- Rewrite large portions of the codebase without justification
- Ignore stated non-goals

---

## Pull Request Checklist

Before submitting changes verify:

- [ ] Specification reviewed
- [ ] Implementation matches specification
- [ ] Tests added or updated
- [ ] Documentation updated if necessary
- [ ] No undocumented behavior introduced
- [ ] Architecture boundaries respected
- [ ] Exit code behavior preserved
- [ ] Logging behavior preserved
- [ ] Performance considerations evaluated

---

## Guiding Principle

> The specification describes what BioAssert should do.
>
> The code exists to implement the specification.
>
> When in doubt, update the specification first and then implement the change.