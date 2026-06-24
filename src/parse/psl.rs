use crate::chrom;
use crate::error::GetinbedError;
use crate::parse::Record;
use flate2::read::GzDecoder;
use memmap2::Mmap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn parse_line(line: &str) -> Option<Record> {
    let line = line.trim_end_matches('\r');
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 17 {
        return None;
    }
    // col 0 must be numeric (matches count); skip header lines
    cols[0].trim().parse::<u64>().ok()?;
    let tname = cols[13];
    let tstart = cols[15].parse::<u64>().ok()?;
    let tend = cols[16].parse::<u64>().ok()?;
    let chrom = chrom::normalize(tname);
    Some(Record {
        chrom,
        start: tstart,
        end: tend,
        raw: cols.iter().map(|s| s.to_string()).collect(),
        chrom_col: 13,
        start_col: 15,
        end_col: 16,
    })
}

pub fn parse(path: &Path) -> Result<Vec<Record>, GetinbedError> {
    let is_gz = path.to_string_lossy().to_lowercase().ends_with(".gz");
    let file = File::open(path)?;
    if is_gz {
        let reader = BufReader::new(GzDecoder::new(file));
        Ok(reader
            .lines()
            .filter_map(|l| l.ok())
            .filter_map(|l| parse_line(&l))
            .collect())
    } else {
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(mmap
            .split(|&b| b == b'\n')
            .filter_map(|l| std::str::from_utf8(l).ok())
            .filter_map(parse_line)
            .collect())
    }
}
