//! Minimal mzML -> openproteo-core `SpectrumRecord` adapter, used by
//! `vendor2mzml validate` so the conformance harness can be run against
//! arbitrary mzML files (including indexed mzML and gzipped mzML).
//!
//! Only fields the conformance harness inspects are populated:
//! `index`, `native_id`, `ms_level`, `polarity`, `retention_time_sec`,
//! `mz`, `intensity`, and `precursor` (presence only). Other fields are
//! left at their defaults.

use std::path::Path;

use mzdata::prelude::*;
use mzdata::spectrum::{RawSpectrum, ScanPolarity};
use mzdata::MzMLReader;

use openproteo_core::{Polarity, PrecursorInfo, SpectrumRecord};

/// Returns true if `path` has an extension suggesting an mzML file.
/// Accepts `.mzml`, `.mzML`, and `.mzml.gz` (any case).
pub fn looks_like_mzml(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".mzml") || lower.ends_with(".mzml.gz")
}

/// Decode an mzML (or mzML.gz) file into a vector of `SpectrumRecord`s.
pub fn read_mzml_records(path: &Path) -> openproteo_io::Result<Vec<SpectrumRecord>> {
    let reader = MzMLReader::open_path(path)
        .map_err(|e| openproteo_io::Error::Mzml(e.to_string()))?;
    let mut out = Vec::new();
    for (i, spectrum) in reader.enumerate() {
        let raw: RawSpectrum = spectrum.into();
        out.push(spectrum_to_record(i, raw));
    }
    Ok(out)
}

fn spectrum_to_record(stream_index: usize, s: RawSpectrum) -> SpectrumRecord {
    let native_id = s.id().to_string();
    let ms_level = u32::from(s.ms_level());
    let rt_min = s.start_time();
    let rt_sec = rt_min * 60.0;
    let polarity = match s.polarity() {
        ScanPolarity::Positive => Some(Polarity::Positive),
        ScanPolarity::Negative => Some(Polarity::Negative),
        ScanPolarity::Unknown => None,
    };
    let mz: Vec<f64> = s.mzs().into_owned();
    let intensity: Vec<f32> = s.intensities().into_owned();
    let precursor = s.precursor().map(|p| {
        let ion = p.ions.first();
        PrecursorInfo {
            target_mz: None,
            selected_mz: ion.map(|x| x.mz),
            isolation_width: None,
            charge: ion.and_then(|x| x.charge),
            intensity: ion.map(|x| f64::from(x.intensity)),
            collision_energy: None,
            ce_is_nce: false,
            precursor_native_id: None,
            activation: None,
            analyzer: None,
        }
    });

    SpectrumRecord {
        index: s.index(),
        scan_number: stream_index as u32 + 1,
        native_id,
        ms_level,
        polarity,
        scan_mode: None,
        analyzer: None,
        filter: None,
        retention_time_sec: rt_sec,
        total_ion_current: None,
        base_peak_mz: None,
        base_peak_intensity: None,
        low_mz: None,
        high_mz: None,
        ion_injection_time_ms: None,
        inv_mobility: None,
        precursor,
        mz,
        intensity,
        inv_mobility_per_peak: None,
    }
}
