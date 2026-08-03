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
    fn test_clean_drops_start_ge_end() {
        let records = vec![
            rec("chr1", 100, 50),  // start > end
            rec("chr1", 100, 100), // start == end
            rec("chr1", 0, 100),
        ];
        let (cleaned, skipped) = clean(records);
        assert_eq!(cleaned.len(), 1);
        assert_eq!(skipped, 2);
    }

    #[test]
    fn test_clean_drops_empty_chrom() {
        let records = vec![
            rec("", 0, 100),
            rec("chr1", 0, 100),
        ];
        let (cleaned, skipped) = clean(records);
        assert_eq!(cleaned.len(), 1);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn test_clean_deduplicates_exact() {
        let records = vec![
            rec("chr1", 0, 100),
            rec("chr1", 0, 100), // duplicate
            rec("chr1", 0, 100), // duplicate
            rec("chr1", 50, 150),
        ];
        let (cleaned, skipped) = clean(records);
        assert_eq!(cleaned.len(), 2);
        assert_eq!(skipped, 2);
    }

    #[test]
    fn test_clean_dedup_keeps_first() {
        // Records with same coords but different raw data — first one kept
        let mut r1 = rec("chr1", 0, 100);
        r1.raw.push("first".to_string());
        let mut r2 = rec("chr1", 0, 100);
        r2.raw.push("second".to_string());
        let (cleaned, _) = clean(vec![r1, r2]);
        assert_eq!(cleaned.len(), 1);
        assert_eq!(cleaned[0].raw.last().unwrap(), "first");
    }

    #[test]
    fn test_clean_dedup_different_chroms() {
        let records = vec![
            rec("chr1", 0, 100),
            rec("chr2", 0, 100), // same coords, different chrom — not a dup
        ];
        let (cleaned, skipped) = clean(records);
        assert_eq!(cleaned.len(), 2);
        assert_eq!(skipped, 0);
    }

    #[test]
    fn test_clean_empty_input() {
        let (cleaned, skipped) = clean(vec![]);
        assert!(cleaned.is_empty());
        assert_eq!(skipped, 0);
    }

    #[test]
    fn test_clean_all_valid_no_dups() {
        let records = vec![
            rec("chr1", 0, 100),
            rec("chr1", 200, 300),
            rec("chr2", 0, 100),
        ];
        let (cleaned, skipped) = clean(records);
        assert_eq!(cleaned.len(), 3);
        assert_eq!(skipped, 0);
    }
}
