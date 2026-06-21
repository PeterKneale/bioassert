# BioAssert

A CLI tool for asserting properties of files using a simple declarative syntax. Designed for validating pipeline outputs
in bioinformatics workflows.

## Installation

### cargo

```bash
cargo install bioassert
```

### Docker

```bash
docker pull ghcr.io/peterkneale/bioassert
docker run --rm -v "$PWD":/data ghcr.io/peterkneale/bioassert assert "/data/output.bam file.exists eq true"
```

Mount your working directory to `/data` and use that path prefix in your assertions. To run an assertions file:

```bash
docker run --rm -v "$PWD":/data ghcr.io/peterkneale/bioassert run /data/checks.txt
```

## Command-line interface

```
bioassert [OPTIONS] <COMMAND>
```

### Commands

| Command           | Description                                                                  |
|-------------------|------------------------------------------------------------------------------|
| `assert "<...>"`  | Evaluate a single assertion passed as one quoted string.                     |
| `run <file>`      | Evaluate every assertion in a file. Blank lines and `#` comments are skipped. |

### Options

| Option                    | Default | Description                                                                                  |
|---------------------------|---------|----------------------------------------------------------------------------------------------|
| `--report-file <FILE>`    | derived | Write the assertion report to `FILE` instead of the default location (see Output below).      |
| `--color <WHEN>`          | `auto`  | When to use ANSI color in console output: `auto`, `always` or `never` (see Color below). Also spelled `--colour`. |
| `--icons <WHEN>`          | `auto`  | When to prefix `PASS`/`FAIL`/`ERROR` console lines with a status icon: `auto`, `always` or `never` (see Icons below). |
| `-h`, `--help`            |         | Print help.                                                                                  |
| `-V`, `--version`         |         | Print the version.                                                                           |

`--report-file`, `--color` and `--icons` are global, so they may appear either before or after the subcommand. For example, `bioassert run checks.txt --color=never` and `bioassert --color=never run checks.txt` are equivalent.

### Results and exit codes

Result lines are written to **stdout**, one per assertion: `PASS.` when the assertion holds and `FAIL.` when it does not. Errors (invalid syntax, an unknown metric, an unreadable file) are written to **stderr** as `ERROR.` lines. The process exit code reflects the worst outcome across all assertions:

| Exit code | Meaning                                              |
|-----------|------------------------------------------------------|
| `0`       | Every assertion passed.                              |
| `1`       | At least one assertion failed (but none errored).    |
| `2`       | At least one assertion could not be evaluated.       |

This makes `bioassert` easy to gate a pipeline step on: a non-zero exit halts the workflow.

### Output

Alongside the console output, `bioassert` writes an **assertion report**: a file built from the results of every executed assertion. It is plain text, one `PASS.`/`FAIL.`/`ERROR.` line per assertion, with no level or module prefixes, no timestamps, no ANSI color and no icons, so it stays easy to read and `grep`. Its location is resolved as follows:

1. `--report-file <FILE>` if given.
2. Otherwise for `run <file>`, the derived path `<file>.log` (for example `checks.txt` reports to `checks.txt.log`).
3. Otherwise `assertions.log` in the current directory.

For example, the report for a failing run reads:

```text
PASS. Expected output.bam file.exists == true, got true
FAIL. Expected results.vcf file.lines > 999, got 0
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

## Examples

### Console output with color and icons

On a terminal, `auto` shows both by default. To force them on (for example when piping into a pager that understands ANSI), use `--color=always --icons=always`:

```bash
bioassert --color=always --icons=always run checks.txt
```

Each result line is prefixed with its status icon, and the leading keyword is colored (green `PASS`, red `FAIL`/`ERROR`):

```text
🟢  PASS. Expected output.bam file.exists == true, got true
🔴  FAIL. Expected results.vcf file.lines > 999, got 0
🔥  ERROR. unknown metric: file.explode
```

(`PASS`/`FAIL` lines go to stdout and `ERROR` lines to stderr; the icons render here, but the green/red keyword coloring only shows on an ANSI-capable terminal.)

### File checks

```bash
# File exists
bioassert assert "output.bam file.exists eq true"

# File is not empty
bioassert assert "results.vcf file.empty eq false"

# File is at least 1 MB
bioassert assert "output.bam file.size gte 1MB"

# Exactly 1000 lines
bioassert assert "results.tsv file.lines eq 1000"

# At least one line
bioassert assert "results.tsv file.lines gte 1"
```

### CSV / TSV / PSV checks

```bash
# CSV has 3 columns
bioassert assert "samples.csv csv.columns.count eq 3"

# TSV has more than 10 data lines
bioassert assert "counts.tsv tsv.lines.count gt 10"

# PSV cell value equals a string
bioassert assert "report.psv psv.line.2.column.1 eq Alice"

# CSV cell starts with a prefix
bioassert assert "samples.csv csv.line.2.column.3 starts New"

# TSV cell matches a regex
bioassert assert "results.tsv tsv.line.2.column.2 matches '^[0-9]+$'"
```

### BAM header checks

Assertions on a BAM file's SAM header live under `bam.header.*`. Read groups (`@RG`) are addressed by 0-based
index. Values containing dots, dashes or colons (read-group IDs, library names, ISO dates, version strings) must
be quoted.

```bash
# At least one read group is present
bioassert assert "output.bam bam.header.rg.count gte 1"

# Sample name of the first read group (what variant callers group by)
bioassert assert "output.bam bam.header.rg.0.sm eq NA12878"

# Sequencing platform
bioassert assert "output.bam bam.header.rg.0.pl eq ILLUMINA"

# Library name (quoted: contains a dash)
bioassert assert "output.bam bam.header.rg.0.lb eq 'Solexa-272222'"

# Read-group ID of the second read group (quoted: contains a dot)
bioassert assert "output.bam bam.header.rg.1.id eq 'H0164.2'"

# A platform unit tag is set on the first read group
bioassert assert "output.bam bam.header.rg.0.pu.present eq true"

# The file is coordinate-sorted
bioassert assert "output.bam bam.header.hd.so eq coordinate"

# Exactly one reference sequence
bioassert assert "output.bam bam.header.sq.count eq 1"
```

### Assertions file

Lines beginning with `#` are comments. Blank lines are ignored.

```
# checks.txt

# Confirm outputs were created
output.bam    file.exists    eq   true
results.vcf   file.exists    eq   true

# Validate sizes
output.bam    file.size      gt   1MB
results.vcf   file.empty     eq   false

# Check line counts
results.vcf   file.lines     gte  1

# Validate CSV structure
samples.csv   csv.columns.count  eq   5
samples.csv   csv.lines.count    gte  2

# Spot-check a cell
samples.csv   csv.line.2.column.1  starts  SAMPLE_

# Validate BAM header provenance
output.bam    bam.header.rg.count   gte  1
output.bam    bam.header.rg.0.sm    eq   NA12878
output.bam    bam.header.rg.0.pl    eq   ILLUMINA
output.bam    bam.header.hd.so      eq   coordinate
```

```bash
bioassert run checks.txt
```

Output (also written to the report file `checks.txt.log`):

```
PASS. Expected output.bam file.exists == true, got true
PASS. Expected results.vcf file.exists == true, got true
PASS. Expected output.bam file.size > 1MB, got 5.00MB
PASS. Expected results.vcf file.empty == false, got false
PASS. Expected results.vcf file.lines >= 1, got 42
PASS. Expected samples.csv csv.columns.count == 5, got 5
PASS. Expected samples.csv csv.lines.count >= 2, got 101
PASS. Expected samples.csv csv.line.2.column.1 starts_with SAMPLE_, got SAMPLE_001
PASS. Expected output.bam bam.header.rg.count >= 1, got 2
PASS. Expected output.bam bam.header.rg.0.sm == NA12878, got NA12878
PASS. Expected output.bam bam.header.rg.0.pl == ILLUMINA, got ILLUMINA
PASS. Expected output.bam bam.header.hd.so == coordinate, got coordinate
```

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
    container "ghcr.io/peterkneale/bioassert:1.3.1"

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

## Syntax

```
<file> <metric> <comparator> <value>
```

## Metrics

### File metrics

| Metric        | Description                    | Comparators                          | Value   |
|---------------|--------------------------------|--------------------------------------|---------|
| `file.exists` | Whether the file exists        | `eq`, `ne`                           | boolean |
| `file.empty`  | Whether the file is zero bytes | `eq`, `ne`                           | boolean |
| `file.size`   | File size                      | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | size    |
| `file.lines`  | Line count                     | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |

### Delimited file metrics (CSV, TSV, PSV)

| Metric                | Description                                     |
|-----------------------|-------------------------------------------------|
| `csv.columns.count`   | Number of columns in the first (header) row     |
| `csv.lines.count`     | Number of lines in the file                     |
| `csv.line.N.column.M` | Content of cell at line N, column M (1-indexed) |

Replace `csv` with `tsv` (tab-separated) or `psv` (pipe-separated) for those formats.

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
| count   | `10`, `1K`, `2M`                 |
| string  | `Alice`, `"New York"`, `'hello'` |
