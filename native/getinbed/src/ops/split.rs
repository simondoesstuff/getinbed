use crate::parse::Record;
use std::collections::HashMap;

/// Groups records by the value in `split_col` of their raw source columns.
/// Records missing that column get an empty-string key.
pub fn split_by_column(records: &[Record], split_col: usize) -> HashMap<String, Vec<Record>> {
    let mut groups: HashMap<String, Vec<Record>> = HashMap::new();
    for r in records {
        let key = r
            .raw
            .get(split_col)
            .map(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        groups.entry(key).or_default().push(r.clone());
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(chrom: &str, start: u64, end: u64, label: &str) -> Record {
        Record {
            chrom: chrom.to_string(),
            start,
            end,
            raw: vec![
                chrom.to_string(),
                start.to_string(),
                end.to_string(),
                label.to_string(),
            ],
            chrom_col: 0,
            start_col: 1,
            end_col: 2,
        }
    }

    #[test]
    fn test_split_basic() {
        let records = vec![
            rec("chr1", 0, 100, "E1"),
            rec("chr1", 200, 300, "E2"),
            rec("chr2", 0, 50, "E1"),
        ];
        let groups = split_by_column(&records, 3);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups["E1"].len(), 2);
        assert_eq!(groups["E2"].len(), 1);
    }

    #[test]
    fn test_split_missing_col_empty_key() {
        let records = vec![Record {
            chrom: "chr1".to_string(),
            start: 0,
            end: 100,
            raw: vec!["chr1".to_string(), "0".to_string(), "100".to_string()],
            chrom_col: 0,
            start_col: 1,
            end_col: 2,
        }];
        let groups = split_by_column(&records, 5); // col 5 doesn't exist
        assert_eq!(groups.len(), 1);
        assert!(groups.contains_key(""));
    }

    #[test]
    fn test_split_single_group() {
        let records = vec![
            rec("chr1", 0, 100, "active"),
            rec("chr2", 0, 100, "active"),
        ];
        let groups = split_by_column(&records, 3);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups["active"].len(), 2);
    }

    #[test]
    fn test_split_empty_input() {
        let groups = split_by_column(&[], 3);
        assert!(groups.is_empty());
    }
}
