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

/// Parse raw bytes in the given format, returning records. Useful for testing
/// and benchmarking without touching the filesystem.
///
/// Only text-based formats are supported (big* formats require a seekable file).
pub fn parse_from_bytes(data: &[u8], format: Format) -> Vec<Record> {
    match format {
        Format::Bed | Format::NarrowPeak | Format::BroadPeak | Format::BedGraph => {
            bed::parse_bytes(data)
        }
        Format::GenePred => genepred::parse_bytes(data),
        Format::Psl => psl::parse_bytes(data),
        Format::Gff3 | Format::Gtf => {
            let trimmed = gff::strip_fasta_section(data);
            gff::parse_bytes(trimmed)
        }
        Format::Vcf | Format::VcfTabix => vcf::parse_bytes(data),
        _ => vec![], // big* formats require a file path
    }
}
