use crate::chrom;
use crate::error::GetinbedError;
use crate::parse::Record;
use bigtools::BigBedRead;
use std::path::Path;

pub fn parse(path: &Path) -> Result<Vec<Record>, GetinbedError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| GetinbedError::Other("non-UTF-8 path".into()))?;
    let mut reader = BigBedRead::open_file(path_str)
        .map_err(|e| GetinbedError::Other(format!("bigBed open error: {e}")))?;

    let chroms = reader.chroms().to_vec();
    let mut records = Vec::new();

    for chrom_info in &chroms {
        let intervals = reader
            .get_interval(&chrom_info.name, 0, chrom_info.length)
            .map_err(|e| GetinbedError::Other(format!("bigBed read error: {e}")))?;

        for result in intervals {
            let iv =
                result.map_err(|e| GetinbedError::Other(format!("bigBed interval error: {e}")))?;
            let mut raw = vec![
                chrom_info.name.clone(),
                iv.start.to_string(),
                iv.end.to_string(),
            ];
            if !iv.rest.is_empty() {
                raw.extend(iv.rest.split('\t').map(|s| s.to_string()));
            }
            let normalized_chrom = chrom::normalize(&chrom_info.name);
            records.push(Record {
                chrom: normalized_chrom,
                start: iv.start as u64,
                end: iv.end as u64,
                raw,
                chrom_col: 0,
                start_col: 1,
                end_col: 2,
            });
        }
    }

    Ok(records)
}
