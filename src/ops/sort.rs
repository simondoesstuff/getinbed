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
        assert_eq!(records[0].chrom, "chr1");
        assert_eq!(records[0].start, 0);
        assert_eq!(records[1].chrom, "chr1");
        assert_eq!(records[1].start, 500);
        assert_eq!(records[2].chrom, "chr2");
        assert_eq!(records[3].chrom, "chr10");
        assert_eq!(records[4].chrom, "chrX");
        assert_eq!(records[5].chrom, "chrM");
    }
}
