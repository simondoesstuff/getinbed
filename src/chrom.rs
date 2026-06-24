pub fn normalize(name: &str) -> String {
    let lc = name.to_lowercase();

    if lc == "mt" || lc == "m" {
        return "chrM".to_string();
    }
    if lc == "x" {
        return "chrX".to_string();
    }
    if lc == "y" {
        return "chrY".to_string();
    }

    if let Some(rest) = lc.strip_prefix("chr") {
        if rest == "mt" || rest == "m" {
            return "chrM".to_string();
        }
        return format!("chr{}", &name[3..]);
    }

    if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
        return format!("chr{}", name);
    }

    name.to_string()
}

pub fn is_standard(chrom: &str) -> bool {
    matches!(
        chrom,
        "chr1" | "chr2" | "chr3" | "chr4" | "chr5" | "chr6" | "chr7" | "chr8" | "chr9"
            | "chr10" | "chr11" | "chr12" | "chr13" | "chr14" | "chr15" | "chr16" | "chr17"
            | "chr18" | "chr19" | "chr20" | "chr21" | "chr22" | "chrX" | "chrY" | "chrM"
    )
}

pub fn chrom_order(chrom: &str) -> u32 {
    match chrom {
        "chr1" => 1,
        "chr2" => 2,
        "chr3" => 3,
        "chr4" => 4,
        "chr5" => 5,
        "chr6" => 6,
        "chr7" => 7,
        "chr8" => 8,
        "chr9" => 9,
        "chr10" => 10,
        "chr11" => 11,
        "chr12" => 12,
        "chr13" => 13,
        "chr14" => 14,
        "chr15" => 15,
        "chr16" => 16,
        "chr17" => 17,
        "chr18" => 18,
        "chr19" => 19,
        "chr20" => 20,
        "chr21" => 21,
        "chr22" => 22,
        "chrX" => 23,
        "chrY" => 24,
        "chrM" => 25,
        _ => 26,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("1"), "chr1");
        assert_eq!(normalize("22"), "chr22");
        assert_eq!(normalize("X"), "chrX");
        assert_eq!(normalize("Y"), "chrY");
        assert_eq!(normalize("M"), "chrM");
        assert_eq!(normalize("MT"), "chrM");
        assert_eq!(normalize("chrMT"), "chrM");
        assert_eq!(normalize("chr1"), "chr1");
        assert_eq!(normalize("chrX"), "chrX");
        assert_eq!(normalize("chrM"), "chrM");
        assert_eq!(normalize("scaffold_1"), "scaffold_1");
    }

    #[test]
    fn test_is_standard() {
        assert!(is_standard("chr1"));
        assert!(is_standard("chr22"));
        assert!(is_standard("chrX"));
        assert!(is_standard("chrY"));
        assert!(is_standard("chrM"));
        assert!(!is_standard("chrMT"));
        assert!(!is_standard("scaffold_1"));
        assert!(!is_standard("chr23"));
    }

    #[test]
    fn test_chrom_order() {
        assert!(chrom_order("chr1") < chrom_order("chr2"));
        assert!(chrom_order("chr22") < chrom_order("chrX"));
        assert!(chrom_order("chrX") < chrom_order("chrY"));
        assert!(chrom_order("chrY") < chrom_order("chrM"));
        assert!(chrom_order("chrM") < chrom_order("scaffold_1"));
    }
}
