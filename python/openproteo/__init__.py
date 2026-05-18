"""OpenProteo: open proteomics vendor reader stack.

This is a thin metapackage. The data plane lives in three vendor reader
packages, each a Rust extension shipped via its own wheel:

* ``opentfraw``   - Thermo `.raw` files
* ``opentimstdf`` - Bruker timsTOF `.d/` bundles
* ``openwraw``    - Waters MassLynx `.raw/` directories

Install one vendor::

    pip install openproteo[thermo]
    pip install openproteo[bruker]
    pip install openproteo[waters]

Or install everything::

    pip install openproteo[all]

Then use the convenience helpers in this module, which detect the format
at a path and dispatch to the matching vendor reader.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Optional

__version__ = "0.1.0"
__all__ = ["__version__", "detect", "open_run", "VENDORS"]

VENDORS = ("thermo", "bruker", "waters")


def detect(path: str | os.PathLike[str]) -> Optional[str]:
    """Return ``"thermo"``, ``"bruker"``, ``"waters"`` or ``None`` for *path*.

    The check is purely structural (extension + sentinel files); no vendor
    reader needs to be importable.
    """
    p = Path(path)
    if not p.exists():
        return None
    if p.is_file() and p.suffix.lower() == ".raw":
        return "thermo"
    if p.is_dir():
        suffix = p.suffix.lower()
        if suffix == ".d" and (p / "analysis.tdf").is_file():
            return "bruker"
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
    raise ValueError(f"unhandled vendor kind: {kind}")
