"""Python bindings for openmassspec-io.

Detect acquisitions from Thermo, Bruker, Waters, Agilent, SCIEX, and Shimadzu; convert them to mzML,
or stream spectra as zero-copy NumPy arrays / pyarrow record batches.
"""

from ._openmassspec_io import (
    Chromatogram,
    RunInfo,
    Spectrum,
    __version__,
    detect,
    iter_spectra,
    read_chromatograms,
    run_info,
    to_mzml,
)

try:
    from ._openmassspec_io import read_arrow  # noqa: F401

    _HAS_ARROW = True
except ImportError:  # pragma: no cover - built without arrow feature
    _HAS_ARROW = False

__all__ = [
    "__version__",
    "detect",
    "to_mzml",
    "iter_spectra",
    "run_info",
    "RunInfo",
    "Chromatogram",
    "read_chromatograms",
    "Spectrum",
]

if _HAS_ARROW:
    __all__.append("read_arrow")

    def read_polars(path, batch_size=1024):
        """Read a vendor acquisition file into a Polars DataFrame via zero-copy Arrow.

        Requires the `polars` extra (`pip install openmassspec-io[polars]`).
        """
        import polars as pl

        return pl.from_arrow(read_arrow(path, batch_size=batch_size).read_all())

    __all__.append("read_polars")
