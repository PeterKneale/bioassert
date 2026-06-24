---
name: bioassert
description: >-
  Use when adding bioassert validation to a Nextflow (DSL2) pipeline, or writing
  bioassert assertions for bioinformatics outputs (BAM, FASTA, VCF, CSV/TSV/PSV,
  plain files). Covers the assertion syntax, the Nextflow process pattern (container,
  heredoc, report capture, exit-code gating, version reporting), and which checks to
  write for each file type. Triggers on requests like "validate my pipeline outputs",
  "add a bioassert step", "check this BAM/FASTA/sample sheet in Nextflow", or
  "gate this process on bioassert".
---

# bioassert in Nextflow pipelines

`bioassert` is a single-binary CLI that asserts properties of pipeline outputs using a
declarative one-line-per-check syntax, then exits non-zero if any check fails. That exit
code is what makes it a clean validation gate inside a Nextflow process.

This skill helps you write correct assertions and wire them into a Nextflow DSL2 process.
Read the two reference files when you need detail:

- `references/metrics.md` for every metric, comparator, value format and quoting rule.
- `references/nextflow.md` for copy-paste process templates (gate, advisory, nf-core module).

## The assertion model in 30 seconds

One assertion per line: `<resource> <metric> <comparator> <value>`

```text
output.bam   file.exists           eq   true
output.bam   file.size             gt   1MB
output.bam   bam.header.hd.so      eq   coordinate
ref.fasta    fasta.seq.count       eq   25
samples.tsv  tsv.columns.count     eq   8
```

- The **resource** (first token) is a file path for the `file.*`, `tsv/csv/psv.*`,
  `bam.*` and `fasta.*` families. (`text.*` treats it as an inline literal instead.)
- Lines starting with `#` are comments, blank lines are ignored, and column whitespace is free.
- An optional **guard** runs a check only when a condition holds (see Guards below).

Exit codes are the gate:

| Exit | Meaning |
|------|---------|
| `0`  | every assertion passed |
| `1`  | at least one assertion **failed** |
| `2`  | at least one assertion **errored** (bad syntax, unknown metric, unreadable file) |

`run` writes one `PASS.`/`FAIL.`/`ERROR.` line per assertion to stdout/stderr and also to a
plain-text report file. Use `--report-file <name>.log` to fix the name so Nextflow can
capture it.

## The core Nextflow pattern

Write the assertions into a file inside the task script, then `run` them. A failing or
erroring assertion exits non-zero, which fails the task and halts the pipeline.

```groovy
process VALIDATE_ALIGNMENT {
    tag "$meta.id"
    container "ghcr.io/peterkneale/bioassert:3.0.0"   // pin to a released version

    input:
    tuple val(meta), path(bam)

    output:
    tuple val(meta), path("*.log"), emit: report

    script:
    """
    cat <<EOF > assertions.txt
    '${bam}' file.exists           eq   true
    '${bam}' file.size             gt   1MB
    '${bam}' bam.header.rg.count   gte  1
    '${bam}' bam.header.hd.so      eq   coordinate
    EOF

    bioassert \\
        --report-file ${meta.id}.bioassert.log \\
        run assertions.txt
    """
}
```

## Rules that bite in Nextflow (read these)

1. **Quote the interpolated path: `'${bam}'`.** Nextflow substitutes `${bam}` with the
   staged basename when it renders the script. Sample-derived filenames often contain
   dashes, dots or spaces (`sample-1.markdup.bam`), and bioassert's grammar needs values
   with dashes/colons/spaces quoted. Wrapping the path in single quotes is always safe,
   because bioassert strips the quotes centrally, so quote it by default rather than
   reasoning about each filename.

2. **Staged inputs are basenames.** Nextflow symlinks each `path()` input into the task
   work dir, so the assertion references just the filename (`${bam}` becomes `aln.bam`),
   never an absolute host path.

3. **Global flags go before `run`.** `--report-file`, `--color` and `--icons` are global:
   `bioassert --report-file out.log run assertions.txt`. The assertions file is `run`'s
   positional argument.

4. **Capture the report as an output.** `--report-file ${meta.id}.bioassert.log` gives a
   predictable name. Declare `path("*.log")` in `output:` and/or `publishDir` it so the
   evidence survives even when the task passes.

5. **Don't mask the exit code.** The whole point is that a non-zero exit fails the task. Do
   **not** append `|| true`. For advisory (non-blocking) checks, use Nextflow's
   `errorStrategy 'ignore'` on the process instead, which keeps the report while letting the
   pipeline continue (see `references/nextflow.md`).

6. **Pin the container.** Use a versioned tag (`ghcr.io/peterkneale/bioassert:<version>`),
   not `:latest`, so validation is reproducible. Check available tags on the project's GitHub
   releases / GHCR. `3.0.0` is the current line, so bump it as new versions ship.

## Guards: conditional checks

Append `if`/`unless` plus a full condition (same `resource metric comparator value` shape) to
run a check only when it makes sense. A guard has three outcomes. Satisfied: the check runs
(PASS/FAIL). Not satisfied: **SKIP**, a neutral outcome that does not affect the exit code.
Condition itself errors: ERROR.

```text
# Only check read groups when the BAM actually exists (file.exists never errors)
out.bam  bam.header.rg.count  gte  1  if out.bam file.exists eq true

# Check emptiness only for files that are present
res.vcf  file.empty  eq  false  unless res.vcf file.empty eq true

# Only assert the line count when the file is bgzf-compressed (ready for tabix)
out.vcf.gz  file.size  gt  0B  if out.vcf.gz file.compression eq bgzf
```

`file.exists eq true` is the safe guard for "only if this output was produced", because
`file.exists` returns `false` rather than erroring on a missing file.

## Choosing what to assert (by output type)

- **BAM**: `file.exists`, `file.size gt 0B`, `bam.header.rg.count gte 1`,
  `bam.header.hd.so eq coordinate`, `bam.header.sq.count` matching the reference contig
  count, and sample/platform tags (`bam.header.rg.0.sm`, `.pl`).
- **FASTA reference**: `fasta.seq.count`, `fasta.length` (total bases), and named contigs
  (`fasta.seq.0.name`, `.length`).
- **VCF / generic text outputs**: `file.exists`, `file.empty eq false`, `file.lines gte N`.
- **Sample sheets / count tables (CSV/TSV/PSV)**: `*.columns.count`, `*.lines.count`,
  specific cells (`*.line.N.column.M`), and whole-column regex
  (`*.column.N.data.all matches '...'`).
- **Compression** (any file): `file.compressed eq true` and `file.compression eq <kind>`,
  where `<kind>` is one of `none`, `gzip`, `bgzf`, `bzip2`, `xz`, `zstd`, `zip`. Detection
  reads only the leading magic bytes and never decompresses, so it is cheap on large genomes.
  The classic bioinformatics check is confirming a file is **bgzf** (block-gzip), the variant
  samtools and tabix require: `out.vcf.gz file.compression eq bgzf`. Note every bgzf file is
  also valid gzip, and bioassert reports `bgzf` in preference when its marker is present.

Prefer cheap structural checks (exists, size, counts, compression) as the gate, and add
content checks (`matches`, specific tag values) where a wrong-but-present output is a real
risk. A compression check pairs well as a guard for an indexing step: only index when the
input is genuinely bgzf-compressed (see Guards above and `references/metrics.md`).

## Verifying assertions before committing them

You can dry-run a single assertion at the shell without writing a file:

```bash
bioassert assert "ref.fasta fasta.seq.count eq 25"
```

When you author an assertions file, validate the syntax against a real fixture before wiring
it into the pipeline. An `ERROR.` (exit 2) usually means a quoting or metric-name mistake,
which `references/metrics.md` resolves.

## Installing this skill in a pipeline repo

This skill ships with the bioassert repo under `skills/bioassert/`. Claude Code discovers
skills under `.claude/skills/`, so to activate it copy the directory there:

```bash
# Per-project (recommended): make it available in your pipeline repo
mkdir -p .claude/skills
cp -r path/to/bioassert/skills/bioassert .claude/skills/

# Or globally, for all your projects
cp -r path/to/bioassert/skills/bioassert ~/.claude/skills/
```
