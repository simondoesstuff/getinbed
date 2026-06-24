/// Karyotypic sort key for a chromosome name. Works on both UCSC-style
/// (`chr1`, `chrX`) and bare (`1`, `X`, `2L`) names. Numeric chromosomes
/// sort first (by number), then sex/mito, then everything else alphabetically.
pub fn chrom_order(chrom: &str) -> u64 {
    // Strip chr/Chr/CHR prefix for analysis
    let bare = if chrom.len() > 3 && chrom[..3].eq_ignore_ascii_case("chr") {
        &chrom[3..]
    } else {
        chrom
    };

    let upper = bare.to_ascii_uppercase();

    // Mitochondrial aliases — check before numeric so "M" doesn't fall through
    if upper == "M" || upper == "MT" {
        return u64::MAX - 2;
    }
    // Sex / dosage chromosomes (Z/W for birds, X/Y for mammals)
    if upper == "X" || upper == "Z" {
        return u64::MAX - 4;
    }
    if upper == "Y" || upper == "W" {
        return u64::MAX - 3;
    }

    // Pure integer: "1", "22", "chr25" (zebrafish)
    if let Ok(n) = bare.parse::<u64>() {
        return n.saturating_mul(256);
    }

    // Integer + arm suffix: "2L", "2R", "3L" (Drosophila)
    // Sort by chromosome number first, then arm letter.
    let digit_end = bare.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digit_end > 0 {
        if let Ok(n) = bare[..digit_end].parse::<u64>() {
            let arm = bare.as_bytes().get(digit_end).copied().unwrap_or(0) as u64;
            return n.saturating_mul(256) + arm;
        }
    }

    // Everything else (scaffolds, roman numerals, etc.) — sort alphabetically
    // among themselves, after numeric/sex/mito chroms.
    u64::MAX - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_order() {
        assert!(chrom_order("1") < chrom_order("2"));
        assert!(chrom_order("9") < chrom_order("10"));
        assert!(chrom_order("22") < chrom_order("25")); // zebrafish
        // chr-prefixed and bare are equivalent in ordering
        assert_eq!(chrom_order("chr1"), chrom_order("1"));
        assert_eq!(chrom_order("chr22"), chrom_order("22"));
    }

    #[test]
    fn test_sex_mito_order() {
        // numeric < sex < mito
        assert!(chrom_order("22") < chrom_order("X"));
        assert!(chrom_order("22") < chrom_order("chrX"));
        assert!(chrom_order("X") < chrom_order("Y"));
        assert!(chrom_order("Y") < chrom_order("M"));
        assert!(chrom_order("Y") < chrom_order("MT"));
        assert!(chrom_order("chrX") < chrom_order("chrY"));
        assert!(chrom_order("chrY") < chrom_order("chrM"));
        // birds
        assert_eq!(chrom_order("Z"), chrom_order("X")); // same group
        assert_eq!(chrom_order("W"), chrom_order("Y")); // same group
    }

    #[test]
    fn test_arm_order() {
        // 2L and 2R sort between chr2 and chr3
        assert!(chrom_order("2") < chrom_order("2L"));
        assert!(chrom_order("2L") < chrom_order("2R"));
        assert!(chrom_order("2R") < chrom_order("3"));
        // chr-prefixed arms
        assert!(chrom_order("chr2L") < chrom_order("chr2R"));
        assert!(chrom_order("chr3L") < chrom_order("chr4"));
    }

    #[test]
    fn test_other_chroms_after_sex_mito() {
        // scaffolds / roman numerals sort after M
        assert!(chrom_order("chrM") < chrom_order("scaffold_1"));
        assert!(chrom_order("M") < chrom_order("chrI")); // yeast/worm roman numerals
    }

    fn sorted(mut names: Vec<&str>) -> Vec<&str> {
        names.sort_by_key(|n| chrom_order(n));
        names
    }

    #[test]
    fn test_full_human_ucsc_order() {
        let input = vec![
            "chrM", "chrY", "chrX", "chr22", "chr21", "chr20", "chr19", "chr18",
            "chr17", "chr16", "chr15", "chr14", "chr13", "chr12", "chr11", "chr10",
            "chr9",  "chr8",  "chr7",  "chr6",  "chr5",  "chr4",  "chr3",  "chr2",  "chr1",
        ];
        let expected = vec![
            "chr1",  "chr2",  "chr3",  "chr4",  "chr5",  "chr6",  "chr7",  "chr8",
            "chr9",  "chr10", "chr11", "chr12", "chr13", "chr14", "chr15", "chr16",
            "chr17", "chr18", "chr19", "chr20", "chr21", "chr22",
            "chrX", "chrY", "chrM",
        ];
        assert_eq!(sorted(input), expected);
    }

    #[test]
    fn test_full_human_ensembl_order() {
        // Same assembly, Ensembl-style bare names (no chr prefix)
        let input = vec![
            "MT", "Y", "X", "22", "21", "20", "19", "18",
            "17", "16", "15", "14", "13", "12", "11", "10",
            "9",  "8",  "7",  "6",  "5",  "4",  "3",  "2",  "1",
        ];
        let expected = vec![
            "1",  "2",  "3",  "4",  "5",  "6",  "7",  "8",
            "9",  "10", "11", "12", "13", "14", "15", "16",
            "17", "18", "19", "20", "21", "22",
            "X", "Y", "MT",
        ];
        assert_eq!(sorted(input), expected);
    }

    #[test]
    fn test_drosophila_ucsc_order() {
        let input = vec!["chrM", "chrY", "chrX", "chr4", "chr3R", "chr3L", "chr2R", "chr2L"];
        let expected = vec!["chr2L", "chr2R", "chr3L", "chr3R", "chr4", "chrX", "chrY", "chrM"];
        assert_eq!(sorted(input), expected);
    }

    #[test]
    fn test_zebrafish_order() {
        // Zebrafish has 25 autosomes; chr23-25 must sort correctly after chr22
        let input = vec!["chrM", "chr25", "chr22", "chr1", "chr10", "chr2"];
        let expected = vec!["chr1", "chr2", "chr10", "chr22", "chr25", "chrM"];
        assert_eq!(sorted(input), expected);
    }
}
