use getinbed::batch::{process_batch, Opts, ProcessResult};
use getinbed::parse::Format;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

fn default_opts(out: &TempDir) -> Opts {
    Opts {
        out: Some(out.path().to_path_buf()),
        format: None,
        extra_columns: vec![],
        blacklist: None,
        split_on: None,
        chroms: None,
        no_clean: false,
        no_sort: false,
        jobs: None,
        quiet: true,
    }
}

fn chroms(list: &[&str]) -> HashSet<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn write_bed(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new()
        .suffix(".bed")
        .tempfile()
        .unwrap();
    write!(f, "{}", content).unwrap();
    f
}

fn read_output(result: &ProcessResult) -> String {
    assert_eq!(result.outputs.len(), 1);
    fs::read_to_string(&result.outputs[0]).unwrap()
}

#[test]
fn test_basic_bed_roundtrip() {
    let out = TempDir::new().unwrap();
    let f = write_bed("chr1\t100\t200\n");
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    assert_eq!(results.len(), 1);
    let content = read_output(&results[0]);
    assert_eq!(content.trim(), "chr1\t100\t200");
}

#[test]
fn test_chrom_passthrough() {
    // Chromosomes are returned exactly as they appear in the file — no normalization.
    let out = TempDir::new().unwrap();
    let f = write_bed("1\t0\t100\nMT\t0\t50\n");
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().any(|l| l.starts_with("1\t")));
    assert!(lines.iter().any(|l| l.starts_with("MT\t")));
}

#[test]
fn test_chrom_whitelist_filters() {
    let out = TempDir::new().unwrap();
    let f = write_bed("chr1\t0\t100\nscaffold_1\t0\t50\nchrUn_gl000220\t0\t10\n");
    let mut opts = default_opts(&out);
    opts.chroms = Some(chroms(&["chr1", "chr2"]));
    let results = process_batch(&[f.path().to_path_buf()], &opts).unwrap();
    let content = read_output(&results[0]);
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "chr1\t0\t100");
}

#[test]
fn test_chrom_whitelist_bare_names() {
    // Works equally well with Ensembl-style bare chromosome names.
    let out = TempDir::new().unwrap();
    let f = write_bed("1\t0\t100\n2\t0\t50\nMT\t0\t10\nscaffold_1\t0\t5\n");
    let mut opts = default_opts(&out);
    opts.chroms = Some(chroms(&["1", "2", "MT"]));
    let results = process_batch(&[f.path().to_path_buf()], &opts).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.lines().count(), 3);
}

#[test]
fn test_no_filter_by_default() {
    // Without --chroms, scaffolds and unusual names pass through.
    let out = TempDir::new().unwrap();
    let f = write_bed("chr1\t0\t100\nscaffold_1\t0\t50\n2L\t0\t200\n");
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.lines().count(), 3);
}

#[test]
fn test_sort_output() {
    let out = TempDir::new().unwrap();
    let f = write_bed("chr2\t0\t100\nchr1\t500\t600\nchr1\t0\t100\n");
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines[0], "chr1\t0\t100");
    assert_eq!(lines[1], "chr1\t500\t600");
    assert_eq!(lines[2], "chr2\t0\t100");
}

#[test]
fn test_deduplication() {
    let out = TempDir::new().unwrap();
    let f = write_bed("chr1\t0\t100\nchr1\t0\t100\nchr1\t100\t200\n");
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.lines().count(), 2);
    assert_eq!(results[0].skipped, 1);
}

#[test]
fn test_malformed_rows_skipped() {
    let out = TempDir::new().unwrap();
    // start >= end is malformed
    let f = write_bed("chr1\t200\t100\nchr1\t0\t100\n");
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.lines().count(), 1);
    assert_eq!(results[0].skipped, 1);
}

#[test]
fn test_comments_and_headers_skipped() {
    let out = TempDir::new().unwrap();
    let f = write_bed(
        "# This is a comment\ntrack name=test\nbrowser position chr1:1-1000\nchr1\t0\t100\n",
    );
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.lines().count(), 1);
    assert_eq!(content.trim(), "chr1\t0\t100");
}

#[test]
fn test_extra_columns() {
    let out = TempDir::new().unwrap();
    let f = write_bed("chr1\t0\t100\tpeak1\t500\t+\n");
    let mut opts = default_opts(&out);
    opts.extra_columns = vec![3, 5]; // name, strand
    let results = process_batch(&[f.path().to_path_buf()], &opts).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.trim(), "chr1\t0\t100\tpeak1\t+");
}

#[test]
fn test_blacklist_subtract() {
    let out = TempDir::new().unwrap();
    let f = write_bed("chr1\t0\t100\nchr1\t500\t600\nchr2\t0\t100\n");

    let mut bl = tempfile::Builder::new()
        .suffix(".bed")
        .tempfile()
        .unwrap();
    write!(bl, "chr1\t50\t150\n").unwrap(); // overlaps first record

    let mut opts = default_opts(&out);
    opts.blacklist = Some(bl.path().to_path_buf());

    let results = process_batch(&[f.path().to_path_buf()], &opts).unwrap();
    let content = read_output(&results[0]);
    let lines: Vec<&str> = content.lines().collect();
    // chr1:0-100 overlaps blacklist chr1:50-150 → removed
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|l| !l.starts_with("chr1\t0\t100")));
}

#[test]
fn test_split_by_column() {
    let out = TempDir::new().unwrap();
    // BED with state label in col 3
    let f = write_bed("chr1\t0\t100\tactive\nchr1\t200\t300\trepressed\nchr2\t0\t100\tactive\n");

    let mut opts = default_opts(&out);
    opts.split_on = Some(3);

    let results = process_batch(&[f.path().to_path_buf()], &opts).unwrap();
    assert_eq!(results[0].outputs.len(), 2);

    // Find the "active" output
    let active_path = results[0]
        .outputs
        .iter()
        .find(|p| p.to_string_lossy().contains("active"))
        .unwrap();
    let content = fs::read_to_string(active_path).unwrap();
    assert_eq!(content.lines().count(), 2);

    let repressed_path = results[0]
        .outputs
        .iter()
        .find(|p| p.to_string_lossy().contains("repressed"))
        .unwrap();
    let content = fs::read_to_string(repressed_path).unwrap();
    assert_eq!(content.lines().count(), 1);
}

#[test]
fn test_batch_multiple_files() {
    let out = TempDir::new().unwrap();
    let f1 = write_bed("chr1\t0\t100\n");
    let f2 = write_bed("chr2\t0\t200\n");
    let results =
        process_batch(&[f1.path().to_path_buf(), f2.path().to_path_buf()], &default_opts(&out))
            .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_explicit_format_override() {
    let out = TempDir::new().unwrap();
    // File has .txt extension but contains BED data — force format
    let mut f = tempfile::Builder::new()
        .suffix(".txt")
        .tempfile()
        .unwrap();
    write!(f, "chr1\t0\t100\n").unwrap();

    let mut opts = default_opts(&out);
    opts.format = Some(Format::Bed);

    let results = process_batch(&[f.path().to_path_buf()], &opts).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.trim(), "chr1\t0\t100");
}

#[test]
fn test_gff_format() {
    let out = TempDir::new().unwrap();
    let mut f = tempfile::Builder::new()
        .suffix(".gff3")
        .tempfile()
        .unwrap();
    // GFF3: 1-based closed → 0-based half-open; chrom passes through as-is
    write!(f, "## gff-version 3\n1\t.\tgene\t1\t1000\t.\t+\t.\tID=g1\n").unwrap();

    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.trim(), "1\t0\t1000");
}

#[test]
fn test_vcf_format() {
    let out = TempDir::new().unwrap();
    let mut f = tempfile::Builder::new()
        .suffix(".vcf")
        .tempfile()
        .unwrap();
    write!(
        f,
        "##fileformat=VCFv4.1\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\nchr1\t101\t.\tACG\tA\t.\tPASS\t.\n"
    )
    .unwrap();

    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    // VCF POS=101 → start=100, REF="ACG" (len=3) → end=103
    assert_eq!(content.trim(), "chr1\t100\t103");
}

#[test]
fn test_no_sort_flag() {
    let out = TempDir::new().unwrap();
    let f = write_bed("chr2\t0\t100\nchr1\t0\t100\n");
    let mut opts = default_opts(&out);
    opts.no_sort = true;
    let results = process_batch(&[f.path().to_path_buf()], &opts).unwrap();
    let content = read_output(&results[0]);
    let lines: Vec<&str> = content.lines().collect();
    // Order preserved (chr2 first since no sort)
    assert_eq!(lines[0], "chr2\t0\t100");
}

#[test]
fn test_no_clean_flag() {
    let out = TempDir::new().unwrap();
    let f = write_bed("chr1\t0\t100\nchr1\t0\t100\nchr1\t200\t100\n");
    let mut opts = default_opts(&out);
    opts.no_clean = true;
    let results = process_batch(&[f.path().to_path_buf()], &opts).unwrap();
    let content = read_output(&results[0]);
    // Without clean: duplicates and malformed rows not removed
    // But note: sort still reorders; all 3 rows should appear
    assert_eq!(content.lines().count(), 3);
    assert_eq!(results[0].skipped, 0);
}

#[test]
fn test_empty_file() {
    let out = TempDir::new().unwrap();
    let f = write_bed("");
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    assert!(content.is_empty());
}

#[test]
fn test_only_comments() {
    let out = TempDir::new().unwrap();
    let f = write_bed("# comment\n# another comment\ntrack name=x\n");
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    assert!(content.is_empty());
}

#[test]
fn test_windows_line_endings() {
    let out = TempDir::new().unwrap();
    let mut f = tempfile::Builder::new()
        .suffix(".bed")
        .tempfile()
        .unwrap();
    f.write_all(b"chr1\t0\t100\r\nchr2\t0\t200\r\n").unwrap();
    let results = process_batch(&[f.path().to_path_buf()], &default_opts(&out)).unwrap();
    let content = read_output(&results[0]);
    assert_eq!(content.lines().count(), 2);
}
