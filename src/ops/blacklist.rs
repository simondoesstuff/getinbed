use crate::chrom;
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
            let line = line.trim_end_matches('\r').to_string();
            if line.is_empty()
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
            let Ok(start) = cols[1].parse::<u64>() else {
                continue;
            };
            let Ok(end) = cols[2].parse::<u64>() else {
                continue;
            };
            let c = chrom::normalize(cols[0]);
            map.entry(c).or_default().push((start, end));
        }

        for intervals in map.values_mut() {
            intervals.sort_unstable();
        }

        Ok(BlacklistIndex { map })
    }

    pub fn overlaps(&self, chrom: &str, start: u64, end: u64) -> bool {
        let Some(intervals) = self.map.get(chrom) else {
            return false;
        };
        // Find first interval where bl_end > start
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
