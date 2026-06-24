# getinbed — Design

**getinbed** ("get in BED") is a fast batch processor for converting genomic interval files into a clean, normalized BED format for downstream pipeline use.

## Problem

Genomic interval files arrive in many formats (BED3–12, narrowPeak, broadPeak, BEDGraph, GFF3, GTF) with inconsistent chromosome naming, arbitrary extra columns, embedded metadata headers, and coordinate systems. The rest of the pipeline needs a predictable, clean BED representation.

This tool is intentionally narrow: it does not interpret biology. It forces files into shape.

## Supported Input Formats

### Plain-text formats

| Format | Description |
|---|---|
| BED3–BED12 | Standard UCSC BED with 3–12 columns |
| narrowPeak | BED6+4 (ENCODE peak format) |
| broadPeak | BED6+3 |
| BEDGraph | chrom, start, end, value |
| genePred | UCSC gene prediction (1-based; txStart/txEnd extracted as interval) |
| PSL | UCSC alignment format (target coordinates used as interval) |
| GFF3 | Tab-delimited, 1-based coordinates |
| GTF | Ensembl gene transfer format |
| VCF | Variant call format (POS converted to 0-based half-open) |

Plain-text formats support `.gz` (gzip) input. Metadata lines are stripped unconditionally: `#` comments, `track` lines, `browser` lines, and GFF pragma lines (`##`).

### Binary UCSC big* formats

| Format | Underlying text format |
|---|---|
| bigBed | BED |
| bigNarrowPeak | narrowPeak |
| bigBroadPeak | broadPeak |
| bigGenePred | genePred |
| bigPsl | PSL |
| bigBarChart | Interval only (bar value arrays are discarded) |

big* files are binary-indexed (B+ tree + R-tree). They are decoded to their underlying text representation in memory using the `bigtools` crate, then fed through the same parse pipeline as their plain-text equivalents. Random-access fetching by region is not used; the full file is streamed for consistency with batch processing semantics.

### Tabix-indexed formats

| Format | Description |
|---|---|
| vcfTabix | bgzf-compressed VCF with `.tbi` index |

vcfTabix files are read by streaming the full bgzf payload via `noodles` (`bgzf` + `vcf` features). The tabix index is not used (full scan for consistency). The `.tbi` file must be co-located with the `.vcf.gz` file but is otherwise ignored.

### Format detection

Format is auto-detected by file extension, then confirmed by header sniffing if ambiguous. Explicit override: `--format <fmt>`. Recognised extensions:

```
.bed .bed.gz
.narrowPeak .narrowPeak.gz
.broadPeak .broadPeak.gz
.bedGraph .bedGraph.gz
.gff .gff3 .gff.gz .gff3.gz
.gtf .gtf.gz
.vcf .vcf.gz
.bigBed .bb
.bigNarrowPeak
.bigBroadPeak
.bigGenePred
.bigPsl
.bigBarChart
```

## Coordinate Systems

All formats are normalised to **0-based, half-open** intervals before any operation runs.

| Format | Source system | Conversion |
|---|---|---|
| BED, narrowPeak, broadPeak, BEDGraph | 0-based half-open | none |
| genePred, PSL | 0-based half-open | none |
| GFF3, GTF | 1-based closed | `start -= 1` (end unchanged) |
| VCF, vcfTabix | 1-based closed | `start = POS - 1`, `end = POS - 1 + len(REF)` |
| all big* | same as underlying text format | per above |

## Output Format

All output is **BED** (tab-delimited, 0-based half-open intervals):

```
chrom  chromStart  chromEnd  [extra columns...]
```

Chromosome names are normalized: `1` → `chr1`, `MT` → `chrM`, etc. Only standard chromosomes are emitted by default (`chr1–22`, `chrX`, `chrY`, `chrM`); unrecognized contigs are dropped unless `--keep-nonstandard` is passed.

## Operations

Operations are applied in this fixed order regardless of flag order:

1. **Parse** — decode input format, strip metadata, normalize chromosome names
2. **Clean** — drop malformed rows (start ≥ end, negative coordinates, missing required fields); deduplicate exact-coordinate duplicates
3. **Extra column select** — append any requested extra columns after chrom/start/end (default: none)
4. **Blacklist subtract** — remove intervals that overlap any blacklist interval
5. **Split** — emit one output file per unique value in the specified column
6. **Sort** — sort by (chrom, start, end) using natural chromosome order

Operations are idempotent; applying them to already-clean input is safe.

### Clean

- Drops rows where `start >= end`
- Drops rows with negative coordinates
- Drops rows with missing required fields (chrom, start, end)
- Deduplicates exact (chrom, start, end) triples; first occurrence wins

Malformed rows are skipped, never fatal. A count of skipped rows is reported per file at the end of processing (suppressed with `--quiet`).

### Extra Column Select (`--extra-columns`)

Comma-separated 0-indexed column indices **in the source file's original column order**. The output always begins with chrom, start, end; `--extra-columns` appends columns after those three.

Example — a BED6 file where column 3 is name and column 5 is strand:
```
--extra-columns 3,5   →   chrom  start  end  name  strand
```

If an index that corresponds to the source chrom, start, or end column is included, it is silently ignored (those are always emitted and cannot be duplicated). Column indices are validated against the source column count; out-of-range indices are an error.

`--split-on` uses the same source-file indexing.

### Blacklist Subtract (`--blacklist`)

Removes any input interval that intersects a blacklist interval. The caller must supply a BED file (e.g. Boyle Lab hg38 ENCODE v2 blacklist); there is no bundled default.

Intersection is defined as any overlap (not just containment): `input.start < bl.end && input.end > bl.start`.

Internally, the blacklist is loaded into a per-chromosome sorted interval array. Each query does a binary search to find candidates and a linear scan over overlapping intervals.

### Split by Column (`--split-on`)

Partitions output by the unique values in a given column (0-indexed). Produces one output file per unique value; filenames are `{stem}.{value}.bed`. Useful for splitting chromatin state files where column 3 holds state labels.

### Sort

Chromosomes are sorted in karyotypic order: `chr1 < chr2 < ... < chr22 < chrX < chrY < chrM < (other)`. Within a chromosome, intervals are sorted by start then end. Sorting uses `voracious_radix_sort` on a packed `u64` key `(chrom_index << 34) | (start << 2) | end_tag`.

## Batch Processing

The primary use case is batch invocation from an Elixir NIF: a list of source files is supplied, operations are applied to all of them, and output files are written to a specified directory.

Each file in the batch is processed independently. Files are processed in parallel using `rayon`; output order matches input order. The batch size is not bounded in the API, but callers should keep batches under ~10k files to avoid excessive memory pressure during sort.

### CLI

```
getinbed [OPTIONS] <FILE>...

Arguments:
  <FILE>...  Input files (BED, narrowPeak, broadPeak, BEDGraph, GFF3, GTF; .gz ok)

Options:
  -o, --out <DIR>          Output directory [default: same as input file]
  -f, --format <FMT>       Force input format (bed|narrowpeak|broadpeak|bedgraph|gff3|gtf)
  --extra-columns <COLS>   Comma-separated 0-indexed source columns to append after chrom/start/end
  --blacklist <FILE>       BED file of regions to subtract (e.g. Boyle hg38 ENCODE v2)
  --split-on <COL>         0-indexed column to split output on
  --keep-nonstandard       Keep non-standard contigs instead of dropping them
  --no-clean               Skip deduplication and malformed-row removal
  --no-sort                Skip sorting
  -j, --jobs <N>           Parallel workers [default: num CPUs]
  -q, --quiet              Suppress progress output
```

Output files: `{stem}.bed` (or `{stem}.{value}.bed` when splitting). If the input is gzipped, the stem is the filename without `.gz`.

### NIF Interface

The Elixir NIF wraps the same batch engine. It accepts:

```elixir
Getinbed.process(files :: [binary()], opts :: keyword()) :: {:ok, [binary()]} | {:error, binary()}
```

- `files` — list of absolute paths to input files
- `opts` — keyword list mirroring CLI options (`:extra_columns`, `:blacklist`, `:split_on`, `:out`, `:no_sort`, etc.)
- Returns list of output file paths in the same order as input, or `{:error, reason}` on failure

The NIF is built with Rustler. It runs the Rust batch engine on a dirty scheduler thread to avoid blocking the BEAM.

## Internal Architecture

A single crate exposes three entry points: a Rust library API, a CLI binary, and a Rustler NIF. All three drive the same `batch.rs` engine.

```
src/
  main.rs          CLI binary (clap)
  lib.rs           Public Rust API + Rustler NIF (feature-gated)
  batch.rs         Batch driver (rayon parallel map); core of all three entry points
  detect.rs        Format detection (extension + header sniff)
  parse/
    bed.rs
    narrowpeak.rs
    broadpeak.rs
    bedgraph.rs
    genepred.rs
    psl.rs
    gff.rs
    vcf.rs       (plain VCF + vcfTabix via noodles bgzf+vcf features)
    big.rs       (all big* formats via bigtools; delegates to underlying parsers)

  ops/
    clean.rs
    select.rs    (extra-columns: source-indexed, appended after chrom/start/end)
    blacklist.rs
    split.rs
    sort.rs
  chrom.rs         Chromosome name normalization + karyotypic ordering
  error.rs
```

### Rust library API

```rust
pub fn process(files: &[PathBuf], opts: &Opts) -> Result<Vec<PathBuf>, GetinbedError>
```

`Opts` is a plain struct mirroring CLI options. Callers that don't want files written to disk can use the lower-level `process_one(path, opts) -> Result<Vec<Interval>>` to get records in memory.

### NIF

`lib.rs` is conditionally compiled with `#[cfg(feature = "nif")]`. The NIF layer is a thin wrapper over `process()` that marshals Elixir terms to/from `Opts` and runs on a dirty scheduler thread.

```elixir
Getinbed.process(files :: [binary()], opts :: keyword()) :: {:ok, [binary()]} | {:error, binary()}
```

`opts` keys mirror the Rust `Opts` struct fields (`:extra_columns`, `:blacklist`, `:split_on`, `:out`, `:no_sort`, etc.).

Files are read via `memmap2` for zero-copy I/O. Gzip files are streamed line-by-line through `flate2`.

### Key dependencies

| Crate | Purpose |
|---|---|
| `bigtools` | bigBed / bigNarrowPeak / bigBroadPeak / bigGenePred / bigPsl / bigBarChart decoding |
| `noodles` (bed, gff, gtf, vcf, bgzf, tabix features) | Parsers for BED, GFF3, GTF, VCF, vcfTabix; replaces hand-rolled parsers for those formats |
| `memmap2` | Zero-copy file I/O for plain-text formats |
| `flate2` | Gzip streaming for `.gz` plain-text inputs |
| `voracious_radix_sort` | Fast radix sort for the sort operation |
| `rayon` | Parallel batch processing |
| `clap` | CLI argument parsing |
| `thiserror` | Error types |

## Non-Goals

- Liftover between genome assemblies
- Merging overlapping intervals (use `bedtools merge`)
- Output formats other than BED
- BAM/SAM/CRAM alignment files
- BCF (binary VCF)
- Coordinate validation against a reference genome
