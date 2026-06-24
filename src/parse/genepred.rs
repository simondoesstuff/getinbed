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
    if cols.len() < 6 {
        return None;
    }
    // genePred: col0=name col1=chrom col2=strand col3=txStart col4=txEnd (0-based half-open)
    // Header lines have a non-numeric first column (e.g. "#name", "name"); real records
    // have transcript IDs. txStart/txEnd are at cols 3 and 4.
    let start = atoi::atoi::<u64>(cols[3].trim().as_bytes())?;
    let end = atoi::atoi::<u64>(cols[4].trim().as_bytes())?;
    let chrom = cols[1].trim().to_string();
    Some(Record {
        chrom,
        start,
        end,
        raw: cols.iter().map(|s| s.to_string()).collect(),
        chrom_col: 1,
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
        Ok(parse_bytes(&mmap))
    }
}

pub fn parse_bytes(data: &[u8]) -> Vec<Record> {
    data.par_split(|&b| b == b'\n')
        // SAFETY: genomic coordinate files are ASCII; ASCII is valid UTF-8.
        .map(|line| unsafe { std::str::from_utf8_unchecked(line) })
        .filter_map(parse_line)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(s: &str) -> Option<Record> {
        parse_line(s)
    }

    #[test]
    fn test_genepred_basic() {
        // col0=name col1=chrom col2=strand col3=txStart col4=txEnd col5=cdsStart col6=cdsEnd ...
        let r = line("NM_000014\tchr12\t+\t9220296\t9268825\t9220304\t9268781\t36\t.\t.\t0").unwrap();
        assert_eq!(r.chrom, "chr12");
        assert_eq!(r.start, 9220296); // col3 = txStart
        assert_eq!(r.end, 9268825);   // col4 = txEnd
    }

    #[test]
    fn test_genepred_comment_skipped() {
        assert!(line("# bin name chrom strand txStart txEnd").is_none());
        assert!(line("#name\tchrom\tstrand\ttxStart\ttxEnd").is_none());
    }

    #[test]
    fn test_genepred_empty_skipped() {
        assert!(line("").is_none());
    }

    #[test]
    fn test_genepred_too_few_cols() {
        assert!(line("NM_001\tchr1\t+\t100").is_none());
    }

    #[test]
    fn test_genepred_chrom_passthrough() {
        let r = line("NM_001\t1\t+\t0\t100\t0\t100\t1\texons\texon_starts\t0").unwrap();
        assert_eq!(r.chrom, "1");
        let r = line("NM_001\tchr1\t+\t0\t100\t0\t100\t1\texons\texon_starts\t0").unwrap();
        assert_eq!(r.chrom, "chr1");
    }

    #[test]
    fn test_genepred_parse_bytes() {
        let data = b"# header\nNM_001\tchr1\t+\t0\t1000\t100\t900\t3\t.\t.\t0\nNM_002\tchrX\t-\t5000\t6000\t5100\t5900\t2\t.\t.\t0\n";
        let records = parse_bytes(data);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].chrom, "chr1");
        assert_eq!(records[1].chrom, "chrX");
    }
}
