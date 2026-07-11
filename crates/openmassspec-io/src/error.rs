//! Typed error for the `openmassspec-io` umbrella crate.
//!
//! Mirrors the `thiserror`-based pattern used by `openmassspec-core` and
//! the vendor crates. Vendor-specific variants are feature-gated so
//! that a build excluding a vendor does not carry that vendor's error
//! type.

use std::path::PathBuf;

/// Errors produced by the umbrella `openmassspec-io` API.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The path did not match any supported vendor signature.
    #[error("unsupported vendor format at {0}")]
    UnsupportedFormat(PathBuf),

    /// A vendor format was detected but its feature was not enabled at
    /// build time.
    #[error("openmassspec-io was built without the '{vendor}' feature")]
    FeatureDisabled { vendor: &'static str },

    /// Wrapping I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Wrapping `openmassspec-core` error.
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

    /// mzML parsing error. Kept as a string until `mzdata` exposes a
    /// typed error suitable for `#[from]`.
    #[error("mzML error: {0}")]
    Mzml(String),
}

/// Convenience alias mirroring the vendor crates.
pub type Result<T> = std::result::Result<T, Error>;
