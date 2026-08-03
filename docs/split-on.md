# Split by Column (`--split-on`)

Partitions each input file into multiple output files based on the unique values in a chosen column.

## What it does

Without `--split-on`, each input file produces exactly one output file:

```
sample.narrowPeak  →  sample.bed
```

With `--split-on`, each input file produces **one output file per unique value** found in the specified column:

```
sample.narrowPeak  →  sample.E1.bed
                       sample.E2.bed
                       sample.Quiescent.bed
                       ...
```

Output files are written in alphabetical order by value. Each file contains only the records whose split column matched that value, fully cleaned and sorted the same as a normal run.

## Column indexing

The column number is **0-indexed** and refers to the **source file's original columns**, before any transformation. This is the same indexing scheme used by `--extra-columns`.

Example — a narrowPeak file has these columns at indices 0–9:

```
0:chrom  1:start  2:end  3:name  4:score  5:strand  6:signalValue  7:pValue  8:qValue  9:peak
```

`--split-on 3` splits on the `name` column.

## Output filenames

```
{stem}.{value}.bed
```

Where `stem` is the input filename without extension (and without `.gz` if gzipped). Characters in the value that are not alphanumeric, `-`, `_`, or `.` are replaced with `_`.

Examples:

| Value in column | Output filename |
|---|---|
| `E1` | `sample.E1.bed` |
| `Quiescent` | `sample.Quiescent.bed` |
| `active enhancer` | `sample.active_enhancer.bed` |
| `state/3` | `sample.state_3.bed` |

## Records with a missing value

If a record does not have the split column (i.e. the row is shorter than expected), it is grouped under an empty string key and written to `{stem}..bed`. This typically indicates a malformed row that survived `--no-clean`; under normal operation (cleaning enabled) such rows are dropped before the split runs.

## Error conditions

- If the split column index is out of range for the first record in the file, the run fails with a `ColumnOutOfRange` error. The index is validated against the actual column count of the parsed data.
- An empty input file (zero records after cleaning) produces no output files.

## CLI usage

```
getinbed --split-on 3 sample.narrowPeak -o ./out/
```

Stdout lists all output paths, one per line (one line per split value per input file):

```
./out/sample.E1.bed
./out/sample.E2.bed
./out/sample.Quiescent.bed
```

## Rust API

```rust
use getinbed::{process, Opts};

let opts = Opts {
    split_on: Some(3),
    out: Some(PathBuf::from("./out")),
    ..Default::default()
};

let output_paths = process(&[PathBuf::from("sample.narrowPeak")], &opts)?;
// output_paths contains one PathBuf per split value
```

`process()` returns a flat `Vec<PathBuf>` of all output files across all input files. If you need to know which output files correspond to which input, use `process_batch()` instead — it returns one `ProcessResult` per input, and each `ProcessResult.outputs` contains the paths for that file's split values.

## Elixir NIF

```elixir
{:ok, paths} = GetInBed.to_bed(["sample.narrowPeak"], split_on: 3, out: "/out")
# paths is a flat list of all output file paths
```

## Combining with `--extra-columns`

`--split-on` and `--extra-columns` are independent. The split column does not need to be listed in `--extra-columns` for the split to work — the split reads from the raw source columns regardless. Whether or not the split column appears in the output is controlled solely by `--extra-columns`.

```
# Split on column 3; include column 3 in output too
--split-on 3 --extra-columns 3

# Split on column 3; output only chrom/start/end (column 3 not included)
--split-on 3
```

## Operation order

Split runs **after** clean, blacklist subtract, and sort, so each output file is already clean and sorted. Records are never split before being cleaned — the grouping is applied to the final set of valid records.
