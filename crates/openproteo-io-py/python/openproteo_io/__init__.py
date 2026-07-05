"""Python bindings for openproteo-io.

Detect a vendor acquisition (Thermo / Bruker / Waters), convert it to mzML,
or stream spectra as zero-copy NumPy arrays / pyarrow record batches.
"""

from ._openproteo_io import (
    Spectrum,
    __version__,
    detect,
    iter_spectra,
    to_mzml,
)

try:
    from ._openproteo_io import read_arrow  # noqa: F401

    _HAS_ARROW = True
except ImportError:  # pragma: no cover - built without arrow feature
    _HAS_ARROW = False

__all__ = [
    "__version__",
    "detect",
    "to_mzml",
    "iter_spectra",
    "Spectrum",
]

if _HAS_ARROW:
    __all__.append("read_arrow")

    def read_polars(path, batch_size=1024):
        """Read a vendor acquisition file into a Polars DataFrame via zero-copy Arrow.

        Requires the `polars` extra (`pip install openproteo-io[polars]`).
        """
        import polars as pl

        return pl.from_arrow(read_arrow(path, batch_size=batch_size).read_all())

    __all__.append("read_polars")
