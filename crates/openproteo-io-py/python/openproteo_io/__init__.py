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
