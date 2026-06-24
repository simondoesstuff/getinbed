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
