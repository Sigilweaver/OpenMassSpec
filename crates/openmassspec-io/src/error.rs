//! Typed error for the `openmassspec-io` umbrella crate.
//!
//! Mirrors the `thiserror`-based pattern used by `openmassspec-core` and
//! the vendor crates. Vendor-specific variants are feature-gated so
//! that a build excluding a vendor does not carry that vendor's error
//! type.

use std::path::PathBuf;

/// Errors produced by the umbrella `openmassspec-io` API.
///
/// A record the vendor reader cannot decode arrives as [`Error::Core`]
/// (wrapping `openmassspec_core::Error::Vendor`); errors opening an input
/// arrive as the vendor variant. New variants may be added in minor
/// releases; match with a wildcard arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The path did not match any supported vendor signature.
    #[error("unsupported vendor format at {0}")]
    UnsupportedFormat(PathBuf),

    /// A vendor format was detected but its feature was not enabled at
    /// build time.
    #[error("openmassspec-io was built without the '{vendor}' feature")]
    FeatureDisabled {
        /// Vendor name as returned by [`VendorFormat::name`](crate::VendorFormat::name).
        vendor: &'static str,
    },

    /// Wrapping I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Wrapping `openmassspec-core` error: an unreadable record, or a
    /// writer failure.
    #[error(transparent)]
    Core(#[from] openmassspec_core::Error),

    /// Thermo (`opentfraw`) error.
    #[cfg(feature = "thermo")]
    #[error(transparent)]
    Thermo(#[from] opentfraw::Error),

    /// Bruker (`opentimstdf`) error.
    #[cfg(feature = "bruker")]
    #[error(transparent)]
    Bruker(#[from] opentimstdf::Error),

    /// Waters (`openwraw`) error.
    #[cfg(feature = "waters")]
    #[error(transparent)]
    Waters(#[from] openwraw::Error),

    /// Agilent (`openaraw`) error.
    #[cfg(feature = "agilent")]
    #[error(transparent)]
    Agilent(#[from] openaraw::Error),

    /// SCIEX (`opensxraw`) error.
    #[cfg(feature = "sciex")]
    #[error(transparent)]
    Sciex(#[from] opensxraw::Error),

    /// Shimadzu (`openszraw`) error.
    #[cfg(feature = "shimadzu")]
    #[error(transparent)]
    Shimadzu(#[from] openszraw::Error),

    /// Error reading an mzML file (used by the `vendor2mzml validate`
    /// mzML path). A message rather than a typed source so this crate does
    /// not depend on an mzML parser.
    #[error("mzML error: {0}")]
    Mzml(String),

    /// Returned by a [`stream`](crate::stream)/[`stream_centroided`](crate::stream_centroided)
    /// `on_spectrum` callback to stop iteration early - not a decode
    /// failure, just a request to bail out (e.g. the consumer went away).
    #[error("streaming was cancelled")]
    Cancelled,
}

/// Convenience alias mirroring the vendor crates.
pub type Result<T> = std::result::Result<T, Error>;
