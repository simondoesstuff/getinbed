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
    if cols.len() < 5 {
        return None;
    }
    // GFF3/GTF: 1-based closed; start col is index 3, end col is index 4
    let start1 = cols[3].parse::<u64>().ok()?;
    if start1 == 0 {
        return None;
    }
    let end = cols[4].parse::<u64>().ok()?;
    let start = start1 - 1;
    let chrom = chrom::normalize(cols[0]);
    Some(Record {
        chrom,
        start,
        end,
        raw: cols.iter().map(|s| s.to_string()).collect(),
        chrom_col: 0,
        start_col: 3,
        end_col: 4,
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
