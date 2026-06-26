# Specification: Negated comparators (a `not` modifier) and the `file.contents` metric

## Context

`bioassert` compares a resource property against an expected value with one of ten comparators:
`eq`, `ne`, `lt`, `lte`, `gt`, `gte`, `starts`, `ends`, `contains`, `matches`
(`src/engine/assertions.pest:23`, `src/core/comparisons/comparator.rs`). There is an asymmetry hiding in that
set. The equality comparator has its negation (`eq` / `ne`, a complete pair), but the four string predicates do
not: `starts`, `ends`, `contains` and `matches` have no opposite. There is no way to assert "this value does not
start with X", "this column has no cell containing X" or, the motivating case, "this log does not contain the word
`Exception`".

The numeric comparators do not need negations, because `lt`/`lte`/`gt`/`gte`/`eq`/`ne` already cover the whole
order. The gap is specific to the string side. Today the only way to express string absence is to pre-process
outside `bioassert` (for example `grep -v Exception out.log > clean.txt` and then `clean.txt file.lines eq 0`),
which is exactly the adapter bioinformatics pipelines write around the tool. The pre-grep is doing real work the
metric set cannot express, not redundant boilerplate.

This spec closes the gap with one composable `not` modifier rather than four new comparators, and adds one new
metric, `file.contents`, as the worked example that the modifier unblocks. With both in place,
`out.log file.contents not contains 'Exception'` is a single assertion that needs no external grep.

```
out.log   file.contents not contains 'Exception'   # PASS when the log has no Exception anywhere
reads.tsv tsv.column.3.data.all not matches '^ERR'  # PASS when no data cell starts with ERR
ref.fasta fasta.seq.0.name not starts 'chrUn'       # PASS when the first record is a placed contig
```

The intended outcome: any string comparator can be negated by prefixing `not`, the negation is applied per
comparison (so it composes correctly with the whole-column aggregates), and a single new file-body metric lets the
modifier retire the most common pre-grep.

## Constraints and key facts

- **The comparator is one grammar token.** `src/engine/assertions.pest:23` is a flat ordered choice
  (`comparator = { ^"gte" | ^"gt" | ... | ^"matches" }`). The parser reads it positionally as the third inner pair
  of an assertion (`src/engine/parser.rs:14`) and stores it on the AST as a raw `String`
  (`Assertion.comparator`, `Condition.comparator`).
- **`Comparator` is the single parse, compare and render point.** `src/core/comparisons/comparator.rs` defines the
  `Comparator` enum, its `FromStr` (the only place a comparator string is parsed), its `Display` (the only place
  one is rendered), and the two comparison entry points: `compare<T: PartialOrd>` for numeric and boolean values,
  and `string_matcher` / `compare_string` for strings. `string_matcher` returns a reusable `StringMatcher` that
  owns its expected value and any compiled regex, so a `matches` pattern compiles once.
- **`StringMatcher` is the single source of truth for string comparison, and it is reused in streaming loops.**
  `src/delimited/column_all/functions.rs::check_column` applies one `StringMatcher` to every cell of a column and
  short-circuits on the first cell where `!matcher.is_match(value)`, reporting that line as the offender. So
  whatever negation we add to `StringMatcher::is_match` is automatically inherited by the whole-column metrics. This
  is the crux of the design (see Design decisions).
- **Negation already exists in the codebase as an XOR against a bool.** Guards carry `Guard.negate`
  (`src/engine/assertion.rs`), set from `if` vs `unless`, and `src/engine/executor.rs:73` evaluates the guard as
  `held ^ guard.negate`. A comparator `not` is the same idea applied one level down, at the comparison rather than
  the guard.
- **The report renders the raw comparator string, not `Display`.** `src/engine/executor.rs:102-105` formats the
  message from `assertion.comparator` (the source string), echoing the comparator exactly as written so a reader
  can grep the report back to the assertion. If the AST keeps the comparator as the source slice (`not contains`),
  the message reads `not contains` with no formatting code change.
- **Dispatch is metric-driven and first-match-wins.** `src/engine/executor.rs::dispatch` tries each
  `*Executor::try_parse(metric)` in turn and carries the locator as an opaque string;
  `AssertionRequest::path()` interprets it as a filesystem path for the file-backed families
  (`src/core/assertion_request.rs`). A new `file.contents` executor slots into this chain with no new machinery.
- **A string-returning metric already exists as a template.** `text.value` returns the locator string and compares
  it with `compare_string` (`src/text/value/functions.rs`), and `Value::StringValue`
  (`src/core/values/value.rs`) is the variant for a string `actual`. `file.contents` is the file-backed twin:
  read the body, compare it as a string.

## Design decisions

- **One composable `not` modifier, not four negated comparators.** Adding `ncontains`, `nmatches`, `nstarts` and
  `nends` would double the string-comparator surface, and the names are inconsistent with the existing negation
  token (it is `ne`, not `neq`, so there is no clean short form for the rest). A single `not` prefix composes with
  every string comparator and with any added later, and it is the first concrete slice of the boolean `not` that
  `specs/conditional-assertions.md` records as planned-but-deferred. The modifier reuses the established
  XOR-a-bool negation pattern.
- **Negation is applied at the comparison, not at the assertion's final result.** This is the decision that makes
  `not` correct rather than merely plausible. For a scalar metric (`file.size`, `text.value`, `file.contents`,
  `bam.header.rg.0.sm`) there is one comparison, so the two are identical. For the whole-column aggregates they
  differ, and only the per-comparison form is intuitive:
  - `tsv.column.3.all contains 'X'` passes when **every** cell contains X.
  - Negating the **final aggregate** gives `NOT(every cell contains X)` = "**some** cell lacks X" (the De Morgan
    dual), which is rarely what an author means.
  - Negating the **per-cell comparison** gives "every cell does-not-contain X" = "**no** cell contains X", which is
    the natural reading of `tsv.column.3.all not contains 'X'`.

  Applying `not` inside `StringMatcher::is_match` gets the per-cell form for free: `check_column` already passes
  only when the (now negated) matcher holds for every cell, and it reports the first cell where it does not (that
  is, the first cell that *does* contain the forbidden pattern). So the existing streaming machinery yields a
  correct "none" semantics, with correct first-offender reporting, with no change to `check_column` itself.
- **`not` is accepted with any comparator and applied uniformly, but it is meaningful only on the four string
  predicates.** With the numeric comparators it duplicates an existing one (`not gt` is `lte`, `not eq` is `ne`), so
  it is redundant rather than wrong. We accept it everywhere for orthogonality rather than special-casing
  string-only, which would need an extra validation rule. One subtlety matters for safety: a string comparator on a
  numeric or boolean metric (for example `file.size not contains 1MB`) is a mis-pairing, and `compare` returns
  `false` for the string operators **regardless of `negate`** rather than XORing a meaningless base. This keeps such
  a mis-pairing a visible always-FAIL, not an always-PASS that would silently hide the mistake in a validation gate.
  The deeper fix (rejecting the pairing outright) is the coherence checking deferred in `specs/resource-types.md`.
- **Fold negation into `Comparator`, so executors and the AST are untouched.** `Comparator` carries a `negate`
  flag; `compare`, `compare_string`, `string_matcher` and the produced `StringMatcher` apply it; `FromStr` parses a
  leading `not`; `Display` prefixes it. Because every executor already calls `request.comparator.compare(..)` or
  `request.comparator.string_matcher(..)`, no executor call site changes, and because the comparator stays a single
  source-slice token on the AST (`not contains`), neither the AST struct nor the request type gains a field and the
  report message is unchanged. The blast radius is the grammar's comparator rule, `comparator.rs`, the new
  `file.contents` module, and docs. This is deliberately the smallest change that is also correct for the aggregate
  case.
- **`file.contents` is the worked example, mirroring `text.value`.** `text.value` is "the inline literal as a
  string"; `file.contents` is "the file body as a string". It reuses the string comparators with no new comparison
  code, exactly as `text.value` does, and it is the metric that turns `not contains` into the grep replacement the
  context motivates. It reads the whole body into memory and is intended for log-sized and config-sized text files,
  not multi-gigabyte genomes; the streaming, line-oriented family is deferred (see Deferred).
- **`file.contents` reports a bounded summary as its `actual`, never the body.** The report renders `got <actual>`.
  Echoing a whole log into the report would be unbounded and could surface sensitive content, so `file.contents`
  returns a compact `Value::StringValue` summary (the body's byte length, for example `480 bytes`) rather than the
  text. The comparison still runs against the full body; only the rendered `actual` is summarized.

## Syntax

### Grammar changes (`src/engine/assertions.pest`)

Split the flat comparator into an optional `not` prefix and the operator, keeping the whole thing one captured
token so the parser still reads exactly four positional pairs:

```pest
comparator_op = { ^"gte" | ^"gt" | ^"lte" | ^"lt" | ^"eq" | ^"ne" | ^"starts" | ^"ends" | ^"contains" | ^"matches" }
comparator    = ${ (^"not" ~ WHITESPACE+)? ~ comparator_op }
```

`${ .. }` is a compound-atomic rule, so the captured `comparator` token is the exact source slice
(`contains` or `not contains`) and the inner whitespace is explicit (`WHITESPACE+`) rather than implicitly skipped.
Two properties make this safe against the existing grammar:

- **The optional `not` never eats a real comparator.** No comparator operator begins with the letters of `not`
  (`ne` shares only `n`, and `^"not"` fails on `ne`'s second character and falls through to `comparator_op`). So an
  un-negated assertion parses exactly as before, and every existing fixture and snapshot is unaffected.
- **A malformed comparator is still a clean parse error.** `x m not Y` where `Y` is not an operator fails: the
  optional `not` matches, `comparator_op` then fails on `Y`, the optional group backtracks, `comparator_op` is
  tried against `not` itself and fails, so the assertion errors. `EOI` (retained on `assertion`) keeps a trailing
  stray token an error as today.

The keyword position is unambiguous: `not` can only appear between the metric and the operator, never where a
resource, metric or value is expected, so a literal value `not` (`text.value eq not`) is unaffected because values
come after the comparator. The match is case-insensitive (`^"not"`), consistent with the existing comparator and
guard keywords.

The same `comparator` rule is reused by `condition` (`src/engine/assertions.pest:42`), so guards gain negated
comparators with no extra grammar.

## Semantics

- **`not` flips the comparator's outcome, evaluated per comparison.** For a scalar metric there is one comparison,
  so `not` is straightforward negation. For an aggregate metric (`*.column.N.all`, `*.column.N.data.all`) the
  per-cell predicate is negated, so `... .all not contains 'X'` means "no checked cell contains X" and the report
  names the first cell that does. A header-only or empty column still passes vacuously, as today.
- **`file.contents`** reads the resource as a UTF-8 string and compares the whole body with the string comparators.
  `contains` is a substring search over the entire body (newlines included), `matches` is a regex search over the
  body, `eq` compares the body exactly. Non-UTF-8 bytes make the read fail and the assertion reports ERROR, which
  is honest for a binary file handed to a text metric. The `actual` rendered in the report is a bounded summary
  (byte length), not the content.
- **Absence is now expressible natively.** `out.log file.contents not contains 'Exception'` passes when the log
  contains no `Exception` anywhere, replacing the `grep -v` pre-step. `not matches` gives regex absence without the
  negative-lookahead that the `regex` crate does not support.
- **`not` with a numeric comparator is redundant, not an error.** `file.size not gt 1MB` is accepted and equals
  `file.size lte 1MB`. A mis-paired comparator stays as visible as it was before `not`: a string comparator on a
  numeric metric (`file.size not contains 1MB`) is an always-FAIL (`compare` returns false for string operators
  regardless of `negate`), and a numeric comparator on a string metric (`file.contents not gt 5`) ERRORs
  (`string_matcher` rejects the numeric operators). Neither flips to a silent pass under `not`.

### Message formats

Unchanged in shape. The message echoes the raw comparator string, so a negated assertion renders its modifier
verbatim:

```
PASS. Expected out.log file.contents not contains Exception, got 480 bytes
FAIL. Expected reads.tsv tsv.column.3.data.all not matches ^ERR, got line 7 = "ERR_X"
```

## Examples

```
# String absence, the motivating case (replaces grep -v Exception)
out.log    file.contents not contains 'Exception'   # PASS when the log has no Exception
out.log    file.contents not matches  'ERROR|FATAL'  # PASS when neither word appears

# Whole-column "none" semantics: not is applied per cell, so this means "no data cell starts with ERR"
reads.tsv  tsv.column.3.data.all not starts 'ERR'    # PASS when no data cell begins with ERR
junctions.tsv tsv.column.6.data.all not matches '[^+-]'  # PASS when every strand cell is + or -

# Scalar string metrics
ref.fasta  fasta.seq.0.name not starts 'chrUn'       # PASS when the first record is a placed contig
output.bam bam.header.rg.0.sm not contains test      # PASS when the sample name has no "test"

# Presence still uses the plain comparator (no not)
out.log    file.contents contains 'completed'        # PASS when the log says completed

# not with a numeric comparator is redundant but accepted
output.bam file.size not gt 1MB                       # same as file.size lte 1MB

# Composes with guards (the condition reuses the same comparator rule)
report.tsv tsv.line.count gt 0 if out.log file.contents not contains 'FATAL'   # run only when no FATAL
```

## Implementation

### 1. Grammar (`src/engine/assertions.pest`)

Replace the flat `comparator` rule with the `comparator_op` plus compound-atomic `comparator` shown in Syntax. No
other rule changes; `condition` already references `comparator`.

### 2. Comparator (`src/core/comparisons/comparator.rs`)

Carry negation on the comparator and apply it at every comparison point. The smallest shape that keeps `Copy` and
leaves executor call sites untouched is to wrap the existing operator enum:

```rust
#[derive(Clone, Copy, PartialEq)]
pub enum Operator { Eq, Ne, Lt, Le, Gt, Ge, Starts, Ends, Contains, Matches }

#[derive(Clone, Copy, PartialEq)]
pub struct Comparator { pub op: Operator, pub negate: bool }
```

- `compare<T: PartialOrd + PartialEq>`: match on `self.op` as today, then return `result ^ self.negate`.
- `string_matcher`: build the `StringMatcher` for `self.op` as today, and carry `self.negate` into it (add a
  `negate: bool` field to each variant, or wrap the matcher in a `StringMatcher::Negated(Box<..>)`; a flag is the
  lighter change). `StringMatcher::is_match` returns `base ^ negate`. `compare_string` continues to delegate to
  `string_matcher`.
- `FromStr`: trim, split on whitespace; if the first word is `not` (case-insensitive) and a second word follows,
  set `negate = true` and parse the second word as the operator; otherwise parse the single word with
  `negate = false`. A leading `not` with no operator, or an unknown operator, is the existing `UnknownComparator`
  error.
- `Display`: render the operator as today, prefixed with `not ` when `negate` is set. (The report message uses the
  raw source string, so this affects only direct `Display` callers and tests.)

Update the comparator unit tests: `not contains` parses to `{ op: Contains, negate: true }`; `is_match` and
`compare` are inverted under negation; `string_matcher` for a negated string comparator inverts per call;
`not gt` parses and equals the inverse of `gt`; a bare `not` errors.

Because `string_matcher` now bakes in negation, `delimited::column_all::check_column` and `text::value` inherit it
with no change: the negated matcher applied per cell yields the "none" semantics described in Semantics.

### 3. New `file.contents` module (`src/file/contents/`)

Mirror the `src/file/lines/` layout:

```
src/file/contents/
  mod.rs            // pub use FileContentsExecutor
  executor.rs       // try_parse: metric == "file.contents"
  functions.rs      // read_contents(&Path) -> Result<String, FileError>, plus a bounded summary helper
```

- `functions::read_contents` opens `request.path()` and reads it to a `String` (`std::fs::read_to_string`), mapping
  an I/O or non-UTF-8 error to `FileError::new(path, e)`.
- `FileContentsExecutor::execute` reads the body, builds `request.comparator.string_matcher(strip_quotes(expected))`
  (the same path `column_all` uses, so a `matches` regex compiles once and negation is applied inside the matcher),
  applies it to the whole body, and returns `AssertionExecutionResult { success, actual }` where `actual` is the
  bounded summary (`Value::StringValue(format!("{} bytes", body.len()))`).
- Re-export `FileContentsExecutor` from `src/file/mod.rs`.

### 4. Dispatch (`src/engine/executor.rs`)

Add one line to `dispatch`, alongside the other `file.*` executors (order within the family does not matter, since
metrics are exact):

```rust
if let Some(e) = FileContentsExecutor::try_parse(metric) { return run(e, request); }
```

Import `FileContentsExecutor` in the `crate::file::{ .. }` use list at the top of the file.

### 5. Docs

- `skills/bioassert/references/metrics.md`: add `file.contents` to the File metrics table (string comparators), add
  a `not` row or note to the Comparators table explaining the modifier and that it is most useful with the four
  string predicates, and add the per-cell "none" semantics note to the delimited section
  (`*.column.N.all not contains 'X'` means no cell contains X). Add worked lines for the grep-replacement case.
- `CLAUDE.md`: note the `not` modifier under the comparator description, the `file.contents` metric in the
  `src/file/` bullet, and that negation is applied per comparison so it composes with the whole-column aggregates.

## Deferred

- **Line-oriented quantifiers for huge files (`file.lines.any`, `file.lines.none`, `file.lines.all`).**
  `file.contents` reads the whole body into memory, which is fine for logs and config but not for a multi-gigabyte
  file. A streaming, line-indexed family mirroring `*.column.N.all` would scan without buffering and would give
  per-line presence (`any`), absence (`none`) and universal (`all`) checks. With the `not` modifier from this spec,
  `file.lines.all not contains 'Exception'` already expresses per-line absence once that family exists, so `none`
  becomes sugar rather than a necessity. Deferred because `file.contents` covers the motivating log-sized case and
  the streaming family is a larger surface.
- **Boolean composition (`and`, `or`, grouped `not`).** `specs/conditional-assertions.md` already defers the full
  boolean layer. The comparator `not` here is a leaf negation on a single comparison, not the expression-level
  `not`, and does not introduce a precedence grammar.
- **Comparator/metric coherence checking.** Negation inherits the existing mis-pairing footgun (a string comparator
  on a numeric metric, now invertible). The clean fix is the coherence check already deferred in
  `specs/resource-types.md`, where each family declares the comparator and value shapes it accepts.

## Test plan

Following the conventions in `src/` (per-metric unit tests in `functions.rs`) and `tests/` (binary-driven
integration tests with `insta` snapshots).

### Unit tests

- **`src/core/comparisons/comparator.rs`**: `FromStr` parses `not contains` to `{ Contains, negate: true }`,
  case-insensitive `NOT`, a bare `not` errors, an unknown operator after `not` errors; `compare` inverts under
  negation (`not gt` equals the inverse of `gt`); `string_matcher` / `compare_string` invert for each string
  operator (`not contains`, `not matches`, `not starts`, `not ends`); `Display` prefixes `not `.
- **`src/delimited/column_all/functions.rs`**: a column where every cell lacks `X` passes under a negated
  `contains` matcher; a column where one cell contains `X` fails and reports that line (confirming the per-cell
  "none" semantics and first-offender reporting).
- **`src/file/contents/functions.rs`**: `read_contents` returns the body for a small text file; a non-UTF-8 file
  errors; the bounded summary reports the byte length.
- **`src/file/contents/executor.rs`** (or via `functions`): `file.contents contains 'Error'` on a log containing
  `Error` passes; `file.contents not contains 'Exception'` on a log without `Exception` passes and on one with it
  fails; `not matches` over the body; `eq` against an exact small body.

### Integration tests and fixtures

| File | Kind | Tracked? | Contents |
|---|---|---|---|
| `tests/data/clean_log.txt` | fixture | yes | A few lines including `completed`, no `Exception` or `FATAL`. Named `.txt`, not `.log`, because `*.log` is gitignored (it is the extension `bioassert` writes its own report under), so a `.log` fixture would never be committed and CI would not have it. |
| `tests/data/negated_comparators.txt` | fixture | yes | A batch of `not contains` / `not matches` / `not starts` lines over `clean_log.txt` and existing delimited fixtures, all of which PASS so the run exits 0. |
| `tests/negated_subcommand_test.rs` | integration test | yes | Run-snapshot plus focused `assert` cases for PASS and FAIL exit codes. |
| `tests/snapshots/negated_subcommand_test__run_negated_stdout.snap` | snapshot | yes | Captured stdout, generated with `cargo insta review` then committed. |

Focused `assert`-subcommand cases:

- `tests/data/clean_log.txt file.contents not contains 'Exception'`: exit 0, stdout `PASS.`
- `tests/data/clean_log.txt file.contents contains 'Exception'`: exit 1, stdout `FAIL.`
- `tests/data/clean_log.txt file.contents not contains 'completed'`: exit 1, `FAIL.` (the word is present)
- a delimited `*.column.N.data.all not starts 'X'` PASS line over an existing fixture: exit 0, `PASS.`
- a binary fixture (for example `tests/data/sample.bam`) with `file.contents contains 'x'`: exit 2, `ERROR.`
  (non-UTF-8 read fails)

Existing snapshots are unaffected because no current fixture uses `not` or `file.contents`, and the comparator
grammar split does not change what previously parsed.

## Critical files

- `src/engine/assertions.pest` — split `comparator` into optional `not` plus `comparator_op`.
- `src/core/comparisons/comparator.rs` — `Comparator` carries `negate`; `FromStr`, `Display`, `compare`,
  `string_matcher` / `compare_string` and `StringMatcher` apply it.
- `src/file/contents/` — new module (`executor.rs`, `functions.rs`, `mod.rs`) following the `src/file/lines/` shape.
- `src/file/mod.rs` — re-export `FileContentsExecutor`.
- `src/engine/executor.rs` — register `FileContentsExecutor` in `dispatch` and import it.
- `skills/bioassert/references/metrics.md`, `CLAUDE.md` — document the `not` modifier, the per-cell aggregate
  semantics and the `file.contents` metric.
- `tests/negated_subcommand_test.rs`, `tests/data/negated_comparators.txt`, `tests/data/clean_log.txt`,
  `tests/snapshots/` — new.

## Verification

1. `cargo build --release` — confirms the grammar split compiles and the `Comparator` refactor threads through
   every comparison call site (the comparator is used pervasively, so a missed arm fails to compile).
2. `cargo test` — runs the new comparator, `file.contents` and integration tests plus the full existing suite,
   proving the comparator change is behaviour-preserving for un-negated assertions.
3. `cargo insta review` (or `INSTA_UPDATE=always cargo test`) — accept the new negated-comparators snapshot.
4. Manual end-to-end:
   ```bash
   cargo run -- assert "tests/data/clean_log.txt file.contents not contains 'Exception'"  # PASS, exit 0
   cargo run -- assert "tests/data/clean_log.txt file.contents contains 'Exception'"      # FAIL, exit 1
   cargo run -- assert "tests/data/clean_log.txt file.contents contains 'completed'"      # PASS, exit 0
   cargo run -- assert "tests/data/junctions.tsv tsv.column.6.data.all not matches '[^+-]'"  # PASS
   cargo run -- run tests/data/negated_comparators.txt                                # batch report
   ```
5. Confirm a run whose assertions all pass exits 0, and that the existing `run` / `bam` / `fasta` / conditional
   snapshots are unchanged.