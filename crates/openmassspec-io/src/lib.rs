//! `openmassspec-io` is the umbrella crate that ties together the open
//! Rust mass-spec parsers (`opentfraw`, `opentimstdf`, `openwraw`,
//! `openaraw`, `opensxraw`, `openszraw`) behind a uniform
//! vendor-detection + mzML-conversion API.
//!
//! Each vendor parser is gated behind a Cargo feature
//! (`thermo`, `bruker`, `waters`, `agilent`, `sciex`, `shimadzu`) and
//! re-exported under [`vendor`]. The `all` meta-feature pulls in every
//! supported vendor.
//!
//! Even with no features enabled, [`detect_format`] is available so
//! callers can probe a path without paying the compile-time cost of a
//! parser they will not use.
//!
//! [`collect`], [`convert_to_mzml`], and [`convert_to_mzml_writer`] each
//! have a `_centroided` sibling that centroids every profile-mode
//! spectrum first via [`openmassspec_core::Centroided`]; this is always
//! opt-in, never the default.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

mod error;
pub use error::{Error, Result};

pub use openmassspec_core as core;

#[cfg(feature = "arrow")]
pub use openmassspec_core::arrow;

/// Re-exports of each vendor parser, gated by feature.
pub mod vendor {
    #[cfg(feature = "agilent")]
    pub use openaraw;
    #[cfg(feature = "sciex")]
    pub use opensxraw;
    #[cfg(feature = "shimadzu")]
    pub use openszraw;
    #[cfg(feature = "thermo")]
    pub use opentfraw;
    #[cfg(feature = "bruker")]
    pub use opentimstdf;
    #[cfg(feature = "waters")]
    pub use openwraw;
}

/// Detected on-disk vendor / format family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Vendor-name string suitable for logs and the CLI.
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
#[derive(Debug, Clone)]
pub struct Detected {
    /// Canonical path to feed back into the matching vendor reader.
    /// For directory-based formats this is the bundle directory; for
    /// Thermo, the `.raw` file itself.
    pub path: PathBuf,
    /// Identified format.
    pub format: VendorFormat,
}

/// Inspect `path` (file or directory) and return the matching vendor
/// format, or `None` when none of the supported signatures match.
///
/// This function is always available, even with no features enabled,
/// so a host application can decide which feature to enable at compile
/// time based on a runtime probe.
pub fn detect_format(path: &Path) -> Option<Detected> {
    if path.is_dir() {
        // Bruker .d/ first, then Agilent .d/, then Waters .raw/.
        // Bruker and Agilent both use a `.d` extension, so they are
        // disambiguated by contents (analysis.tdf vs AcqData/MSScan.bin),
        // not by the directory name.
        if path.join("analysis.tdf").is_file() && path.join("analysis.tdf_bin").is_file() {
            return Some(Detected {
                path: path.to_path_buf(),
                format: VendorFormat::BrukerTdf,
            });
        }
        if path.join("AcqData").join("MSScan.bin").is_file() {
            return Some(Detected {
                path: path.to_path_buf(),
                format: VendorFormat::AgilentMassHunter,
            });
        }
        if path.join("_HEADER.TXT").is_file() {
            return Some(Detected {
                path: path.to_path_buf(),
                format: VendorFormat::WatersRaw,
            });
        }
        return None;
    }
    if path.is_file() {
        if is_thermo_raw(path) {
            return Some(Detected {
                path: path.to_path_buf(),
                format: VendorFormat::ThermoRaw,
            });
        }
        if is_sciex_wiff(path) {
            return Some(Detected {
                path: path.to_path_buf(),
                format: VendorFormat::SciexWiff,
            });
        }
        if is_shimadzu_labsolutions(path) {
            return Some(Detected {
                path: path.to_path_buf(),
                format: VendorFormat::ShimadzuLabSolutions,
            });
        }
        return None;
    }
    None
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
    use std::fs::File;
    use std::io::Read;
    let Ok(mut f) = File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 8];
    if f.read_exact(&mut buf).is_err() {
        return false;
    }
    const CFBF_MAGIC: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
    buf == CFBF_MAGIC
}

/// Returns `true` if the file looks like a Thermo Finnigan `.raw`.
///
/// The Finnigan signature is the UTF-16LE string `Finnigan` starting at
/// offset 2 (the first two bytes are a small header version word).
fn is_thermo_raw(path: &Path) -> bool {
    use std::fs::File;
    use std::io::Read;
    let Ok(mut f) = File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 18];
    if f.read_exact(&mut buf).is_err() {
        return false;
    }
    // "Finnigan" in UTF-16LE: F.i.n.n.i.g.a.n. (16 bytes) at offset 2.
    const FINNIGAN_UTF16LE: [u8; 16] = [
        0x46, 0x00, 0x69, 0x00, 0x6e, 0x00, 0x6e, 0x00, 0x69, 0x00, 0x67, 0x00, 0x61, 0x00, 0x6e,
        0x00,
    ];
    buf[2..18] == FINNIGAN_UTF16LE
}

/// Convert a detected vendor file to mzML at `output`. Picks the
/// correct vendor crate's `write_mzml` (or `write_indexed_mzml`) based
/// on `indexed`.
#[allow(clippy::needless_pass_by_value)] // for symmetry with detect_format
pub fn convert_to_mzml(detected: Detected, output: &Path, indexed: bool) -> Result<()> {
    use std::fs::File;
    use std::io::BufWriter;
    let f = File::create(output)?;
    let mut w = BufWriter::new(f);
    write_to(detected.format, &detected.path, &mut w, indexed)
}

/// Like [`convert_to_mzml`] but writes to an arbitrary writer instead
/// of a path. Useful for streaming output to gzip, stdout, or any other
/// sink.
#[allow(clippy::needless_pass_by_value)]
pub fn convert_to_mzml_writer<W: std::io::Write>(
    detected: Detected,
    writer: &mut W,
    indexed: bool,
) -> Result<()> {
    write_to(detected.format, &detected.path, writer, indexed)
}

fn write_to(
    format: VendorFormat,
    path: &Path,
    w: &mut impl std::io::Write,
    indexed: bool,
) -> Result<()> {
    match format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                thermo_convert(path, w, indexed)
            }
            #[cfg(not(feature = "thermo"))]
            {
                let _ = (path, w, indexed);
                Err(Error::FeatureDisabled { vendor: "thermo" })
            }
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                if indexed {
                    opentimstdf::mzml::write_indexed_mzml(path, w)?;
                } else {
                    opentimstdf::mzml::write_mzml(path, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "bruker"))]
            {
                let _ = (path, w, indexed);
                Err(Error::FeatureDisabled { vendor: "bruker" })
            }
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                if indexed {
                    openwraw::mzml::write_indexed_mzml(path, w)?;
                } else {
                    openwraw::mzml::write_mzml(path, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "waters"))]
            {
                let _ = (path, w, indexed);
                Err(Error::FeatureDisabled { vendor: "waters" })
            }
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                // openaraw has no mzml module of its own; its Reader
                // implements openmassspec_core::SpectrumSource, so drive
                // the core writer directly.
                let mut reader = openaraw::reader::Reader::open(path)?;
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut reader, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut reader, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "agilent"))]
            {
                let _ = (path, w, indexed);
                Err(Error::FeatureDisabled { vendor: "agilent" })
            }
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                let mut reader = opensxraw::reader::Reader::open(path)?;
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut reader, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut reader, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "sciex"))]
            {
                let _ = (path, w, indexed);
                Err(Error::FeatureDisabled { vendor: "sciex" })
            }
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                // openszraw has no mzml module of its own; its Reader
                // implements openmassspec_core::SpectrumSource, so drive
                // the core writer directly (same pattern as Agilent/SCIEX).
                let mut reader = openszraw::reader::Reader::open(path)?;
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut reader, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut reader, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "shimadzu"))]
            {
                let _ = (path, w, indexed);
                Err(Error::FeatureDisabled { vendor: "shimadzu" })
            }
        }
    }
}

#[cfg(feature = "thermo")]
fn thermo_convert(path: &Path, out: &mut impl std::io::Write, indexed: bool) -> Result<()> {
    use std::fs::File;
    use std::io::BufReader;
    let raw = opentfraw::RawFileReader::open_path(path)?;
    let mut source = BufReader::with_capacity(2 << 20, File::open(path)?);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.raw");
    if indexed {
        opentfraw::mzml::write_indexed_mzml(&raw, &mut source, out, filename, false)?;
    } else {
        opentfraw::mzml::write_mzml(&raw, &mut source, out, filename, false)?;
    }
    Ok(())
}

/// Like [`convert_to_mzml`], but every profile-mode spectrum is centroided
/// first via [`openmassspec_core::Centroided`]. `min_intensity`, when
/// `Some`, discards picked peaks below that noise floor.
#[allow(clippy::needless_pass_by_value)]
pub fn convert_to_mzml_centroided(
    detected: Detected,
    output: &Path,
    indexed: bool,
    min_intensity: Option<f32>,
) -> Result<()> {
    use std::fs::File;
    use std::io::BufWriter;
    let f = File::create(output)?;
    let mut w = BufWriter::new(f);
    write_to_centroided(
        detected.format,
        &detected.path,
        &mut w,
        indexed,
        min_intensity,
    )
}

/// Like [`convert_to_mzml_writer`], but every profile-mode spectrum is
/// centroided first via [`openmassspec_core::Centroided`]. `min_intensity`,
/// when `Some`, discards picked peaks below that noise floor.
#[allow(clippy::needless_pass_by_value)]
pub fn convert_to_mzml_writer_centroided<W: std::io::Write>(
    detected: Detected,
    writer: &mut W,
    indexed: bool,
    min_intensity: Option<f32>,
) -> Result<()> {
    write_to_centroided(
        detected.format,
        &detected.path,
        writer,
        indexed,
        min_intensity,
    )
}

/// Unlike [`write_to`], every vendor arm here drives
/// `openmassspec_core::write_mzml`/`write_indexed_mzml` directly over a
/// [`openmassspec_core::Centroided`]-wrapped source, rather than each
/// vendor crate's own `write_mzml` convenience wrapper - centroiding has
/// to happen between "open the source" and "hand it to the writer", so
/// the shortcut those wrappers take (path in, mzML out, no source object
/// exposed) doesn't compose here.
fn write_to_centroided(
    format: VendorFormat,
    path: &Path,
    w: &mut impl std::io::Write,
    indexed: bool,
    min_intensity: Option<f32>,
) -> Result<()> {
    #[allow(unused_imports)]
    use openmassspec_core::SpectrumSource;
    match format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                use std::fs::File;
                use std::io::BufReader;
                let raw = opentfraw::RawFileReader::open_path(path)?;
                let mut source = BufReader::with_capacity(2 << 20, File::open(path)?);
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.raw");
                let src = opentfraw::mzml::OpenTfRawSource::new(&raw, &mut source, filename, false);
                let mut src = with_min_intensity_opt(src, min_intensity);
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut src, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut src, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "thermo"))]
            {
                let _ = (path, w, indexed, min_intensity);
                Err(Error::FeatureDisabled { vendor: "thermo" })
            }
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                let src = opentimstdf::mzml::TdfSource::open(path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut src, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut src, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "bruker"))]
            {
                let _ = (path, w, indexed, min_intensity);
                Err(Error::FeatureDisabled { vendor: "bruker" })
            }
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                let src = openwraw::mzml::WatersSource::open(path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut src, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut src, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "waters"))]
            {
                let _ = (path, w, indexed, min_intensity);
                Err(Error::FeatureDisabled { vendor: "waters" })
            }
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                let reader = openaraw::reader::Reader::open(path)?;
                let mut src = with_min_intensity_opt(reader, min_intensity);
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut src, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut src, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "agilent"))]
            {
                let _ = (path, w, indexed, min_intensity);
                Err(Error::FeatureDisabled { vendor: "agilent" })
            }
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                let reader = opensxraw::reader::Reader::open(path)?;
                let mut src = with_min_intensity_opt(reader, min_intensity);
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut src, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut src, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "sciex"))]
            {
                let _ = (path, w, indexed, min_intensity);
                Err(Error::FeatureDisabled { vendor: "sciex" })
            }
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                let reader = openszraw::reader::Reader::open(path)?;
                let mut src = with_min_intensity_opt(reader, min_intensity);
                if indexed {
                    openmassspec_core::write_indexed_mzml(&mut src, w)?;
                } else {
                    openmassspec_core::write_mzml(&mut src, w)?;
                }
                Ok(())
            }
            #[cfg(not(feature = "shimadzu"))]
            {
                let _ = (path, w, indexed, min_intensity);
                Err(Error::FeatureDisabled { vendor: "shimadzu" })
            }
        }
    }
}

/// Apply an optional noise floor to a freshly wrapped [`Centroided`]
/// source. Shared by every `collect_centroided` / `write_to_centroided`
/// vendor arm.
///
/// [`Centroided`]: openmassspec_core::Centroided
#[allow(dead_code)]
fn with_min_intensity_opt<S: openmassspec_core::SpectrumSource>(
    src: S,
    min_intensity: Option<f32>,
) -> openmassspec_core::Centroided<S> {
    let centroided = openmassspec_core::Centroided::new(src);
    match min_intensity {
        Some(v) => centroided.with_min_intensity(v),
        None => centroided,
    }
}

/// Open the appropriate vendor source for `detected`, collect every
/// spectrum into a `Vec`, and return both the records and the
/// run-level metadata. Used by tools that need a second pass over the
/// data (conformance validation, `info` summaries, Arrow batching).
///
/// This dispatches to the same vendor code paths as
/// [`convert_to_mzml`], so a feature-gated build that excludes a
/// vendor will return an error here for that vendor.
#[allow(clippy::needless_pass_by_value)]
pub fn collect(
    detected: Detected,
) -> Result<(
    Vec<openmassspec_core::SpectrumRecord>,
    openmassspec_core::RunMetadata,
)> {
    #[allow(unused_imports)]
    use openmassspec_core::SpectrumSource;
    match detected.format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                use std::fs::File;
                use std::io::BufReader;
                let raw = opentfraw::RawFileReader::open_path(&detected.path)?;
                let mut source = BufReader::with_capacity(2 << 20, File::open(&detected.path)?);
                let filename = detected
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.raw");
                let mut src =
                    opentfraw::mzml::OpenTfRawSource::new(&raw, &mut source, filename, false);
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "thermo"))]
            Err(Error::FeatureDisabled { vendor: "thermo" })
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                let mut src = opentimstdf::mzml::TdfSource::open(&detected.path)?;
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "bruker"))]
            Err(Error::FeatureDisabled { vendor: "bruker" })
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                let mut src = openwraw::mzml::WatersSource::open(&detected.path)?;
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "waters"))]
            Err(Error::FeatureDisabled { vendor: "waters" })
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                let mut src = openaraw::reader::Reader::open(&detected.path)?;
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "agilent"))]
            Err(Error::FeatureDisabled { vendor: "agilent" })
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                let mut src = opensxraw::reader::Reader::open(&detected.path)?;
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "sciex"))]
            Err(Error::FeatureDisabled { vendor: "sciex" })
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                let mut src = openszraw::reader::Reader::open(&detected.path)?;
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "shimadzu"))]
            Err(Error::FeatureDisabled { vendor: "shimadzu" })
        }
    }
}

/// Like [`collect`], but every profile-mode spectrum is centroided first
/// via [`openmassspec_core::Centroided`]. Already-centroided spectra pass
/// through unchanged. `min_intensity`, when `Some`, discards picked peaks
/// below that noise floor.
#[allow(clippy::needless_pass_by_value, unused_variables)]
pub fn collect_centroided(
    detected: Detected,
    min_intensity: Option<f32>,
) -> Result<(
    Vec<openmassspec_core::SpectrumRecord>,
    openmassspec_core::RunMetadata,
)> {
    #[allow(unused_imports)]
    use openmassspec_core::SpectrumSource;
    match detected.format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                use std::fs::File;
                use std::io::BufReader;
                let raw = opentfraw::RawFileReader::open_path(&detected.path)?;
                let mut source = BufReader::with_capacity(2 << 20, File::open(&detected.path)?);
                let filename = detected
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.raw");
                let src = opentfraw::mzml::OpenTfRawSource::new(&raw, &mut source, filename, false);
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "thermo"))]
            Err(Error::FeatureDisabled { vendor: "thermo" })
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                let src = opentimstdf::mzml::TdfSource::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "bruker"))]
            Err(Error::FeatureDisabled { vendor: "bruker" })
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                let src = openwraw::mzml::WatersSource::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "waters"))]
            Err(Error::FeatureDisabled { vendor: "waters" })
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                let src = openaraw::reader::Reader::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "agilent"))]
            Err(Error::FeatureDisabled { vendor: "agilent" })
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                let src = opensxraw::reader::Reader::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "sciex"))]
            Err(Error::FeatureDisabled { vendor: "sciex" })
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                let src = openszraw::reader::Reader::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                let recs: Vec<_> = src.iter_spectra().collect();
                Ok((recs, meta))
            }
            #[cfg(not(feature = "shimadzu"))]
            Err(Error::FeatureDisabled { vendor: "shimadzu" })
        }
    }
}

/// Like [`collect`], but visits each spectrum through `on_spectrum` as soon
/// as it is decoded instead of buffering the whole run into a `Vec` first.
/// Memory use is bounded by whatever `on_spectrum` itself retains, not by
/// the acquisition size - the mzML/Arrow writers already stream this way
/// internally; this gives callers that need their own second pass (Arrow
/// batching, conformance checks, summaries) the same property instead of
/// going through [`collect`].
///
/// If `on_spectrum` returns an error, iteration stops immediately and that
/// error is returned.
#[allow(clippy::needless_pass_by_value)]
pub fn stream(
    detected: Detected,
    mut on_spectrum: impl FnMut(openmassspec_core::SpectrumRecord) -> Result<()>,
) -> Result<openmassspec_core::RunMetadata> {
    #[allow(unused_imports)]
    use openmassspec_core::SpectrumSource;
    match detected.format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                use std::fs::File;
                use std::io::BufReader;
                let raw = opentfraw::RawFileReader::open_path(&detected.path)?;
                let mut source = BufReader::with_capacity(2 << 20, File::open(&detected.path)?);
                let filename = detected
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.raw");
                let mut src =
                    opentfraw::mzml::OpenTfRawSource::new(&raw, &mut source, filename, false);
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "thermo"))]
            Err(Error::FeatureDisabled { vendor: "thermo" })
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                let mut src = opentimstdf::mzml::TdfSource::open(&detected.path)?;
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "bruker"))]
            Err(Error::FeatureDisabled { vendor: "bruker" })
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                let mut src = openwraw::mzml::WatersSource::open(&detected.path)?;
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "waters"))]
            Err(Error::FeatureDisabled { vendor: "waters" })
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                let mut src = openaraw::reader::Reader::open(&detected.path)?;
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "agilent"))]
            Err(Error::FeatureDisabled { vendor: "agilent" })
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                let mut src = opensxraw::reader::Reader::open(&detected.path)?;
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "sciex"))]
            Err(Error::FeatureDisabled { vendor: "sciex" })
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                let mut src = openszraw::reader::Reader::open(&detected.path)?;
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "shimadzu"))]
            Err(Error::FeatureDisabled { vendor: "shimadzu" })
        }
    }
}

/// Like [`stream`], but every profile-mode spectrum is centroided first via
/// [`openmassspec_core::Centroided`]. Already-centroided spectra pass
/// through unchanged. `min_intensity`, when `Some`, discards picked peaks
/// below that noise floor.
#[allow(clippy::needless_pass_by_value, unused_variables)]
pub fn stream_centroided(
    detected: Detected,
    min_intensity: Option<f32>,
    mut on_spectrum: impl FnMut(openmassspec_core::SpectrumRecord) -> Result<()>,
) -> Result<openmassspec_core::RunMetadata> {
    #[allow(unused_imports)]
    use openmassspec_core::SpectrumSource;
    match detected.format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                use std::fs::File;
                use std::io::BufReader;
                let raw = opentfraw::RawFileReader::open_path(&detected.path)?;
                let mut source = BufReader::with_capacity(2 << 20, File::open(&detected.path)?);
                let filename = detected
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.raw");
                let src = opentfraw::mzml::OpenTfRawSource::new(&raw, &mut source, filename, false);
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "thermo"))]
            Err(Error::FeatureDisabled { vendor: "thermo" })
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                let src = opentimstdf::mzml::TdfSource::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "bruker"))]
            Err(Error::FeatureDisabled { vendor: "bruker" })
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                let src = openwraw::mzml::WatersSource::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "waters"))]
            Err(Error::FeatureDisabled { vendor: "waters" })
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                let src = openaraw::reader::Reader::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "agilent"))]
            Err(Error::FeatureDisabled { vendor: "agilent" })
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                let src = opensxraw::reader::Reader::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "sciex"))]
            Err(Error::FeatureDisabled { vendor: "sciex" })
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                let src = openszraw::reader::Reader::open(&detected.path)?;
                let mut src = with_min_intensity_opt(src, min_intensity);
                let meta = src.run_metadata();
                for rec in src.iter_spectra() {
                    on_spectrum(rec)?;
                }
                Ok(meta)
            }
            #[cfg(not(feature = "shimadzu"))]
            Err(Error::FeatureDisabled { vendor: "shimadzu" })
        }
    }
}

/// Return only the run-level metadata for `detected`, without decoding any
/// spectra. Metadata (instrument, source file format, native ID format,
/// software name/version) is already available as soon as the vendor
/// source is opened, so callers that only need it - like the Python
/// binding's `run_info` - can skip the decode pass entirely instead of
/// going through [`collect`] and discarding the records.
#[allow(clippy::needless_pass_by_value)]
pub fn metadata_only(detected: Detected) -> Result<openmassspec_core::RunMetadata> {
    #[allow(unused_imports)]
    use openmassspec_core::SpectrumSource;
    match detected.format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                use std::fs::File;
                use std::io::BufReader;
                let raw = opentfraw::RawFileReader::open_path(&detected.path)?;
                let mut source = BufReader::with_capacity(2 << 20, File::open(&detected.path)?);
                let filename = detected
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.raw");
                let src = opentfraw::mzml::OpenTfRawSource::new(&raw, &mut source, filename, false);
                Ok(src.run_metadata())
            }
            #[cfg(not(feature = "thermo"))]
            Err(Error::FeatureDisabled { vendor: "thermo" })
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                let src = opentimstdf::mzml::TdfSource::open(&detected.path)?;
                Ok(src.run_metadata())
            }
            #[cfg(not(feature = "bruker"))]
            Err(Error::FeatureDisabled { vendor: "bruker" })
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                let src = openwraw::mzml::WatersSource::open(&detected.path)?;
                Ok(src.run_metadata())
            }
            #[cfg(not(feature = "waters"))]
            Err(Error::FeatureDisabled { vendor: "waters" })
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                let src = openaraw::reader::Reader::open(&detected.path)?;
                Ok(src.run_metadata())
            }
            #[cfg(not(feature = "agilent"))]
            Err(Error::FeatureDisabled { vendor: "agilent" })
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                let src = opensxraw::reader::Reader::open(&detected.path)?;
                Ok(src.run_metadata())
            }
            #[cfg(not(feature = "sciex"))]
            Err(Error::FeatureDisabled { vendor: "sciex" })
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                let src = openszraw::reader::Reader::open(&detected.path)?;
                Ok(src.run_metadata())
            }
            #[cfg(not(feature = "shimadzu"))]
            Err(Error::FeatureDisabled { vendor: "shimadzu" })
        }
    }
}

/// Decode chromatogram traces from a vendor acquisition without decoding
/// its spectra. The returned records retain their native channel IDs and
/// time/intensity arrays; vendors without decoded traces return an empty vec.
#[allow(clippy::needless_pass_by_value)]
pub fn collect_chromatograms(
    detected: Detected,
) -> Result<Vec<openmassspec_core::ChromatogramRecord>> {
    #[allow(unused_imports)]
    use openmassspec_core::SpectrumSource;
    match detected.format {
        VendorFormat::ThermoRaw => {
            #[cfg(feature = "thermo")]
            {
                use std::fs::File;
                use std::io::BufReader;
                let raw = opentfraw::RawFileReader::open_path(&detected.path)?;
                let mut source = BufReader::with_capacity(2 << 20, File::open(&detected.path)?);
                let filename = detected
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown.raw");
                let mut src =
                    opentfraw::mzml::OpenTfRawSource::new(&raw, &mut source, filename, false);
                Ok(src.iter_chromatograms().collect())
            }
            #[cfg(not(feature = "thermo"))]
            Err(Error::FeatureDisabled { vendor: "thermo" })
        }
        VendorFormat::BrukerTdf => {
            #[cfg(feature = "bruker")]
            {
                let mut src = opentimstdf::mzml::TdfSource::open(&detected.path)?;
                Ok(src.iter_chromatograms().collect())
            }
            #[cfg(not(feature = "bruker"))]
            Err(Error::FeatureDisabled { vendor: "bruker" })
        }
        VendorFormat::WatersRaw => {
            #[cfg(feature = "waters")]
            {
                let mut src = openwraw::mzml::WatersSource::open(&detected.path)?;
                Ok(src.iter_chromatograms().collect())
            }
            #[cfg(not(feature = "waters"))]
            Err(Error::FeatureDisabled { vendor: "waters" })
        }
        VendorFormat::AgilentMassHunter => {
            #[cfg(feature = "agilent")]
            {
                let mut src = openaraw::reader::Reader::open(&detected.path)?;
                Ok(src.iter_chromatograms().collect())
            }
            #[cfg(not(feature = "agilent"))]
            Err(Error::FeatureDisabled { vendor: "agilent" })
        }
        VendorFormat::SciexWiff => {
            #[cfg(feature = "sciex")]
            {
                let mut src = opensxraw::reader::Reader::open(&detected.path)?;
                Ok(src.iter_chromatograms().collect())
            }
            #[cfg(not(feature = "sciex"))]
            Err(Error::FeatureDisabled { vendor: "sciex" })
        }
        VendorFormat::ShimadzuLabSolutions => {
            #[cfg(feature = "shimadzu")]
            {
                let mut src = openszraw::reader::Reader::open(&detected.path)?;
                Ok(src.iter_chromatograms().collect())
            }
            #[cfg(not(feature = "shimadzu"))]
            Err(Error::FeatureDisabled { vendor: "shimadzu" })
        }
    }
}

/// A trivial in-memory [`openmassspec_core::SpectrumSource`] backed by a
/// `Vec<SpectrumRecord>` + a [`openmassspec_core::RunMetadata`]. Hand it
/// to `openmassspec_core::write_mzml` when you already have the records
/// in hand and just want to emit mzML.
pub struct VecSource {
    pub metadata: openmassspec_core::RunMetadata,
    pub records: Vec<openmassspec_core::SpectrumRecord>,
}

impl VecSource {
    pub fn new(
        metadata: openmassspec_core::RunMetadata,
        records: Vec<openmassspec_core::SpectrumRecord>,
    ) -> Self {
        Self { metadata, records }
    }
}

impl openmassspec_core::SpectrumSource for VecSource {
    fn run_metadata(&self) -> openmassspec_core::RunMetadata {
        self.metadata.clone()
    }
    fn iter_spectra<'s>(
        &'s mut self,
    ) -> Box<dyn Iterator<Item = openmassspec_core::SpectrumRecord> + 's> {
        Box::new(self.records.drain(..))
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
}
