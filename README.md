# BioAssert

A CLI tool for asserting properties of files using a simple declarative syntax. Designed for validating pipeline outputs
in bioinformatics workflows.

## Quick start

Install with Homebrew (see [Installation](#installation) for prebuilt binaries, Docker and cargo):

```bash
brew install PeterKneale/tap/bioassert
```

Write your checks to an assertions file. Each line is `<file> <metric> <comparator> <value>`; blank lines and lines
beginning with `#` are ignored, and whitespace between fields is flexible so you can align columns for readability:

```text
# assertions.txt

# Outputs were created and are non-trivial
output.bam     file.exists   eq   true
output.bam     file.size     gt   1MB
results.vcf    file.empty    eq   false

# The sample sheet has the columns we expect
samples.csv    csv.columns.count   eq   5
samples.csv    csv.lines.count     gte  2

# BAM header provenance is intact
output.bam     bam.header.rg.count   gte  1
output.bam     bam.header.hd.so      eq   coordinate

# The reference genome has the right number of contigs
ref.fasta      fasta.seq.count   eq   25
```

Run them:

```bash
bioassert run assertions.txt
```

`run` defaults to `assertions.txt` when you give no path, so plain `bioassert run` works too.

Each assertion prints a `PASS.`/`FAIL.` line, and the process exits non-zero if any assertion fails or errors, so it
gates a pipeline step cleanly:

```text
PASS. Expected output.bam file.exists eq true, got true
PASS. Expected output.bam file.size gt 1MB, got 5.00MB
PASS. Expected results.vcf file.empty eq false, got false
PASS. Expected samples.csv csv.columns.count eq 5, got 5
PASS. Expected samples.csv csv.lines.count gte 2, got 101
PASS. Expected output.bam bam.header.rg.count gte 1, got 2
PASS. Expected output.bam bam.header.hd.so eq coordinate, got coordinate
PASS. Expected ref.fasta fasta.seq.count eq 25, got 25
```

The same lines are written to a report file (here `assertions.txt.log`) so they can be captured as a pipeline artifact.
See [Writing assertions](#writing-assertions) for more examples, [Metrics](#metrics) for the full reference, and
[Results and exit codes](#results-and-exit-codes) for how exit codes map to outcomes.

## Installation

### Homebrew (macOS and Linux)

```bash
brew install PeterKneale/tap/bioassert
```

### Prebuilt binaries

Download the archive for your platform from the [releases page](https://github.com/PeterKneale/bioassert/releases),
unpack it, and move the `bioassert` binary onto your `PATH`. Builds are provided for macOS (Apple Silicon and Intel)
and Linux (x86_64 and arm64), each with a matching `.sha256` checksum:

```bash
# Example for macOS Apple Silicon (substitute the current version and your platform)
VERSION=3.1.1
curl -LO "https://github.com/PeterKneale/bioassert/releases/download/v${VERSION}/bioassert-${VERSION}-macos-arm64.tar.gz"
tar xzf "bioassert-${VERSION}-macos-arm64.tar.gz"
mv bioassert /usr/local/bin/   # or any directory on your PATH
```

Asset names follow `bioassert-<version>-<platform>.tar.gz`, where `<platform>` is `macos-arm64`, `macos-x86_64`,
`linux-x86_64` or `linux-arm64`.

### Docker

```bash
docker pull ghcr.io/peterkneale/bioassert
```

Mount your working directory to `/data` and use that path prefix in your assertions. To run an assertions file:

```bash
docker run --rm -v "$PWD":/data ghcr.io/peterkneale/bioassert run /data/assertions.txt
```

A single assertion can also be passed inline (see the `assert` subcommand below):

```bash
docker run --rm -v "$PWD":/data ghcr.io/peterkneale/bioassert assert "/data/output.bam file.exists eq true"
```

### cargo

If you have a Rust toolchain, install from [crates.io](https://crates.io/crates/bioassert):

```bash
cargo install bioassert
```

## Command-line interface

```
bioassert [OPTIONS] <COMMAND>
```

### Commands

| Command           | Description                                                                  |
|-------------------|------------------------------------------------------------------------------|
| `run [file]`      | Evaluate every assertion in a file (defaults to `assertions.txt`). Blank lines and `#` comments are skipped. |
| `assert "<...>"`  | Evaluate a single assertion passed as one quoted string.                     |

`run` is the primary workflow: keep your checks in a version-controlled assertions file and run them as a pipeline step.
`assert` is handy for one-off checks and quick experimentation at the shell.

### Options

| Option                    | Default | Description                                                                                  |
|---------------------------|---------|----------------------------------------------------------------------------------------------|
| `--report-file <FILE>`    | derived | Write the assertion report to `FILE` instead of the default location (see Output below).      |
| `--color <WHEN>`          | `auto`  | When to use ANSI color in console output: `auto`, `always` or `never` (see Color below). Also spelled `--colour`. |
| `--icons <WHEN>`          | `auto`  | When to prefix `PASS`/`FAIL`/`ERROR` console lines with a status icon: `auto`, `always` or `never` (see Icons below). |
| `-h`, `--help`            |         | Print help.                                                                                  |
| `-v`, `--version`         |         | Print the version.                                                                           |

`--report-file`, `--color` and `--icons` are global, so they may appear either before or after the subcommand. For example, `bioassert run checks.txt --color=never` and `bioassert --color=never run checks.txt` are equivalent.

### Results and exit codes

Result lines are written to **stdout**, one per assertion: `PASS.` when the assertion holds and `FAIL.` when it does not. A guarded assertion whose condition is not met is reported as `SKIP.`, a neutral outcome. Errors (invalid syntax, an unknown metric, an unreadable file) are written to **stderr** as `ERROR.` lines. The process exit code reflects the worst outcome across all assertions:

| Exit code | Meaning                                              |
|-----------|------------------------------------------------------|
| `0`       | Every assertion passed.                              |
| `1`       | At least one assertion failed (but none errored).    |
| `2`       | At least one assertion could not be evaluated.       |

A `SKIP` is neutral and never changes the exit code, so a run whose results are only `PASS` and `SKIP` still exits `0`. This makes `bioassert` easy to gate a pipeline step on: a non-zero exit halts the workflow.

### Output

Alongside the console output, `bioassert` writes an **assertion report**: a file built from the results of every executed assertion. It is plain text, one `PASS.`/`FAIL.`/`ERROR.` line per assertion, with no level or module prefixes, no timestamps, no ANSI color and no icons, so it stays easy to read and `grep`. Its location is resolved as follows:

1. `--report-file <FILE>` if given.
2. Otherwise for `run <file>`, the derived path `<file>.log` (for example `checks.txt` reports to `checks.txt.log`).
3. Otherwise `assertions.log` in the current directory.

For example, the report for a failing run reads:

```text
PASS. Expected output.bam file.exists eq true, got true
FAIL. Expected results.vcf file.lines gt 999, got 0
```

### Color

`--color` controls ANSI color on the console only. The leading `PASS` keyword is colored green, and `FAIL`/`ERROR` are colored red. The assertion report file is never colored, regardless of this setting. The flag is also accepted spelled `--colour`.

| Value    | Behaviour                                                                                  |
|----------|--------------------------------------------------------------------------------------------|
| `auto`   | Color only when stdout is a terminal and the `NO_COLOR` environment variable is unset. This is the default, so output stays plain when piped or redirected. |
| `always` | Always color, even when piped. Overrides `NO_COLOR`.                                        |
| `never`  | Never color.                                                                               |

`auto` follows the [`NO_COLOR`](https://no-color.org) convention: setting `NO_COLOR` to any non-empty value disables color. An explicit `--color=always` takes priority over it.

### Icons

`--icons` prefixes each console result line with a status icon: 🟢 for `PASS`, 🔴 for `FAIL` and 🔥 for `ERROR`. It takes the same `auto`/`always`/`never` values as `--color` and follows the same resolution logic, but controls the icons rather than the color:

| Value    | Behaviour                                                                                  |
|----------|--------------------------------------------------------------------------------------------|
| `auto`   | Icons only when stdout is a terminal and `NO_COLOR` is unset. This is the default, so output stays plain when piped or redirected. |
| `always` | Always show icons, even when piped. Overrides `NO_COLOR`.                                   |
| `never`  | Never show icons.                                                                          |

Color and icons are resolved independently, so either can be on while the other is off. The assertion report file is never iconified, so it stays easy to `grep`.

## Writing assertions

An assertions file has one assertion per line:

```
<resource> <metric> <comparator> <value>
```

The first token is a resource locator: a file path for the `file.*`, `csv.*`/`tsv.*`/`psv.*`, `bam.*` and `fasta.*`
metrics, or an inline literal string for the `text.*` family.

Lines beginning with `#` are comments and blank lines are ignored. Whitespace between fields is flexible, so columns can
be aligned for readability. Pass the file to `bioassert run`.

Bare values must be alphanumeric. Values that contain dots, dashes, colons, or spaces (read-group IDs, version strings,
sequence names, descriptions) must be single- or double-quoted: `'H0164.2'`, `'1.6'`, `'Solexa-272222'`,
`'NC_000001.11'`, `'Homo sapiens chromosome 1'`.

### File checks

```text
output.bam     file.exists       eq   true     # file exists
results.vcf    file.empty        eq   false    # not empty
output.bam     file.size         gte  1MB      # at least 1 MB
results.tsv    file.lines        eq   1000     # exactly 1000 lines
results.tsv    file.lines        gte  1        # at least one line
reads.bam      file.compression  eq   bgzf     # block-gzip (samtools/tabix); also gzip, bzip2, xz, zstd, zip, none
reads.fastq.gz file.compressed   eq   true     # compressed in some format
```

### CSV / TSV / PSV checks

Replace `csv` with `tsv` (tab-separated) or `psv` (pipe-separated) for those formats. Lines and columns are 1-indexed.

```text
samples.csv   csv.columns.count       eq   3                  # 3 columns
counts.tsv    tsv.lines.count         gt   10                 # more than 10 lines
report.psv    psv.line.2.column.1     eq   Alice              # cell value
samples.csv   csv.line.2.column.3     starts  New             # cell prefix
results.tsv   tsv.line.2.column.2     matches '^[0-9]+$'      # cell regex
junctions.tsv tsv.column.6.data.all   matches '^[+-]$'        # every data row in column 6
junctions.tsv tsv.column.4.all        matches '^JUNC[0-9]+$'  # every row, header included
```

`column.N.all` applies the comparison to every cell in column N (1-indexed) and passes only when they all hold;
`column.N.data.all` does the same but skips the header (line 1), which is what you usually want for a file with a
header row. A header-only or empty file passes vacuously. On failure the report names the first offending row and its
value (e.g. `got line 3 = "NDA"`). Any string comparator works, not just `matches`.

### BAM header checks

Assertions on a BAM file's SAM header live under `bam.header.*`. Read groups (`@RG`) are addressed by 0-based index.
Values containing dots, dashes or colons (read-group IDs, library names, ISO dates, version strings) must be quoted.

```text
output.bam   bam.header.rg.count        gte  1                  # at least one read group
output.bam   bam.header.rg.0.sm         eq   NA12878            # sample name (callers group by this)
output.bam   bam.header.rg.0.pl         eq   ILLUMINA           # sequencing platform
output.bam   bam.header.rg.0.lb         eq   'Solexa-272222'    # library name (quoted: dash)
output.bam   bam.header.rg.1.id         eq   'H0164.2'          # second read-group ID (quoted: dot)
output.bam   bam.header.rg.0.pu.present eq   true               # a platform-unit tag is set
output.bam   bam.header.hd.so           eq   coordinate         # file is coordinate-sorted
output.bam   bam.header.sq.count        eq   1                  # exactly one reference sequence
```

### FASTA checks

Assertions on a FASTA file's records live under `fasta.seq.*`, with `fasta.length` as a whole-file total across all
records. Records are addressed by 0-based index, following file order. Sequence names and descriptions that contain
dots, dashes, colons, or spaces must be quoted.

```text
ref.fasta   fasta.seq.count          eq   25                          # 25 records
ref.fasta   fasta.length             gte  3000000000                  # total bases across all records
ref.fasta   fasta.seq.0.name         eq   chr1                        # first record's name
ref.fasta   fasta.seq.0.length       gte  20000000                    # first record's length
ref.fasta   fasta.seq.0.description  eq   'Homo sapiens chromosome 1' # description (quoted: spaces)
ref.fasta   fasta.seq.0.name         matches '^chr[0-9XYM]+$'         # name matches a regex
ref.fasta   fasta.seq.24.present     eq   true                        # a record exists at this index
```

### Text checks

The `text.*` family treats the first token as an inline literal string rather than a file path, so it does no file I/O
and never errors on a missing file. It is handy for quick checks and as a [guard](#conditional-assertions-guards) input.
Quote the literal if it contains dots, dashes, colons or spaces.

```text
sampleA       text.value    eq      sampleA      # literal equals
sampleA       text.value    starts  sample       # literal prefix
sampleA       text.length   eq      7            # character count (Unicode scalars)
```

### Conditional assertions (guards)

Append `if` or `unless` and a full condition to evaluate an assertion only when that condition holds. The condition has
the same `<resource> <metric> <comparator> <value>` shape as an assertion and may target a different file:

```text
report.tsv  tsv.columns.count    eq   18   if      report.tsv  file.exists  eq  true
reads.bam   bam.header.rg.count  gte  1    unless  reads.bam   file.empty   eq  true
```

- `if` runs the assertion when the condition is satisfied; `unless` runs it when the condition is **not** satisfied.
- When the guard is not satisfied the assertion is reported as **SKIP**, a neutral outcome that does not affect the
  exit code (see [Results and exit codes](#results-and-exit-codes)).
- If the guard itself cannot be evaluated (for example its file is missing) the assertion is reported as **ERROR**.
  `file.exists` is the safe guard because it returns `false` rather than erroring on a missing file.

### A single inline assertion

For a one-off check you can skip the file and pass a single assertion to the `assert` subcommand:

```bash
bioassert assert "output.bam file.size gte 1MB"
```

### Console output with color and icons

On a terminal, `auto` shows both by default. To force them on (for example when piping into a pager that understands ANSI), use `--color=always --icons=always`:

```bash
bioassert --color=always --icons=always run checks.txt
```

Each result line is prefixed with its status icon, and the leading keyword is colored (green `PASS`, red `FAIL`/`ERROR`):

```text
🟢  PASS. Expected output.bam file.exists eq true, got true
🔴  FAIL. Expected results.vcf file.lines gt 999, got 0
🔥  ERROR. unknown metric: file.explode
```

(`PASS`/`FAIL` lines go to stdout and `ERROR` lines to stderr; the icons render here, but the green/red keyword coloring only shows on an ANSI-capable terminal.)

### Nextflow

`bioassert` works well as a validation step in a [Nextflow](https://www.nextflow.io/) pipeline. Write the
assertions to a file, then run them with the `run` subcommand. The `--report-file` flag persists the report
so it can be captured as a process output, and a non-zero exit code (1 for a failed assertion, 2 for an error)
fails the task automatically.

```groovy
process REFERENCE_GENOME_ANNOTATIONS_ASSERTIONS {

    tag "reference_genome_annotations"
    label 'process_medium'
    conda "${moduleDir}/environment.yml"
    container "ghcr.io/peterkneale/bioassert:3.1.1"

    input:
    path(annotation)
    path(annotation_decompressed)

    output:
    path("*.log"), emit: log
    tuple val("${task.process}"), val('bioassert'), eval('bioassert --version | sed "1!d;s/.* //"'), emit: versions_bioassert, topic: versions

    script:
    """
    cat <<EOF > assertions.txt
    ${annotation} file.exists eq true
    ${annotation} file.size gt 10MB
    ${annotation_decompressed} file.exists eq true
    ${annotation_decompressed} file.size gt 50MB
    EOF

    bioassert \\
        --report-file reference_genome_annotations_assertions.log \\
        run assertions.txt
    """
}
```

Notes:

- Assertions reference the staged file paths directly. The unquoted heredoc (`<<EOF`) lets Nextflow interpolate
  `${annotation}` into the real filename, and the grammar accepts dotted names like `genome.gff.gz` unquoted.
- Global flags such as `--report-file` go before the `run` subcommand; the assertions file is its positional argument.

### Claude Code skill

A [Claude Code](https://claude.com/claude-code) skill that helps developers and bioinformaticians write bioassert
assertions and wire them into Nextflow processes ships in this repo under [`skills/bioassert/`](skills/bioassert/). It
covers the assertion syntax, the full metric reference, and copy-paste Nextflow process templates (hard gate, advisory,
nf-core module).

The repo doubles as a Claude Code plugin marketplace, so install the skill with the `claude` CLI:

```bash
claude plugin marketplace add PeterKneale/bioassert
claude plugin install bioassert@peterkneale
```

Update or remove it later:

```bash
claude plugin update bioassert@peterkneale
claude plugin uninstall bioassert@peterkneale
```

Installing the plugin makes the skill available to Claude; it does not install the `bioassert` binary itself (see
[Installation](#installation)). Alternatively, copy the skill directory in by hand — Claude Code also discovers skills
under `.claude/skills/`:

```bash
cp -r skills/bioassert /path/to/your-pipeline/.claude/skills/
```

## Syntax

```
<resource> <metric> <comparator> <value> [if|unless <resource> <metric> <comparator> <value>]
```

The first token is a **resource locator**. For file-backed metrics (`file.*`, `csv.*`/`tsv.*`/`psv.*`, `bam.*`,
`fasta.*`) it is a filesystem path; for the `text.*` family it is an inline literal string. The optional `if`/`unless`
clause is a [guard](#conditional-assertions-guards).

## Metrics

### File metrics

| Metric        | Description                    | Comparators                          | Value   |
|---------------|--------------------------------|--------------------------------------|---------|
| `file.exists`      | Whether the file exists        | `eq`, `ne`                           | boolean |
| `file.empty`       | Whether the file is zero bytes | `eq`, `ne`                           | boolean |
| `file.size`        | File size                      | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | size    |
| `file.lines`       | Line count                     | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |
| `file.compression` | Compression format, detected from leading magic bytes (no decompression) | `eq`, `ne`, `starts`, `ends`, `contains`, `matches` | string: `none`, `gzip`, `bgzf`, `bzip2`, `xz`, `zstd`, `zip` |
| `file.compressed`  | Whether the file is compressed in any format | `eq`, `ne`             | boolean |

### Delimited file metrics (CSV, TSV, PSV)

| Metric                | Description                                     |
|-----------------------|-------------------------------------------------|
| `csv.columns.count`     | Number of columns in the first (header) row                          |
| `csv.lines.count`       | Number of lines in the file                                          |
| `csv.line.N.column.M`   | Content of cell at line N, column M (1-indexed)                      |
| `csv.column.N.all`      | Holds for every cell in column N, header included (1-indexed)        |
| `csv.column.N.data.all` | Holds for every cell in column N, skipping the header row            |

Replace `csv` with `tsv` (tab-separated) or `psv` (pipe-separated) for those formats. The `column.N.all` /
`column.N.data.all` metrics accept any string comparator (`eq`, `ne`, `starts`, `ends`, `contains`, `matches`) and
pass only when it holds for every checked cell; a header-only or empty file passes vacuously.

### BAM header metrics

Metrics on a BAM file's SAM header, all under the `bam.header.*` namespace. Read groups (`@RG`) are addressed by
0-based index `N`, following header order. `<tag>` is a lowercased 2-letter SAM tag.

| Metric                          | Description                                            | Comparators                          | Value   |
|---------------------------------|--------------------------------------------------------|--------------------------------------|---------|
| `bam.header.rg.count`           | Number of `@RG` read groups                            | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |
| `bam.header.sq.count`           | Number of `@SQ` reference sequences                    | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |
| `bam.header.pg.count`           | Number of `@PG` program records                        | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |
| `bam.header.rg.N.<tag>`         | Tag value of read group N (`id`, `sm`, `lb`, `pl`, `pu`, ...) | `eq`, `ne`, `starts`, `ends`, `contains`, `matches` | string |
| `bam.header.rg.N.present`       | Whether read group N exists                            | `eq`, `ne`                           | boolean |
| `bam.header.rg.N.<tag>.present` | Whether `<tag>` is set on read group N                 | `eq`, `ne`                           | boolean |
| `bam.header.hd.vn`              | `@HD` format version (VN)                              | `eq`, `ne`, `starts`, `ends`, `contains`, `matches` | string |
| `bam.header.hd.so`              | `@HD` sort order (SO), e.g. `coordinate`               | `eq`, `ne`, `starts`, `ends`, `contains`, `matches` | string |

`id` resolves to the read-group identifier (the `@RG ID` field); other tags resolve to the read group's
remaining fields. Reading a tag value (or `@HD` field) that is not set, or a read group whose index is out of
range, is an **error**; use the `.present` form to test for presence without erroring. Quote values that contain
dots, dashes or colons (`'H0164.2'`, `'Solexa-272222'`, `'1.6'`).

### FASTA metrics

Metrics on a FASTA file's records, under the `fasta.seq.*` namespace for per-record metrics plus the `fasta.length`
whole-file aggregate. Records are addressed by 0-based index `N`, following file order.

| Metric                            | Description                                                | Comparators                          | Value   |
|-----------------------------------|------------------------------------------------------------|--------------------------------------|---------|
| `fasta.seq.count`                 | Number of sequence records                                 | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |
| `fasta.length`                    | Total bases summed across all records                      | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |
| `fasta.seq.N.name`                | Name (ID) of record N — the first whitespace-delimited header token | `eq`, `ne`, `starts`, `ends`, `contains`, `matches` | string |
| `fasta.seq.N.description`         | Description of record N — header text after the name       | `eq`, `ne`, `starts`, `ends`, `contains`, `matches` | string |
| `fasta.seq.N.length`              | Length in bases of record N's sequence                     | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |
| `fasta.seq.N.present`             | Whether a record exists at index N                         | `eq`, `ne`                           | boolean |
| `fasta.seq.N.description.present` | Whether record N has a (non-empty) description             | `eq`, `ne`                           | boolean |

`name` is always present for a record; `description` is optional. Reading the name, description, or length of a record
whose index is out of range is an **error**; use the `.present` form to test for presence without erroring. Quote names
and descriptions that contain dots, dashes, colons or spaces (`'NC_000001.11'`, `'Homo sapiens chromosome 1'`).

### Text metrics

Metrics on an inline literal string under the `text.*` namespace. The first token is the literal itself (not a file
path), so these never touch the filesystem. Quote the literal if it contains dots, dashes, colons or spaces.

| Metric        | Description                       | Comparators                                         | Value  |
|---------------|-----------------------------------|-----------------------------------------------------|--------|
| `text.value`  | The literal string itself         | `eq`, `ne`, `starts`, `ends`, `contains`, `matches` | string |
| `text.length` | Character count (Unicode scalars) | `eq`, `ne`, `lt`, `lte`, `gt`, `gte`                | count  |

### Comparators

| Comparator | Meaning       | Use with      |
|------------|---------------|---------------|
| `eq`       | equal         | any           |
| `ne`       | not equal     | any           |
| `lt`       | less than     | size, count   |
| `lte`      | less or equal | size, count   |
| `gt`       | greater than  | size, count   |
| `gte`      | >=            | size, count   |
| `starts`   | starts with   | string        |
| `ends`     | ends with     | string        |
| `contains` | contains      | string        |
| `matches`  | regex match   | string        |

### Values

| Type    | Examples                         |
|---------|----------------------------------|
| boolean | `true`, `false`                  |
| size    | `5B`, `1KB`, `2MB`, `1GB`        |
| count   | `0`, `10`, `1000`, `5K`, `5M`, `5G` |
| string  | `Alice`, `"New York"`, `'hello'` |

Count values accept optional decimal multiplier suffixes: `K` = 1,000, `M` = 1,000,000,
`G` = 1,000,000,000 (so `5K` is `5000`). These are case-insensitive and distinct from the
binary, 1024-based size suffixes (`KB`, `MB`, `GB`) used by `file.size`.
