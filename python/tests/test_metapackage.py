"""Smoke tests for the `openproteo` metapackage.

These tests do not require any vendor corpus. They exercise:

* metadata (`__version__`, `__all__`),
* structural `detect()` against synthesized directories/files,
* `open_run()` error paths,
* presence of the `openproteo_io` re-exports (when installed).
"""

from __future__ import annotations

from pathlib import Path

import openproteo
import pytest


def test_version_string():
    assert isinstance(openproteo.__version__, str)
    assert openproteo.__version__.count(".") >= 1


def test_vendors_tuple():
    assert openproteo.VENDORS == ("thermo", "bruker", "waters")


def test_detect_thermo_file(tmp_path: Path):
    f = tmp_path / "sample.raw"
    f.write_bytes(b"")
    assert openproteo.detect(f) == "thermo"


def test_detect_bruker_d(tmp_path: Path):
    d = tmp_path / "sample.d"
    d.mkdir()
    (d / "analysis.tdf").write_bytes(b"")
    assert openproteo.detect(d) == "bruker"


def test_detect_waters_raw(tmp_path: Path):
    d = tmp_path / "sample.raw"
    d.mkdir()
    (d / "_HEADER.TXT").write_bytes(b"")
    assert openproteo.detect(d) == "waters"


def test_detect_unknown_returns_none(tmp_path: Path):
    p = tmp_path / "something.txt"
    p.write_bytes(b"hello")
    assert openproteo.detect(p) is None


def test_detect_missing_path_returns_none(tmp_path: Path):
    assert openproteo.detect(tmp_path / "does-not-exist") is None


def test_open_run_unknown_raises(tmp_path: Path):
    p = tmp_path / "nope.txt"
    p.write_bytes(b"")
    with pytest.raises(ValueError):
        openproteo.open_run(p)


def test_openproteo_io_reexports_present():
    # The base install pulls openproteo_io; the re-exports should be
    # importable callables. If openproteo_io is genuinely missing the
    # module falls back to None and we skip.
    if openproteo.to_mzml is None:
        pytest.skip("openproteo_io not importable in this environment")
    assert callable(openproteo.to_mzml)
    assert callable(openproteo.iter_spectra)
    assert callable(openproteo.detect_format)


def test_version_matches_installed_metadata():
    """Catch ``__version__`` drift from ``pyproject.toml`` early."""
    from importlib.metadata import PackageNotFoundError, version

    try:
        installed = version("openproteo")
    except PackageNotFoundError:
        pytest.skip("openproteo not installed (running from source)")
    assert openproteo.__version__ == installed


def test_open_run_thermo_dispatch(monkeypatch, tmp_path: Path):
    """``open_run`` on a thermo file imports opentfraw and calls ``RawFile``."""
    import sys
    import types

    f = tmp_path / "sample.raw"
    f.write_bytes(b"")
    calls: list[str] = []

    fake = types.ModuleType("opentfraw")
    fake.RawFile = lambda p: calls.append(("thermo", p)) or "thermo-handle"  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "opentfraw", fake)

    assert openproteo.open_run(f) == "thermo-handle"
    assert calls == [("thermo", str(f))]


def test_open_run_bruker_dispatch(monkeypatch, tmp_path: Path):
    import sys
    import types

    d = tmp_path / "sample.d"
    d.mkdir()
    (d / "analysis.tdf").write_bytes(b"")
    calls: list[str] = []

    fake = types.ModuleType("opentimstdf")
    fake.Reader = lambda p: calls.append(("bruker", p)) or "bruker-handle"  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "opentimstdf", fake)

    assert openproteo.open_run(d) == "bruker-handle"
    assert calls == [("bruker", str(d))]


def test_open_run_waters_dispatch(monkeypatch, tmp_path: Path):
    import sys
    import types

    d = tmp_path / "sample.raw"
    d.mkdir()
    (d / "_HEADER.TXT").write_bytes(b"")
    calls: list[str] = []

    fake = types.ModuleType("openwraw")
    fake.RawReader = lambda p: calls.append(("waters", p)) or "waters-handle"  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "openwraw", fake)

    assert openproteo.open_run(d) == "waters-handle"
    assert calls == [("waters", str(d))]


def test_vendors_is_immutable_tuple():
    assert isinstance(openproteo.VENDORS, tuple)
    with pytest.raises((TypeError, AttributeError)):
        openproteo.VENDORS[0] = "nope"  # type: ignore[index]


def test_public_api_surface():
    expected = {
        "__version__",
        "VENDORS",
        "Spectrum",
        "detect",
        "detect_format",
        "iter_spectra",
        "open_run",
        "to_mzml",
    }
    assert set(openproteo.__all__) == expected
