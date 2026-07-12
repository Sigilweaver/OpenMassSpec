//! `vendor2mzml`: convert Thermo / Bruker / Waters / Agilent / SCIEX acquisitions to mzML,
//! print a one-pass summary of one, or validate a vendor or mzML input
//! against the openmassspec-core conformance harness.

mod mzml_reader;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::{Args, Parser, Subcommand, ValueEnum};
use flate2::write::GzEncoder;
use flate2::Compression;
use openmassspec_core::conformance::assert_iter_invariants;
use openmassspec_core::SpectrumRecord;
use openmassspec_io::{
    collect, collect_centroided, convert_to_mzml_writer, convert_to_mzml_writer_centroided,
    detect_format, Detected,
};

#[derive(Parser, Debug)]
#[command(
    name = "vendor2mzml",
    version,
    long_version = long_version_str(),
    about = "Convert Thermo / Bruker / Waters / Agilent / SCIEX raw acquisitions to mzML."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Convert a vendor acquisition to mzML.
    Convert(ConvertArgs),
    /// Print a one-pass summary of a vendor acquisition without writing mzML.
    Info(InfoArgs),
    /// Run the openmassspec-core conformance harness against a vendor
    /// acquisition or an mzML file. Exits 0 on pass, 2 on unsupported
    /// input, 3 on conformance failure.
    Validate(ValidateArgs),
}

#[derive(Args, Debug)]
struct ConvertArgs {
    /// Input path: a `.raw` file (Thermo) or a `.d/` / `.raw/` bundle
    /// directory (Bruker, Waters).
    input: PathBuf,
    /// Output mzML path. Use a `.mzML.gz` extension to write gzipped
    /// output (auto-detected from the file name).
    output: PathBuf,
    /// Emit indexed mzML (writes an `<indexList>` and SHA-1 hash).
    #[arg(long)]
    indexed: bool,
    /// Centroid every profile-mode spectrum before writing (local-maxima
    /// peak picking; already-centroid spectra pass through unchanged).
    #[arg(long)]
    centroid: bool,
    /// Discard picked peaks below this intensity when `--centroid` is
    /// set. Ignored otherwise.
    #[arg(long)]
    centroid_min_intensity: Option<f32>,
    /// Emit timing and record counts on stderr in the chosen format.
    #[arg(long, value_enum)]
    profile: Option<ProfileFormat>,
}

#[derive(Args, Debug)]
struct ValidateArgs {
    /// Input path: a vendor acquisition (`.raw` / `.d/` / `.raw/`) or
    /// an mzML file (`.mzml` or `.mzml.gz`).
    input: PathBuf,
    /// Emit the result as a single JSON object on stdout.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct InfoArgs {
    /// Input path.
    input: PathBuf,
    /// Centroid every profile-mode spectrum before summarizing.
    #[arg(long)]
    centroid: bool,
    /// Discard picked peaks below this intensity when `--centroid` is
    /// set. Ignored otherwise.
    #[arg(long)]
    centroid_min_intensity: Option<f32>,
    /// Emit the summary as a single JSON object on stdout.
    #[arg(long)]
    json: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum ProfileFormat {
    Json,
    Text,
}

const LONG_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\n  openmassspec-core ",);

fn long_version_str() -> &'static str {
    use std::sync::OnceLock;
    static S: OnceLock<String> = OnceLock::new();
    S.get_or_init(|| format!("{LONG_VERSION}{}", openmassspec_core::VERSION))
        .as_str()
}

fn main() -> ExitCode {
    match Cli::parse().cmd {
        Cmd::Convert(args) => run_convert(args),
        Cmd::Info(args) => run_info(args),
        Cmd::Validate(args) => run_validate(args),
    }
}

// ---------------------------------------------------------------------
// convert
// ---------------------------------------------------------------------

fn run_convert(args: ConvertArgs) -> ExitCode {
    let Some(detected) = detect_format(&args.input) else {
        eprintln!(
            "error: {} does not look like a supported vendor format",
            args.input.display()
        );
        eprintln!("  (recognized: Thermo .raw, Bruker .d/, Waters .raw/)");
        return ExitCode::from(1);
    };
    eprintln!(
        "detected: {} <- {}",
        detected.format.name(),
        args.input.display()
    );

    let t_start = Instant::now();

    let mut writer = match open_writer(&args.output) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: open output: {e}");
            return ExitCode::from(1);
        }
    };
    let convert_result = if args.centroid {
        convert_to_mzml_writer_centroided(
            detected,
            &mut writer,
            args.indexed,
            args.centroid_min_intensity,
        )
    } else {
        convert_to_mzml_writer(detected, &mut writer, args.indexed)
    };
    if let Err(e) = convert_result {
        eprintln!("error: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = writer.flush() {
        eprintln!("error: flush: {e}");
        return ExitCode::from(1);
    }
    report(
        args.profile,
        &args.output,
        args.indexed,
        0,
        false,
        t_start.elapsed().as_secs_f64(),
    );
    ExitCode::SUCCESS
}

fn open_writer(path: &Path) -> std::io::Result<Box<dyn Write>> {
    let f = File::create(path)?;
    let buf = BufWriter::with_capacity(2 << 20, f);
    let is_gz = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".gz"));
    if is_gz {
        Ok(Box::new(GzEncoder::new(buf, Compression::default())))
    } else {
        Ok(Box::new(buf))
    }
}

fn report(
    profile: Option<ProfileFormat>,
    output: &Path,
    indexed: bool,
    spectra: usize,
    validated: bool,
    elapsed_sec: f64,
) {
    match profile {
        None => {
            let tag = if indexed { " (indexed)" } else { "" };
            eprintln!("wrote {} in {:.1}s{tag}", output.display(), elapsed_sec);
        }
        Some(ProfileFormat::Text) => {
            eprintln!(
                "output={} indexed={} validated={} spectra={} elapsed_sec={:.3}",
                output.display(),
                indexed,
                validated,
                spectra,
                elapsed_sec
            );
        }
        Some(ProfileFormat::Json) => {
            let out_str = output.display().to_string();
            eprintln!(
                "{{\"output\":{:?},\"indexed\":{},\"validated\":{},\"spectra\":{},\"elapsed_sec\":{:.3}}}",
                out_str, indexed, validated, spectra, elapsed_sec
            );
        }
    }
}

// ---------------------------------------------------------------------
// info
// ---------------------------------------------------------------------

fn run_info(args: InfoArgs) -> ExitCode {
    let Some(detected) = detect_format(&args.input) else {
        eprintln!(
            "error: {} does not look like a supported vendor format",
            args.input.display()
        );
        return ExitCode::from(1);
    };
    let t_start = Instant::now();
    let collect_result = if args.centroid {
        collect_centroided(detected.clone(), args.centroid_min_intensity)
    } else {
        collect(detected.clone())
    };
    let (records, metadata) = match collect_result {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: collect: {e}");
            return ExitCode::from(1);
        }
    };
    let summary = summarize(
        &detected,
        &metadata,
        &records,
        t_start.elapsed().as_secs_f64(),
    );

    if args.json {
        println!("{}", summary.to_json());
    } else {
        summary.print_text();
    }
    ExitCode::SUCCESS
}

struct Summary {
    vendor: &'static str,
    path: PathBuf,
    instrument: String,
    instrument_accession: &'static str,
    source_format: String,
    native_id_format: String,
    software: String,
    spectrum_count: usize,
    ms_levels: BTreeMap<u32, usize>,
    polarity_counts: BTreeMap<String, usize>,
    rt_min_sec: f64,
    rt_max_sec: f64,
    decode_elapsed_sec: f64,
}

impl Summary {
    fn print_text(&self) {
        println!("vendor:        {}", self.vendor);
        println!("path:          {}", self.path.display());
        println!(
            "instrument:    {} ({})",
            self.instrument, self.instrument_accession
        );
        println!("source format: {}", self.source_format);
        println!("native id:     {}", self.native_id_format);
        println!("software:      {}", self.software);
        println!("spectra:       {}", self.spectrum_count);
        for (lvl, n) in &self.ms_levels {
            println!("  ms{lvl}:         {n}");
        }
        for (pol, n) in &self.polarity_counts {
            println!("  {pol:>3}:         {n}");
        }
        if self.spectrum_count > 0 {
            println!(
                "rt range:      {:.3} - {:.3} s ({:.2} - {:.2} min)",
                self.rt_min_sec,
                self.rt_max_sec,
                self.rt_min_sec / 60.0,
                self.rt_max_sec / 60.0
            );
        }
        println!("decode time:   {:.2} s", self.decode_elapsed_sec);
    }

    fn to_json(&self) -> String {
        let ms = self
            .ms_levels
            .iter()
            .map(|(k, v)| format!("\"{k}\":{v}"))
            .collect::<Vec<_>>()
            .join(",");
        let pol = self
            .polarity_counts
            .iter()
            .map(|(k, v)| format!("\"{k}\":{v}"))
            .collect::<Vec<_>>()
            .join(",");
        let path_str = self.path.display().to_string();
        format!(
            "{{\"vendor\":\"{}\",\"path\":{:?},\"instrument\":{:?},\
            \"instrument_accession\":\"{}\",\"source_format\":{:?},\
            \"native_id_format\":{:?},\"software\":{:?},\
            \"spectrum_count\":{},\"ms_levels\":{{{}}},\
            \"polarity_counts\":{{{}}},\"rt_min_sec\":{:.6},\
            \"rt_max_sec\":{:.6},\"decode_elapsed_sec\":{:.3}}}",
            self.vendor,
            path_str,
            self.instrument,
            self.instrument_accession,
            self.source_format,
            self.native_id_format,
            self.software,
            self.spectrum_count,
            ms,
            pol,
            self.rt_min_sec,
            self.rt_max_sec,
            self.decode_elapsed_sec,
        )
    }
}

fn summarize(
    detected: &Detected,
    meta: &openmassspec_core::RunMetadata,
    records: &[SpectrumRecord],
    decode_elapsed_sec: f64,
) -> Summary {
    let mut ms_levels: BTreeMap<u32, usize> = BTreeMap::new();
    let mut polarity_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut rt_min = f64::INFINITY;
    let mut rt_max = f64::NEG_INFINITY;
    for r in records {
        *ms_levels.entry(r.ms_level).or_insert(0) += 1;
        let key = match r.polarity {
            Some(openmassspec_core::Polarity::Positive) => "pos",
            Some(openmassspec_core::Polarity::Negative) => "neg",
            None => "unk",
        };
        *polarity_counts.entry(key.to_string()).or_insert(0) += 1;
        if r.retention_time_sec < rt_min {
            rt_min = r.retention_time_sec;
        }
        if r.retention_time_sec > rt_max {
            rt_max = r.retention_time_sec;
        }
    }
    if !rt_min.is_finite() {
        rt_min = 0.0;
        rt_max = 0.0;
    }
    Summary {
        vendor: detected.format.name(),
        path: detected.path.clone(),
        instrument: meta.instrument.name.clone(),
        instrument_accession: meta.instrument.accession,
        source_format: format!(
            "{} ({})",
            meta.source_file_format.name, meta.source_file_format.accession
        ),
        native_id_format: format!(
            "{} ({})",
            meta.native_id_format.name, meta.native_id_format.accession
        ),
        software: format!("{} {}", meta.software_name, meta.software_version),
        spectrum_count: records.len(),
        ms_levels,
        polarity_counts,
        rt_min_sec: rt_min,
        rt_max_sec: rt_max,
        decode_elapsed_sec,
    }
}

// ---------------------------------------------------------------------
// validate
// ---------------------------------------------------------------------

fn run_validate(args: ValidateArgs) -> ExitCode {
    let t_start = Instant::now();

    // 1) Vendor input wins if detect_format recognizes it.
    let records_res: Result<(Vec<SpectrumRecord>, &'static str), String> =
        if let Some(detected) = detect_format(&args.input) {
            let vendor = detected.format.name();
            match collect(detected) {
                Ok((records, _meta)) => Ok((records, vendor)),
                Err(e) => Err(format!("collect: {e}")),
            }
        } else if mzml_reader::looks_like_mzml(&args.input) {
            match mzml_reader::read_mzml_records(&args.input) {
                Ok(records) => Ok((records, "mzML")),
                Err(e) => Err(format!("read mzML: {e}")),
            }
        } else {
            emit_validate_result(
                args.json,
                &args.input,
                "unknown",
                0,
                Err("input is not a recognized vendor format or mzML file"),
                0.0,
            );
            return ExitCode::from(2);
        };
    let (records, kind) = match records_res {
        Ok(p) => p,
        Err(e) => {
            emit_validate_result(
                args.json,
                &args.input,
                "error",
                0,
                Err(&e),
                t_start.elapsed().as_secs_f64(),
            );
            return ExitCode::from(1);
        }
    };

    let count = records.len();
    let result = assert_iter_invariants(records);
    let elapsed = t_start.elapsed().as_secs_f64();
    match result {
        Ok(n) => {
            emit_validate_result(args.json, &args.input, kind, n, Ok(()), elapsed);
            ExitCode::SUCCESS
        }
        Err(e) => {
            let msg = format!("{}", e);
            let variant = conformance_variant(&e);
            emit_validate_result_with_variant(
                args.json,
                &args.input,
                kind,
                count,
                Err(&msg),
                Some(variant),
                elapsed,
            );
            ExitCode::from(3)
        }
    }
}

fn conformance_variant(e: &openmassspec_core::conformance::ConformanceError) -> &'static str {
    use openmassspec_core::conformance::ConformanceError as C;
    match e {
        C::PeakArrayLengthMismatch { .. } => "PeakArrayLengthMismatch",
        C::MobilityArrayLengthMismatch { .. } => "MobilityArrayLengthMismatch",
        C::TicMismatch { .. } => "TicMismatch",
        C::BasePeakIntensityMismatch { .. } => "BasePeakIntensityMismatch",
        C::MissingPrecursor { .. } => "MissingPrecursor",
        C::RetentionTimeNonMonotonic { .. } => "RetentionTimeNonMonotonic",
        C::IndexSequence { .. } => "IndexSequence",
        C::EmptySpectrum { .. } => "EmptySpectrum",
    }
}

fn emit_validate_result(
    json: bool,
    input: &Path,
    kind: &str,
    spectrum_count: usize,
    result: Result<(), &str>,
    elapsed_sec: f64,
) {
    emit_validate_result_with_variant(json, input, kind, spectrum_count, result, None, elapsed_sec);
}

fn emit_validate_result_with_variant(
    json: bool,
    input: &Path,
    kind: &str,
    spectrum_count: usize,
    result: Result<(), &str>,
    variant: Option<&'static str>,
    elapsed_sec: f64,
) {
    let ok = result.is_ok();
    let err = result.err().unwrap_or("");
    if json {
        let path_str = input.display().to_string();
        let variant_field = match variant {
            Some(v) => format!(",\"error_kind\":\"{v}\""),
            None => String::new(),
        };
        let err_field = if ok {
            String::new()
        } else {
            format!(",\"error\":{:?}{}", err, variant_field)
        };
        println!(
            "{{\"ok\":{},\"input\":{:?},\"kind\":\"{}\",\"spectrum_count\":{},\"elapsed_sec\":{:.3}{}}}",
            ok, path_str, kind, spectrum_count, elapsed_sec, err_field
        );
    } else if ok {
        eprintln!(
            "ok: {} ({} spectra, {:.2}s, {})",
            input.display(),
            spectrum_count,
            elapsed_sec,
            kind
        );
    } else {
        eprintln!("FAIL: {} ({}): {}", input.display(), kind, err);
    }
}
