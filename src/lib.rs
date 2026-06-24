pub mod batch;
mod chrom;
mod detect;
mod error;
pub mod ops;
pub mod parse;

pub use batch::{process_batch, Opts, ProcessResult};
pub use detect::parse_format_str;
pub use error::GetinbedError;
pub use parse::{Format, Record};

use std::path::{Path, PathBuf};

/// Process a batch of files, writing output to disk. Returns all output paths.
pub fn process(files: &[PathBuf], opts: &Opts) -> Result<Vec<PathBuf>, GetinbedError> {
    let results = batch::process_batch(files, opts)?;
    Ok(results.into_iter().flat_map(|r| r.outputs).collect())
}

/// Parse and process a single file, returning records in memory without writing.
pub fn process_one(path: &Path, opts: &Opts) -> Result<Vec<Record>, GetinbedError> {
    batch::process_one_for_memory(path, opts)
}
