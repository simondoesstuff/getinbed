use crate::chrom;
use crate::error::GetinbedError;
use crate::parse::Record;
use flate2::read::GzDecoder;
use memmap2::Mmap;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Trim the GFF3 `##FASTA` section: return only the bytes before that directive.
pub fn strip_fasta_section(data: &[u8]) -> &[u8] {
    const MARKER: &[u8] = b"\n##FASTA";
    if let Some(pos) = data.windows(MARKER.len()).position(|w| w == MARKER) {
        &data[..pos + 1] // keep the newline before the marker
    } else if data.starts_with(b"##FASTA") {
        b""
    } else {
        data
    }
}

fn parse_line(line: &str) -> Option<Record> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.trim().is_empty() || line.starts_with('#') {
        return None;
    }
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 5 {
        return None;
    }
    // GFF3/GTF: 1-based closed → 0-based half-open
    let start1 = cols[3].trim().parse::<u64>().ok()?;
    if start1 == 0 {
        return None; // GFF3 coordinates are 1-based; 0 is invalid
    }
    let end = cols[4].trim().parse::<u64>().ok()?;
    let start = start1 - 1;
    let chrom = chrom::normalize(cols[0].trim());
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
        let mut past_fasta = false;
        Ok(reader
            .lines()
            .filter_map(|l| l.ok())
            .take_while(|l| {
                if l.starts_with("##FASTA") {
                    past_fasta = true;
                }
                !past_fasta
            })
            .filter_map(|l| parse_line(&l))
            .collect())
    } else {
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(parse_bytes(strip_fasta_section(&mmap)))
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

    fn line(s: &str) -> Option<Record> {
        parse_line(s)
    }

    #[test]
    fn test_gff_basic() {
        // GFF3: start=1-based, end=1-based closed → output start=0-based, end=same
        let r = line("chr1\t.\tgene\t1\t1000\t.\t+\t.\tID=gene1").unwrap();
        assert_eq!(r.chrom, "chr1");
        assert_eq!(r.start, 0);  // 1 - 1
        assert_eq!(r.end, 1000);
    }

    #[test]
    fn test_gff_coordinate_conversion() {
        let r = line("chr2\t.\texon\t101\t200\t.\t-\t.\tParent=gene1").unwrap();
        assert_eq!(r.start, 100); // 101 - 1
        assert_eq!(r.end, 200);
    }

    #[test]
    fn test_gff_pragma_skipped() {
        assert!(line("## gff-version 3").is_none());
        assert!(line("# comment line").is_none());
        assert!(line("##sequence-region chr1 1 248956422").is_none());
    }

    #[test]
    fn test_gff_fasta_section_stripped() {
        let data = b"chr1\t.\tgene\t1\t100\t.\t+\t.\tID=g1\n##FASTA\n>chr1\nACGT\n";
        let records = parse_bytes(strip_fasta_section(data));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].chrom, "chr1");
    }

    #[test]
    fn test_gff_fasta_at_start() {
        let data = b"##FASTA\n>chr1\nACGT\n";
        let records = parse_bytes(strip_fasta_section(data));
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn test_gff_too_few_cols() {
        assert!(line("chr1\t.\tgene\t1").is_none());
    }

    #[test]
    fn test_gff_chrom_normalization() {
        let r = line("1\t.\tgene\t1\t100\t.\t+\t.\t.").unwrap();
        assert_eq!(r.chrom, "chr1");
        let r = line("MT\t.\tgene\t1\t100\t.\t+\t.\t.").unwrap();
        assert_eq!(r.chrom, "chrM");
    }

    #[test]
    fn test_gff_crlf() {
        let r = line("chr1\t.\tgene\t1\t100\t.\t+\t.\tID=g1\r").unwrap();
        assert_eq!(r.end, 100);
    }

    #[test]
    fn test_gff_empty_lines_skipped() {
        assert!(line("").is_none());
        assert!(line("   ").is_none());
    }
}
