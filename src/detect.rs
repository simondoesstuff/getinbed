use crate::error::GetinbedError;
use crate::parse::Format;
use std::path::Path;

pub fn detect_format(path: &Path) -> Result<Format, GetinbedError> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let lower = name.to_lowercase();

    // Strip .gz suffix for extension matching
    let base = lower.strip_suffix(".gz").unwrap_or(&lower);

    if base.ends_with(".bed") {
        return Ok(Format::Bed);
    }
    if base.ends_with(".narrowpeak") {
        return Ok(Format::NarrowPeak);
    }
    if base.ends_with(".broadpeak") {
        return Ok(Format::BroadPeak);
    }
    if base.ends_with(".bedgraph") {
        return Ok(Format::BedGraph);
    }
    if base.ends_with(".gff3") || base.ends_with(".gff") {
        return Ok(Format::Gff3);
    }
    if base.ends_with(".gtf") {
        return Ok(Format::Gtf);
    }
    if base.ends_with(".vcf") {
        // Check for co-located .tbi index
        let tbi = path.with_extension("gz.tbi");
        let tbi_alt = {
            let mut p = path.to_path_buf();
            let fname = format!("{}.tbi", name);
            p.set_file_name(fname);
            p
        };
        if lower.ends_with(".vcf.gz") && (tbi.exists() || tbi_alt.exists()) {
            return Ok(Format::VcfTabix);
        }
        return Ok(Format::Vcf);
    }
    if base.ends_with(".bigbed") || base.ends_with(".bb") {
        return Ok(Format::BigBed);
    }
    if base.ends_with(".bignarrowpeak") {
        return Ok(Format::BigNarrowPeak);
    }
    if base.ends_with(".bigbroadpeak") {
        return Ok(Format::BigBroadPeak);
    }
    if base.ends_with(".biggenepred") {
        return Ok(Format::BigGenePred);
    }
    if base.ends_with(".bigpsl") {
        return Ok(Format::BigPsl);
    }
    if base.ends_with(".bigbarchart") {
        return Ok(Format::BigBarChart);
    }
    if base.ends_with(".genepred") {
        return Ok(Format::GenePred);
    }
    if base.ends_with(".psl") {
        return Ok(Format::Psl);
    }

    Err(GetinbedError::UnknownFormat(
        path.extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_else(|| "(no extension)".into()),
    ))
}

pub fn parse_format_str(s: &str) -> Result<Format, GetinbedError> {
    match s.to_lowercase().as_str() {
        "bed" => Ok(Format::Bed),
        "narrowpeak" => Ok(Format::NarrowPeak),
        "broadpeak" => Ok(Format::BroadPeak),
        "bedgraph" => Ok(Format::BedGraph),
        "gff3" | "gff" => Ok(Format::Gff3),
        "gtf" => Ok(Format::Gtf),
        "vcf" => Ok(Format::Vcf),
        "vcftabix" => Ok(Format::VcfTabix),
        "genepred" => Ok(Format::GenePred),
        "psl" => Ok(Format::Psl),
        "bigbed" | "bb" => Ok(Format::BigBed),
        "bignarrowpeak" => Ok(Format::BigNarrowPeak),
        "bigbroadpeak" => Ok(Format::BigBroadPeak),
        "biggenepred" => Ok(Format::BigGenePred),
        "bigpsl" => Ok(Format::BigPsl),
        "bigbarchart" => Ok(Format::BigBarChart),
        _ => Err(GetinbedError::UnknownFormat(s.to_string())),
    }
}
