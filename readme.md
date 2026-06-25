# getinbed

Fast batch converter for genomic interval files into clean, normalized BED.

Handles the messy reality of interval files: mixed formats, inconsistent chromosome naming, metadata headers, coordinate system differences. You hand it a pile of files; it hands back predictable BED.

See [`docs/design.md`](docs/design.md) for full format and architecture details.

## Supported formats

Plain text (`.gz` input supported): BED3–12, narrowPeak, broadPeak, BEDGraph, GFF3, GTF, VCF, genePred, PSL

Binary: bigBed, bigNarrowPeak, bigBroadPeak, bigGenePred, bigPsl, bigBarChart

Tabix-indexed: vcfTabix (`.vcf.gz` + `.tbi`)

Format is auto-detected by extension; override with `--format`.

## Getting started

### CLI

```sh
cargo build --release
# or: just run -- --help
```

```sh
getinbed *.bed *.narrowPeak -o out/
```

All output is written as `{stem}.bed` in the target directory. Files are processed in parallel.

```
Options:
  -o, --out <DIR>            Output directory [default: same as input]
  -f, --format <FMT>         Force input format (bed|narrowpeak|broadpeak|bedgraph|gff3|gtf|vcf|...)
  --extra-columns <COLS>     0-indexed source columns to append after chrom/start/end
  --blacklist <FILE>         BED file of regions to subtract
  --split-on <COL>           Split output into one file per unique value in this column
  --chroms <CHR,...>         Keep only these chromosome names
  --keep-nonstandard         Keep non-standard contigs (default: drop them)
  --no-clean                 Skip deduplication and malformed-row removal
  --no-sort                  Skip sorting
  -j, --jobs <N>             Parallel workers [default: num CPUs]
  -q, --quiet                Suppress progress output
```

### Elixir / NIF

The Rust engine ships as a Hex package with precompiled NIFs — no Rust toolchain required. It runs on a dirty scheduler thread and does not block the BEAM.

```elixir
# mix.exs
{:getinbed, "~> 0.1"}
```

```elixir
{:ok, out_paths} = GetInBed.to_bed(
  ["/data/peaks.narrowPeak", "/data/regions.bed.gz"],
  out: "/data/out",
  extra_columns: [3, 5],
  blacklist: "/ref/hg38-blacklist.bed",
  chroms: ~w[chr1 chr2 chr3 chrX chrY chrM]
)
```

Options mirror the CLI flags (snake_case atoms): `:out`, `:format`, `:extra_columns`, `:blacklist`, `:split_on`, `:chroms`, `:no_clean`, `:no_sort`, `:jobs`.

Returns `{:ok, [output_path]}` or `{:error, reason}`.

## What it does to your data

Operations run in a fixed order regardless of flag sequence:

1. **Parse** — decode format, strip metadata headers, normalize chromosome names (`1` → `chr1`, `MT` → `chrM`)
2. **Clean** — drop malformed rows (start ≥ end, negative coords, missing fields); deduplicate exact triples
3. **Select** — append any requested extra columns after chrom/start/end
4. **Blacklist** — subtract intervals overlapping the blacklist
5. **Split** — partition output by column value (one file per unique value)
6. **Sort** — karyotypic order (`chr1 < chr2 < ... < chrX < chrY < chrM`), then by start/end

All coordinates are normalized to **0-based, half-open** intervals. GFF3/GTF and VCF are converted on parse.

## Development

```sh
just test          # Rust tests
just test-elixir   # Elixir NIF tests
just bench         # benchmarks
```

Nix shell (`nix develop`) provides Rust, Elixir, htslib, bedtools, and liftOver.
