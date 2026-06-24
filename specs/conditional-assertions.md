# Specification: Conditional assertions (guards)

## Context

`bioassert` validates pipeline outputs with declarative assertions (`<file> <metric> <comparator> <value>`).
Every assertion today is unconditional: each line is always evaluated and always contributes a PASS, FAIL or
ERROR. Bioinformatics pipelines are branchy. An optional output may or may not be produced, a step may be skipped
for some sample types, and a file may legitimately be absent on one run and present on the next. With only
unconditional assertions the author has two bad choices: assert the property and accept a spurious FAIL/ERROR when
the file is absent, or drop the assertion entirely and lose the check when the file *is* present.

This feature adds a guard clause so an assertion runs only when a condition holds:

```
data.tsv tsv.columns.count eq 18 if data.tsv file.exists eq true
```

The intended outcome: the author writes the check once, and it is evaluated only when its guard is satisfied. When
the guard is not satisfied the assertion is reported as SKIP and does not affect the exit code, so "check the
column count, but only when the file is there" becomes a single line that is honest in both cases.

## Constraints and key facts

- **The grammar is `src/engine/assertions.pest`.** The current top rule is
  `assertion = { file ~ metric ~ comparator ~ value ~ EOI }`. The conditional clause is an optional suffix added
  before `EOI`. The wrapper rules (`assertions`, `file_contents`) are unchanged.
- **The AST is positional.** `src/engine/assertion.rs` defines
  `Assertion { file, metric, comparator, expected }`, and `src/engine/parser.rs::parse_assertion` fills it by
  calling `inner.next()` four times. Adding a field touches every `Assertion { .. }` literal (the parser, and the
  test builder in `src/engine/report.rs`).
- **Evaluation is a first-match dispatch.** `src/engine/executor.rs::evaluate` builds an `AssertionRequest`
  (`file`, parsed `comparator`, `expected`) and tries each `*Executor::try_parse(metric)` in turn (lines 46 to
  60), then `dispatch` runs the matched executor and formats the message. `execute` maps the result to PASS / FAIL,
  and any `BioAssertError` to ERROR.
- **`Outcome` has three variants and is matched exhaustively in two places.** `src/engine/report.rs` defines
  `Outcome { Pass, Fail, Error }` and its `label()`. The binary's `src/report.rs::format_outcome` matches all
  three with no wildcard (so the compiler will force a new arm), and `print_report` routes `Error` to stderr and
  everything else to stdout. The exit code (`src/main.rs::exit_code`) is 2 if `has_errors`, else 1 if
  `has_failures`, else 0.
- **The key semantic fact that makes guards useful: `file.exists` returns `false`, it does not error, when the
  file is absent** (`src/file/exists/functions.rs`). Almost every other metric (`file.size`, `tsv.*`, `bam.*`,
  `fasta.*`) errors when its file cannot be opened. So `file.exists` is the natural guard: absent file gives a
  clean `false` that turns into SKIP, rather than an ERROR.

## Design decisions

- **A single guard per assertion, with `if` and `unless`.** One condition covers the motivating cases. Boolean
  composition (`and`, `or`, `not`, parentheses) needs an expression grammar with precedence and is explicitly
  deferred. `unless` is the negation of `if` and reads naturally (`... unless data.tsv file.empty eq true`), so it is included now
  because it is one boolean flag in the AST, not a grammar of its own.
- **One condition form, identical to a main assertion.** A guard condition is a full
  `<resource> <metric> <comparator> <value>`, possibly against a different resource
  (`if other.bam bam.header.rg.count gt 0`). There is no shorthand: the resource and comparator are always stated,
  so a guard reads exactly like the assertion it guards and never implies a resource or an `eq true`. An earlier
  revision shipped a bare-metric shorthand (`if file.exists`, expanded to `<main-resource> file.exists eq true`),
  but it was removed to consolidate on a single consistent syntax. The shorthand bought brevity at the cost of two
  divergent forms and a foot-gun: a bare numeric metric (`if tsv.columns.count`) expanded to `eq true` and then
  ERRORed at execution. Requiring the full form makes that line a clean parse error and leaves the grammar with one
  rule. To guard a resource on its own existence, name it explicitly (`if data.tsv file.exists eq true`).
- **A guard reuses the same dispatch as a normal assertion.** The condition is evaluated by the exact same
  executor chain, so any metric can guard any other, including across files. There is no separate "condition
  language".
- **A guard has three outcomes, not two.** This is the crux of the behaviour:

  | Guard evaluates to | Main assertion | Reported outcome |
  |---|---|---|
  | satisfied | runs normally | PASS / FAIL / ERROR |
  | not satisfied | not run | **SKIP** (new outcome) |
  | errors | not run | ERROR (guard could not be evaluated) |

  The third row is deliberate. A guard that *errors* (for example `if other.bam bam.header.rg.count gt 0` when
  `other.bam` is missing) is a misconfiguration, not a skip, so it surfaces as ERROR. Conflating guard errors with
  skips would hide real problems. This is exactly why `file.exists` is the right guard: it gives `false`, not an
  error, so it skips cleanly.
- **SKIP is a new, exit-code-neutral outcome.** A skipped assertion is neither a pass nor a failure. It must not
  flip the exit code or `has_failures()`, otherwise a guarded-out check would fail the run and defeat the purpose.

## Syntax

### Grammar additions (`src/engine/assertions.pest`)

```pest
guard_keyword = { ^"if" | ^"unless" }
condition     = { resource ~ metric ~ comparator ~ value }

assertion = {
    resource ~ metric ~ comparator ~ value ~ (guard_keyword ~ condition)? ~ EOI
}
```

One property makes this safe against the existing grammar:

- **The keyword position is unambiguous.** `if` / `unless` can only appear after `value`, so they never collide
  with `resource` (first token) or `metric` (a dotted chain). `EOI` is retained, so a trailing token that is not a
  valid guard still fails, preserving the existing `rejects_an_extra_trailing_token` test. Because `value` always
  matches exactly one token (a quoted run or an `ASCII_ALPHANUMERIC+` run) and tokens are whitespace-separated, a
  following `if` is seen as the keyword rather than swallowed into the value. A guard that is not a complete
  `resource metric comparator value` (e.g. the bare `if file.exists`) leaves the optional group unmatched and then
  fails `EOI`, so it is a parse error.

The keyword match is case-insensitive (`^"if"`) to match the existing `comparator` rule. The one residual
ambiguity is a bare unquoted value that is literally the word `if` or `unless`: it is parsed as the value, not a
keyword. To use those words as a literal expected value, quote them. This is worth a sentence in the docs.

## Semantics

- **No implicit defaults.** A guard states its resource, metric, comparator and value in full, so nothing is
  inferred. Boolean-producing metrics (`file.exists`, `file.empty`, the various `*.present` metrics) are the
  natural fit for an `eq true` / `eq false` guard. A non-boolean value mismatch (e.g. a numeric metric compared
  against `eq true`) still errors at execution as a safe, visible failure mode.
- **`if` vs `unless`.** With `if`, the main assertion runs when the condition is satisfied. With `unless`, it runs
  when the condition is *not* satisfied. In code this is one XOR against a `negate` flag.
- **Guard errors are ERROR, not SKIP** (see the table in Design decisions). The reported message should make
  clear it was the guard that failed to evaluate, not the main metric.
- **SKIP does not affect the exit code.** `exit_code` already keys only off `has_errors` and `has_failures`, so a
  run whose every assertion either passes or is skipped exits 0. A guard that errors still yields exit 2.

### Message formats

- Ran: unchanged from today, `Expected <file> <metric> <comparator> <expected>, got <actual>`.
- Skipped: `Skipped: guard <cond-file> <cond-metric> <cond-comparator> <cond-expected> not satisfied (got <actual>)`.
  For `unless`, `... satisfied (got <actual>)`.
- Guard error: prefix the underlying error, for example `guard could not be evaluated: <inner error>`.

## Examples

Evaluated against a directory where `present.tsv` exists with 18 columns and `absent.tsv` does not exist. The
trailing comment is the expected outcome.

```
# Same-resource guard, stated in full
present.tsv tsv.columns.count eq 18 if present.tsv file.exists eq true   # PASS (file present, 18 columns)
absent.tsv  tsv.columns.count eq 18 if absent.tsv  file.exists eq true   # SKIP (file absent, guard false)
present.tsv tsv.columns.count eq 99 if present.tsv file.exists eq true   # FAIL (file present, but not 99 columns)

# unless: run only when the condition does NOT hold
present.tsv tsv.columns.count eq 18 unless present.tsv file.empty eq true  # PASS (file is not empty, so it runs)

# Guard on a different file / different metric
report.tsv  tsv.line.count    gt 0  if present.tsv file.size gt 0    # PASS
report.tsv  tsv.line.count    gt 0  if absent.tsv  file.exists eq true  # SKIP

# Guard that errors is ERROR, not SKIP (exit code 2)
present.tsv tsv.columns.count eq 18 if absent.tsv file.size gt 0     # ERROR (guard file missing)
```

## Implementation

### 1. Grammar (`src/engine/assertions.pest`)

Add `guard_keyword` and `condition` as shown in Syntax, and append `~ (guard_keyword ~ condition)?` before `EOI`
in the `assertion` rule. `condition` is the same `resource ~ metric ~ comparator ~ value` shape as the assertion.

### 2. AST (`src/engine/assertion.rs`)

```rust
#[derive(Debug, Clone)]
pub struct Assertion {
    pub file: String,
    pub metric: String,
    pub comparator: String,
    pub expected: String,
    pub guard: Option<Guard>,
}

#[derive(Debug, Clone)]
pub struct Guard {
    pub negate: bool, // false for `if`, true for `unless`
    pub condition: Condition,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub file: String,
    pub metric: String,
    pub comparator: String,
    pub expected: String,
}
```

`Condition` is a separate type (rather than a recursive `Assertion`) so guards are exactly one level deep and carry
no guard of their own. Adding the `guard` field requires updating every `Assertion { .. }` literal: the parser, and
the `result(..)` test helper in `src/engine/report.rs` (add `guard: None`).

### 3. Parser (`src/engine/parser.rs`)

After reading the four mandatory inner pairs (`file`, `metric`, `comparator`, `value`), check for an optional
`guard_keyword` pair. If present:

- `negate = guard_keyword.as_str().eq_ignore_ascii_case("unless")`.
- The next pair is `condition`; read its four inner pairs (`resource`, `metric`, `comparator`, `value`)
  positionally into the `Condition`.

Set `guard: None` when no keyword is present. Add parser tests: a full-form guard parses all four condition slots;
`unless` sets `negate = true`; a line with no guard parses to `guard: None`; a trailing keyword with no condition
is a parse error; a bare-metric guard (`if file.exists`) is a parse error.

### 4. New `Outcome::Skip` and report changes

- `src/engine/report.rs`: add `Skip` to `Outcome`, return `"SKIP"` from `label()`. Optionally add `count(Skip)`
  usage in any summary. Leave `has_failures` / `has_errors` untouched so SKIP stays neutral.
- `src/report.rs::format_outcome`: add a `Skip` arm to the exhaustive match. Suggested decoration: a neutral icon
  (for example a white circle) and no color (reset), distinct from green PASS and red FAIL/ERROR. `print_report`
  already routes everything except `Error` to stdout, so SKIP lands on stdout with no change.

### 5. Evaluation (`src/engine/executor.rs`)

Factor the current dispatch chain (lines 46 to 60) into a helper that both the main assertion and the guard call:

```rust
fn run_metric(file: &str, metric: &str, comparator: &str, expected: &str)
    -> Result<(bool, Value), BioAssertError>
{
    let request = AssertionRequest {
        file: PathBuf::from(file),
        comparator: comparator.parse()?,
        expected: expected.to_string(),
    };
    if let Some(e) = FileExistsExecutor::try_parse(metric) { let r = e.execute(&request)?; return Ok((r.success, r.actual)); }
    // ... the same try_parse chain as today, one line per executor ...
    Err(BioAssertError::Metric(metric.to_string()))
}
```

Introduce a small result enum so `execute` can map to the three outcomes:

```rust
enum Evaluation {
    Ran { success: bool, message: String },
    Skipped { message: String },
}

fn evaluate(assertion: &Assertion) -> Result<Evaluation, BioAssertError> {
    if let Some(guard) = &assertion.guard {
        let c = &guard.condition;
        let (held, actual) = run_metric(&c.file, &c.metric, &c.comparator, &c.expected)
            .map_err(/* annotate as a guard error */)?;
        let active = held ^ guard.negate;
        if !active {
            return Ok(Evaluation::Skipped {
                message: format!(
                    "Skipped: guard {} {} {} {} not satisfied (got {})",
                    c.file, c.metric, c.comparator, c.expected, actual
                ),
            });
        }
    }
    let (success, actual) =
        run_metric(&assertion.file, &assertion.metric, &assertion.comparator, &assertion.expected)?;
    Ok(Evaluation::Ran {
        success,
        message: format!(
            "Expected {} {} {} {}, got {}",
            assertion.file, assertion.metric, assertion.comparator, assertion.expected, actual
        ),
    })
}
```

`execute` gains a `Skipped` arm mapping to `Outcome::Skip`; the `Ran` and `Err` arms keep today's behaviour. The
guard-error annotation can be a new `BioAssertError` variant wrapping the inner error, or a string prefix, so the
report distinguishes a guard failure from a main-metric failure.

### 6. Exit code (`src/main.rs`)

No change required: `exit_code` keys off `has_errors` and `has_failures` only, so SKIP is already neutral. A guard
that errors still produces exit 2 via the normal ERROR path.

### 7. Docs (`CLAUDE.md`)

Document the guard syntax (`if` / `unless`, the full `resource metric comparator value` form), the three guard
outcomes including the new SKIP, the rule that `file.exists` is the safe guard because it returns `false` rather
than erroring, and the note that a literal `if` / `unless` value must be quoted. Note that boolean composition is
intentionally not yet supported.

## Test plan

Following the conventions in `tests/` (see `tests/run_subcommand_test.rs` and `tests/bam_subcommand_test.rs`): the
per-feature run snapshot is driven by an all-passing-or-skipped assertions file (the integration test asserts exit
0), while FAIL and ERROR paths are covered by focused `assert`-subcommand cases checking exit codes 1 and 2.

### Unit tests

- **`src/engine/parser.rs`**: a full-form guard `if data.tsv file.exists eq true` parses to a guard with
  `negate = false` and all four condition slots; `unless ... file.empty eq true` sets `negate = true`; the
  cross-file form `if other.tsv tsv.line.count gt 0` parses all four slots; a line with no guard parses to
  `guard: None`; `... eq 18 if` (keyword, no condition) is a parse error; a bare-metric guard (`if file.exists`)
  is a parse error; case-insensitive `IF` / `UNLESS` parse.
- **`src/engine/executor.rs`**: guard satisfied runs the main assertion (PASS or FAIL as appropriate); guard not
  satisfied yields `Outcome::Skip`; `unless` inverts both; a guard whose file is missing yields `Outcome::Error`
  with a guard-error message; a guard with a non-boolean value (`tsv.columns.count eq true`) yields
  `Outcome::Error`.
- **`src/engine/report.rs`**: `Outcome::Skip.label() == "SKIP"`; SKIP does not set `has_failures` or `has_errors`.
- **`src/report.rs`**: `format_outcome(Outcome::Skip, ..)` renders the chosen icon/label with no color.

### Integration tests and fixtures

| File | Kind | Tracked? | Contents |
|---|---|---|---|
| `tests/data/conditional_assertions.txt` | fixture | yes | A batch mixing PASS, SKIP and (for `unless`) further PASS lines against existing fixtures, with a guard on an absent file to produce SKIP. All lines either pass or skip so the run exits 0. |
| `tests/conditional_subcommand_test.rs` | integration test | yes | Run-snapshot plus focused `assert` cases. |
| `tests/snapshots/conditional_subcommand_test__run_conditional_stdout.snap` | snapshot | yes | Captured stdout of the run. Generated with `cargo insta review`, then committed. |

The run fixture should reference an absent path (for example `tests/data/missing.tsv`, which is **not** created) to
exercise the SKIP path deterministically. Focused `assert`-subcommand cases:

- guard satisfied, main passes: exit 0, stdout contains `PASS.`
- guard satisfied, main fails: exit 1, stdout contains `FAIL.`
- guard not satisfied: exit 0, stdout contains `SKIP.`
- `unless` form skips when its condition holds: exit 0, `SKIP.`
- guard file missing (full form against a missing file): exit 2, stderr contains `ERROR.` and the guard-error message
- bare-metric guard (`if file.exists`): exit 2, `ERROR.` (now a parse error)

Existing snapshots are unaffected because no current fixture contains a guard.

## Critical files

- `src/engine/assertions.pest` — guard rules and the optional suffix on `assertion`.
- `src/engine/assertion.rs` — `guard` field, `Guard` and `Condition` types.
- `src/engine/parser.rs` — parse the optional guard's full condition positionally.
- `src/engine/executor.rs` — `run_metric` helper, `Evaluation` enum, guard handling, SKIP mapping.
- `src/engine/report.rs` — `Outcome::Skip` and `label()`.
- `src/report.rs` — `format_outcome` SKIP arm.
- `tests/conditional_subcommand_test.rs`, `tests/data/conditional_assertions.txt`, `tests/snapshots/` — new.
- `CLAUDE.md` — docs.

## Verification

1. `cargo build --release` — confirms the grammar compiles and the new `Outcome` arm is handled everywhere
   (the exhaustive matches will fail to compile if an arm is missed).
2. `cargo test` — runs the new parser, executor and integration tests; first run builds the binary.
3. `cargo insta review` (or `INSTA_UPDATE=always cargo test`) — accept the new conditional snapshot.
4. Manual end-to-end:
   ```bash
   cargo run -- assert "tests/data/present.tsv tsv.columns.count eq 18 if tests/data/present.tsv file.exists eq true"   # PASS, exit 0
   cargo run -- assert "tests/data/missing.tsv tsv.columns.count eq 18 if tests/data/missing.tsv file.exists eq true"   # SKIP, exit 0
   cargo run -- assert "tests/data/present.tsv tsv.columns.count eq 99 if tests/data/present.tsv file.exists eq true"   # FAIL, exit 1
   cargo run -- assert "tests/data/x.tsv tsv.line.count gt 0 if tests/data/missing.tsv file.size gt 0" # ERROR, exit 2
   cargo run -- run tests/data/conditional_assertions.txt                                # batch report
   ```
5. Confirm that a run whose assertions all pass or skip exits 0, and that a guard that errors yields exit 2.
