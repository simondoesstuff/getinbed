use crate::chrom::chrom_order;
use crate::parse::Record;
use std::collections::HashMap;
use voracious_radix_sort::{Radixable, RadixSort};

/// A (packed_key, original_index) pair that can be radix-sorted.
///
/// Key layout (64 bits):
///   [6-bit chrom rank][29-bit start][29-bit end]
///
/// Covers up to 64 unique chromosomes and coordinates up to 536 Mbp —
/// enough for any known reference genome (hg38 max: 249 Mbp).
#[derive(Copy, Clone, PartialEq, PartialOrd)]
struct KeyedIdx {
    key: u64,
    idx: u32,
}

impl Radixable<u64> for KeyedIdx {
    type Key = u64;
    fn key(&self) -> u64 {
        self.key
    }
}

const COORD_BITS: u32 = 29;
const COORD_MASK: u64 = (1u64 << COORD_BITS) - 1; // 0x1FFF_FFFF

/// Sorts records in karyotypic order: (chrom_order, start, end).
///
/// Builds a chrom_order cache (~25 entries) then packs (rank, start, end) into
/// a u64 key and radix-sorts a lightweight index array. Records are permuted in
/// O(n) using Option::take — no Record cloning.
pub fn sort(records: &mut Vec<Record>) {
    if records.len() <= 1 {
        return;
    }

    // ── Step 1: chrom_order cache (one call per unique chromosome) ──────────
    let mut order_cache: HashMap<String, u64> = HashMap::with_capacity(32);
    for r in records.iter() {
        if !order_cache.contains_key(&r.chrom) {
            order_cache.insert(r.chrom.clone(), chrom_order(&r.chrom));
        }
    }

    // ── Step 2: rank unique chrom_order values densely 0..N ─────────────────
    let mut orders: Vec<u64> = order_cache.values().copied().collect();
    orders.sort_unstable();
    orders.dedup();
    let rank_of = |co: u64| -> u64 {
        orders.binary_search(&co).unwrap() as u64
    };

    // ── Step 3: build keyed index array ─────────────────────────────────────
    let mut keyed: Vec<KeyedIdx> = records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let rank = rank_of(order_cache[&r.chrom]);
            let key = (rank << (2 * COORD_BITS))
                | ((r.start & COORD_MASK) << COORD_BITS)
                | (r.end & COORD_MASK);
            KeyedIdx { key, idx: i as u32 }
        })
        .collect();

    // ── Step 4: radix sort on the packed u64 key ────────────────────────────
    keyed.voracious_sort();

    // ── Step 5: apply permutation without cloning Records ───────────────────
    let mut slot: Vec<Option<Record>> = records.drain(..).map(Some).collect();
    records.reserve(slot.len());
    for ke in keyed {
        records.push(slot[ke.idx as usize].take().unwrap());
    }
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
        sort(&mut records);
    }

    #[test]
    fn test_sort_single() {
        let mut records = vec![rec("chr5", 100, 200)];
        sort(&mut records);
        assert_eq!(records[0].chrom, "chr5");
    }

    #[test]
    fn test_sort_large_coordinates() {
        // Coordinates up to 249 Mbp (hg38 chr1 length) must sort correctly.
        let mut records = vec![
            rec("chr1", 248_956_422, 248_956_500),
            rec("chr1", 100_000_000, 100_000_100),
            rec("chr1", 0, 100),
        ];
        sort(&mut records);
        assert_eq!(records[0].start, 0);
        assert_eq!(records[1].start, 100_000_000);
        assert_eq!(records[2].start, 248_956_422);
    }
}
