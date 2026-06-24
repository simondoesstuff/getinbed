use crate::detect;
use crate::error::GetinbedError;
use crate::ops;
use crate::ops::blacklist::BlacklistIndex;
use crate::parse::{self, Format, Record};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashSet;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Opts {
    pub out: Option<PathBuf>,
    pub format: Option<Format>,
    pub extra_columns: Vec<usize>,
    pub blacklist: Option<PathBuf>,
    pub split_on: Option<usize>,
    /// When Some, only records whose chrom is in this set are kept.
    /// When None, all chromosomes pass through unfiltered.
    pub chroms: Option<HashSet<String>>,
    pub no_clean: bool,
    pub no_sort: bool,
    pub jobs: Option<usize>,
    pub quiet: bool,
}

pub struct ProcessResult {
    pub outputs: Vec<PathBuf>,
    pub skipped: usize,
}

pub fn process_batch(
    files: &[PathBuf],
    opts: &Opts,
) -> Result<Vec<ProcessResult>, GetinbedError> {
    if let Some(jobs) = opts.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global()
            .ok();
    }

    let blacklist: Option<Arc<BlacklistIndex>> = opts
        .blacklist
        .as_ref()
        .map(|p| BlacklistIndex::load(p).map(Arc::new))
        .transpose()?;

    let pb = make_progress_bar(files.len(), opts.quiet);

    let results = files
        .par_iter()
        .map(|path| {
            let r = process_one_file(path, opts, blacklist.as_deref());
            if let Some(pb) = &pb {
                pb.inc(1);
            }
            r
        })
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    Ok(results)
}

pub fn process_one_for_memory(path: &Path, opts: &Opts) -> Result<Vec<Record>, GetinbedError> {
    let format = match opts.format {
        Some(f) => f,
        None => detect::detect_format(path)?,
    };

    let mut records = parse_file(path, format)?;

    if let Some(whitelist) = &opts.chroms {
        records.retain(|r| whitelist.contains(&r.chrom));
    }

    if !opts.no_clean {
        let (cleaned, _) = ops::clean::clean(records);
        records = cleaned;
    }

    if let Some(bl_path) = &opts.blacklist {
        let bl = BlacklistIndex::load(bl_path)?;
        records.retain(|r| !bl.overlaps(&r.chrom, r.start, r.end));
    }

    if !opts.no_sort {
        ops::sort::sort(&mut records);
    }

    Ok(records)
}

fn process_one_file(
    path: &Path,
    opts: &Opts,
    blacklist: Option<&BlacklistIndex>,
) -> Result<ProcessResult, GetinbedError> {
    let format = match opts.format {
        Some(f) => f,
        None => detect::detect_format(path)?,
    };

    let mut records = parse_file(path, format)?;

    if let Some(whitelist) = &opts.chroms {
        records.retain(|r| whitelist.contains(&r.chrom));
    }

    let mut skipped = 0;

    if !opts.no_clean {
        let (cleaned, n) = ops::clean::clean(records);
        records = cleaned;
        skipped += n;
    }

    if let Some(bl) = blacklist {
        records.retain(|r| !bl.overlaps(&r.chrom, r.start, r.end));
    }

    if !opts.no_sort {
        ops::sort::sort(&mut records);
    }

    validate_column_indices(&records, &opts.extra_columns, opts.split_on)?;

    let out_dir = opts
        .out
        .as_deref()
        .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")));

    let stem = output_stem(path);

    let outputs = if let Some(split_col) = opts.split_on {
        let groups = ops::split::split_by_column(&records, split_col);
        let mut output_paths = Vec::new();
        let mut group_vec: Vec<(String, Vec<Record>)> = groups.into_iter().collect();
        group_vec.sort_by(|a, b| a.0.cmp(&b.0));
        for (value, group_records) in group_vec {
            let filename = format!("{}.{}.bed", stem, sanitize_filename(&value));
            let out_path = out_dir.join(filename);
            write_records(&group_records, &out_path, &opts.extra_columns)?;
            output_paths.push(out_path);
        }
        output_paths
    } else {
        let out_path = out_dir.join(format!("{}.bed", stem));
        write_records(&records, &out_path, &opts.extra_columns)?;
        vec![out_path]
    };

    Ok(ProcessResult { outputs, skipped })
}

fn parse_file(path: &Path, format: Format) -> Result<Vec<Record>, GetinbedError> {
    match format {
        Format::Bed | Format::NarrowPeak | Format::BroadPeak | Format::BedGraph => {
            parse::bed::parse(path)
        }
        Format::GenePred => parse::genepred::parse(path),
        Format::Psl => parse::psl::parse(path),
        Format::Gff3 | Format::Gtf => parse::gff::parse(path),
        Format::Vcf | Format::VcfTabix => parse::vcf::parse(path),
        Format::BigBed
        | Format::BigNarrowPeak
        | Format::BigBroadPeak
        | Format::BigGenePred
        | Format::BigPsl
        | Format::BigBarChart => parse::big::parse(path),
    }
}

fn validate_column_indices(
    records: &[Record],
    extra_cols: &[usize],
    split_on: Option<usize>,
) -> Result<(), GetinbedError> {
    if records.is_empty() {
        return Ok(());
    }
    let ncols = records[0].raw.len();
    for &col in extra_cols {
        if col >= ncols {
            return Err(GetinbedError::ColumnOutOfRange(col, ncols));
        }
    }
    if let Some(split_col) = split_on {
        if split_col >= ncols {
            return Err(GetinbedError::ColumnOutOfRange(split_col, ncols));
        }
    }
    Ok(())
}

fn write_records(
    records: &[Record],
    path: &Path,
    extra_cols: &[usize],
) -> Result<(), GetinbedError> {
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    for r in records {
        write!(writer, "{}\t{}\t{}", r.chrom, r.start, r.end)?;
        for val in ops::select::extra_values(r, extra_cols) {
            write!(writer, "\t{}", val)?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

fn output_stem(path: &Path) -> String {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let base = if name.to_lowercase().ends_with(".gz") {
        &name[..name.len() - 3]
    } else {
        &name
    };
    Path::new(base)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| base.to_string())
}

fn make_progress_bar(n: usize, quiet: bool) -> Option<ProgressBar> {
    if quiet || n <= 1 {
        return None;
    }
    let pb = ProgressBar::new(n as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} files  {elapsed_precise}",
        )
        .unwrap()
        .progress_chars("=>-"),
    );
    Some(pb)
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
