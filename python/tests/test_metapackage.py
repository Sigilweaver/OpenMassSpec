"""Smoke tests for the `openproteo` metapackage.

These tests do not require any vendor corpus. They exercise:

* metadata (`__version__`, `__all__`),
* structural `detect()` against synthesized directories/files,
* `open_run()` error paths,
* presence of the `openproteo_io` re-exports (when installed).
"""

from __future__ import annotations

from pathlib import Path

import pytest

import openproteo


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
