//! Minimal mzML -> openmassspec-core `SpectrumRecord` adapter, used by
//! `vendor2mzml validate` so the conformance harness can be run against
//! arbitrary mzML files (including indexed mzML and gzipped mzML).
//!
//! Only fields the conformance harness inspects are populated:
//! `index`, `native_id`, `ms_level`, `polarity`, `retention_time_sec`,
//! `mz`, `intensity`, and `precursor` (isolation target m/z, selected ion
//! m/z, charge and intensity, and the precursor `spectrumRef`). Other
//! fields are left at their defaults.

use std::path::Path;

use mzdata::prelude::*;
use mzdata::spectrum::bindata::ArrayRetrievalError;
use mzdata::spectrum::{IsolationWindowState, RawSpectrum, ScanPolarity};
use mzdata::MzMLReader;

use openmassspec_core::{Polarity, PrecursorInfo, SpectrumRecord};

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
pub fn read_mzml_records(path: &Path) -> openmassspec_io::Result<Vec<SpectrumRecord>> {
    let reader =
        MzMLReader::open_path(path).map_err(|e| openmassspec_io::Error::Mzml(e.to_string()))?;
    let mut out = Vec::new();
    for (i, spectrum) in reader.enumerate() {
        let raw: RawSpectrum = spectrum.into();
        out.push(spectrum_to_record(i, raw)?);
    }
    Ok(out)
}

/// Decode the m/z and intensity arrays of one spectrum.
///
/// mzML makes `binaryDataArrayList` optional (`minOccurs="0"` on
/// `SpectrumType`), and the openmassspec-core writer omits it for spectra
/// with no peaks. A spectrum carrying neither array is therefore read as
/// an empty spectrum. A spectrum carrying only one of the two arrays, or
/// an array that fails to decode, is an error: the harness must not see a
/// silently truncated spectrum.
fn peak_arrays(s: &RawSpectrum) -> openmassspec_io::Result<(Vec<f64>, Vec<f32>)> {
    let mz = s.arrays.mzs();
    let intensity = s.arrays.intensities();
    let fail = |what: String| {
        openmassspec_io::Error::Mzml(format!(
            "spectrum {:?} (index {}): {what}",
            s.id(),
            s.index()
        ))
    };
    match (mz, intensity) {
        (Ok(mz), Ok(intensity)) => Ok((mz.into_owned(), intensity.into_owned())),
        (Err(ArrayRetrievalError::NotFound(_)), Err(ArrayRetrievalError::NotFound(_))) => {
            Ok((Vec::new(), Vec::new()))
        }
        (Ok(_), Err(ArrayRetrievalError::NotFound(_))) => {
            Err(fail("has an m/z array but no intensity array".into()))
        }
        (Err(ArrayRetrievalError::NotFound(_)), Ok(_)) => {
            Err(fail("has an intensity array but no m/z array".into()))
        }
        (Err(e), _) | (_, Err(e)) => Err(fail(format!("cannot decode binary array: {e}"))),
    }
}

fn spectrum_to_record(
    stream_index: usize,
    s: RawSpectrum,
) -> openmassspec_io::Result<SpectrumRecord> {
    let native_id = s.id().to_string();
    let ms_level = u32::from(s.ms_level());
    let rt_min = s.start_time();
    let rt_sec = rt_min * 60.0;
    let polarity = match s.polarity() {
        ScanPolarity::Positive => Some(Polarity::Positive),
        ScanPolarity::Negative => Some(Polarity::Negative),
        ScanPolarity::Unknown => None,
    };
    let (mz, intensity) = peak_arrays(&s)?;
    let precursor = s.precursor().map(|p| {
        let ion = p.ions.first();
        // The writer emits `isolation window target m/z` without a
        // selectedIonList when only the target is known (e.g. Agilent
        // Q-TOF MS2), so the target is read here too; dropping it would
        // make the harness report a missing precursor.
        let window = &p.isolation_window;
        let target_mz = (!matches!(
            window.flags,
            IsolationWindowState::Unknown | IsolationWindowState::NoIsolation
        ))
        .then_some(f64::from(window.target));
        PrecursorInfo {
            target_mz,
            selected_mz: ion.map(|x| x.mz),
            isolation_width: None,
            charge: ion.and_then(|x| x.charge),
            intensity: ion.map(|x| f64::from(x.intensity)),
            collision_energy: None,
            ce_is_nce: false,
            precursor_native_id: p.precursor_id.clone(),
            activation: None,
            analyzer: None,
            ccs: None,
        }
    });

    Ok(SpectrumRecord {
        extra: ::std::collections::BTreeMap::new(),
        acquisition_event_id: None,
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
        faims_cv: None,
        precursor,
        mz,
        intensity,
        inv_mobility_per_peak: None,
    })
}
