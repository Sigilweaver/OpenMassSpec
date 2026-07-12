"""Smoke tests for openmassspec-io Python bindings.

These tests require the OpenTFRaw / OpenTimsTDF / OpenWRaw test corpora.
Set the env vars OPENMASSSPEC_THERMO_RAW, OPENMASSSPEC_BRUKER_D,
OPENMASSSPEC_WATERS_RAW to point at one sample each; tests for vendors
without a corpus path are skipped.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

import numpy as np
import openmassspec_io as opio
import pytest


def _corpus(envvar: str) -> Path | None:
    val = os.environ.get(envvar)
    if not val:
        return None
    p = Path(val)
    return p if p.exists() else None


CORPORA = {
    "thermo": _corpus("OPENMASSSPEC_THERMO_RAW"),
    "bruker": _corpus("OPENMASSSPEC_BRUKER_D"),
    "waters": _corpus("OPENMASSSPEC_WATERS_RAW"),
}


def test_version():
    assert opio.__version__


def test_detect_returns_none_for_garbage(tmp_path: Path):
    f = tmp_path / "garbage.bin"
    f.write_bytes(b"not a raw file")
    assert opio.detect(str(f)) is None


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_detect_matches_vendor(vendor: str):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    assert opio.detect(str(path)) == vendor


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_to_mzml_roundtrip(vendor: str, tmp_path: Path):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    out = tmp_path / f"{vendor}.mzML"
    opio.to_mzml(str(path), str(out), indexed=True)
    head = out.read_bytes()[:512]
    assert b"<indexedmzML" in head or b"<mzML" in head
    assert out.stat().st_size > 1024


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_iter_spectra_yields_numpy_arrays(vendor: str):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    n = 0
    saw_ms2 = False
    for spec in opio.iter_spectra(str(path)):
        assert spec.native_id
        assert spec.ms_level >= 1
        mz = spec.mz
        intensity = spec.intensity
        assert isinstance(mz, np.ndarray)
        assert mz.dtype == np.float64
        assert isinstance(intensity, np.ndarray)
        assert intensity.dtype == np.float32
        assert mz.shape == intensity.shape
        if spec.ms_level >= 2:
            saw_ms2 = True
            assert spec.precursor is not None
        n += 1
        if n >= 50:
            break
    assert n > 0
    # MS2 presence is a soft check; not every file has MS2 in the first 50.
    del saw_ms2


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_to_mzml_centroid_removes_profile_spectra(vendor: str, tmp_path: Path):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    out = tmp_path / f"{vendor}-centroid.mzML"
    opio.to_mzml(str(path), str(out), centroid=True)
    text = out.read_text(errors="replace")
    assert 'accession="MS:1000128"' not in text, "profile spectrum cvParam survived --centroid"
    assert 'accession="MS:1000127"' in text, "no centroid spectrum cvParam at all"


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_iter_spectra_centroid_flag_marks_every_spectrum_centroid(vendor: str):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    n = 0
    for spec in opio.iter_spectra(str(path), centroid=True):
        assert spec.scan_mode == "centroid"
        n += 1
        if n >= 50:
            break
    assert n > 0


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_read_arrow(vendor: str):
    if not hasattr(opio, "read_arrow"):
        pytest.skip("built without arrow feature")
    pyarrow = pytest.importorskip("pyarrow")
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    reader = opio.read_arrow(str(path), batch_size=64)
    total = 0
    for batch in reader:
        assert isinstance(batch, pyarrow.RecordBatch)
        total += batch.num_rows
        if total >= 64:
            break
    assert total > 0


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))
