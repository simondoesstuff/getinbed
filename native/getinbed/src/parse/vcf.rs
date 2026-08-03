use crate::error::GetinbedError;
use crate::parse::Record;
use flate2::read::GzDecoder;
use memmap2::Mmap;
use noodles::bgzf;
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
    if cols.len() < 4 {
        return None;
    }
    let pos1 = atoi::atoi::<u64>(cols[1].trim().as_bytes())?;
    if pos1 == 0 {
        return None;
    }
    let start = pos1 - 1;
    // REF allele determines the end; use the first allele if comma-separated
    let ref_allele = cols[3].split(',').next().unwrap_or("N");
    let end = start + (ref_allele.len() as u64).max(1);
    let chrom = cols[0].trim().to_string();
    Some(Record {
        chrom,
        start,
        end,
        raw: cols.iter().map(|s| s.to_string()).collect(),
        chrom_col: 0,
        start_col: 1,
        end_col: usize::MAX, // end is derived, not a direct column
    })
}

pub fn parse(path: &Path) -> Result<Vec<Record>, GetinbedError> {
    let lower = path.to_string_lossy().to_lowercase();
    if lower.ends_with(".vcf.gz") {
        // bgzf-compressed VCF (includes vcfTabix)
        let file = File::open(path)?;
        let reader = BufReader::new(bgzf::io::Reader::new(file));
        Ok(reader
            .lines()
            .filter_map(|l| l.ok())
            .filter_map(|l| parse_line(&l))
            .collect())
    } else if lower.ends_with(".gz") {
        let file = File::open(path)?;
        let reader = BufReader::new(GzDecoder::new(file));
        Ok(reader
            .lines()
            .filter_map(|l| l.ok())
            .filter_map(|l| parse_line(&l))
            .collect())
    } else {
        let file = File::open(path)?;
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
    fn test_vcf_meta_skipped() {
        assert!(line("##fileformat=VCFv4.1").is_none());
        assert!(line("##INFO=<ID=DP,Number=1,Type=Integer>").is_none());
        assert!(line("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO").is_none());
    }

    #[test]
    fn test_vcf_pos_to_0based() {
        // POS=1 → start=0
        let r = line("chr1\t1\t.\tA\tT\t.\tPASS\t.").unwrap();
        assert_eq!(r.chrom, "chr1");
        assert_eq!(r.start, 0);
        assert_eq!(r.end, 1); // REF "A" → len=1
    }

    #[test]
    fn test_vcf_ref_len() {
        // REF of length 4: end = start + 4
        let r = line("chr2\t101\t.\tACGT\tA\t.\tPASS\t.").unwrap();
        assert_eq!(r.start, 100);
        assert_eq!(r.end, 104);
    }

    #[test]
    fn test_vcf_chrom_passthrough() {
        let r = line("1\t1\t.\tA\tT\t.\tPASS\t.").unwrap();
        assert_eq!(r.chrom, "1");
        let r = line("MT\t1\t.\tA\tT\t.\tPASS\t.").unwrap();
        assert_eq!(r.chrom, "MT");
        let r = line("chr1\t1\t.\tA\tT\t.\tPASS\t.").unwrap();
        assert_eq!(r.chrom, "chr1");
    }

    #[test]
    fn test_vcf_too_few_cols() {
        assert!(line("chr1\t1\t.").is_none());
    }

    #[test]
    fn test_vcf_empty_skipped() {
        assert!(line("").is_none());
        assert!(line("   ").is_none());
    }

    #[test]
    fn test_vcf_crlf() {
        let r = line("chr1\t100\t.\tA\tT\t.\tPASS\t.\r").unwrap();
        assert_eq!(r.start, 99);
    }

    #[test]
    fn test_vcf_parse_bytes() {
        let data = b"##fileformat=VCFv4.1\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\nchr1\t1\t.\tA\tT\t.\tPASS\t.\nchr2\t200\t.\tGG\t.\t.\tPASS\t.\n";
        let records = parse_bytes(data);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].start, 0);
        assert_eq!(records[0].end, 1);
        assert_eq!(records[1].start, 199);
        assert_eq!(records[1].end, 201); // "GG" → len 2
    }
}
