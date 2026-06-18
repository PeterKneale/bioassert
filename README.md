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
docker pull cmri/bioassert
docker run --rm -v "$PWD":/data cmri/bioassert assert "/data/output.bam file.exists eq true"
```

Mount your working directory to `/data` and use that path prefix in your assertions. To run an assertions file:

```bash
docker run --rm -v "$PWD":/data cmri/bioassert run /data/checks.txt
```

## Examples

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
```

```bash
bioassert run checks.txt
```

Output:

```
Running assertions in checks.txt
PASS. Expected output.bam file.exists == true, got true
PASS. Expected results.vcf file.exists == true, got true
PASS. Expected output.bam file.size > 1048576, got 5242880
PASS. Expected results.vcf file.empty == false, got false
PASS. Expected results.vcf file.lines >= 1, got 42
PASS. Expected samples.csv csv.columns.count == 5, got 5
PASS. Expected samples.csv csv.lines.count >= 2, got 101
PASS. Expected samples.csv csv.line.2.column.1 starts_with SAMPLE_, got SAMPLE_001
```

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

### Comparators

| Comparator | Meaning       | Use with      |
|------------|---------------|---------------|
| `eq`       | equal         | any           |
| `ne`       | not equal     | any           |
| `lt`       | less than     | size, count   |
| `lte`      | less or equal | size, count   |
| `gt`       | greater than  | size, count   |
| `gte`      | >=            | size, count   |
| `starts`   | starts with   | string (cell) |
| `ends`     | ends with     | string (cell) |
| `contains` | contains      | string (cell) |
| `matches`  | regex match   | string (cell) |

### Values

| Type    | Examples                         |
|---------|----------------------------------|
| boolean | `true`, `false`                  |
| size    | `5B`, `1KB`, `2MB`, `1GB`        |
| count   | `10`, `1K`, `2M`                 |
| string  | `Alice`, `"New York"`, `'hello'` |
