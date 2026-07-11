"""OpenMassSpec: open mass spectrometry vendor reader stack.

This metapackage is the single pip install surface for the stack. The
base install always brings ``openmassspec_io`` (the Rust-backed reader
that converts vendor inputs to mzML / Arrow). It has every vendor -
including SCIEX - compiled in, so ``detect_format`` / ``to_mzml`` /
``iter_spectra`` work for all supported formats out of the box. The
per-vendor extras additionally install the standalone Python binding
for direct use:

* ``opentfraw``   - Thermo `.raw` files
* ``opentimstdf`` - Bruker timsTOF `.d/` bundles
* ``openwraw``    - Waters MassLynx `.raw/` directories
* ``openaraw``    - Agilent MassHunter `.d/` bundles
* SCIEX `.wiff` reading is available from the base install; there is no
  standalone SCIEX Python package yet, so no ``sciex`` extra.

Install the umbrella::

    pip install openmassspec            # openmassspec_io only (all vendors readable)
    pip install openmassspec[thermo]    # + opentfraw
    pip install openmassspec[bruker]    # + opentimstdf
    pip install openmassspec[waters]    # + openwraw
    pip install openmassspec[agilent]   # + openaraw
    pip install openmassspec[all]       # + all standalone vendor bindings

Top-level helpers fall into two layers:

* ``detect_format``, ``to_mzml``, ``iter_spectra`` are re-exports from
  ``openmassspec_io`` - the vendor-agnostic reader.
* ``detect``, ``open_run`` use only structural checks and dispatch to
  the vendor extension that matches the input path (requires the
  corresponding extra).
"""

from __future__ import annotations

import os
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _pkg_version
from pathlib import Path
from typing import Optional

try:
    __version__ = _pkg_version("openmassspec")
except PackageNotFoundError:  # pragma: no cover - source checkout fallback
    __version__ = "0.0.0+unknown"

# Re-export the openmassspec_io reader surface so callers can write
# ``from openmassspec import to_mzml, iter_spectra, detect_format``.
try:
    from openmassspec_io import (  # type: ignore[import-not-found]
        Spectrum,
        iter_spectra,
        to_mzml,
    )
    from openmassspec_io import detect as detect_format  # type: ignore[import-not-found]
except ImportError:  # pragma: no cover - openmassspec_io is a hard dep
    Spectrum = None  # type: ignore[assignment]
    detect_format = None  # type: ignore[assignment]
    iter_spectra = None  # type: ignore[assignment]
    to_mzml = None  # type: ignore[assignment]

__all__ = [
    "__version__",
    "VENDORS",
    "Spectrum",
    "detect",
    "detect_format",
    "iter_spectra",
    "open_run",
    "to_mzml",
]

VENDORS = ("thermo", "bruker", "waters", "agilent", "sciex")


def detect(path: str | os.PathLike[str]) -> Optional[str]:
    """Return ``"thermo"``, ``"bruker"``, ``"waters"``, ``"agilent"``,
    ``"sciex"`` or ``None`` for *path*.

    The check is purely structural (extension + sentinel files); no vendor
    reader needs to be importable.
    """
    p = Path(path)
    if not p.exists():
        return None
    if p.is_file():
        if p.suffix.lower() == ".raw":
            return "thermo"
        # SCIEX: a .wiff file with its paired .wiff.scan alongside.
        if p.suffix.lower() == ".wiff" and Path(str(p) + ".scan").is_file():
            return "sciex"
    if p.is_dir():
        suffix = p.suffix.lower()
        # Bruker and Agilent both use a .d directory; disambiguate by contents.
        if suffix == ".d" and (p / "analysis.tdf").is_file():
            return "bruker"
        if (p / "AcqData" / "MSScan.bin").is_file():
            return "agilent"
        if suffix == ".raw" and any(
            (p / name).exists()
            for name in ("_FUNCTNS.INF", "_extern.inf", "_HEADER.TXT")
        ):
            return "waters"
    return None


def open_run(path: str | os.PathLike[str]):
    """Detect *path*, import the matching vendor package, and open the run.

    Raises ``ImportError`` if the matching vendor extra is not installed and
    ``ValueError`` if the format cannot be detected.
    """
    kind = detect(path)
    if kind is None:
        raise ValueError(f"no supported vendor format detected at {path}")
    if kind == "thermo":
        import opentfraw  # type: ignore[import-not-found]

        return opentfraw.RawFile(str(path))
    if kind == "bruker":
        import opentimstdf  # type: ignore[import-not-found]

        return opentimstdf.Reader(str(path))
    if kind == "waters":
        import openwraw  # type: ignore[import-not-found]

        return openwraw.RawReader(str(path))
    if kind == "agilent":
        import openaraw  # type: ignore[import-not-found]

        return openaraw.RawReader(str(path))
    if kind == "sciex":
        raise ImportError(
            "SCIEX .wiff reading has no standalone Python package yet; use "
            "openmassspec.to_mzml() / iter_spectra() / detect_format(), which "
            "read .wiff through the built-in openmassspec_io reader."
        )
    raise ValueError(f"unhandled vendor kind: {kind}")
