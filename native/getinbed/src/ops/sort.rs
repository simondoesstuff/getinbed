use crate::chrom::chrom_order;
use crate::parse::Record;

/// Sorts records in karyotypic order: (chrom_order, start, end).
pub fn sort(records: &mut Vec<Record>) {
    records.sort_unstable_by(|a, b| {
        let oa = chrom_order(&a.chrom);
        let ob = chrom_order(&b.chrom);
        oa.cmp(&ob)
            .then(a.start.cmp(&b.start))
            .then(a.end.cmp(&b.end))
            .then(a.chrom.cmp(&b.chrom))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(chrom: &str, start: u64, end: u64) -> Record {
        Record {
            chrom: chrom.to_string(),
            start,
            end,
            raw: vec![],
            chrom_col: 0,
            start_col: 1,
            end_col: 2,
        }
    }

    #[test]
    fn test_sort_karyotypic() {
        let mut records = vec![
            rec("chrM", 0, 100),
            rec("chr10", 0, 100),
            rec("chr2", 0, 100),
            rec("chr1", 500, 600),
            rec("chr1", 0, 100),
            rec("chrX", 0, 100),
        ];
        sort(&mut records);
        let chroms: Vec<&str> = records.iter().map(|r| r.chrom.as_str()).collect();
        assert_eq!(chroms, &["chr1", "chr1", "chr2", "chr10", "chrX", "chrM"]);
        assert_eq!(records[0].start, 0);
        assert_eq!(records[1].start, 500);
    }

    #[test]
    fn test_sort_by_start_within_chrom() {
        let mut records = vec![
            rec("chr1", 1000, 2000),
            rec("chr1", 0, 100),
            rec("chr1", 500, 600),
        ];
        sort(&mut records);
        assert_eq!(records[0].start, 0);
        assert_eq!(records[1].start, 500);
        assert_eq!(records[2].start, 1000);
    }

    #[test]
    fn test_sort_by_end_on_equal_start() {
        let mut records = vec![
            rec("chr1", 100, 300),
            rec("chr1", 100, 200),
            rec("chr1", 100, 400),
        ];
        sort(&mut records);
        assert_eq!(records[0].end, 200);
        assert_eq!(records[1].end, 300);
        assert_eq!(records[2].end, 400);
    }

    #[test]
    fn test_sort_chry_before_chrm() {
        let mut records = vec![rec("chrM", 0, 100), rec("chrY", 0, 100)];
        sort(&mut records);
        assert_eq!(records[0].chrom, "chrY");
        assert_eq!(records[1].chrom, "chrM");
    }

    #[test]
    fn test_sort_nonstandard_after_chrm() {
        let mut records = vec![
            rec("scaffold_1", 0, 100),
            rec("chrM", 0, 100),
            rec("chr1", 0, 100),
        ];
        sort(&mut records);
        assert_eq!(records[0].chrom, "chr1");
        assert_eq!(records[1].chrom, "chrM");
        assert_eq!(records[2].chrom, "scaffold_1");
    }

    #[test]
    fn test_sort_empty() {
        let mut records: Vec<Record> = vec![];
        sort(&mut records); // should not panic
    }

    #[test]
    fn test_sort_single() {
        let mut records = vec![rec("chr5", 100, 200)];
        sort(&mut records);
        assert_eq!(records[0].chrom, "chr5");
    }
}
