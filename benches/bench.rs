use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use getinbed::ops::{blacklist::BlacklistIndex, clean::clean, sort::sort};
use getinbed::parse::{parse_from_bytes, Format, Record};
use std::io::Write;
use tempfile::NamedTempFile;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_record(chrom: &str, start: u64, end: u64) -> Record {
    Record {
        chrom: chrom.to_string(),
        start,
        end,
        raw: vec![chrom.to_string(), start.to_string(), end.to_string()],
        chrom_col: 0,
        start_col: 1,
        end_col: 2,
    }
}

/// Generate `n` BED3 lines spread across the standard autosomes.
fn make_bed_bytes(n: usize) -> Vec<u8> {
    let chroms = [
        "chr1", "chr2", "chr3", "chr4", "chr5", "chr6", "chr7", "chr8", "chr9", "chr10",
        "chr11", "chr12", "chr13", "chr14", "chr15", "chr16", "chr17", "chr18", "chr19",
        "chr20", "chr21", "chr22",
    ];
    let mut buf = Vec::with_capacity(n * 24);
    for i in 0..n {
        let chrom = chroms[i % chroms.len()];
        let start = (i as u64) * 200;
        let end = start + 150;
        writeln!(buf, "{chrom}\t{start}\t{end}").unwrap();
    }
    buf
}

fn make_records(n: usize) -> Vec<Record> {
    let chroms = [
        "chr1", "chr5", "chr10", "chr22", "chrX", "chrM", "chr3", "chr17",
    ];
    (0..n)
        .map(|i| {
            let chrom = chroms[i % chroms.len()];
            let start = ((i as u64).wrapping_mul(1_000_003)) % 200_000_000;
            let end = start + 150;
            make_record(chrom, start, end)
        })
        .collect()
}

// ── Parse benchmarks ─────────────────────────────────────────────────────────

fn bench_parse_bed(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse/bed");
    for n in [10_000, 100_000, 1_000_000] {
        let data = make_bed_bytes(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &data, |b, data| {
            b.iter(|| parse_from_bytes(black_box(data), Format::Bed))
        });
    }
    group.finish();
}

fn bench_parse_gff(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse/gff");
    let n = 100_000usize;
    let mut buf: Vec<u8> = b"## gff-version 3\n".to_vec();
    for i in 0..n {
        let start = i as u64 * 200 + 1; // 1-based
        let end = start + 149;
        writeln!(buf, "chr1\t.\tgene\t{start}\t{end}\t.\t+\t.\tID=g{i}").unwrap();
    }
    group.throughput(Throughput::Elements(n as u64));
    group.bench_function(BenchmarkId::from_parameter(n), |b| {
        b.iter(|| parse_from_bytes(black_box(&buf), Format::Gff3))
    });
    group.finish();
}

fn bench_parse_vcf(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse/vcf");
    let n = 100_000usize;
    let mut buf: Vec<u8> =
        b"##fileformat=VCFv4.1\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n".to_vec();
    for i in 0..n {
        let pos = i as u64 * 10 + 1; // 1-based
        writeln!(buf, "chr1\t{pos}\t.\tA\tT\t.\tPASS\t.").unwrap();
    }
    group.throughput(Throughput::Elements(n as u64));
    group.bench_function(BenchmarkId::from_parameter(n), |b| {
        b.iter(|| parse_from_bytes(black_box(&buf), Format::Vcf))
    });
    group.finish();
}

// ── Sort benchmarks ───────────────────────────────────────────────────────────

fn bench_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("ops/sort");
    for n in [10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || make_records(n),
                |mut records| sort(black_box(&mut records)),
                criterion::BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

// ── Clean benchmarks ──────────────────────────────────────────────────────────

fn bench_clean(c: &mut Criterion) {
    let mut group = c.benchmark_group("ops/clean");
    // ~10% duplicates
    for n in [10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let mut records = make_records(n);
                    // Inject duplicates every 10th record
                    for i in (10..n).step_by(10) {
                        records[i] = records[i - 10].clone();
                    }
                    records
                },
                |records| clean(black_box(records)),
                criterion::BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

// ── Blacklist benchmarks ──────────────────────────────────────────────────────

fn bench_blacklist(c: &mut Criterion) {
    let mut group = c.benchmark_group("ops/blacklist");

    // Build a blacklist file with 1000 regions spread across chr1
    let mut bl_file = NamedTempFile::new().unwrap();
    for i in 0..1000usize {
        let start = i * 50_000;
        let end = start + 1000;
        writeln!(bl_file, "chr1\t{start}\t{end}").unwrap();
    }

    let bl = BlacklistIndex::load(bl_file.path()).unwrap();
    let records = make_records(100_000);

    group.throughput(Throughput::Elements(records.len() as u64));
    group.bench_function("100k_records_1k_bl", |b| {
        b.iter(|| {
            let mut count = 0usize;
            for r in &records {
                if !bl.overlaps(black_box(&r.chrom), black_box(r.start), black_box(r.end)) {
                    count += 1;
                }
            }
            count
        })
    });

    group.finish();
}

// ── Pipeline benchmark ────────────────────────────────────────────────────────

fn bench_pipeline(c: &mut Criterion) {
    let n = 500_000;
    let data = make_bed_bytes(n);
    c.bench_function("pipeline/parse+clean+sort_500k", |b| {
        b.iter(|| {
            let mut records = parse_from_bytes(black_box(&data), Format::Bed);
            let (mut records, _skipped) = clean(records);
            sort(&mut records);
            records
        })
    });
}

criterion_group!(
    benches,
    bench_parse_bed,
    bench_parse_gff,
    bench_parse_vcf,
    bench_sort,
    bench_clean,
    bench_blacklist,
    bench_pipeline,
);
criterion_main!(benches);
