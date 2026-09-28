"""Field and value parity checks for all six vendor Python surfaces.

Set OPENMASSSPEC_<VENDOR> paths to real public corpus files. Native package
comparisons run when the matching standalone binding is installed. Set
OPENMASSSPEC_REQUIRE_PARITY=1 to fail instead of skipping when a fixture or
native package is missing.
"""

from __future__ import annotations

import importlib
import os
from pathlib import Path

import numpy as np
import openmassspec_io as opio
import pytest


VENDORS = {
    "thermo": ("OPENMASSSPEC_THERMO_RAW", "opentfraw"),
    "bruker": ("OPENMASSSPEC_BRUKER_D", "opentimstdf"),
    "waters": ("OPENMASSSPEC_WATERS_RAW", "openwraw"),
    "agilent": ("OPENMASSSPEC_AGILENT_D", "openaraw"),
    "sciex": ("OPENMASSSPEC_SCIEX_WIFF", "opensxraw"),
    "shimadzu": ("OPENMASSSPEC_SHIMADZU_RAW", "openszraw"),
}

SPECTRUM_FIELDS = (
    "index", "scan_number", "native_id", "ms_level", "polarity",
    "scan_mode", "analyzer", "acquisition_event_id", "filter",
    "retention_time_sec", "total_ion_current", "base_peak_mz",
    "base_peak_intensity", "reported_total_ion_current",
    "reported_base_peak_mz", "reported_base_peak_intensity", "low_mz",
    "high_mz", "ion_injection_time_ms", "inv_mobility", "faims_cv",
    "precursor", "extra", "inv_mobility_per_peak",
)

PRECURSOR_FIELDS = {
    "target_mz", "selected_mz", "isolation_width", "charge", "intensity",
    "collision_energy", "ce_is_nce", "precursor_native_id", "activation",
    "analyzer", "ccs",
}

RUN_FIELDS = (
    "source_file_name", "source_file_format", "native_id_format",
    "instrument", "instrument_serial_number", "software_name",
    "software_version", "acquisition_software_name",
    "acquisition_software_version", "start_timestamp",
    "mobility_array_kind", "analyzers", "extra",
)


# Spectra walked in lockstep while looking for the first MS2 to compare.
MS2_SEARCH_LIMIT = 5000


def skip_or_fail(reason: str):
    if os.environ.get("OPENMASSSPEC_REQUIRE_PARITY") == "1":
        pytest.fail(reason)
    pytest.skip(reason)


def corpus_path(vendor: str) -> Path:
    variable = VENDORS[vendor][0]
    value = os.environ.get(variable)
    if not value or not Path(value).exists():
        skip_or_fail(f"set {variable} to a real vendor acquisition")
    return Path(value)


@pytest.mark.parametrize("vendor", VENDORS)
def test_shared_fields(vendor: str):
    path = corpus_path(vendor)
    assert opio.detect(str(path)) == vendor
    run = opio.run_info(str(path))
    for field in RUN_FIELDS:
        getattr(run, field)
    assert run.source_file_format["accession"]
    assert run.native_id_format["accession"]
    assert run.instrument["accession"]
    assert isinstance(run.extra, dict)

    spectrum = next(opio.iter_spectra(str(path)))
    for field in SPECTRUM_FIELDS:
        getattr(spectrum, field)
    assert spectrum.native_id
    assert isinstance(spectrum.extra, dict)
    if spectrum.precursor is not None:
        assert PRECURSOR_FIELDS <= spectrum.precursor.keys()
    mz, intensity = spectrum.mz, spectrum.intensity
    assert mz.dtype == np.float64
    assert intensity.dtype == np.float32
    assert mz.shape == intensity.shape

    for chrom in opio.read_chromatograms(str(path)):
        assert chrom.id
        assert chrom.chromatogram_type is None or chrom.chromatogram_type["accession"]
        assert len(chrom.time_sec) == len(chrom.intensity) == len(chrom)
        assert chrom.time_sec is chrom.time_sec
        assert chrom.intensity is chrom.intensity


def native_spectra(vendor: str, native, reader, path: Path):
    """Yield (native_id, field getter, mz, intensity) for each native spectrum."""
    if vendor in {"agilent", "sciex", "shimadzu"}:
        for record in native.iter_spectra(str(path)):
            yield record.native_id, (lambda f, r=record: getattr(r, f)), record.mz, record.intensity
    else:
        for record in reader.iter_records():
            yield record["native_id"], record.__getitem__, record["mz"], record["intensity"]


def assert_spectrum_parity(vendor: str, shared, native_record) -> None:
    native_id, get_field, mz, intensity = native_record
    for field in SPECTRUM_FIELDS:
        if field == "inv_mobility_per_peak":
            shared_mobility = getattr(shared, field)
            native_mobility = get_field(field)
            if shared_mobility is None:
                assert native_mobility is None
            else:
                np.testing.assert_array_equal(shared_mobility, native_mobility)
        else:
            assert getattr(shared, field) == get_field(field), (vendor, shared.index, field)
    assert shared.native_id == native_id
    np.testing.assert_array_equal(shared.mz, mz)
    np.testing.assert_array_equal(shared.intensity, np.asarray(intensity, dtype=np.float32))


@pytest.mark.parametrize("vendor", VENDORS)
def test_native_record_parity(vendor: str):
    path = corpus_path(vendor)
    module_name = VENDORS[vendor][1]
    try:
        native = importlib.import_module(module_name)
    except ImportError:
        skip_or_fail(f"{module_name} is not installed")

    reader_class = native.RawFile if vendor == "thermo" else (
        native.Reader if vendor == "bruker" else native.RawReader
    )
    reader = reader_class(str(path))
    shared_run = opio.run_info(str(path))
    native_run = reader.run_info()
    for field in RUN_FIELDS:
        assert getattr(shared_run, field) == native_run[field], (vendor, field)

    # Walk both streams together: native IDs must line up, and the first
    # spectrum plus the first MS2 (so precursor fields are exercised) are
    # compared field by field.
    pairs = zip(opio.iter_spectra(str(path)), native_spectra(vendor, native, reader, path))
    for position, (shared, native_record) in enumerate(pairs):
        if position >= MS2_SEARCH_LIMIT:
            break
        assert shared.native_id == native_record[0], (vendor, position)
        if position == 0 or shared.ms_level >= 2:
            assert_spectrum_parity(vendor, shared, native_record)
        if shared.ms_level >= 2:
            assert shared.precursor is not None
            break

    shared_chroms = opio.read_chromatograms(str(path))
    native_chroms = reader.read_chromatograms()
    assert [c.id for c in shared_chroms] == [c["id"] for c in native_chroms]
    for shared_chrom, native_chrom in zip(shared_chroms, native_chroms):
        np.testing.assert_array_equal(
            shared_chrom.time_sec, np.asarray(native_chrom["time_sec"], dtype=np.float32)
        )
        np.testing.assert_array_equal(
            shared_chrom.intensity, np.asarray(native_chrom["intensity"], dtype=np.float32)
        )


@pytest.mark.parametrize("vendor", VENDORS)
def test_arrow_preserves_vendor_fields(vendor: str):
    pyarrow = pytest.importorskip("pyarrow")
    if not hasattr(opio, "read_arrow"):
        pytest.skip("Arrow support is disabled")
    path = corpus_path(vendor)
    spectrum = next(opio.iter_spectra(str(path)))
    batch = opio.read_arrow(str(path), batch_size=1).read_next_batch()
    assert pyarrow.types.is_map(batch.schema.field("extra").type)
    assert batch.column("acquisition_event_id")[0].as_py() == spectrum.acquisition_event_id
    assert dict(batch.column("extra")[0].as_py()) == spectrum.extra
