//! PyO3 bindings for `openmassspec-io`.
//!
//! Exposes a small, vendor-neutral surface:
//!
//! * `detect(path) -> str | None`
//! * `to_mzml(input, output, *, indexed=True, centroid=False, centroid_min_intensity=None) -> None`
//! * `iter_spectra(path, *, centroid=False, centroid_min_intensity=None) -> Iterator[Spectrum]`
//! * `read_arrow(path, batch_size=1024, *, centroid=False, centroid_min_intensity=None) -> pyarrow.RecordBatchReader`
//!   (built when the `arrow` feature is enabled, which is the default).
//!
//! `centroid=True` centroids every profile-mode spectrum (local-maxima
//! peak picking via `openmassspec_core::Centroided`); already-centroid
//! spectra pass through unchanged. `centroid_min_intensity` discards
//! picked peaks below that noise floor and is ignored unless `centroid`
//! is set.
//!
//! All m/z, intensity, and inverse-mobility arrays handed to Python are
//! created via `numpy::PyArray1::from_vec_bound`, which transfers
//! ownership of the underlying Rust `Vec` to NumPy without copying any
//! element. The Python object becomes the sole owner.

#![forbid(unsafe_code)]
#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use numpy::PyArray1;
use openmassspec_core::{Activation, Polarity, PrecursorInfo, SpectrumRecord};
use openmassspec_io::{detect_format, Detected};
use pyo3::exceptions::{PyFileNotFoundError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

fn detected_or_err(path: &Path) -> PyResult<Detected> {
    if !path.exists() {
        return Err(PyFileNotFoundError::new_err(format!(
            "no such file or directory: {}",
            path.display()
        )));
    }
    detect_format(path).ok_or_else(|| {
        PyValueError::new_err(format!(
            "no supported vendor format detected at {}",
            path.display()
        ))
    })
}

/// Collect every spectrum from `detected` into a single `Vec`, optionally
/// centroiding every profile-mode spectrum first. Delegates to
/// `openmassspec_io::collect`/`collect_centroided`, which already know how
/// to construct each vendor's reader.
fn collect_records(
    detected: &Detected,
    centroid: bool,
    min_intensity: Option<f32>,
) -> openmassspec_io::Result<(Vec<SpectrumRecord>, openmassspec_core::RunMetadata)> {
    if centroid {
        openmassspec_io::collect_centroided(detected.clone(), min_intensity)
    } else {
        openmassspec_io::collect(detected.clone())
    }
}

fn map_err<E: std::fmt::Display>(e: E) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

fn polarity_str(p: Polarity) -> &'static str {
    match p {
        Polarity::Positive => "positive",
        Polarity::Negative => "negative",
    }
}

fn activation_str(a: Activation) -> &'static str {
    match a {
        Activation::HCD => "hcd",
        Activation::MPID => "mpid",
        Activation::ETD => "etd",
        Activation::CID => "cid",
        Activation::ECD => "ecd",
        Activation::IRMPD => "irmpd",
        Activation::PD => "pd",
        Activation::PQD => "pqd",
        Activation::UVPD => "uvpd",
        Activation::SID => "sid",
        Activation::EThcD => "ethcd",
    }
}

// ---------------------------------------------------------------------
// Spectrum (Python-facing record)
// ---------------------------------------------------------------------

/// One decoded spectrum, exposed to Python with NumPy-backed peak arrays.
#[pyclass(module = "openmassspec_io._openmassspec_io")]
pub struct Spectrum {
    rec: Option<SpectrumRecord>,
}

#[pymethods]
impl Spectrum {
    #[getter]
    fn index(&self) -> PyResult<usize> {
        self.rec_ref().map(|r| r.index)
    }
    #[getter]
    fn scan_number(&self) -> PyResult<u32> {
        self.rec_ref().map(|r| r.scan_number)
    }
    #[getter]
    fn native_id(&self) -> PyResult<String> {
        self.rec_ref().map(|r| r.native_id.clone())
    }
    #[getter]
    fn ms_level(&self) -> PyResult<u32> {
        self.rec_ref().map(|r| r.ms_level)
    }
    #[getter]
    fn retention_time_sec(&self) -> PyResult<f64> {
        self.rec_ref().map(|r| r.retention_time_sec)
    }
    #[getter]
    fn polarity(&self) -> PyResult<Option<&'static str>> {
        self.rec_ref().map(|r| r.polarity.map(polarity_str))
    }
    #[getter]
    fn total_ion_current(&self) -> PyResult<f64> {
        self.rec_ref().map(|r| r.effective_tic())
    }
    #[getter]
    fn base_peak_mz(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.effective_base_peak().map(|t| t.0))
    }
    #[getter]
    fn base_peak_intensity(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.effective_base_peak().map(|t| t.1))
    }
    #[getter]
    fn inv_mobility(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.inv_mobility)
    }
    #[getter]
    fn scan_mode(&self) -> PyResult<Option<&'static str>> {
        self.rec_ref().map(|r| {
            r.scan_mode.map(|m| match m {
                openmassspec_core::ScanMode::Centroid => "centroid",
                openmassspec_core::ScanMode::Profile => "profile",
            })
        })
    }
    #[getter]
    fn analyzer(&self) -> PyResult<Option<&'static str>> {
        self.rec_ref().map(|r| {
            r.analyzer.map(|a| match a {
                openmassspec_core::Analyzer::ITMS => "itms",
                openmassspec_core::Analyzer::TQMS => "tqms",
                openmassspec_core::Analyzer::SQMS => "sqms",
                openmassspec_core::Analyzer::TOFMS => "tofms",
                openmassspec_core::Analyzer::FTMS => "ftms",
                openmassspec_core::Analyzer::Sector => "sector",
            })
        })
    }
    #[getter]
    fn filter(&self) -> PyResult<Option<String>> {
        self.rec_ref().map(|r| r.filter.clone())
    }
    #[getter]
    fn ion_injection_time_ms(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.ion_injection_time_ms)
    }
    #[getter]
    fn low_mz(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.low_mz)
    }
    #[getter]
    fn high_mz(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.high_mz)
    }

    /// Zero-copy NumPy view over the m/z array (owned by NumPy after this
    /// access; the spectrum no longer holds the peaks).
    #[getter]
    fn mz<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let rec = self.rec.as_mut().ok_or_else(consumed_err)?;
        let v = std::mem::take(&mut rec.mz);
        Ok(PyArray1::from_vec(py, v))
    }

    #[getter]
    fn intensity<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyArray1<f32>>> {
        let rec = self.rec.as_mut().ok_or_else(consumed_err)?;
        let v = std::mem::take(&mut rec.intensity);
        Ok(PyArray1::from_vec(py, v))
    }

    #[getter]
    fn inv_mobility_per_peak<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyArray1<f32>>>> {
        let rec = self.rec.as_mut().ok_or_else(consumed_err)?;
        Ok(rec
            .inv_mobility_per_peak
            .take()
            .map(|v| PyArray1::from_vec(py, v)))
    }

    /// Precursor metadata for MS2+ spectra, as a `dict`, or `None`.
    #[getter]
    fn precursor<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        let rec = self.rec_ref()?;
        let Some(p) = &rec.precursor else {
            return Ok(None);
        };
        Ok(Some(precursor_to_dict(py, p)?))
    }

    fn __repr__(&self) -> PyResult<String> {
        let r = self.rec_ref()?;
        Ok(format!(
            "<Spectrum index={} ms_level={} native_id={:?} rt_sec={:.3} n_peaks={}>",
            r.index,
            r.ms_level,
            r.native_id,
            r.retention_time_sec,
            r.mz.len()
        ))
    }
}

impl Spectrum {
    fn rec_ref(&self) -> PyResult<&SpectrumRecord> {
        self.rec.as_ref().ok_or_else(consumed_err)
    }
}

fn consumed_err() -> PyErr {
    PyRuntimeError::new_err("spectrum peak arrays have already been consumed")
}

fn precursor_to_dict<'py>(py: Python<'py>, p: &PrecursorInfo) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("target_mz", p.target_mz)?;
    d.set_item("selected_mz", p.selected_mz)?;
    d.set_item("isolation_width", p.isolation_width)?;
    d.set_item("charge", p.charge)?;
    d.set_item("intensity", p.intensity)?;
    d.set_item("collision_energy", p.collision_energy)?;
    d.set_item("ce_is_nce", p.ce_is_nce)?;
    d.set_item("precursor_native_id", p.precursor_native_id.clone())?;
    d.set_item("activation", p.activation.map(activation_str))?;
    Ok(d)
}

// ---------------------------------------------------------------------
// RunInfo
// ---------------------------------------------------------------------

/// Run-level metadata for a vendor acquisition.
#[pyclass(module = "openmassspec_io._openmassspec_io")]
struct RunInfo {
    meta: openmassspec_core::RunMetadata,
}

#[pymethods]
impl RunInfo {
    /// Acquisition start timestamp when available (vendor-formatted string).
    #[getter]
    fn start_timestamp(&self) -> Option<&str> {
        self.meta.start_timestamp.as_deref()
    }
    /// Instrument model name from the PSI-MS CV term.
    #[getter]
    fn instrument_name(&self) -> &str {
        &self.meta.instrument.name
    }
    /// PSI-MS CV accession for the instrument (e.g. "MS:1001910").
    #[getter]
    fn instrument_accession(&self) -> &str {
        self.meta.instrument.accession
    }
    /// Source file name (basename of the vendor acquisition path).
    #[getter]
    fn source_file_name(&self) -> &str {
        &self.meta.source_file_name
    }
    /// Parser crate name (e.g. "opentfraw").
    #[getter]
    fn software_name(&self) -> &str {
        &self.meta.software_name
    }
    /// Parser crate version string.
    #[getter]
    fn software_version(&self) -> &str {
        &self.meta.software_version
    }
    fn __repr__(&self) -> String {
        format!(
            "<RunInfo instrument='{}' source='{}' software='{} {}'>",
            self.meta.instrument.name,
            self.meta.source_file_name,
            self.meta.software_name,
            self.meta.software_version,
        )
    }
}

// ---------------------------------------------------------------------
// SpectrumIter
// ---------------------------------------------------------------------

#[pyclass(module = "openmassspec_io._openmassspec_io")]
struct SpectrumIter {
    records: std::vec::IntoIter<SpectrumRecord>,
}

#[pymethods]
impl SpectrumIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Spectrum> {
        slf.records.next().map(|rec| Spectrum { rec: Some(rec) })
    }
}

// ---------------------------------------------------------------------
// Module-level functions
// ---------------------------------------------------------------------

/// Detect the vendor format of `path`. Returns one of `"thermo"`,
/// `"bruker"`, `"waters"`, or `None`.
#[pyfunction]
fn detect(path: PathBuf) -> Option<&'static str> {
    detect_format(&path).map(|d| d.format.name())
}

/// Convert a vendor acquisition to mzML. When `centroid` is set, every
/// profile-mode spectrum is centroided first (local-maxima peak picking;
/// already-centroid spectra pass through unchanged). `centroid_min_intensity`
/// discards picked peaks below that noise floor and is ignored unless
/// `centroid` is set.
#[pyfunction]
#[pyo3(signature = (input, output, *, indexed = true, centroid = false, centroid_min_intensity = None))]
fn to_mzml(
    input: PathBuf,
    output: PathBuf,
    indexed: bool,
    centroid: bool,
    centroid_min_intensity: Option<f32>,
) -> PyResult<()> {
    let detected = detected_or_err(&input)?;
    if centroid {
        openmassspec_io::convert_to_mzml_centroided(
            detected,
            &output,
            indexed,
            centroid_min_intensity,
        )
        .map_err(map_err)
    } else {
        openmassspec_io::convert_to_mzml(detected, &output, indexed).map_err(map_err)
    }
}

/// Return run-level metadata for a vendor acquisition without iterating spectra.
#[pyfunction]
#[pyo3(signature = (path, *, centroid = false, centroid_min_intensity = None))]
fn run_info(
    py: Python<'_>,
    path: PathBuf,
    centroid: bool,
    centroid_min_intensity: Option<f32>,
) -> PyResult<RunInfo> {
    let detected = detected_or_err(&path)?;
    let (_records, meta) = py
        .detach(|| collect_records(&detected, centroid, centroid_min_intensity))
        .map_err(map_err)?;
    Ok(RunInfo { meta })
}

/// Iterate every spectrum in a vendor acquisition.
#[pyfunction]
#[pyo3(signature = (path, *, centroid = false, centroid_min_intensity = None))]
fn iter_spectra(
    py: Python<'_>,
    path: PathBuf,
    centroid: bool,
    centroid_min_intensity: Option<f32>,
) -> PyResult<Py<SpectrumIter>> {
    let detected = detected_or_err(&path)?;
    let (records, _meta) = py
        .detach(|| collect_records(&detected, centroid, centroid_min_intensity))
        .map_err(map_err)?;
    Py::new(
        py,
        SpectrumIter {
            records: records.into_iter(),
        },
    )
}

// ---------------------------------------------------------------------
// Arrow stream
// ---------------------------------------------------------------------

#[cfg(feature = "arrow")]
mod arrow_bridge {
    use super::*;
    use arrow::pyarrow::ToPyArrow;
    use arrow::record_batch::RecordBatch;
    use openmassspec_core::arrow::SpectrumBatchBuilder;

    /// Build a `pyarrow.RecordBatchReader` over every spectrum in the
    /// acquisition, batched at `batch_size` rows (default 1024).
    #[pyfunction]
    #[pyo3(signature = (path, *, batch_size = 1024, centroid = false, centroid_min_intensity = None))]
    pub(super) fn read_arrow<'py>(
        py: Python<'py>,
        path: PathBuf,
        batch_size: usize,
        centroid: bool,
        centroid_min_intensity: Option<f32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        if batch_size == 0 {
            return Err(PyValueError::new_err("batch_size must be > 0"));
        }
        let detected = detected_or_err(&path)?;
        let (records, meta) = py
            .detach(|| collect_records(&detected, centroid, centroid_min_intensity))
            .map_err(map_err)?;
        let mobility_kind = meta.mobility_array_kind;
        let batches = py
            .detach(|| build_batches(records, batch_size, mobility_kind))
            .map_err(map_err)?;

        // Hand the batches to pyarrow as a RecordBatchReader.
        let schema = openmassspec_core::arrow::spectrum_record_schema();
        let pa = py.import("pyarrow")?;
        let py_schema = schema.to_pyarrow(py)?;
        let py_batches: Vec<Bound<'py, PyAny>> = batches
            .into_iter()
            .map(|b| b.to_pyarrow(py))
            .collect::<PyResult<_>>()?;
        pa.getattr("RecordBatchReader")?
            .call_method1("from_batches", (py_schema, py_batches))
    }

    fn build_batches(
        records: Vec<SpectrumRecord>,
        batch_size: usize,
        mobility_kind: Option<openmassspec_core::MobilityArrayKind>,
    ) -> Result<Vec<RecordBatch>, ::arrow::error::ArrowError> {
        let mut out = Vec::new();
        let mut builder = SpectrumBatchBuilder::new(mobility_kind);
        let mut n = 0usize;
        for rec in records {
            builder.push(&rec);
            n += 1;
            if n == batch_size {
                out.push(builder.finish()?);
                builder = SpectrumBatchBuilder::new(mobility_kind);
                n = 0;
            }
        }
        if n > 0 {
            out.push(builder.finish()?);
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// Module init
// ---------------------------------------------------------------------

#[pymodule]
fn _openmassspec_io(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<Spectrum>()?;
    m.add_class::<SpectrumIter>()?;
    m.add_class::<RunInfo>()?;
    m.add_function(wrap_pyfunction!(detect, m)?)?;
    m.add_function(wrap_pyfunction!(to_mzml, m)?)?;
    m.add_function(wrap_pyfunction!(iter_spectra, m)?)?;
    m.add_function(wrap_pyfunction!(run_info, m)?)?;
    #[cfg(feature = "arrow")]
    m.add_function(wrap_pyfunction!(arrow_bridge::read_arrow, m)?)?;
    Ok(())
}
