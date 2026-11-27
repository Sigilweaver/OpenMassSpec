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
    "agilent": _corpus("OPENMASSSPEC_AGILENT_D"),
    "sciex": _corpus("OPENMASSSPEC_SCIEX_WIFF"),
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
    summary = opio.to_mzml(str(path), str(out), indexed=True)
    head = out.read_bytes()[:512]
    assert b"<indexedmzML" in head or b"<mzML" in head
    assert out.stat().st_size > 1024
    assert summary["spectra_written"] > 0
    assert set(summary) == {
        "spectra_written",
        "spectra_skipped",
        "spectra_invalid",
        "chromatograms_written",
        "chromatograms_skipped",
        "chromatograms_invalid",
        "warning_counts",
        "issues",
        "issues_truncated",
        "spectrum_count_mismatch",
    }


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_to_mzml_strict_matches_default_on_clean_input(vendor: str, tmp_path: Path):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    default = opio.to_mzml(str(path), str(tmp_path / "a.mzML"))
    if default["spectra_skipped"] or default["chromatograms_skipped"]:
        pytest.skip("fixture has unreadable records")
    strict = opio.to_mzml(str(path), str(tmp_path / "b.mzML"), skip_unreadable=False)
    assert strict == default


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


def _first_spectra(path: Path, n: int, read_arrays_first: bool):
    """Summaries of the first `n` non-empty spectra, reading the peak
    arrays before or after the derived values."""
    out = []
    for spec in opio.iter_spectra(str(path)):
        if read_arrays_first:
            n_peaks = len(spec.mz)
            len(spec.intensity)
        tic = spec.total_ion_current
        base_mz = spec.base_peak_mz
        base_int = spec.base_peak_intensity
        if not read_arrays_first:
            n_peaks = len(spec.mz)
            len(spec.intensity)
        if n_peaks == 0:
            continue
        out.append((spec.index, n_peaks, tic, base_mz, base_int))
        if len(out) == n:
            break
    return out


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_spectrum_getters_are_repeatable(vendor: str):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    n = 0
    for spec in opio.iter_spectra(str(path)):
        if len(spec.mz) == 0:
            continue
        first_mz = spec.mz.copy()
        first_int = spec.intensity.copy()
        # A second read returns the same data, not an empty array.
        assert len(first_mz) > 0
        assert np.array_equal(spec.mz, first_mz)
        assert np.array_equal(spec.intensity, first_int)
        assert f"n_peaks={len(first_mz)}" in repr(spec)
        mob_1 = spec.inv_mobility_per_peak
        mob_2 = spec.inv_mobility_per_peak
        assert (mob_1 is None) == (mob_2 is None)
        if mob_1 is not None:
            assert np.array_equal(mob_1, mob_2)
        n += 1
        if n == 20:
            break
    assert n > 0


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_derived_values_do_not_depend_on_read_order(vendor: str):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    arrays_first = _first_spectra(path, 20, read_arrays_first=True)
    arrays_last = _first_spectra(path, 20, read_arrays_first=False)
    assert arrays_first
    assert arrays_first == arrays_last


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_analyzer_labels_match_arrow(vendor: str):
    if not hasattr(opio, "read_arrow"):
        pytest.skip("built without arrow feature")
    pytest.importorskip("pyarrow")
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    py_labels = []
    for spec in opio.iter_spectra(str(path)):
        py_labels.append(spec.analyzer)
        if len(py_labels) == 64:
            break
    batch = next(iter(opio.read_arrow(str(path), batch_size=64)))
    arrow_labels = batch.column("analyzer").to_pylist()[: len(py_labels)]
    assert py_labels == arrow_labels


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


@pytest.mark.parametrize("vendor", ["thermo", "bruker", "waters"])
def test_spectrum_warnings_and_collision_energies(vendor: str):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    n = 0
    for spec in opio.iter_spectra(str(path)):
        warnings = spec.warnings
        assert isinstance(warnings, list)
        for w in warnings:
            assert set(w) == {"code", "message"}
        if len(spec.mz) == 0:
            # An empty spectrum is flagged and declares no base peak.
            assert "empty_spectrum" in [w["code"] for w in warnings]
            assert spec.base_peak_mz is None
        if spec.precursor is not None:
            p = spec.precursor
            assert "ce_is_nce" not in p
            for ce in p["collision_energies"]:
                assert set(ce) == {"value", "unit"}
                assert ce["unit"] in ("ev", "nce", None)
        assert spec.retention_time_sec is None or spec.retention_time_sec >= 0
        n += 1
        if n >= 200:
            break
    assert n > 0
    run = opio.run_info(str(path))
    assert run.unstated == []
    assert isinstance(run.warnings, list)


@pytest.mark.parametrize(
    ("vendor", "check", "code"),
    [
        ("agilent", "unknown_collision_energy_unit", "unknown_collision_energy_unit"),
        ("sciex", "unknown_scan_mode", "unknown_scan_mode"),
    ],
)
def test_unstated_formats_convert_without_allow(vendor: str, check: str, code: str, tmp_path: Path):
    path = CORPORA[vendor]
    if path is None:
        pytest.skip(f"no corpus for {vendor}")
    assert opio.run_info(str(path)).unstated == [check]
    with pytest.warns(RuntimeWarning, match=code):
        summary = opio.to_mzml(str(path), str(tmp_path / "out.mzML"))
    assert summary["spectra_skipped"] == 0
    assert summary["warning_counts"][code] > 0
    assert summary["warning_counts"]["unstated_by_source"] == 1
    flagged = [
        spec
        for spec in opio.iter_spectra(str(path))
        if code in [w["code"] for w in spec.warnings]
    ]
    assert flagged, "no spectrum carries the warning"


def test_to_mzml_spool_dir_is_accepted(tmp_path: Path):
    path = CORPORA["thermo"]
    if path is None:
        pytest.skip("no corpus for thermo")
    spool = tmp_path / "spool"
    spool.mkdir()
    opio.to_mzml(str(path), str(tmp_path / "x.mzML"), spool_dir=str(spool))
    assert list(spool.iterdir()) == []


def test_unknown_check_name_is_a_value_error(tmp_path: Path):
    path = CORPORA["thermo"]
    if path is None:
        pytest.skip("no corpus for thermo")
    with pytest.raises(ValueError, match="unknown_scan_mode"):
        opio.to_mzml(str(path), str(tmp_path / "x.mzML"), allow=["nope"])


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))
