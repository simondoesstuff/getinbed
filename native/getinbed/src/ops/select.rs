use crate::parse::Record;

/// Returns the values of the requested extra columns for a record, silently
/// skipping any index that corresponds to the record's chrom/start/end columns.
pub fn extra_values<'a>(record: &'a Record, extra_cols: &[usize]) -> Vec<&'a str> {
    extra_cols
        .iter()
        .filter(|&&i| {
            i != record.chrom_col && i != record.start_col && i != record.end_col
        })
        .filter(|&&i| i < record.raw.len())
        .map(|&i| record.raw[i].as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bed6(chrom: &str, start: u64, end: u64, name: &str, score: &str, strand: &str) -> Record {
        Record {
            chrom: chrom.to_string(),
            start,
            end,
            raw: vec![
                chrom.to_string(),
                start.to_string(),
                end.to_string(),
                name.to_string(),
                score.to_string(),
                strand.to_string(),
            ],
            chrom_col: 0,
            start_col: 1,
            end_col: 2,
        }
    }

    #[test]
    fn test_select_basic() {
        let r = bed6("chr1", 0, 100, "peak1", "100", "+");
        // Select name (col 3) and strand (col 5)
        let vals = extra_values(&r, &[3, 5]);
        assert_eq!(vals, vec!["peak1", "+"]);
    }

    #[test]
    fn test_select_skips_coord_cols() {
        let r = bed6("chr1", 0, 100, "peak1", "100", "+");
        // Include chrom (0), start (1), end (2) — all should be silently dropped
        let vals = extra_values(&r, &[0, 1, 2, 3]);
        assert_eq!(vals, vec!["peak1"]);
    }

    #[test]
    fn test_select_empty() {
        let r = bed6("chr1", 0, 100, "peak1", "100", "+");
        let vals = extra_values(&r, &[]);
        assert!(vals.is_empty());
    }

    #[test]
    fn test_select_out_of_range_skipped() {
        let r = bed6("chr1", 0, 100, "peak1", "100", "+");
        // col 99 is out of range — silently skipped
        let vals = extra_values(&r, &[3, 99]);
        assert_eq!(vals, vec!["peak1"]);
    }

    #[test]
    fn test_select_score_only() {
        let r = bed6("chr1", 100, 200, "gene", "500", "-");
        let vals = extra_values(&r, &[4]);
        assert_eq!(vals, vec!["500"]);
    }
}
