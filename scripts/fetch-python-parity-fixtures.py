"""Fetch the six public acquisitions used by Python field-parity CI."""

from __future__ import annotations

import shutil
import urllib.request
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1] / "corpus"
BASE = "https://ftp.pride.ebi.ac.uk/pride/data/archive"
FILES = {
    "thermo/PXD068962_Q_Exactive_UHMR_insource-CID.raw":
        f"{BASE}/2025/12/PXD068962/insource-CID.raw",
    "waters/molecular_mass_P15_01.raw.zip":
        f"{BASE}/2025/05/PXD058812/molecular_mass_P15_01.raw.zip",
    "bruker/NQO1-F107C_coi-N2-P_200-0C_3996.d.zip":
        f"{BASE}/2022/11/PXD036417/NQO1-F107C_coi-N2-P_200-0C_3996.d.zip",
    "agilent/180814-Sample19.d.zip":
        f"{BASE}/2022/07/PXD030293/180814-Sample19.d.zip",
    "sciex/PXD022088/Rcor2KOESC1.wiff":
        f"{BASE}/2020/12/PXD022088/Rcor2KOESC1.wiff",
    "sciex/PXD022088/Rcor2KOESC1.wiff.scan":
        f"{BASE}/2020/12/PXD022088/Rcor2KOESC1.wiff.scan",
    "shimadzu/PXD034978/49_27a__8122021_11.qgd":
        f"{BASE}/2022/08/PXD034978/49_27a__8122021_11.qgd",
}


def fetch(relative: str, url: str) -> Path:
    destination = ROOT / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.is_file() and destination.stat().st_size > 0:
        return destination
    request = urllib.request.Request(url, headers={"User-Agent": "OpenMassSpec-CI/1.0"})
    partial = destination.with_name(destination.name + ".part")
    with urllib.request.urlopen(request, timeout=180) as response, partial.open("wb") as output:
        shutil.copyfileobj(response, output)
    partial.replace(destination)
    return destination


def extract(archive: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    root = destination.resolve()
    with zipfile.ZipFile(archive) as bundle:
        members = bundle.infolist()
        for member in members:
            target = (root / member.filename).resolve()
            if target != root and root not in target.parents:
                raise ValueError(f"unsafe zip member in {archive}: {member.filename}")
        for member in members:
            target = root / member.filename
            # Skip files a previous run already extracted; some archives mark
            # their members read-only, so overwriting them fails.
            if not member.is_dir() and target.is_file() and target.stat().st_size == member.file_size:
                continue
            bundle.extract(member, root)


def main() -> None:
    for relative, url in FILES.items():
        archive = fetch(relative, url)
        print(f"fetched {relative} ({archive.stat().st_size} bytes)", flush=True)
        if relative.endswith(".zip"):
            extract(archive, archive.parent)


if __name__ == "__main__":
    main()
