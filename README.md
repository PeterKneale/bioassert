# bioassert

A CLI tool for asserting properties of files using a simple declarative syntax. Useful for validating pipeline outputs
in bioinformatics workflows.

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/bioassert`.

## Usage

### Single assertion

```bash
bioassert assert "<file> <metric> <comparator> <value>"
```

### Run an assertions file

```bash
bioassert run assertions.txt
```

## Assertion syntax

```
<file> <metric> <comparator> <value>
```

| Part         | Options                                                      |
|--------------|--------------------------------------------------------------|
| `file`       | Path to the file (unquoted, single-quoted, or double-quoted) |
| `metric`     | `file.exists`, `file.size`, `file.empty`, `file.lines`       |
| `comparator` | `eq`, `ne`, `lt`, `lte`, `gt`, `gte`                         |
| `value`      | See below                                                    |

### Values

- **Size:** `5B`, `1KB`, `2MB`, `1GB` (case-insensitive)
- **Count:** `10`, `1K`, `2M`
- **Boolean:** `true`, `false`

## Examples

```bash
# Check a file exists
bioassert assert "output.bam file.exists eq true"

# Check file size is at least 1KB
bioassert assert "output.bam file.size gte 1KB"

# Check a file has exactly 100 lines
bioassert assert "results.tsv file.lines eq 100"
```

### Assertions file

Lines starting with `#` are comments. Blank lines are ignored.

```
# Check outputs exist
output.bam file.exists eq true
results.tsv file.exists eq true

# Validate sizes
output.bam file.size gt 1MB
results.tsv file.empty eq false

# Check line counts
results.tsv file.lines gte 1
```

Run it:

```bash
bioassert run checks.txt
```

Output:

```
Running assertions in checks.txt
PASS. Expected output.bam file.exists == true, got true
FAIL. Expected results.tsv file.lines >= 1, got 0
```

## Metrics

| Metric        | Description                     | Value type |
|---------------|---------------------------------|------------|
| `file.exists` | Whether the file exists         | boolean    |
| `file.size`   | File size in bytes              | size       |
| `file.empty`  | Whether the file has zero bytes | boolean    |
| `file.lines`  | Number of lines in the file     | count      |

## Development

```bash
cargo test
cargo run -- assert "tests/empty_file.txt file.exists eq true"
cargo run -- run tests/assertions.txt
```
