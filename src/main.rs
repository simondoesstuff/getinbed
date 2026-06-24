use clap::Parser;
use getinbed::{parse_format_str, process_batch, GetinbedError, Opts};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "getinbed", about = "Batch converter to normalized BED format")]
struct Cli {
    /// Input files (BED, narrowPeak, broadPeak, BEDGraph, GFF3, GTF, VCF, big*; .gz ok)
    #[arg(required = true, value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Output directory [default: same as input file]
    #[arg(short = 'o', long, value_name = "DIR")]
    out: Option<PathBuf>,

    /// Force input format (bed|narrowpeak|broadpeak|bedgraph|gff3|gtf|vcf|genepred|psl|bigbed|...)
    #[arg(short = 'f', long, value_name = "FMT")]
    format: Option<String>,

    /// Comma-separated 0-indexed source columns to append after chrom/start/end
    #[arg(long, value_name = "COLS")]
    extra_columns: Option<String>,

    /// BED file of regions to subtract (e.g. Boyle hg38 ENCODE v2 blacklist)
    #[arg(long, value_name = "FILE")]
    blacklist: Option<PathBuf>,

    /// 0-indexed source column to split output on
    #[arg(long, value_name = "COL")]
    split_on: Option<usize>,

    /// Keep non-standard contigs instead of dropping them
    #[arg(long)]
    keep_nonstandard: bool,

    /// Skip deduplication and malformed-row removal
    #[arg(long)]
    no_clean: bool,

    /// Skip sorting
    #[arg(long)]
    no_sort: bool,

    /// Parallel workers [default: num CPUs]
    #[arg(short = 'j', long, value_name = "N")]
    jobs: Option<usize>,

    /// Output format: bed (default) | tsv
    #[arg(long, default_value = "bed", value_name = "FMT")]
    output_format: String,

    /// Suppress per-file skip counts
    #[arg(short = 'q', long)]
    quiet: bool,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), GetinbedError> {
    let cli = Cli::parse();

    let format = cli.format.as_deref().map(parse_format_str).transpose()?;

    let extra_columns = cli
        .extra_columns
        .as_deref()
        .map(parse_extra_columns)
        .transpose()?
        .unwrap_or_default();

    let opts = Opts {
        out: cli.out,
        format,
        extra_columns,
        blacklist: cli.blacklist,
        split_on: cli.split_on,
        keep_nonstandard: cli.keep_nonstandard,
        no_clean: cli.no_clean,
        no_sort: cli.no_sort,
        jobs: cli.jobs,
        quiet: cli.quiet,
    };

    let results = process_batch(&cli.files, &opts)?;

    for (path, result) in cli.files.iter().zip(results.iter()) {
        if !opts.quiet && result.skipped > 0 {
            eprintln!(
                "{}: skipped {} malformed/duplicate rows",
                path.display(),
                result.skipped
            );
        }
        for out in &result.outputs {
            println!("{}", out.display());
        }
    }

    Ok(())
}

fn parse_extra_columns(s: &str) -> Result<Vec<usize>, GetinbedError> {
    s.split(',')
        .map(|tok| {
            tok.trim()
                .parse::<usize>()
                .map_err(|_| GetinbedError::Parse(format!("invalid column index: {tok}")))
        })
        .collect()
}
