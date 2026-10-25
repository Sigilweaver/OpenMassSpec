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
    // The mzML reader ends its iterator at the first parse error (a
    // truncated file, a corrupt payload) without saying so, so the number
    // of spectra read is checked against the count the file declares.
    let declared = reader.spectrum_count_hint();
    let mut out = Vec::new();
    for (i, spectrum) in reader.enumerate() {
        let raw: RawSpectrum = spectrum.into();
        out.push(spectrum_to_record(i, raw)?);
    }
    check_declared_count(declared, out.len())?;
    Ok(out)
}

/// The file's `<spectrumList count>` must be known and equal to the number
/// of spectra read.
fn check_declared_count(declared: Option<u64>, read: usize) -> openmassspec_io::Result<()> {
    match declared {
        Some(n) if n == read as u64 => Ok(()),
        Some(n) => Err(openmassspec_io::Error::Mzml(format!(
            "read {read} of the {n} spectra the file declares; the file is truncated or a \
             spectrum could not be parsed"
        ))),
        None => Err(openmassspec_io::Error::Mzml(format!(
            "the file declares no spectrum count, so {read} spectra read cannot be \
             checked for truncation"
        ))),
    }
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
        PrecursorInfo::default()
            .with_target_mz(target_mz)
            .with_selected_mz(ion.map(|x| x.mz))
            .with_charge(ion.and_then(|x| x.charge))
            .with_intensity(ion.map(|x| f64::from(x.intensity)))
            .with_precursor_native_id(p.precursor_id.clone())
    });

    Ok(
        SpectrumRecord::new(s.index(), stream_index as u32 + 1, native_id)
            .with_ms_level(ms_level)
            .with_polarity(polarity)
            .with_retention_time_sec(rt_sec)
            .with_precursor(precursor)
            .with_peaks(mz, intensity),
    )
}
