use crate::error::GetinbedError;
use coitrees::{COITree, Interval, IntervalTree};
use flate2::read::GzDecoder;
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct BlacklistIndex {
    chroms: HashMap<String, COITree<(), u32>>,
}

impl BlacklistIndex {
    pub fn load(path: &Path) -> Result<Self, GetinbedError> {
        if !path.exists() {
            return Err(GetinbedError::Other(format!(
                "Blacklist file not found: {}",
                path.display()
            )));
        }

        let mut by_chrom: HashMap<String, Vec<Interval<()>>> = HashMap::new();

        let file = File::open(path)?;
        let is_gz = path.to_string_lossy().to_lowercase().ends_with(".gz");

        if is_gz {
            let reader = BufReader::new(GzDecoder::new(file));
            for line in reader.lines().filter_map(|l| l.ok()) {
                parse_entry(line.trim_end_matches(['\r', '\n']), &mut by_chrom);
            }
        } else {
            let mmap = unsafe { Mmap::map(&file)? };
            for chunk in mmap.split(|&b| b == b'\n') {
                // SAFETY: genomic coordinate files are ASCII; ASCII is valid UTF-8.
                let line = unsafe { std::str::from_utf8_unchecked(chunk) };
                parse_entry(line.trim_end_matches(['\r', '\n']), &mut by_chrom);
            }
        }

        let chroms = by_chrom
            .into_iter()
            .map(|(k, v)| (k, COITree::new(&v)))
            .collect();

        Ok(BlacklistIndex { chroms })
    }

    /// Returns true if `[start, end)` overlaps any blacklist interval on `chrom`.
    pub fn overlaps(&self, chrom: &str, start: u64, end: u64) -> bool {
        let Some(tree) = self.chroms.get(chrom) else {
            return false;
        };
        // coitrees uses inclusive [first, last]; convert half-open [start, end) → [start, end-1]
        tree.query_count(start as i32, (end as i32).saturating_sub(1)) > 0
    }
}

fn parse_entry(line: &str, by_chrom: &mut HashMap<String, Vec<Interval<()>>>) {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("track")
        || trimmed.starts_with("browser")
    {
        return;
    }
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 3 {
        return;
    }
    let Some(start) = atoi::atoi::<u64>(cols[1].trim().as_bytes()) else {
        return;
    };
    let Some(end) = atoi::atoi::<u64>(cols[2].trim().as_bytes()) else {
        return;
    };
    if start >= end {
        return;
    }
    let chrom = cols[0].trim().to_string();
    // coitrees uses inclusive [first, last]; convert half-open [start, end) → [start, end-1]
    by_chrom.entry(chrom).or_default().push(Interval {
        first: start as i32,
        last: (end - 1) as i32,
        metadata: (),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_index(entries: &[(&str, u64, u64)]) -> BlacklistIndex {
        let mut f = NamedTempFile::new().unwrap();
        for &(chrom, start, end) in entries {
            writeln!(f, "{}\t{}\t{}", chrom, start, end).unwrap();
        }
        BlacklistIndex::load(f.path()).unwrap()
    }

    #[test]
    fn test_overlaps_yes() {
        let idx = make_index(&[("chr1", 100, 200)]);
        assert!(idx.overlaps("chr1", 150, 250)); // partial overlap right
        assert!(idx.overlaps("chr1", 50, 150));  // partial overlap left
        assert!(idx.overlaps("chr1", 100, 200)); // exact match
        assert!(idx.overlaps("chr1", 120, 180)); // contained within
        assert!(idx.overlaps("chr1", 50, 250));  // blacklist contained within query
    }

    #[test]
    fn test_overlaps_no() {
        let idx = make_index(&[("chr1", 100, 200)]);
        assert!(!idx.overlaps("chr1", 200, 300)); // adjacent right — no overlap
        assert!(!idx.overlaps("chr1", 0, 100));   // adjacent left — no overlap
        assert!(!idx.overlaps("chr1", 300, 400)); // completely separate
        assert!(!idx.overlaps("chr2", 100, 200)); // different chrom
    }

    #[test]
    fn test_multiple_blacklist_regions() {
        let idx = make_index(&[("chr1", 100, 200), ("chr1", 500, 600), ("chr1", 1000, 2000)]);
        assert!(idx.overlaps("chr1", 150, 250));
        assert!(idx.overlaps("chr1", 550, 650));
        assert!(idx.overlaps("chr1", 1500, 1600));
        assert!(!idx.overlaps("chr1", 200, 500)); // gap between first two
        assert!(!idx.overlaps("chr1", 600, 1000)); // gap between last two
    }

    #[test]
    fn test_load_from_file() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "# comment").unwrap();
        writeln!(f, "track name=blacklist").unwrap();
        writeln!(f, "chr1\t100\t200").unwrap();
        writeln!(f, "chrX\t500\t600").unwrap();
        let idx = BlacklistIndex::load(f.path()).unwrap();
        assert!(idx.overlaps("chr1", 150, 250));
        assert!(idx.overlaps("chrX", 550, 650));
        assert!(!idx.overlaps("chr2", 100, 200));
    }
}
