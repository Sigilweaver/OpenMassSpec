//! Typed error for the `openmassspec-io` umbrella crate.
//!
//! Vendor reader errors are carried as boxed trait objects in
//! [`Error::Reader`], so this type does not change when a reader crate
//! releases a new major version.

use std::path::PathBuf;

/// A vendor reader's own error, type-erased.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Errors produced by the umbrella `openmassspec-io` API.
///
/// Every failure that comes from a vendor reader, whether opening the
/// input or decoding one spectrum or chromatogram, is an
/// [`Error::Reader`] holding that reader's own error. Use
/// [`Error::reader_error`] and `downcast_ref` when you need the reader's
/// typed error; see the example there.
///
/// New variants may be added in minor releases; match with a wildcard arm.
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

    /// A vendor reader could not open the input or decode a record.
    #[error("{vendor} reader: {source}")]
    #[non_exhaustive]
    Reader {
        /// Vendor name as returned by [`VendorFormat::name`](crate::VendorFormat::name).
        vendor: &'static str,
        /// The reader crate's own error (for example `opentfraw::Error`).
        #[source]
        source: BoxError,
    },

    /// Wrapping I/O error (creating or flushing the output, opening the
    /// input file).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Wrapping `openmassspec-core` error from the mzML writer or the
    /// conformance harness. Reader errors that travel through core as
    /// `openmassspec_core::Error::Vendor` are reported as
    /// [`Error::Reader`] instead.
    #[error(transparent)]
    Core(openmassspec_core::Error),

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

impl Error {
    /// Wrap a vendor reader's error for `vendor` (a
    /// [`VendorFormat::name`](crate::VendorFormat::name) value).
    pub fn reader<E>(vendor: &'static str, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Error::Reader {
            vendor,
            source: Box::new(err),
        }
    }

    /// The vendor reader's own error, when this error came from a reader.
    ///
    /// Downcast it to the reader crate's error type to inspect
    /// reader-specific detail. The reader crate must then be a direct
    /// dependency of your crate; this crate does not re-export it.
    ///
    /// ```
    /// use openmassspec_io::Error;
    ///
    /// // Stand-in for a reader crate's error type, e.g. `opentfraw::Error`.
    /// #[derive(Debug)]
    /// struct ScanTruncated(u32);
    /// impl std::fmt::Display for ScanTruncated {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "scan {} is truncated", self.0)
    ///     }
    /// }
    /// impl std::error::Error for ScanTruncated {}
    ///
    /// let err = Error::reader("thermo", ScanTruncated(7));
    /// match err.reader_error().and_then(|e| e.downcast_ref::<ScanTruncated>()) {
    ///     Some(ScanTruncated(scan)) => assert_eq!(*scan, 7),
    ///     None => unreachable!(),
    /// }
    /// ```
    pub fn reader_error(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        match self {
            Error::Reader { source, .. } => Some(source.as_ref()),
            Error::Core(openmassspec_core::Error::Vendor(source)) => Some(source.as_ref()),
            _ => None,
        }
    }

    /// The vendor whose reader produced this error, if any.
    pub fn reader_vendor(&self) -> Option<&'static str> {
        match self {
            Error::Reader { vendor, .. } => Some(vendor),
            _ => None,
        }
    }

    /// Attribute a reader error that arrived through core as
    /// `openmassspec_core::Error::Vendor` to `vendor`.
    pub(crate) fn attribute_to(self, vendor: &'static str) -> Self {
        match self {
            Error::Core(openmassspec_core::Error::Vendor(source)) => {
                Error::Reader { vendor, source }
            }
            other => other,
        }
    }
}

impl From<openmassspec_core::Error> for Error {
    fn from(e: openmassspec_core::Error) -> Self {
        Error::Core(e)
    }
}

/// Convenience alias mirroring the vendor crates.
pub type Result<T> = std::result::Result<T, Error>;
