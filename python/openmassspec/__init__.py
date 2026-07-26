"""OpenMassSpec: open mass spectrometry vendor reader stack.

This metapackage is the single pip install surface for the stack. The
base install always brings ``openmassspec_io`` (the Rust-backed reader
that converts vendor inputs to mzML / Arrow). It has every vendor -
including SCIEX and Shimadzu - compiled in, so ``detect_format`` /
``to_mzml`` / ``iter_spectra`` work for all supported formats out of
the box. The per-vendor extras additionally install the standalone
Python binding for direct use:

* ``opentfraw``   - Thermo `.raw` files
* ``opentimstdf`` - Bruker timsTOF `.d/` bundles
* ``openwraw``    - Waters MassLynx `.raw/` directories
* ``openaraw``    - Agilent MassHunter `.d/` bundles
* ``opensxraw``   - SCIEX `.wiff`/`.wiff.scan` files
* ``openszraw``   - Shimadzu LabSolutions `.qgd`/`.lcd` files

Install the umbrella::

    pip install openmassspec            # openmassspec_io only (all vendors readable)
    pip install openmassspec[thermo]    # + opentfraw
    pip install openmassspec[bruker]    # + opentimstdf
    pip install openmassspec[waters]    # + openwraw
    pip install openmassspec[agilent]   # + openaraw
    pip install openmassspec[sciex]     # + opensxraw
    pip install openmassspec[shimadzu]  # + openszraw
    pip install openmassspec[all]       # + all standalone vendor bindings

Top-level helpers fall into two layers:

* ``detect_format``, ``to_mzml``, ``iter_spectra`` are re-exports from
  ``openmassspec_io`` - the vendor-agnostic reader.
* ``detect``, ``open_run`` delegate to ``detect_format`` (there is a
  single, canonical implementation of the structural checks, in
  ``openmassspec-io``'s Rust ``detect_format``) and dispatch to the
  vendor extension that matches the input path (requires the
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

VENDORS = ("thermo", "bruker", "waters", "agilent", "sciex", "shimadzu")


def detect(path: str | os.PathLike[str]) -> Optional[str]:
    """Return ``"thermo"``, ``"bruker"``, ``"waters"``, ``"agilent"``,
    ``"sciex"``, ``"shimadzu"`` or ``None`` for *path*.

    Delegates to :func:`detect_format` (the ``openmassspec_io`` Rust
    binding), which is the single, canonical implementation of every
    vendor's structural signature - extension and/or sentinel files,
    plus, where applicable, content checks such as Shimadzu's CFBF/OLE2
    magic bytes or Thermo's "Finnigan" marker. This function used to
    reimplement those checks by hand and had drifted from the Rust
    side (it never verified Shimadzu's magic bytes the way the Rust
    side does, and matched a wider, undocumented set of Waters sentinel
    files than the canonical ``_HEADER.TXT`` check); it now forwards to
    ``detect_format`` so there is one source of truth. See
    ``crates/openmassspec-io/src/lib.rs``'s ``detect_format`` (and
    ``docs/docs/format-detection.md``) for the exact per-vendor rules.

    The check is purely structural; no vendor reader needs to be
    importable.
    """
    if detect_format is None:
        raise ImportError(
            "openmassspec_io is required for detect(); it is a hard "
            "dependency of the openmassspec base install"
        )
    return detect_format(Path(path))


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
        import opensxraw  # type: ignore[import-not-found]

        return opensxraw.RawReader(str(path))
    if kind == "shimadzu":
        import openszraw  # type: ignore[import-not-found]

        return openszraw.RawReader(str(path))
    raise ValueError(f"unhandled vendor kind: {kind}")
