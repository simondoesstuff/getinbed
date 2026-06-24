use crate::chrom;
use crate::error::GetinbedError;
use crate::parse::Record;
use flate2::read::GzDecoder;
use memmap2::Mmap;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn parse_line(line: &str) -> Option<Record> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.trim().is_empty() || line.starts_with('#') {
        return None;
    }
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 17 {
        return None;
    }
    // PSL col0 must be numeric (match count); this skips the 5-line ASCII header
    cols[0].trim().parse::<u64>().ok()?;
    // Target (genome) coordinates: col13=tName, col15=tStart, col16=tEnd (0-based half-open)
    let tname = cols[13].trim();
    let tstart = cols[15].trim().parse::<u64>().ok()?;
    let tend = cols[16].trim().parse::<u64>().ok()?;
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
        Ok(parse_bytes(&mmap))
    }
}

pub fn parse_bytes(data: &[u8]) -> Vec<Record> {
    data.par_split(|&b| b == b'\n')
        .filter_map(|line| std::str::from_utf8(line).ok())
        .filter_map(parse_line)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid PSL line (21 fields, but we only need 17)
    fn make_psl(tname: &str, tstart: u64, tend: u64) -> String {
        // PSL fields: match misMatch repMatch nCount qNumInsert qBaseInsert tNumInsert tBaseInsert
        //             strand qName qSize qStart qEnd tName tSize tStart tEnd blockCount blockSizes qStarts tStarts
        format!(
            "100\t0\t0\t0\t0\t0\t0\t0\t+\tquery1\t200\t0\t100\t{}\t1000000\t{}\t{}\t1\t100,\t0,\t{},",
            tname, tstart, tend, tstart
        )
    }

    fn line(s: &str) -> Option<Record> {
        parse_line(s)
    }

    #[test]
    fn test_psl_basic() {
        let l = make_psl("chr7", 1000, 1100);
        let r = line(&l).unwrap();
        assert_eq!(r.chrom, "chr7");
        assert_eq!(r.start, 1000);
        assert_eq!(r.end, 1100);
    }

    #[test]
    fn test_psl_header_skipped() {
        // PSL 5-line header looks like "psLayout version 3" etc. — non-numeric col0
        assert!(line("psLayout version 3").is_none());
        assert!(line("match\tmis-\trepeat\t...").is_none());
        assert!(line("-----\t-----\t-----\t...").is_none());
    }

    #[test]
    fn test_psl_comment_skipped() {
        assert!(line("# comment").is_none());
        assert!(line("  ").is_none());
    }

    #[test]
    fn test_psl_too_few_cols() {
        assert!(line("100\t0\t0\t0\t0\t0").is_none());
    }

    #[test]
    fn test_psl_chrom_normalization() {
        let l = make_psl("1", 500, 600);
        let r = line(&l).unwrap();
        assert_eq!(r.chrom, "chr1");
    }

    #[test]
    fn test_psl_parse_bytes() {
        let l1 = make_psl("chr1", 0, 100);
        let l2 = make_psl("chrX", 500, 600);
        let data = format!("psLayout version 3\n# comment\n{}\n{}\n", l1, l2);
        let records = parse_bytes(data.as_bytes());
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].chrom, "chr1");
        assert_eq!(records[1].chrom, "chrX");
    }
}
