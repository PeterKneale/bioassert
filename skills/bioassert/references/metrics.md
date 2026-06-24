# bioassert metrics reference

Full reference for every metric, comparator and value format. Use it to author assertions
and to decode `ERROR.` lines (an error almost always means a wrong metric name, a comparator
the metric does not accept, or an unquoted value).

## Assertion shape

```
<resource> <metric> <comparator> <value>  [if|unless <resource> <metric> <comparator> <value>]
```

The first segment of the metric (`file`, `csv`/`tsv`/`psv`, `bam`, `fasta`, `text`) names the
resource type. The resource locator is read as a **file path** for every family except
`text.*`, which reads it as an inline literal.

## File metrics (`file.*`)

| Metric             | Description                    | Comparators                          | Value   |
|--------------------|--------------------------------|--------------------------------------|---------|
| `file.exists`      | Whether the file exists        | `eq`, `ne`                           | boolean |
| `file.empty`       | Whether the file is zero bytes | `eq`, `ne`                           | boolean |
| `file.size`        | File size in bytes             | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | size    |
| `file.lines`       | Line count                     | `eq`, `ne`, `lt`, `lte`, `gt`, `gte` | count   |
| `file.compressed`  | Whether the file is compressed | `eq`, `ne`                           | boolean |
| `file.compression` | Compression kind (label)       | `eq`, `ne`, `starts`, `ends`, `contains`, `matches` | string |

```text
output.bam    file.exists       eq   true
results.vcf   file.empty        eq   false
output.bam    file.size         gte  1MB
results.tsv   file.lines        gte  1
reads.fastq.gz   file.compressed   eq   true
out.vcf.gz       file.compression  eq   bgzf
ref.fasta        file.compression  eq   none
```

`file.compression` reports one of `none`, `gzip`, `bgzf`, `bzip2`, `xz`, `zstd`, `zip`.
Detection reads only the leading magic bytes (at most 18) and never decompresses, so it is
cheap even on multi-gigabyte genomes. `bgzf` (the block-gzip variant used by samtools and
tabix) is reported in preference to `gzip` when its `BC` marker is present, since every bgzf
file is also a valid gzip file. This makes `file.compression eq bgzf` the right check before
an indexing step, and a natural guard: `if reads.gz file.compression eq bgzf`.

## Delimited metrics (`csv.*`, `tsv.*`, `psv.*`)

`csv` is comma, `tsv` is tab, `psv` is pipe. Lines and columns are **1-indexed**.

| Metric                  | Description                                              |
|-------------------------|----------------------------------------------------------|
| `csv.columns.count`     | Number of columns in the first (header) row              |
| `csv.lines.count`       | Number of lines in the file                              |
| `csv.line.N.column.M`   | Content of the cell at line N, column M                  |
| `csv.column.N.all`      | Holds for **every** cell in column N, header included    |
| `csv.column.N.data.all` | Holds for every cell in column N, **skipping** the header (line 1) |

- `*.columns.count` and `*.lines.count` take numeric comparators (`eq`, `ne`, `lt`, `lte`,
  `gt`, `gte`) against a count value.
- `*.line.N.column.M` and the `*.column.N.*` whole-column metrics take string comparators
  (`eq`, `ne`, `starts`, `ends`, `contains`, `matches`).
- A whole-column metric passes only when the comparison holds for every checked cell.
  On failure the report names the first offending row (`got line 3 = "NDA"`). A header-only
  or empty file passes vacuously.

```text
samples.csv   csv.columns.count       eq   3
counts.tsv    tsv.lines.count         gt   10
report.psv    psv.line.2.column.1     eq   Alice
samples.csv   csv.line.2.column.3     starts  New
results.tsv   tsv.line.2.column.2     matches '^[0-9]+$'
junctions.tsv tsv.column.6.data.all   matches '^[+-]$'
junctions.tsv tsv.column.4.all        matches '^JUNC[0-9]+$'
```

## BAM header metrics (`bam.header.*`)

All metrics read the SAM header only (header-only files work). Read groups (`@RG`) are
addressed by **0-based** index `N` in header order. `<tag>` is a lowercased 2-letter SAM tag.

| Metric                          | Description                                  | Comparators | Value   |
|---------------------------------|----------------------------------------------|-------------|---------|
| `bam.header.rg.count`           | Number of `@RG` read groups                  | numeric     | count   |
| `bam.header.sq.count`           | Number of `@SQ` reference sequences          | numeric     | count   |
| `bam.header.pg.count`           | Number of `@PG` program records              | numeric     | count   |
| `bam.header.rg.N.<tag>`         | Tag value of read group N (`id`, `sm`, `lb`, `pl`, `pu`, ...) | string | string |
| `bam.header.rg.N.present`       | Whether read group N exists                  | `eq`, `ne`  | boolean |
| `bam.header.rg.N.<tag>.present` | Whether `<tag>` is set on read group N       | `eq`, `ne`  | boolean |
| `bam.header.hd.vn`              | `@HD` format version (VN)                    | string      | string  |
| `bam.header.hd.so`              | `@HD` sort order (SO), e.g. `coordinate`     | string      | string  |

- numeric is `eq`, `ne`, `lt`, `lte`, `gt`, `gte`. string is `eq`, `ne`, `starts`, `ends`,
  `contains`, `matches`.
- `id` resolves to the `@RG ID` field. Other tags resolve to the read group's other fields.
- Reading a tag or `@HD` field that is **not set**, or a read group index **out of range**,
  is an **ERROR**. Use the `.present` form to test presence without erroring.

```text
output.bam   bam.header.rg.count        gte  1
output.bam   bam.header.rg.0.sm         eq   NA12878
output.bam   bam.header.rg.0.pl         eq   ILLUMINA
output.bam   bam.header.rg.0.lb         eq   'Solexa-272222'    # quoted: dash
output.bam   bam.header.rg.1.id         eq   'H0164.2'          # quoted: dot
output.bam   bam.header.rg.0.pu.present eq   true
output.bam   bam.header.hd.so           eq   coordinate
output.bam   bam.header.sq.count        eq   1
```

## FASTA metrics (`fasta.*`)

Per-record metrics live under `fasta.seq.*`. `fasta.length` is the whole-file total. Records
are addressed by **0-based** index `N` in file order.

| Metric                            | Description                                       | Comparators | Value   |
|-----------------------------------|---------------------------------------------------|-------------|---------|
| `fasta.seq.count`                 | Number of sequence records                        | numeric     | count   |
| `fasta.length`                    | Total bases summed across all records             | numeric     | count   |
| `fasta.seq.N.name`                | Name (ID) of record N (first header token)        | string      | string  |
| `fasta.seq.N.description`         | Description of record N (header text after name)  | string      | string  |
| `fasta.seq.N.length`              | Length in bases of record N's sequence            | numeric     | count   |
| `fasta.seq.N.present`             | Whether a record exists at index N                | `eq`, `ne`  | boolean |
| `fasta.seq.N.description.present` | Whether record N has a non-empty description      | `eq`, `ne`  | boolean |

- `name` is always present. `description` is optional. Reading name/description/length of an
  out-of-range index is an **ERROR**, so use `.present` to test first.

```text
ref.fasta   fasta.seq.count          eq   25
ref.fasta   fasta.length             gte  3000000000
ref.fasta   fasta.seq.0.name         eq   chr1
ref.fasta   fasta.seq.0.length       gte  20000000
ref.fasta   fasta.seq.0.description  eq   'Homo sapiens chromosome 1'   # quoted: spaces
ref.fasta   fasta.seq.0.name         matches '^chr[0-9XYM]+$'
ref.fasta   fasta.seq.24.present     eq   true
```

## Text metrics (`text.*`)

The resource is an **inline literal**, not a path. There is no I/O, so these never ERROR.
They only PASS or FAIL, which makes them mainly useful as a guard input.

| Metric        | Description                              | Comparators | Value  |
|---------------|------------------------------------------|-------------|--------|
| `text.value`  | The literal compared as a string         | string      | string |
| `text.length` | Its character count (Unicode scalars)    | numeric     | count  |

```text
'NC_000001.11'              text.value   matches '^NC_'
'Homo sapiens chromosome 1' text.value   contains sapiens
'abc'                       text.length  eq   3
```

## Comparators

| Comparator | Meaning          | Use with    |
|------------|------------------|-------------|
| `eq`       | equal            | any         |
| `ne`       | not equal        | any         |
| `lt`       | less than        | size, count |
| `lte`      | less or equal    | size, count |
| `gt`       | greater than     | size, count |
| `gte`      | greater or equal | size, count |
| `starts`   | starts with      | string      |
| `ends`     | ends with        | string      |
| `contains` | contains         | string      |
| `matches`  | regex match      | string      |

## Values and quoting

| Type    | Examples                              |
|---------|---------------------------------------|
| boolean | `true`, `false`                       |
| size    | `5B`, `1KB`, `2MB`, `1GB`, `1TB` (binary 1024-based, for `file.size`) |
| count   | `0`, `10`, `1000`, `5K`, `5M`, `5G` (decimal: `K`=1e3, `M`=1e6, `G`=1e9) |
| string  | `Alice`, `"New York"`, `'hello'`      |

**Quoting rule (the #1 source of ERROR lines):** a bare value must be alphanumeric. Any value
containing **dots, dashes, colons or spaces** must be single- or double-quoted:
`'H0164.2'`, `'1.6'`, `'Solexa-272222'`, `'NC_000001.11'`, `'Homo sapiens chromosome 1'`.
The same applies to the **resource locator** when it contains those characters (quote the
whole path), which is why interpolated Nextflow paths should be wrapped as `'${bam}'`.

Size units (`KB`/`MB`/`GB`, 1024-based) are distinct from count units (`K`/`M`/`G`,
1000-based). `file.size gt 1MB` means 1,048,576 bytes. `fasta.length gt 5M` means 5,000,000.

## Reading the output

Each assertion prints one line:

```text
PASS. Expected output.bam file.size > 1MB, got 5.00MB
FAIL. Expected results.vcf file.lines > 999, got 0
ERROR. unknown metric: file.explode
SKIP. ...   # a guarded assertion whose condition was not satisfied
```

`PASS`/`FAIL`/`SKIP` go to stdout, `ERROR` to stderr. The same lines (no color, no icons)
land in the report file for capture as a pipeline artifact.
