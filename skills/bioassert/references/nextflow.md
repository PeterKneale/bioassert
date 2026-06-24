# bioassert Nextflow patterns

Copy-paste DSL2 process templates and the reasoning behind each. All assume a pinned
container (`ghcr.io/peterkneale/bioassert:<version>`). Swap the tag for a released version.

## Pattern A: hard gate (default)

A failing or erroring assertion exits non-zero, which fails the task and halts the run. This
is the normal way to use bioassert: bad output stops the pipeline.

```groovy
process VALIDATE_ALIGNMENT {
    tag "$meta.id"
    label 'process_single'
    container "ghcr.io/peterkneale/bioassert:3.0.0"

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

On failure the task work dir still holds `assertions.txt` and the `.log`, so you can inspect
exactly which line failed.

## Pattern B: advisory (never fails the pipeline)

When you want the verdict recorded and published but the pipeline to continue regardless,
disable exit-on-error around the run, capture the status, then exit 0. This is robust no
matter how Nextflow configures `set -e`, and the report is always emitted. An
`errorStrategy 'ignore'` task does **not** reliably publish its outputs, so prefer this when
you need the report downstream.

```groovy
process CHECK_OUTPUTS_ADVISORY {
    tag "$meta.id"
    container "ghcr.io/peterkneale/bioassert:3.0.0"
    publishDir "${params.outdir}/qc/bioassert", mode: 'copy'

    input:
    tuple val(meta), path(vcf)

    output:
    tuple val(meta), path("*.log"), emit: report

    script:
    """
    cat <<EOF > assertions.txt
    '${vcf}' file.exists   eq   true
    '${vcf}' file.empty    eq   false
    '${vcf}' file.lines    gt   100
    EOF

    set +e
    bioassert --report-file ${meta.id}.bioassert.log run assertions.txt
    rc=\$?
    set -e
    echo "bioassert_exit=\$rc" >> ${meta.id}.bioassert.log
    exit 0
    """
}
```

Note the `\$?` and `\$rc`: the backslash keeps the `$` from being interpolated by Nextflow so
bash expands it at runtime. The recorded `bioassert_exit=` line lets a reviewer or a
downstream step see the verdict without halting the workflow.

## Pattern C: nf-core-style module with version reporting

Mirrors the nf-core module shape: `tag`/`label`, both `conda` and `container`, a published
report, a version captured via a topic channel, and a `stub`. The `eval(...)` output and
`topic:` require a recent Nextflow (topic channels). Drop the versions line if your Nextflow
predates them.

```groovy
process BIOASSERT_VALIDATE {
    tag "$meta.id"
    label 'process_single'

    conda "${moduleDir}/environment.yml"
    container "ghcr.io/peterkneale/bioassert:3.0.0"

    input:
    tuple val(meta), path(bam)
    path(reference)

    output:
    tuple val(meta), path("*.bioassert.log"), emit: report
    tuple val("${task.process}"), val('bioassert'),
        eval('bioassert --version | sed "1!d;s/.* //"'),
        emit: versions, topic: versions

    when:
    task.ext.when == null || task.ext.when

    script:
    def prefix = task.ext.prefix ?: "${meta.id}"
    """
    cat <<EOF > assertions.txt
    '${bam}'       file.exists          eq   true
    '${bam}'       bam.header.rg.count  gte  1
    '${bam}'       bam.header.rg.0.sm   eq   ${meta.id}
    '${reference}' fasta.seq.count      gt   0
    EOF

    bioassert \\
        --report-file ${prefix}.bioassert.log \\
        run assertions.txt
    """

    stub:
    def prefix = task.ext.prefix ?: "${meta.id}"
    """
    echo "PASS. stub" > ${prefix}.bioassert.log
    """
}
```

## Pattern D: committed assertions file instead of a heredoc

When the file names are stable (for example a reference genome always staged as
`genome.fasta`), keep the rules in a version-controlled file and stage it as a `path` input.
This is cleaner than a heredoc for large rule sets and keeps the checks reviewable in source
control. Use the heredoc form (Patterns A to C) when paths are sample-derived and must be
interpolated.

```groovy
process VALIDATE_REFERENCE {
    container "ghcr.io/peterkneale/bioassert:3.0.0"

    input:
    path(genome)              // staged as e.g. genome.fasta
    path(rules)               // committed reference.assertions.txt, references "genome.fasta"

    output:
    path("reference.bioassert.log"), emit: report

    script:
    """
    bioassert --report-file reference.bioassert.log run ${rules}
    """
}
```

## Wiring it into a workflow

```groovy
workflow {
    BWA_MEM ( reads_ch, index )
    VALIDATE_ALIGNMENT ( BWA_MEM.out.bam )       // halts the run if a BAM is bad
    MARK_DUPLICATES ( BWA_MEM.out.bam )          // proceeds in parallel
}
```

Because Pattern A fails the task on a bad output, place it on the channel you want to gate.
For an end-of-pipeline QC summary, collect the advisory reports:

```groovy
CHECK_OUTPUTS_ADVISORY.out.report
    .map { meta, log -> log }
    .collectFile(name: 'bioassert_summary.log', storeDir: "${params.outdir}/qc")
```

## Gotchas specific to Nextflow

- **Interpolation versus bash expansion.** Inside a `"""..."""` script block Nextflow
  replaces every `${...}` when it renders the script, before bash runs. So `${bam}` becomes
  the staged filename regardless of the heredoc delimiter. To pass a literal `$` to bash (for
  example `$?`), escape it as `\$`.
- **Quote interpolated paths** as `'${bam}'`. Sample-derived names with dashes/dots/spaces
  would otherwise break bioassert's value grammar. Verified: a single-quoted path with a
  space resolves, and bioassert strips the quotes.
- **Leading indentation is fine.** A plain `<<EOF` heredoc keeps the script block's
  indentation in each line, and bioassert tolerates leading whitespace before the resource,
  so you don't need `<<-EOF` or column-1 lines. Verified.
- **Global flags before the subcommand.** Use `bioassert --report-file x.log run
  assertions.txt`, not `bioassert run assertions.txt --report-file x.log`.
- **Container choice.** The published image is `ghcr.io/peterkneale/bioassert`. Pin a tag.
  For a Conda or `environment.yml`-based module you must provide a channel that ships the
  `bioassert` binary, so the container route is the most portable.
- **Stub runs** should emit a placeholder `.log` so `-stub-run` satisfies the declared output.
- **`set -e` awareness.** Nextflow runs task scripts under bash with errexit, so a failing
  `bioassert` aborts the script immediately (the gate you want). To capture the status for an
  advisory check, bracket it with `set +e` and `set -e` as in Pattern B.
