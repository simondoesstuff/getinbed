use crate::parse::Record;
use std::collections::HashSet;

/// Drops malformed rows and deduplicates exact (chrom, start, end) triples.
/// Returns (cleaned_records, n_skipped).
pub fn clean(records: Vec<Record>) -> (Vec<Record>, usize) {
    let mut seen: HashSet<(String, u64, u64)> = HashSet::new();
    let mut skipped = 0;
    let mut out = Vec::with_capacity(records.len());

    for r in records {
        if r.chrom.is_empty() || r.start >= r.end {
            skipped += 1;
            continue;
        }
        let key = (r.chrom.clone(), r.start, r.end);
        if seen.contains(&key) {
            skipped += 1;
            continue;
        }
        seen.insert(key);
        out.push(r);
    }

    (out, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(chrom: &str, start: u64, end: u64) -> Record {
        Record {
            chrom: chrom.to_string(),
            start,
            end,
            raw: vec![chrom.to_string(), start.to_string(), end.to_string()],
            chrom_col: 0,
            start_col: 1,
            end_col: 2,
        }
    }

    #[test]
    fn test_clean_drops_invalid() {
        let records = vec![
            rec("chr1", 100, 50),  // start >= end
            rec("", 0, 100),        // empty chrom
            rec("chr1", 0, 100),
        ];
        let (cleaned, skipped) = clean(records);
        assert_eq!(cleaned.len(), 1);
        assert_eq!(skipped, 2);
    }

    #[test]
    fn test_clean_deduplicates() {
        let records = vec![
            rec("chr1", 0, 100),
            rec("chr1", 0, 100),
            rec("chr1", 50, 150),
        ];
        let (cleaned, skipped) = clean(records);
        assert_eq!(cleaned.len(), 2);
        assert_eq!(skipped, 1);
    }
}
