use crate::chrom;
use crate::error::GetinbedError;
use crate::parse::Record;
use flate2::read::GzDecoder;
use memmap2::Mmap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn is_metadata(line: &str) -> bool {
    line.starts_with('#') || line.starts_with("track") || line.starts_with("browser")
}

fn parse_line(line: &str) -> Option<Record> {
    let line = line.trim_end_matches('\r');
    if line.is_empty() || is_metadata(line) {
        return None;
    }
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 3 {
        return None;
    }
    let start = cols[1].parse::<u64>().ok()?;
    let end = cols[2].parse::<u64>().ok()?;
    let chrom = chrom::normalize(cols[0]);
    Some(Record {
        chrom,
        start,
        end,
        raw: cols.iter().map(|s| s.to_string()).collect(),
        chrom_col: 0,
        start_col: 1,
        end_col: 2,
    })
}

pub fn parse(path: &Path) -> Result<Vec<Record>, GetinbedError> {
    let is_gz = path
        .to_string_lossy()
        .to_lowercase()
        .ends_with(".gz");
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
        Ok(parse_mmap(&mmap))
    }
}

fn parse_mmap(data: &[u8]) -> Vec<Record> {
    data.split(|&b| b == b'\n')
        .filter_map(|line| std::str::from_utf8(line).ok())
        .filter_map(parse_line)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_basic() {
        let r = parse_line("chr1\t100\t200\tname\t0\t+").unwrap();
        assert_eq!(r.chrom, "chr1");
        assert_eq!(r.start, 100);
        assert_eq!(r.end, 200);
        assert_eq!(r.raw.len(), 6);
    }

    #[test]
    fn test_parse_line_skips_metadata() {
        assert!(parse_line("# comment").is_none());
        assert!(parse_line("track name=foo").is_none());
        assert!(parse_line("browser position chr1:1-100").is_none());
    }

    #[test]
    fn test_parse_line_normalizes_chrom() {
        let r = parse_line("1\t0\t100").unwrap();
        assert_eq!(r.chrom, "chr1");
    }

    #[test]
    fn test_parse_line_too_few_cols() {
        assert!(parse_line("chr1\t100").is_none());
    }
}
