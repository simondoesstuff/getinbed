use crate::error::GetinbedError;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct BlacklistIndex {
    map: HashMap<String, Vec<(u64, u64)>>,
}

impl BlacklistIndex {
    pub fn load(path: &Path) -> Result<Self, GetinbedError> {
        if !path.exists() {
            return Err(GetinbedError::Other(format!(
                "Blacklist file not found: {}",
                path.display()
            )));
        }

        let file = File::open(path)?;
        let is_gz = path.to_string_lossy().to_lowercase().ends_with(".gz");

        let reader: Box<dyn BufRead> = if is_gz {
            Box::new(BufReader::new(GzDecoder::new(file)))
        } else {
            Box::new(BufReader::new(file))
        };

        let mut map: HashMap<String, Vec<(u64, u64)>> = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            if line.trim().is_empty()
                || line.starts_with('#')
                || line.starts_with("track")
                || line.starts_with("browser")
            {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 3 {
                continue;
            }
            let Ok(start) = cols[1].trim().parse::<u64>() else {
                continue;
            };
            let Ok(end) = cols[2].trim().parse::<u64>() else {
                continue;
            };
            let c = cols[0].trim().to_string();
            map.entry(c).or_default().push((start, end));
        }

        for intervals in map.values_mut() {
            intervals.sort_unstable();
        }

        Ok(BlacklistIndex { map })
    }

    /// Returns true if (chrom, start, end) overlaps any blacklist interval.
    /// Overlap: input.start < bl.end && input.end > bl.start.
    pub fn overlaps(&self, chrom: &str, start: u64, end: u64) -> bool {
        let Some(intervals) = self.map.get(chrom) else {
            return false;
        };
        // Binary search: first interval where bl_end > start
        let pos = intervals.partition_point(|&(_, bl_end)| bl_end <= start);
        for &(bl_start, _) in &intervals[pos..] {
            if bl_start >= end {
                break;
            }
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_index(entries: &[(&str, u64, u64)]) -> BlacklistIndex {
        let mut map: HashMap<String, Vec<(u64, u64)>> = HashMap::new();
        for &(chrom, start, end) in entries {
            map.entry(chrom.to_string()).or_default().push((start, end));
        }
        for v in map.values_mut() {
            v.sort_unstable();
        }
        BlacklistIndex { map }
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
