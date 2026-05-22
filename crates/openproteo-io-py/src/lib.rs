//! PyO3 bindings for `openproteo-io`.
//!
//! Exposes a small, vendor-neutral surface:
//!
//! * `detect(path) -> str | None`
//! * `to_mzml(input, output, *, indexed=True) -> None`
//! * `iter_spectra(path) -> Iterator[Spectrum]`
//! * `read_arrow(path, batch_size=1024) -> pyarrow.RecordBatchReader`
//!   (built when the `arrow` feature is enabled, which is the default).
//!
//! All m/z, intensity, and inverse-mobility arrays handed to Python are
//! created via `numpy::PyArray1::from_vec_bound`, which transfers
//! ownership of the underlying Rust `Vec` to NumPy without copying any
//! element. The Python object becomes the sole owner.

#![forbid(unsafe_code)]
#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use numpy::PyArray1;
use openproteo_core::{Activation, Polarity, PrecursorInfo, SpectrumRecord, SpectrumSource};
use openproteo_io::{detect_format, Detected, VendorFormat};
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

/// Collect every spectrum from `detected` into a single `Vec`. This is
/// the v0.1.0 strategy: simple, predictable, and matches what the
/// Bruker and Waters adapters already do internally. A streaming
/// variant is on the roadmap.
fn collect_records(
    detected: &Detected,
) -> openproteo_io::Result<(Vec<SpectrumRecord>, openproteo_core::RunMetadata)> {
    match detected.format {
        VendorFormat::ThermoRaw => collect_thermo(&detected.path),
        VendorFormat::BrukerTdf => {
            let mut src = opentimstdf::mzml::TdfSource::open(&detected.path)?;
            let meta = src.run_metadata();
            let recs: Vec<_> = src.iter_spectra().collect();
            Ok((recs, meta))
        }
        VendorFormat::WatersRaw => {
            let mut src = openwraw::mzml::WatersSource::open(&detected.path)?;
            let meta = src.run_metadata();
            let recs: Vec<_> = src.iter_spectra().collect();
            Ok((recs, meta))
        }
    }
}

fn collect_thermo(
    path: &Path,
) -> openproteo_io::Result<(Vec<SpectrumRecord>, openproteo_core::RunMetadata)> {
    use std::fs::File;
    use std::io::BufReader;
    let raw = opentfraw::RawFileReader::open_path(path)?;
    let mut source = BufReader::with_capacity(2 << 20, File::open(path)?);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.raw");
    let mut src = opentfraw::mzml::OpenTfRawSource::new(&raw, &mut source, filename, false);
    let meta = src.run_metadata();
    let recs: Vec<_> = src.iter_spectra().collect();
    Ok((recs, meta))
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
#[pyclass(module = "openproteo_io._openproteo_io")]
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
// SpectrumIter
// ---------------------------------------------------------------------

#[pyclass(module = "openproteo_io._openproteo_io")]
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

/// Convert a vendor acquisition to mzML.
#[pyfunction]
#[pyo3(signature = (input, output, *, indexed = true))]
fn to_mzml(input: PathBuf, output: PathBuf, indexed: bool) -> PyResult<()> {
    let detected = detected_or_err(&input)?;
    openproteo_io::convert_to_mzml(detected, &output, indexed).map_err(map_err)
}

/// Iterate every spectrum in a vendor acquisition.
#[pyfunction]
fn iter_spectra(py: Python<'_>, path: PathBuf) -> PyResult<Py<SpectrumIter>> {
    let detected = detected_or_err(&path)?;
    let (records, _meta) = py.detach(|| collect_records(&detected)).map_err(map_err)?;
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
    use openproteo_core::arrow::SpectrumBatchBuilder;

    /// Build a `pyarrow.RecordBatchReader` over every spectrum in the
    /// acquisition, batched at `batch_size` rows (default 1024).
    #[pyfunction]
    #[pyo3(signature = (path, *, batch_size = 1024))]
    pub(super) fn read_arrow<'py>(
        py: Python<'py>,
        path: PathBuf,
        batch_size: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        if batch_size == 0 {
            return Err(PyValueError::new_err("batch_size must be > 0"));
        }
        let detected = detected_or_err(&path)?;
        let (records, meta) = py.detach(|| collect_records(&detected)).map_err(map_err)?;
        let mobility_kind = meta.mobility_array_kind;
        let batches = py
            .detach(|| build_batches(records, batch_size, mobility_kind))
            .map_err(map_err)?;

        // Hand the batches to pyarrow as a RecordBatchReader.
        let schema = openproteo_core::arrow::spectrum_record_schema();
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
        mobility_kind: Option<openproteo_core::MobilityArrayKind>,
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
fn _openproteo_io(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<Spectrum>()?;
    m.add_class::<SpectrumIter>()?;
    m.add_function(wrap_pyfunction!(detect, m)?)?;
    m.add_function(wrap_pyfunction!(to_mzml, m)?)?;
    m.add_function(wrap_pyfunction!(iter_spectra, m)?)?;
    #[cfg(feature = "arrow")]
    m.add_function(wrap_pyfunction!(arrow_bridge::read_arrow, m)?)?;
    Ok(())
}
