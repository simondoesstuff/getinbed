use crate::chrom;
use crate::error::GetinbedError;
use crate::parse::Record;
use flate2::read::GzDecoder;
use memmap2::Mmap;
use noodles::bgzf;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

// noodles 0.111: bgzf reader lives at bgzf::io::Reader
fn open_bgzf(file: File) -> impl BufRead {
    BufReader::new(bgzf::io::Reader::new(file))
}

fn parse_line(line: &str) -> Option<Record> {
    let line = line.trim_end_matches('\r');
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 4 {
        return None;
    }
    let pos1 = cols[1].parse::<u64>().ok()?;
    if pos1 == 0 {
        return None;
    }
    let start = pos1 - 1;
    let ref_allele = cols[3];
    // REF may contain comma-separated alleles in some representations; use the
    // first allele length.
    let ref_len = ref_allele
        .split(',')
        .next()
        .map(|s| s.len() as u64)
        .unwrap_or(1);
    let end = start + ref_len;
    let chrom = chrom::normalize(cols[0]);
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
    if lower.ends_with(".vcf.gz") || lower.ends_with(".bcf") {
        // bgzf-compressed VCF (includes vcfTabix)
        let file = File::open(path)?;
        let reader = open_bgzf(file);
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
        Ok(mmap
            .split(|&b| b == b'\n')
            .filter_map(|l| std::str::from_utf8(l).ok())
            .filter_map(parse_line)
            .collect())
    }
}
