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

#[cfg(feature = "nif")]
mod nif {
    use super::*;
    use rustler::{Encoder, Env, Error as NifError, NifResult, Term};
    use std::collections::HashSet;

    rustler::atoms! {
        ok,
        error,
        nil,
    }

    // Decode a keyword list option by atom key, returning None if absent or nil.
    fn kw_get<'a>(kw: &[(Term<'a>, Term<'a>)], key: &str) -> Option<Term<'a>> {
        kw.iter().find_map(|(k, v)| {
            if k.atom_to_string().ok().as_deref() == Some(key) {
                Some(*v)
            } else {
                None
            }
        })
    }

    fn is_nil(term: Term<'_>) -> bool {
        term.atom_to_string().ok().as_deref() == Some("nil")
    }

    fn decode_opts(opts_term: Term<'_>) -> NifResult<Opts> {
        let kw: Vec<(Term<'_>, Term<'_>)> = opts_term.decode()?;

        let out = kw_get(&kw, "out")
            .filter(|t| !is_nil(*t))
            .map(|t| -> NifResult<PathBuf> {
                let s: String = t.decode()?;
                Ok(PathBuf::from(s))
            })
            .transpose()?;

        let format = kw_get(&kw, "format")
            .filter(|t| !is_nil(*t))
            .map(|t| -> NifResult<Format> {
                let s: String = t.atom_to_string().or_else(|_| t.decode())?;
                parse_format_str(&s)
                    .map_err(|e| NifError::Term(Box::new(e.to_string())))
            })
            .transpose()?;

        let extra_columns: Vec<usize> = kw_get(&kw, "extra_columns")
            .filter(|t| !is_nil(*t))
            .map(|t| -> NifResult<Vec<usize>> {
                let list: Vec<u64> = t.decode()?;
                Ok(list.into_iter().map(|n| n as usize).collect())
            })
            .transpose()?
            .unwrap_or_default();

        let blacklist = kw_get(&kw, "blacklist")
            .filter(|t| !is_nil(*t))
            .map(|t| -> NifResult<PathBuf> {
                let s: String = t.decode()?;
                Ok(PathBuf::from(s))
            })
            .transpose()?;

        let split_on = kw_get(&kw, "split_on")
            .filter(|t| !is_nil(*t))
            .map(|t| -> NifResult<usize> {
                let n: u64 = t.decode()?;
                Ok(n as usize)
            })
            .transpose()?;

        let chroms: Option<HashSet<String>> = kw_get(&kw, "chroms")
            .filter(|t| !is_nil(*t))
            .map(|t| -> NifResult<HashSet<String>> {
                let list: Vec<String> = t.decode()?;
                Ok(list.into_iter().collect())
            })
            .transpose()?;

        let no_clean = kw_get(&kw, "no_clean")
            .and_then(|t| t.decode::<bool>().ok())
            .unwrap_or(false);

        let no_sort = kw_get(&kw, "no_sort")
            .and_then(|t| t.decode::<bool>().ok())
            .unwrap_or(false);

        let jobs = kw_get(&kw, "jobs")
            .filter(|t| !is_nil(*t))
            .and_then(|t| t.decode::<u64>().ok())
            .map(|n| n as usize);

        Ok(Opts {
            out,
            format,
            extra_columns,
            blacklist,
            split_on,
            chroms,
            no_clean,
            no_sort,
            jobs,
            quiet: true, // always quiet inside the NIF
        })
    }

    #[rustler::nif(schedule = "DirtyCpu")]
    fn process_nif<'a>(
        env: Env<'a>,
        files: Vec<String>,
        opts_term: Term<'a>,
    ) -> Term<'a> {
        let file_paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();

        let opts = match decode_opts(opts_term) {
            Ok(o) => o,
            Err(e) => {
                let msg = format!("{:?}", e);
                return (error(), msg).encode(env);
            }
        };

        match process_batch(&file_paths, &opts) {
            Ok(results) => {
                let paths: Vec<String> = results
                    .into_iter()
                    .flat_map(|r| r.outputs)
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                (ok(), paths).encode(env)
            }
            Err(e) => (error(), e.to_string()).encode(env),
        }
    }

    rustler::init!("Elixir.Getinbed");
}
