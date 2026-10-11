//! `openmassspec-io` is the umbrella crate that ties together the open
//! Rust mass-spec parsers (`opentfraw`, `opentimstdf`, `openwraw`,
//! `openaraw`, `opensxraw`, `openszraw`) behind a uniform
//! vendor-detection + mzML-conversion API.
//!
//! Each vendor parser is gated behind a Cargo feature
//! (`thermo`, `bruker`, `waters`, `agilent`, `sciex`, `shimadzu`); the
//! `all` meta-feature pulls in every supported vendor. The reader crates
//! are private dependencies: no reader type appears in this crate's API,
//! so a reader's major release does not change it. Reader errors arrive
//! as [`Error::Reader`]; [`Error::reader_error`] gives the reader's own
//! error for downcasting. The `arrow` feature re-exports
//! [`openmassspec-arrow`](https://docs.rs/openmassspec-arrow) as
//! [`arrow`]; no Arrow type appears anywhere else in this crate's API.
//!
//! Even with no features enabled, [`detect_format`] is available so
//! callers can probe a path without paying the compile-time cost of a
//! parser they will not use.
//!
//! # Errors in the record stream
//!
//! Every vendor source yields `Result` records: a spectrum or chromatogram
//! the parser cannot decode is reported as an error, never dropped.
//! [`collect`], [`stream`], and [`collect_chromatograms`] stop at the first
//! such error and return it. [`convert_to_mzml`] does the same by default;
//! [`ConvertOptions::with_record_error_policy`] with
//! [`RecordErrorPolicy::Skip`]
//! leaves unreadable records out instead and reports how many in the
//! returned [`WriteSummary`].
//!
//! # Centroiding
//!
//! [`ConvertOptions::with_centroid`], [`collect_centroided`], and
//! [`stream_centroided`] centroid every profile-mode spectrum first via
//! [`openmassspec_core::Centroided`]. Centroiding is always opt-in.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use openmassspec_io::core::RecordErrorPolicy;
//! use openmassspec_io::{convert_to_mzml, detect_format, ConvertOptions};
//!
//! let detected = detect_format(Path::new("run.raw")).expect("supported format");
//! let opts = ConvertOptions::new()
//!     .with_indexed(true)
//!     .with_record_error_policy(RecordErrorPolicy::Skip);
//! let summary = convert_to_mzml(&detected, Path::new("run.mzML"), &opts)?;
//! if summary.skipped() > 0 {
//!     eprintln!("warning: {} unreadable records skipped", summary.skipped());
//! }
//! # Ok::<(), openmassspec_io::Error>(())
//! ```

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::{Path, PathBuf};

use openmassspec_core::{
    ChromatogramIter, ChromatogramRecord, Compression, CvTerm, RecordErrorPolicy, RunMetadata,
    SpectrumIter, SpectrumRecord, SpectrumSource, WriteOptions, WriteSummary,
};

mod error;
pub use error::{BoxError, Error, Result};

pub use openmassspec_core as core;

/// The [`openmassspec-arrow`](https://docs.rs/openmassspec-arrow) crate:
/// the Arrow schema and `RecordBatch` builder for spectrum records.
#[cfg(feature = "arrow")]
pub use openmassspec_arrow as arrow;

/// Detected on-disk vendor / format family.
///
/// New vendors are added as new variants in minor releases; match with a
/// wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VendorFormat {
    /// Thermo Fisher Finnigan `.raw` (file).
    ThermoRaw,
    /// Bruker timsTOF TDF (directory ending in `.d/` containing
    /// `analysis.tdf` + `analysis.tdf_bin`).
    BrukerTdf,
    /// Waters MassLynx bundle (directory ending in `.raw/` containing
    /// `_HEADER.TXT`).
    WatersRaw,
    /// Agilent MassHunter bundle (directory containing an `AcqData/`
    /// subdirectory with `MSScan.bin`).
    AgilentMassHunter,
    /// SCIEX legacy `.wiff` file (paired with a sibling `.wiff.scan`).
    SciexWiff,
    /// Shimadzu LabSolutions `.qgd` (GC-MS) or `.lcd` (LC-MS, IT-TOF or
    /// QTOF) file. `openszraw::reader::Reader` auto-detects which of
    /// the three on-disk variants it is from the file's own CFBF
    /// stream layout, so a single `VendorFormat` variant covers all of
    /// them here too.
    ShimadzuLabSolutions,
}

impl VendorFormat {
    /// Vendor-name string suitable for logs and the CLI: `"thermo"`,
    /// `"bruker"`, `"waters"`, `"agilent"`, `"sciex"`, or `"shimadzu"`.
    pub fn name(self) -> &'static str {
        match self {
            Self::ThermoRaw => "thermo",
            Self::BrukerTdf => "bruker",
            Self::WatersRaw => "waters",
            Self::AgilentMassHunter => "agilent",
            Self::SciexWiff => "sciex",
            Self::ShimadzuLabSolutions => "shimadzu",
        }
    }
}

/// Result of probing a filesystem path for a supported vendor format.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Detected {
    /// Canonical path to feed back into the matching vendor reader.
    /// For directory-based formats this is the bundle directory; for
    /// Thermo, the `.raw` file itself.
    pub path: PathBuf,
    /// Identified format.
    pub format: VendorFormat,
}

impl Detected {
    /// Pair a path with a format without probing the filesystem. Use this
    /// to force a format that [`detect_format`] does not recognize; the
    /// vendor reader still validates the input when it is opened.
    pub fn new(path: impl Into<PathBuf>, format: VendorFormat) -> Self {
        Self {
            path: path.into(),
            format,
        }
    }
}

/// Inspect `path` (file or directory) and return the matching vendor
/// format, or `None` when none of the supported signatures match.
///
/// This function is always available, even with no features enabled,
/// so a host application can decide which feature to enable at compile
/// time based on a runtime probe.
pub fn detect_format(path: &Path) -> Option<Detected> {
    let format = if path.is_dir() {
        // Bruker .d/ first, then Agilent .d/, then Waters .raw/.
        // Bruker and Agilent both use a `.d` extension, so they are
        // disambiguated by contents (analysis.tdf vs AcqData/MSScan.bin),
        // not by the directory name.
        if path.join("analysis.tdf").is_file() && path.join("analysis.tdf_bin").is_file() {
            VendorFormat::BrukerTdf
        } else if path.join("AcqData").join("MSScan.bin").is_file() {
            VendorFormat::AgilentMassHunter
        } else if path.join("_HEADER.TXT").is_file() {
            VendorFormat::WatersRaw
        } else {
            return None;
        }
    } else if path.is_file() {
        if is_thermo_raw(path) {
            VendorFormat::ThermoRaw
        } else if is_sciex_wiff(path) {
            VendorFormat::SciexWiff
        } else if is_shimadzu_labsolutions(path) {
            VendorFormat::ShimadzuLabSolutions
        } else {
            return None;
        }
    } else {
        return None;
    };
    Some(Detected::new(path, format))
}

/// Returns `true` if `path` looks like a SCIEX legacy `.wiff` file: a
/// `.wiff` extension with a sibling `.wiff.scan` file alongside it (the
/// scan data the reader needs). The extension check is case-insensitive.
fn is_sciex_wiff(path: &Path) -> bool {
    let is_wiff_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wiff"));
    if !is_wiff_ext {
        return false;
    }
    let mut scan = path.as_os_str().to_os_string();
    scan.push(".scan");
    Path::new(&scan).is_file()
}

/// Returns `true` if `path` looks like a Shimadzu LabSolutions `.qgd` or
/// `.lcd` file: one of those two extensions (case-insensitive) whose
/// first 8 bytes are the CFBF/OLE2 container signature. Unlike the
/// SCIEX `.wiff` check, there is no sibling file to corroborate against
/// (Shimadzu raw files are self-contained), so the magic-byte check is
/// the only content-level signal available - still strictly more
/// verification than a bare extension check.
fn is_shimadzu_labsolutions(path: &Path) -> bool {
    let is_shimadzu_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("qgd") || e.eq_ignore_ascii_case("lcd"));
    if !is_shimadzu_ext {
        return false;
    }
    const CFBF_MAGIC: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
    read_prefix::<8>(path).is_some_and(|buf| buf == CFBF_MAGIC)
}

/// Returns `true` if the file looks like a Thermo Finnigan `.raw`.
///
/// The Finnigan signature is the UTF-16LE string `Finnigan` starting at
/// offset 2 (the first two bytes are a small header version word).
fn is_thermo_raw(path: &Path) -> bool {
    // "Finnigan" in UTF-16LE: F.i.n.n.i.g.a.n. (16 bytes) at offset 2.
    const FINNIGAN_UTF16LE: [u8; 16] = [
        0x46, 0x00, 0x69, 0x00, 0x6e, 0x00, 0x6e, 0x00, 0x69, 0x00, 0x67, 0x00, 0x61, 0x00, 0x6e,
        0x00,
    ];
    read_prefix::<18>(path).is_some_and(|buf| buf[2..18] == FINNIGAN_UTF16LE)
}

/// First `N` bytes of `path`, or `None` when the file cannot be opened or
/// is shorter than `N` bytes. Detection treats both as "not this format".
fn read_prefix<const N: usize>(path: &Path) -> Option<[u8; N]> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; N];
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

// ---------------------------------------------------------------------
// Source dispatch
// ---------------------------------------------------------------------

/// Open the vendor source for `detected` and run `f` on it.
///
/// This is the single dispatch point every other function in this crate
/// goes through. Use it to drive a source in a way the convenience
/// functions do not cover, for example a custom
/// [`WriteOptions`] or a consumer that
/// handles `Err` records itself.
///
/// The source is closed when `f` returns.
///
/// # Errors
///
/// [`Error::FeatureDisabled`] when the vendor's feature is off,
/// [`Error::Reader`] when the input cannot be opened, or whatever `f`
/// returns. An `openmassspec_core::Error::Vendor` that `f` returns (an
/// unreadable record) is reported as [`Error::Reader`] for this vendor.
pub fn with_source<T>(
    detected: &Detected,
    f: impl FnOnce(&mut dyn SpectrumSource) -> Result<T>,
) -> Result<T> {
    let vendor = detected.format.name();
    with_source_inner(detected, f).map_err(|e| e.attribute_to(vendor))
}

/// Error mapper for a vendor reader's own error type.
#[allow(dead_code)]
fn reader_err<E>(vendor: &'static str) -> impl FnOnce(E) -> Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    move |e| Error::reader(vendor, e)
}

#[cfg_attr(
    not(all(
        feature = "thermo",
        feature = "bruker",
        feature = "waters",
        feature = "agilent",
        feature = "sciex",
        feature = "shimadzu"
    )),
    allow(unused_variables)
)]
fn with_source_inner<T>(
    detected: &Detected,
    f: impl FnOnce(&mut dyn SpectrumSource) -> Result<T>,
) -> Result<T> {
    let path = detected.path.as_path();
    match detected.format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                let raw =
                    opentfraw::RawFileReader::open_path(path).map_err(reader_err("thermo"))?;
                let mut file =
                    std::io::BufReader::with_capacity(2 << 20, std::fs::File::open(path)?);
                let filename = path
                    .file_name()
                    .map_or_else(|| "unknown.raw".into(), |n| n.to_string_lossy());
                let mut src =
                    opentfraw::mzml::OpenTfRawSource::new(&raw, &mut file, &filename, false);
                f(&mut src)
            }
            #[cfg(not(feature = "thermo"))]
            Err(Error::FeatureDisabled { vendor: "thermo" })
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                f(&mut opentimstdf::mzml::TdfSource::open(path).map_err(reader_err("bruker"))?)
            }
            #[cfg(not(feature = "bruker"))]
            Err(Error::FeatureDisabled { vendor: "bruker" })
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                f(&mut openwraw::mzml::WatersSource::open(path).map_err(reader_err("waters"))?)
            }
            #[cfg(not(feature = "waters"))]
            Err(Error::FeatureDisabled { vendor: "waters" })
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                f(&mut openaraw::reader::Reader::open(path).map_err(reader_err("agilent"))?)
            }
            #[cfg(not(feature = "agilent"))]
            Err(Error::FeatureDisabled { vendor: "agilent" })
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                f(&mut opensxraw::reader::Reader::open(path).map_err(reader_err("sciex"))?)
            }
            #[cfg(not(feature = "sciex"))]
            Err(Error::FeatureDisabled { vendor: "sciex" })
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                f(&mut openszraw::reader::Reader::open(path).map_err(reader_err("shimadzu"))?)
            }
            #[cfg(not(feature = "shimadzu"))]
            Err(Error::FeatureDisabled { vendor: "shimadzu" })
        }
    }
}

/// Like [`with_source`], but every profile-mode spectrum is centroided
/// via [`openmassspec_core::Centroided`] before `f` sees it.
fn with_centroided_source<T>(
    detected: &Detected,
    min_intensity: Option<f32>,
    f: impl FnOnce(&mut dyn SpectrumSource) -> Result<T>,
) -> Result<T> {
    with_source(detected, |src| {
        let mut centroided = openmassspec_core::Centroided::new(DynSource(src));
        if let Some(v) = min_intensity {
            centroided = centroided.with_min_intensity(v);
        }
        f(&mut centroided)
    })
}

/// Sized wrapper so a `&mut dyn SpectrumSource` can go into adapters that
/// take an owned `S: SpectrumSource`.
struct DynSource<'a>(&'a mut dyn SpectrumSource);

impl SpectrumSource for DynSource<'_> {
    fn run_metadata(&self) -> RunMetadata {
        self.0.run_metadata()
    }
    fn iter_spectra<'s>(&'s mut self) -> SpectrumIter<'s> {
        self.0.iter_spectra()
    }
    fn iter_chromatograms<'s>(&'s mut self) -> ChromatogramIter<'s> {
        self.0.iter_chromatograms()
    }
    fn spectrum_count_hint(&self) -> Option<usize> {
        self.0.spectrum_count_hint()
    }
    fn additional_processing_steps(&self) -> Vec<CvTerm> {
        self.0.additional_processing_steps()
    }
}

// ---------------------------------------------------------------------
// mzML conversion
// ---------------------------------------------------------------------

/// Options for [`convert_to_mzml`] and [`convert_to_mzml_writer`].
///
/// The defaults write un-indexed mzML with zlib-compressed arrays, no
/// centroiding, and stop at the first unreadable record.
///
/// ```
/// use openmassspec_io::core::RecordErrorPolicy;
/// use openmassspec_io::ConvertOptions;
///
/// let opts = ConvertOptions::new()
///     .with_indexed(true)
///     .with_centroid(true)
///     .with_record_error_policy(RecordErrorPolicy::Skip);
/// assert!(opts.indexed);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct ConvertOptions {
    /// Emit indexed mzML (`<indexList>` and `<fileChecksum>`).
    pub indexed: bool,
    /// Centroid every profile-mode spectrum before writing (local-maxima
    /// peak picking; already-centroided spectra pass through unchanged).
    pub centroid: bool,
    /// When centroiding, discard picked peaks below this intensity.
    /// Ignored unless [`centroid`](Self::centroid) is set.
    pub centroid_min_intensity: Option<f32>,
    /// Writer options from `openmassspec-core`: binary array compression
    /// and the record-error policy.
    pub write: WriteOptions,
}

impl ConvertOptions {
    /// Default options (see the type-level docs).
    pub fn new() -> Self {
        Self::default()
    }

    /// Emit indexed mzML.
    #[must_use]
    pub fn with_indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }

    /// Centroid profile-mode spectra before writing.
    #[must_use]
    pub fn with_centroid(mut self, centroid: bool) -> Self {
        self.centroid = centroid;
        self
    }

    /// Noise floor for centroiding.
    #[must_use]
    pub fn with_centroid_min_intensity(mut self, min_intensity: impl Into<Option<f32>>) -> Self {
        self.centroid_min_intensity = min_intensity.into();
        self
    }

    /// Binary data array compression.
    #[must_use]
    pub fn with_compression(mut self, compression: Compression) -> Self {
        self.write = self.write.with_compression(compression);
        self
    }

    /// What to do with a spectrum or chromatogram the reader cannot
    /// decode. [`RecordErrorPolicy::Fail`] (the default) stops and returns
    /// the error; [`RecordErrorPolicy::Skip`] leaves it out and counts it
    /// in the returned [`WriteSummary`].
    #[must_use]
    pub fn with_record_error_policy(mut self, policy: RecordErrorPolicy) -> Self {
        self.write = self.write.with_record_error_policy(policy);
        self
    }

    /// Replace the writer options wholesale.
    #[must_use]
    pub fn with_write_options(mut self, write: WriteOptions) -> Self {
        self.write = write;
        self
    }
}

/// Convert a detected vendor acquisition to an mzML file at `output`.
///
/// Returns what was written, including how many records were skipped when
/// `opts` selects [`RecordErrorPolicy::Skip`]. Callers that skip must
/// surface [`WriteSummary::skipped`] to their users.
///
/// # Errors
///
/// The vendor reader's error when the input cannot be opened, the first
/// unreadable record under [`RecordErrorPolicy::Fail`], or an I/O error.
/// On error `output` may hold a truncated document.
pub fn convert_to_mzml(
    detected: &Detected,
    output: &Path,
    opts: &ConvertOptions,
) -> Result<WriteSummary> {
    let mut w = std::io::BufWriter::new(std::fs::File::create(output)?);
    let summary = convert_to_mzml_writer(detected, &mut w, opts)?;
    w.flush()?;
    Ok(summary)
}

/// Like [`convert_to_mzml`] but writes to an arbitrary writer (gzip,
/// stdout, an in-memory buffer). The writer is not flushed.
///
/// # Errors
///
/// As for [`convert_to_mzml`].
pub fn convert_to_mzml_writer<W: Write>(
    detected: &Detected,
    writer: &mut W,
    opts: &ConvertOptions,
) -> Result<WriteSummary> {
    let write = |src: &mut dyn SpectrumSource| -> Result<WriteSummary> {
        let summary = if opts.indexed {
            openmassspec_core::write_indexed_mzml_with_options(src, writer, &opts.write)?
        } else {
            openmassspec_core::write_mzml_with_options(src, writer, &opts.write)?
        };
        Ok(summary)
    };
    if opts.centroid {
        with_centroided_source(detected, opts.centroid_min_intensity, write)
    } else {
        with_source(detected, write)
    }
}

// ---------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------

fn collect_from(src: &mut dyn SpectrumSource) -> Result<(Vec<SpectrumRecord>, RunMetadata)> {
    let meta = src.run_metadata();
    let recs = src
        .iter_spectra()
        .collect::<openmassspec_core::Result<Vec<_>>>()?;
    Ok((recs, meta))
}

fn stream_from(
    src: &mut dyn SpectrumSource,
    on_spectrum: &mut dyn FnMut(SpectrumRecord) -> Result<()>,
) -> Result<RunMetadata> {
    let meta = src.run_metadata();
    for rec in src.iter_spectra() {
        on_spectrum(rec?)?;
    }
    Ok(meta)
}

/// Decode every spectrum of `detected` into a `Vec`, together with the
/// run-level metadata. Used by tools that need a second pass over the
/// data (conformance validation, `info` summaries).
///
/// # Errors
///
/// [`Error::FeatureDisabled`] when the vendor's feature is off, the vendor
/// reader's error when the input cannot be opened, and the first spectrum
/// the reader cannot decode.
pub fn collect(detected: &Detected) -> Result<(Vec<SpectrumRecord>, RunMetadata)> {
    with_source(detected, collect_from)
}

/// Like [`collect`], but every profile-mode spectrum is centroided first
/// via [`openmassspec_core::Centroided`]. Already-centroided spectra pass
/// through unchanged. `min_intensity`, when `Some`, discards picked peaks
/// below that noise floor.
///
/// # Errors
///
/// As for [`collect`].
pub fn collect_centroided(
    detected: &Detected,
    min_intensity: Option<f32>,
) -> Result<(Vec<SpectrumRecord>, RunMetadata)> {
    with_centroided_source(detected, min_intensity, collect_from)
}

/// Like [`collect`], but visits each spectrum through `on_spectrum` as soon
/// as it is decoded instead of buffering the whole run into a `Vec` first.
/// Memory use is bounded by whatever `on_spectrum` itself retains, not by
/// the acquisition size.
///
/// # Errors
///
/// As for [`collect`]. If `on_spectrum` returns an error, iteration stops
/// immediately and that error is returned; [`Error::Cancelled`] is the
/// conventional value for "stop, nothing went wrong".
pub fn stream(
    detected: &Detected,
    mut on_spectrum: impl FnMut(SpectrumRecord) -> Result<()>,
) -> Result<RunMetadata> {
    with_source(detected, |src| stream_from(src, &mut on_spectrum))
}

/// Like [`stream`], but every profile-mode spectrum is centroided first via
/// [`openmassspec_core::Centroided`]. Already-centroided spectra pass
/// through unchanged. `min_intensity`, when `Some`, discards picked peaks
/// below that noise floor.
///
/// # Errors
///
/// As for [`stream`].
pub fn stream_centroided(
    detected: &Detected,
    min_intensity: Option<f32>,
    mut on_spectrum: impl FnMut(SpectrumRecord) -> Result<()>,
) -> Result<RunMetadata> {
    with_centroided_source(detected, min_intensity, |src| {
        stream_from(src, &mut on_spectrum)
    })
}

/// Return only the run-level metadata for `detected`, without decoding any
/// spectra.
///
/// # Errors
///
/// [`Error::FeatureDisabled`] when the vendor's feature is off, or the
/// vendor reader's error when the input cannot be opened.
pub fn metadata_only(detected: &Detected) -> Result<RunMetadata> {
    with_source(detected, |src| Ok(src.run_metadata()))
}

/// Decode chromatogram traces from a vendor acquisition without decoding
/// its spectra. The returned records retain their native channel IDs and
/// time/intensity arrays; vendors without decoded traces return an empty
/// vec.
///
/// # Errors
///
/// As for [`collect`], with the first chromatogram the reader cannot
/// decode in place of the first spectrum.
pub fn collect_chromatograms(detected: &Detected) -> Result<Vec<ChromatogramRecord>> {
    with_source(detected, |src| {
        Ok(src
            .iter_chromatograms()
            .collect::<openmassspec_core::Result<Vec<_>>>()?)
    })
}

/// A trivial in-memory [`SpectrumSource`] backed by a `Vec<SpectrumRecord>`
/// and a [`RunMetadata`]. Hand it to `openmassspec_core::write_mzml` when
/// you already have the records in hand and just want to emit mzML.
///
/// `iter_spectra` drains `records`, so the source yields its records once.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct VecSource {
    /// Run-level metadata returned by `run_metadata`.
    pub metadata: RunMetadata,
    /// Records yielded, in order, by `iter_spectra`.
    pub records: Vec<SpectrumRecord>,
}

impl VecSource {
    /// Wrap `records` with `metadata`.
    pub fn new(metadata: RunMetadata, records: Vec<SpectrumRecord>) -> Self {
        Self { metadata, records }
    }
}

impl SpectrumSource for VecSource {
    fn run_metadata(&self) -> RunMetadata {
        self.metadata.clone()
    }
    fn iter_spectra<'s>(&'s mut self) -> SpectrumIter<'s> {
        Box::new(self.records.drain(..).map(Ok))
    }
    fn spectrum_count_hint(&self) -> Option<usize> {
        Some(self.records.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detect_returns_none_for_garbage_file() {
        let tmp = tempfile_path();
        std::fs::write(&tmp, b"hello").unwrap();
        assert!(detect_format(&tmp).is_none());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn detect_returns_thermo_for_finnigan_magic() {
        let tmp = tempfile_path();
        let mut f = std::fs::File::create(&tmp).unwrap();
        // 2-byte version word + "Finnigan" in UTF-16LE + trailing garbage.
        f.write_all(&[
            0x01, 0xa1, 0x46, 0x00, 0x69, 0x00, 0x6e, 0x00, 0x6e, 0x00, 0x69, 0x00, 0x67, 0x00,
            0x61, 0x00, 0x6e, 0x00, 0xff, 0xff,
        ])
        .unwrap();
        let det = detect_format(&tmp).expect("detect");
        assert_eq!(det.format, VendorFormat::ThermoRaw);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn detect_returns_bruker_for_tdf_layout() {
        let tmp = tempfile_dir();
        std::fs::write(tmp.join("analysis.tdf"), b"").unwrap();
        std::fs::write(tmp.join("analysis.tdf_bin"), b"").unwrap();
        let det = detect_format(&tmp).expect("detect");
        assert_eq!(det.format, VendorFormat::BrukerTdf);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detect_returns_waters_for_header_layout() {
        let tmp = tempfile_dir();
        std::fs::write(tmp.join("_HEADER.TXT"), b"$$ FAKE\n").unwrap();
        let det = detect_format(&tmp).expect("detect");
        assert_eq!(det.format, VendorFormat::WatersRaw);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detect_returns_agilent_for_acqdata_layout() {
        let tmp = tempfile_dir();
        let acq = tmp.join("AcqData");
        std::fs::create_dir_all(&acq).unwrap();
        std::fs::write(acq.join("MSScan.bin"), b"").unwrap();
        let det = detect_format(&tmp).expect("detect");
        assert_eq!(det.format, VendorFormat::AgilentMassHunter);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detect_returns_sciex_for_wiff_with_scan_sibling() {
        let dir = tempfile_dir();
        let wiff = dir.join("run.wiff");
        std::fs::write(&wiff, b"\xd0\xcf\x11\xe0").unwrap();
        std::fs::write(dir.join("run.wiff.scan"), b"").unwrap();
        let det = detect_format(&wiff).expect("detect");
        assert_eq!(det.format, VendorFormat::SciexWiff);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_returns_none_for_wiff_without_scan_sibling() {
        let dir = tempfile_dir();
        let wiff = dir.join("lonely.wiff");
        std::fs::write(&wiff, b"\xd0\xcf\x11\xe0").unwrap();
        // No .wiff.scan alongside -> not a usable SCIEX pair.
        assert!(detect_format(&wiff).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    const CFBF_MAGIC_8: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];

    #[test]
    fn detect_returns_shimadzu_for_lcd_with_cfbf_magic() {
        let dir = tempfile_dir();
        let lcd = dir.join("run.lcd");
        std::fs::write(&lcd, CFBF_MAGIC_8).unwrap();
        let det = detect_format(&lcd).expect("detect");
        assert_eq!(det.format, VendorFormat::ShimadzuLabSolutions);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_returns_shimadzu_for_qgd_with_cfbf_magic() {
        let dir = tempfile_dir();
        let qgd = dir.join("run.qgd");
        std::fs::write(&qgd, CFBF_MAGIC_8).unwrap();
        let det = detect_format(&qgd).expect("detect");
        assert_eq!(det.format, VendorFormat::ShimadzuLabSolutions);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_returns_none_for_lcd_without_cfbf_magic() {
        let dir = tempfile_dir();
        let lcd = dir.join("not_really.lcd");
        std::fs::write(&lcd, b"not a real container").unwrap();
        assert!(detect_format(&lcd).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_returns_none_for_unrelated_extension_with_cfbf_magic() {
        // The CFBF/OLE2 signature alone is not Shimadzu-specific (SCIEX's
        // legacy .wiff also uses it) - the extension must match too.
        let dir = tempfile_dir();
        let other = dir.join("run.xyz");
        std::fs::write(&other, CFBF_MAGIC_8).unwrap();
        assert!(detect_format(&other).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn tempfile_path() -> PathBuf {
        let pid = std::process::id();
        let mut p = std::env::temp_dir();
        p.push(format!("msio-test-{pid}-{:p}", &pid));
        p
    }

    fn tempfile_dir() -> PathBuf {
        let p = tempfile_path();
        let _ = std::fs::create_dir_all(&p);
        p
    }

    #[test]
    fn convert_unsupported_format_returns_typed_error() {
        // `detect_format` returns None here, so callers can't reach
        // `convert_to_mzml`. Exercise the FeatureDisabled / Mzml paths
        // through the public `Error` variants directly to keep this
        // test feature-agnostic.
        let e: Error = std::io::Error::other("boom").into();
        assert!(matches!(e, Error::Io(_)));
        let e = Error::FeatureDisabled { vendor: "thermo" };
        assert_eq!(
            e.to_string(),
            "openmassspec-io was built without the 'thermo' feature"
        );
        let e = Error::UnsupportedFormat(PathBuf::from("/tmp/nope"));
        assert!(matches!(e, Error::UnsupportedFormat(_)));
    }

    #[test]
    fn convert_options_default_is_strict_unindexed_uncentroided() {
        let o = ConvertOptions::new();
        assert!(!o.indexed);
        assert!(!o.centroid);
        assert_eq!(o.centroid_min_intensity, None);
        assert_eq!(o.write.record_error_policy, RecordErrorPolicy::Fail);
        let o = o
            .with_record_error_policy(RecordErrorPolicy::Skip)
            .with_centroid_min_intensity(5.0);
        assert_eq!(o.write.record_error_policy, RecordErrorPolicy::Skip);
        assert_eq!(o.centroid_min_intensity, Some(5.0));
    }

    fn meta() -> RunMetadata {
        RunMetadata::new(
            "t.raw",
            CvTerm::new("MS:1000563", "Thermo RAW format"),
            CvTerm::new("MS:1000768", "Thermo nativeID format"),
            CvTerm::new("MS:1000483", "Thermo Fisher Scientific instrument model"),
            "test",
            "0",
        )
    }

    #[test]
    fn vec_source_yields_every_record_once() {
        let recs = vec![
            SpectrumRecord::new(0, 1, "scan=1"),
            SpectrumRecord::new(1, 2, "scan=2"),
        ];
        let mut src = VecSource::new(meta(), recs);
        assert_eq!(src.spectrum_count_hint(), Some(2));
        let got = src
            .iter_spectra()
            .collect::<openmassspec_core::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(src.iter_spectra().count(), 0);
    }

    /// Source whose second spectrum is unreadable.
    struct OneBad;

    impl SpectrumSource for OneBad {
        fn run_metadata(&self) -> RunMetadata {
            meta()
        }
        fn iter_spectra<'s>(&'s mut self) -> SpectrumIter<'s> {
            Box::new(
                vec![
                    Ok(SpectrumRecord::new(0, 1, "scan=1").with_peaks(vec![1.0], vec![1.0])),
                    Err(openmassspec_core::Error::vendor(std::io::Error::other(
                        "scan 2 truncated",
                    ))),
                    Ok(SpectrumRecord::new(1, 3, "scan=3").with_peaks(vec![1.0], vec![1.0])),
                ]
                .into_iter(),
            )
        }
    }

    #[test]
    fn collect_and_stream_surface_unreadable_records() {
        let err = collect_from(&mut OneBad).unwrap_err();
        assert!(matches!(err, Error::Core(_)), "{err:?}");
        let source = err.reader_error().expect("reader error");
        assert!(source.downcast_ref::<std::io::Error>().is_some());
        let err = err.attribute_to("thermo");
        assert_eq!(err.reader_vendor(), Some("thermo"));
        assert!(err.to_string().starts_with("thermo reader: "), "{err}");
        assert!(err.to_string().contains("scan 2 truncated"));

        let mut seen = 0;
        let err = stream_from(&mut OneBad, &mut |_| {
            seen += 1;
            Ok(())
        })
        .unwrap_err();
        assert!(matches!(err, Error::Core(_)));
        assert_eq!(seen, 1);
    }

    #[test]
    fn centroided_dyn_source_forwards_processing_steps_and_errors() {
        let mut inner = OneBad;
        let mut src = openmassspec_core::Centroided::new(DynSource(&mut inner));
        assert_eq!(src.additional_processing_steps().len(), 1);
        assert!(collect_from(&mut src).is_err());
    }
}
