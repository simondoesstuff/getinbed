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
