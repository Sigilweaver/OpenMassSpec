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
use openmassspec_core::{
    Activation, Analyzer, ChromatogramRecord, CvTerm, MobilityArrayKind, Polarity, PrecursorInfo,
    SpectrumRecord,
};
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

fn analyzer_str(a: Analyzer) -> &'static str {
    match a {
        Analyzer::ITMS => "itms",
        Analyzer::TQMS => "tqms",
        Analyzer::SQMS => "sqms",
        Analyzer::TOFMS => "tofms",
        Analyzer::FTMS => "ftms",
        Analyzer::Sector => "sector",
    }
}

fn mobility_kind_str(k: MobilityArrayKind) -> &'static str {
    match k {
        MobilityArrayKind::InverseReducedVsPerCm2 => "inverse_reduced_k0",
        MobilityArrayKind::DriftTimeMilliseconds => "drift_time_ms",
    }
}

fn cv_term_to_dict<'py>(py: Python<'py>, term: &CvTerm) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("accession", term.accession)?;
    d.set_item("name", &term.name)?;
    Ok(d)
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
    fn reported_total_ion_current(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.total_ion_current)
    }
    #[getter]
    fn base_peak_mz(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.effective_base_peak().map(|t| t.0))
    }
    #[getter]
    fn reported_base_peak_mz(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.base_peak_mz)
    }
    #[getter]
    fn base_peak_intensity(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.effective_base_peak().map(|t| t.1))
    }
    #[getter]
    fn reported_base_peak_intensity(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.base_peak_intensity)
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
        self.rec_ref().map(|r| r.analyzer.map(analyzer_str))
    }
    #[getter]
    fn acquisition_event_id(&self) -> PyResult<Option<u32>> {
        self.rec_ref().map(|r| r.acquisition_event_id)
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
    #[getter]
    fn faims_cv(&self) -> PyResult<Option<f64>> {
        self.rec_ref().map(|r| r.faims_cv)
    }
    #[getter]
    fn extra(&self) -> PyResult<std::collections::BTreeMap<String, String>> {
        self.rec_ref().map(|r| r.extra.clone())
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
    d.set_item("analyzer", p.analyzer.map(analyzer_str))?;
    d.set_item("ccs", p.ccs)?;
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
    #[getter]
    fn source_file_format<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        cv_term_to_dict(py, &self.meta.source_file_format)
    }
    #[getter]
    fn native_id_format<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        cv_term_to_dict(py, &self.meta.native_id_format)
    }
    #[getter]
    fn instrument<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        cv_term_to_dict(py, &self.meta.instrument)
    }
    #[getter]
    fn instrument_serial_number(&self) -> Option<&str> {
        self.meta.instrument_serial_number.as_deref()
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
    #[getter]
    fn acquisition_software_name(&self) -> Option<&str> {
        self.meta.acquisition_software_name.as_deref()
    }
    #[getter]
    fn acquisition_software_version(&self) -> Option<&str> {
        self.meta.acquisition_software_version.as_deref()
    }
    #[getter]
    fn mobility_array_kind(&self) -> Option<&'static str> {
        self.meta.mobility_array_kind.map(mobility_kind_str)
    }
    #[getter]
    fn analyzers(&self) -> Vec<&'static str> {
        self.meta
            .analyzers
            .iter()
            .copied()
            .map(analyzer_str)
            .collect()
    }
    #[getter]
    fn extra(&self) -> std::collections::BTreeMap<String, String> {
        self.meta.extra.clone()
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

/// One decoded chromatogram, with NumPy-backed time and intensity arrays.
#[pyclass(module = "openmassspec_io._openmassspec_io")]
struct Chromatogram {
    rec: ChromatogramRecord,
    n_points: usize,
    time_sec: Py<PyArray1<f32>>,
    intensity: Py<PyArray1<f32>>,
}

#[pymethods]
impl Chromatogram {
    #[getter]
    fn index(&self) -> usize {
        self.rec.index
    }
    #[getter]
    fn id(&self) -> &str {
        &self.rec.id
    }
    #[getter]
    fn chromatogram_type<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        self.rec
            .chromatogram_type
            .as_ref()
            .map(|term| cv_term_to_dict(py, term))
            .transpose()
    }
    #[getter]
    fn precursor_mz(&self) -> Option<f64> {
        self.rec.precursor_mz
    }
    #[getter]
    fn product_mz(&self) -> Option<f64> {
        self.rec.product_mz
    }
    #[getter]
    fn time_sec(&self, py: Python<'_>) -> Py<PyArray1<f32>> {
        self.time_sec.clone_ref(py)
    }
    #[getter]
    fn intensity(&self, py: Python<'_>) -> Py<PyArray1<f32>> {
        self.intensity.clone_ref(py)
    }
    fn __len__(&self) -> usize {
        self.n_points
    }
}

// ---------------------------------------------------------------------
// SpectrumIter
// ---------------------------------------------------------------------

/// Sent from the background decode thread to the Python-facing iterator.
/// A bounded (capacity-1) channel keeps at most one decoded-but-not-yet-
/// consumed spectrum in flight, so memory is bounded by one spectrum
/// rather than the whole run.
enum StreamMsg {
    Record(Box<SpectrumRecord>),
    Done(Result<(), String>),
}

#[pyclass(module = "openmassspec_io._openmassspec_io")]
struct SpectrumIter {
    // `mpsc::Receiver` is `Send` but not `Sync`; pyclasses must be both, so
    // it's parked behind a `Mutex` even though only one thread (the one
    // holding the GIL) ever calls `recv` at a time.
    rx: std::sync::Mutex<std::sync::mpsc::Receiver<StreamMsg>>,
    // Kept alive so the decode thread is detached (not joined) only once
    // the iterator itself is dropped; we don't join explicitly since
    // dropping `rx` already makes the thread's next `send` fail and
    // return promptly.
    _handle: std::thread::JoinHandle<()>,
    finished: bool,
}

impl SpectrumIter {
    fn spawn(detected: Detected, centroid: bool, min_intensity: Option<f32>) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<StreamMsg>(1);
        let handle = std::thread::spawn(move || {
            let send_rec = |tx: &std::sync::mpsc::SyncSender<StreamMsg>, rec: SpectrumRecord| {
                if tx.send(StreamMsg::Record(Box::new(rec))).is_ok() {
                    Ok(())
                } else {
                    Err(openmassspec_io::Error::Cancelled)
                }
            };
            let result = if centroid {
                openmassspec_io::stream_centroided(detected, min_intensity, |rec| {
                    send_rec(&tx, rec)
                })
            } else {
                openmassspec_io::stream(detected, |rec| send_rec(&tx, rec))
            };
            let outcome = match result {
                Ok(_meta) => Ok(()),
                Err(openmassspec_io::Error::Cancelled) => return,
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(StreamMsg::Done(outcome));
        });
        SpectrumIter {
            rx: std::sync::Mutex::new(rx),
            _handle: handle,
            finished: false,
        }
    }
}

#[pymethods]
impl SpectrumIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<Spectrum>> {
        if slf.finished {
            return Ok(None);
        }
        let rx = &slf.rx;
        match py.detach(|| rx.lock().expect("iter mutex poisoned").recv()) {
            Ok(StreamMsg::Record(rec)) => Ok(Some(Spectrum { rec: Some(*rec) })),
            Ok(StreamMsg::Done(Ok(()))) => {
                slf.finished = true;
                Ok(None)
            }
            Ok(StreamMsg::Done(Err(e))) => {
                slf.finished = true;
                Err(PyRuntimeError::new_err(e))
            }
            Err(_recv_error) => {
                slf.finished = true;
                Err(PyRuntimeError::new_err(
                    "spectrum decode thread ended without a result",
                ))
            }
        }
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
///
/// `centroid`/`centroid_min_intensity` are accepted for signature stability
/// but have no effect: centroiding only reshapes per-spectrum peaks, not
/// the run-level metadata this returns, so no spectrum needs to be decoded
/// at all.
#[pyfunction]
#[pyo3(signature = (path, *, centroid = false, centroid_min_intensity = None))]
fn run_info(
    py: Python<'_>,
    path: PathBuf,
    centroid: bool,
    centroid_min_intensity: Option<f32>,
) -> PyResult<RunInfo> {
    let _ = (centroid, centroid_min_intensity);
    let detected = detected_or_err(&path)?;
    let meta = py
        .detach(|| openmassspec_io::metadata_only(detected))
        .map_err(map_err)?;
    Ok(RunInfo { meta })
}

/// Read the chromatogram traces available in a vendor acquisition.
#[pyfunction]
fn read_chromatograms(py: Python<'_>, path: PathBuf) -> PyResult<Vec<Chromatogram>> {
    let detected = detected_or_err(&path)?;
    let records = py
        .detach(|| openmassspec_io::collect_chromatograms(detected))
        .map_err(map_err)?;
    Ok(records
        .into_iter()
        .map(|mut rec| {
            let n_points = rec.time_sec.len();
            let time_sec = PyArray1::from_vec(py, std::mem::take(&mut rec.time_sec)).unbind();
            let intensity = PyArray1::from_vec(py, std::mem::take(&mut rec.intensity)).unbind();
            Chromatogram {
                rec,
                n_points,
                time_sec,
                intensity,
            }
        })
        .collect())
}

/// Iterate every spectrum in a vendor acquisition. Spectra are decoded on a
/// background thread and handed across one at a time, so memory is bounded
/// by a single in-flight spectrum rather than the whole run.
#[pyfunction]
#[pyo3(signature = (path, *, centroid = false, centroid_min_intensity = None))]
fn iter_spectra(
    py: Python<'_>,
    path: PathBuf,
    centroid: bool,
    centroid_min_intensity: Option<f32>,
) -> PyResult<Py<SpectrumIter>> {
    let detected = detected_or_err(&path)?;
    Py::new(
        py,
        SpectrumIter::spawn(detected, centroid, centroid_min_intensity),
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
    use std::sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Mutex,
    };

    /// Sent from the background decode thread. `Done` is always the last
    /// message, so a closed channel without one means the thread died.
    enum ArrowMsg {
        Batch(RecordBatch),
        Done(Result<(), String>),
    }

    #[pyclass(module = "openmassspec_io._openmassspec_io")]
    pub(super) struct ArrowBatchIter {
        receiver: Mutex<Receiver<ArrowMsg>>,
        finished: bool,
    }

    #[pymethods]
    impl ArrowBatchIter {
        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
            if slf.finished {
                return Ok(None);
            }
            let receiver = &slf.receiver;
            let message = py.detach(|| {
                receiver
                    .lock()
                    .map_err(|_| "Arrow stream lock poisoned".to_string())?
                    .recv()
                    .map_err(|_| "Arrow decode thread ended without a result".to_string())
            });
            match message {
                Ok(ArrowMsg::Batch(batch)) => Ok(Some(batch.to_pyarrow(py)?.unbind())),
                Ok(ArrowMsg::Done(Ok(()))) => {
                    slf.finished = true;
                    Ok(None)
                }
                Ok(ArrowMsg::Done(Err(error))) | Err(error) => {
                    slf.finished = true;
                    Err(PyRuntimeError::new_err(error))
                }
            }
        }
    }

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
        // Metadata (including the mobility array kind the batch builder
        // needs) is available as soon as the source is opened, so it does
        // not require decoding any spectra.
        let meta = py
            .detach(|| openmassspec_io::metadata_only(detected.clone()))
            .map_err(map_err)?;
        let mobility_kind = meta.mobility_array_kind;
        let schema = openmassspec_core::arrow::spectrum_record_schema();
        let pa = py.import("pyarrow")?;
        let py_schema = schema.to_pyarrow(py)?;
        let (sender, receiver) = sync_channel(2);
        let batches = Py::new(
            py,
            ArrowBatchIter {
                receiver: Mutex::new(receiver),
                finished: false,
            },
        )?;
        std::thread::spawn(move || {
            let result = stream_batches(
                detected,
                centroid,
                centroid_min_intensity,
                batch_size,
                mobility_kind,
                &sender,
            );
            // Fails only when the reader was dropped early; nothing to report.
            let _ = sender.send(ArrowMsg::Done(result));
        });
        pa.getattr("RecordBatchReader")?
            .call_method1("from_batches", (py_schema, batches))
    }

    /// Push spectra into `batch_size`-row Arrow batches as they are
    /// decoded. The bounded channel holds at most two finished batches.
    fn stream_batches(
        detected: Detected,
        centroid: bool,
        min_intensity: Option<f32>,
        batch_size: usize,
        mobility_kind: Option<openmassspec_core::MobilityArrayKind>,
        sender: &SyncSender<ArrowMsg>,
    ) -> Result<(), String> {
        let mut builder = SpectrumBatchBuilder::new(mobility_kind);
        let mut n = 0usize;
        // `stream`'s callback must return `openmassspec_io::Result<()>`; an
        // Arrow batch-finish failure isn't one of that enum's variants, so
        // it's stashed here and `Cancelled` is used to stop iteration, then
        // surfaced below once streaming has returned.
        let mut arrow_err: Option<::arrow::error::ArrowError> = None;

        let on_spectrum = |rec: SpectrumRecord| -> openmassspec_io::Result<()> {
            builder.push(&rec);
            n += 1;
            if n == batch_size {
                // `finish` consumes `self`, so swap in a fresh builder
                // rather than moving `builder` out of this `FnMut`'s
                // captured environment.
                let finished =
                    std::mem::replace(&mut builder, SpectrumBatchBuilder::new(mobility_kind));
                match finished.finish() {
                    Ok(b) => {
                        sender
                            .send(ArrowMsg::Batch(b))
                            .map_err(|_| openmassspec_io::Error::Cancelled)?;
                        n = 0;
                    }
                    Err(e) => {
                        arrow_err = Some(e);
                        return Err(openmassspec_io::Error::Cancelled);
                    }
                }
            }
            Ok(())
        };

        let stream_result = if centroid {
            openmassspec_io::stream_centroided(detected, min_intensity, on_spectrum)
        } else {
            openmassspec_io::stream(detected, on_spectrum)
        };

        if let Some(e) = arrow_err {
            return Err(e.to_string());
        }
        stream_result.map_err(|e| e.to_string())?;

        if n > 0 {
            sender
                .send(ArrowMsg::Batch(
                    builder.finish().map_err(|e| e.to_string())?,
                ))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
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
    m.add_class::<Chromatogram>()?;
    m.add_function(wrap_pyfunction!(detect, m)?)?;
    m.add_function(wrap_pyfunction!(to_mzml, m)?)?;
    m.add_function(wrap_pyfunction!(iter_spectra, m)?)?;
    m.add_function(wrap_pyfunction!(run_info, m)?)?;
    m.add_function(wrap_pyfunction!(read_chromatograms, m)?)?;
    #[cfg(feature = "arrow")]
    m.add_class::<arrow_bridge::ArrowBatchIter>()?;
    #[cfg(feature = "arrow")]
    m.add_function(wrap_pyfunction!(arrow_bridge::read_arrow, m)?)?;
    Ok(())
}
