pub mod bed;
pub mod big;
pub mod genepred;
pub mod gff;
pub mod psl;
pub mod vcf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Bed,
    NarrowPeak,
    BroadPeak,
    BedGraph,
    GenePred,
    Psl,
    Gff3,
    Gtf,
    Vcf,
    VcfTabix,
    BigBed,
    BigNarrowPeak,
    BigBroadPeak,
    BigGenePred,
    BigPsl,
    BigBarChart,
}

/// One row from a source file, after coordinate normalization but before any ops.
#[derive(Debug, Clone)]
pub struct Record {
    /// Normalized chromosome name (e.g. "chr1").
    pub chrom: String,
    /// 0-based start coordinate.
    pub start: u64,
    /// Half-open end coordinate.
    pub end: u64,
    /// All original source columns as strings (raw[0] is the first column).
    pub raw: Vec<String>,
    /// Index into raw[] that holds the chromosome column.
    pub chrom_col: usize,
    /// Index into raw[] that holds the start coordinate column.
    pub start_col: usize,
    /// Index into raw[] that holds the end coordinate column (usize::MAX if
    /// end is derived, not stored directly, e.g. VCF).
    pub end_col: usize,
}
