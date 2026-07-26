"""Smoke tests for the `openmassspec` metapackage.

These tests do not require any vendor corpus. They exercise:

* metadata (`__version__`, `__all__`),
* structural `detect()` against synthesized directories/files,
* `open_run()` error paths,
* presence of the `openmassspec_io` re-exports (when installed).
"""

from __future__ import annotations

from pathlib import Path

import openmassspec
import pytest

# `detect()` forwards to `openmassspec_io.detect_format`, which - like the
# Rust `detect_format` it binds - verifies content, not just extension, for
# Thermo and Shimadzu. These are the exact byte sequences
# `crates/openmassspec-io/src/lib.rs`'s `is_thermo_raw`/
# `is_shimadzu_labsolutions` check for; see `docs/docs/format-detection.md`.
FINNIGAN_HEADER = bytes(
    [
        0x01,
        0xA1,
        0x46,
        0x00,
        0x69,
        0x00,
        0x6E,
        0x00,
        0x6E,
        0x00,
        0x69,
        0x00,
        0x67,
        0x00,
        0x61,
        0x00,
        0x6E,
        0x00,
    ]
)
CFBF_MAGIC = bytes([0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1])


def test_version_string():
    assert isinstance(openmassspec.__version__, str)
    assert openmassspec.__version__.count(".") >= 1


def test_vendors_tuple():
    assert openmassspec.VENDORS == (
        "thermo",
        "bruker",
        "waters",
        "agilent",
        "sciex",
        "shimadzu",
    )


def test_detect_thermo_file(tmp_path: Path):
    f = tmp_path / "sample.raw"
    f.write_bytes(FINNIGAN_HEADER)
    assert openmassspec.detect(f) == "thermo"


def test_detect_thermo_file_without_finnigan_magic_is_none(tmp_path: Path):
    """A bare `.raw` extension is not enough - content must match too.

    Regression test for the bug in #19: the old hand-rolled `detect()`
    trusted the `.raw` extension alone and never checked the Finnigan
    magic bytes the way the canonical Rust `detect_format` does.
    """
    f = tmp_path / "sample.raw"
    f.write_bytes(b"not actually a thermo file")
    assert openmassspec.detect(f) is None


def test_detect_bruker_d(tmp_path: Path):
    d = tmp_path / "sample.d"
    d.mkdir()
    (d / "analysis.tdf").write_bytes(b"")
    (d / "analysis.tdf_bin").write_bytes(b"")
    assert openmassspec.detect(d) == "bruker"


def test_detect_waters_raw(tmp_path: Path):
    d = tmp_path / "sample.raw"
    d.mkdir()
    (d / "_HEADER.TXT").write_bytes(b"")
    assert openmassspec.detect(d) == "waters"


def test_detect_agilent_d(tmp_path: Path):
    d = tmp_path / "sample.d"
    d.mkdir()
    (d / "AcqData").mkdir()
    (d / "AcqData" / "MSScan.bin").write_bytes(b"")
    assert openmassspec.detect(d) == "agilent"


def test_detect_sciex_wiff(tmp_path: Path):
    f = tmp_path / "sample.wiff"
    f.write_bytes(b"")
    (tmp_path / "sample.wiff.scan").write_bytes(b"")
    assert openmassspec.detect(f) == "sciex"


def test_detect_shimadzu_lcd(tmp_path: Path):
    f = tmp_path / "sample.lcd"
    f.write_bytes(CFBF_MAGIC)
    assert openmassspec.detect(f) == "shimadzu"


def test_detect_shimadzu_qgd(tmp_path: Path):
    f = tmp_path / "sample.qgd"
    f.write_bytes(CFBF_MAGIC)
    assert openmassspec.detect(f) == "shimadzu"


def test_detect_shimadzu_lcd_without_cfbf_magic_is_none(tmp_path: Path):
    """A bare `.lcd`/`.qgd` extension is not enough - content must match too.

    Regression test for the bug in #19: the old hand-rolled `detect()`
    trusted the extension alone for Shimadzu and never checked the
    CFBF/OLE2 magic bytes the way the canonical Rust `detect_format`
    does - so it would have wrongly called a renamed/garbage file
    Shimadzu.
    """
    f = tmp_path / "sample.lcd"
    f.write_bytes(b"not a real container")
    assert openmassspec.detect(f) is None


def test_detect_wiff_without_scan_is_none(tmp_path: Path):
    f = tmp_path / "lonely.wiff"
    f.write_bytes(b"")
    assert openmassspec.detect(f) is None


def test_detect_unknown_returns_none(tmp_path: Path):
    p = tmp_path / "something.txt"
    p.write_bytes(b"hello")
    assert openmassspec.detect(p) is None


def test_detect_missing_path_returns_none(tmp_path: Path):
    assert openmassspec.detect(tmp_path / "does-not-exist") is None


def test_open_run_unknown_raises(tmp_path: Path):
    p = tmp_path / "nope.txt"
    p.write_bytes(b"")
    with pytest.raises(ValueError):
        openmassspec.open_run(p)


def test_openmassspec_io_reexports_present():
    # The base install pulls openmassspec_io; the re-exports should be
    # importable callables. If openmassspec_io is genuinely missing the
    # module falls back to None and we skip.
    if openmassspec.to_mzml is None:
        pytest.skip("openmassspec_io not importable in this environment")
    assert callable(openmassspec.to_mzml)
    assert callable(openmassspec.iter_spectra)
    assert callable(openmassspec.detect_format)


def test_version_matches_installed_metadata():
    """Catch ``__version__`` drift from ``pyproject.toml`` early."""
    from importlib.metadata import PackageNotFoundError, version

    try:
        installed = version("openmassspec")
    except PackageNotFoundError:
        pytest.skip("openmassspec not installed (running from source)")
    assert openmassspec.__version__ == installed


def test_open_run_thermo_dispatch(monkeypatch, tmp_path: Path):
    """``open_run`` on a thermo file imports opentfraw and calls ``RawFile``."""
    import sys
    import types

    f = tmp_path / "sample.raw"
    f.write_bytes(FINNIGAN_HEADER)
    calls: list[str] = []

    fake = types.ModuleType("opentfraw")
    fake.RawFile = lambda p: calls.append(("thermo", p)) or "thermo-handle"  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "opentfraw", fake)

    assert openmassspec.open_run(f) == "thermo-handle"
    assert calls == [("thermo", str(f))]


def test_open_run_bruker_dispatch(monkeypatch, tmp_path: Path):
    import sys
    import types

    d = tmp_path / "sample.d"
    d.mkdir()
    (d / "analysis.tdf").write_bytes(b"")
    (d / "analysis.tdf_bin").write_bytes(b"")
    calls: list[str] = []

    fake = types.ModuleType("opentimstdf")
    fake.Reader = lambda p: calls.append(("bruker", p)) or "bruker-handle"  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "opentimstdf", fake)

    assert openmassspec.open_run(d) == "bruker-handle"
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

    assert openmassspec.open_run(d) == "waters-handle"
    assert calls == [("waters", str(d))]


def test_open_run_agilent_dispatch(monkeypatch, tmp_path: Path):
    import sys
    import types

    d = tmp_path / "sample.d"
    d.mkdir()
    (d / "AcqData").mkdir()
    (d / "AcqData" / "MSScan.bin").write_bytes(b"")
    calls: list[str] = []

    fake = types.ModuleType("openaraw")
    fake.RawReader = lambda p: calls.append(("agilent", p)) or "agilent-handle"  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "openaraw", fake)

    assert openmassspec.open_run(d) == "agilent-handle"
    assert calls == [("agilent", str(d))]


def test_open_run_sciex_dispatch(monkeypatch, tmp_path: Path):
    """``open_run`` on a SCIEX file imports opensxraw and calls ``RawReader``."""
    import sys
    import types

    f = tmp_path / "sample.wiff"
    f.write_bytes(b"")
    (tmp_path / "sample.wiff.scan").write_bytes(b"")
    calls: list[str] = []

    fake = types.ModuleType("opensxraw")
    fake.RawReader = lambda p: calls.append(("sciex", p)) or "sciex-handle"  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "opensxraw", fake)

    assert openmassspec.open_run(f) == "sciex-handle"
    assert calls == [("sciex", str(f))]


def test_open_run_shimadzu_dispatch(monkeypatch, tmp_path: Path):
    """``open_run`` on a Shimadzu file imports openszraw and calls ``RawReader``."""
    import sys
    import types

    f = tmp_path / "sample.lcd"
    f.write_bytes(CFBF_MAGIC)
    calls: list[str] = []

    fake = types.ModuleType("openszraw")
    fake.RawReader = lambda p: calls.append(("shimadzu", p)) or "shimadzu-handle"  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, "openszraw", fake)

    assert openmassspec.open_run(f) == "shimadzu-handle"
    assert calls == [("shimadzu", str(f))]


def test_vendors_is_immutable_tuple():
    assert isinstance(openmassspec.VENDORS, tuple)
    with pytest.raises((TypeError, AttributeError)):
        openmassspec.VENDORS[0] = "nope"  # type: ignore[index]


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
    assert set(openmassspec.__all__) == expected
