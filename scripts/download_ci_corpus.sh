#!/usr/bin/env bash
# Download the vendor fixtures the ubuntu-latest CI leg tests against into
# corpus/. CI caches corpus/ keyed on this file and download_fixture.py, so
# changing a fixture here downloads a fresh set once.
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p corpus/thermo corpus/waters corpus/bruker \
  corpus/shimadzu/PXD034978 corpus/shimadzu/MTBLS432 corpus/shimadzu/MSV000084197 \
  corpus/agilent corpus/sciex/PXD022088

# Thermo (PXD068962, PRIDE, CC0)
python3 scripts/download_fixture.py \
  "https://ftp.pride.ebi.ac.uk/pride/data/archive/2025/12/PXD068962/insource-CID.raw" \
  corpus/thermo/PXD068962_Q_Exactive_UHMR_insource-CID.raw

# Waters (PXD058812, PRIDE, CC0) - zipped bundle directory
python3 scripts/download_fixture.py \
  "https://ftp.pride.ebi.ac.uk/pride/data/archive/2025/05/PXD058812/molecular_mass_P15_01.raw.zip" \
  waters.raw.zip
unzip -q -d corpus/waters waters.raw.zip
rm waters.raw.zip
test -f corpus/waters/molecular_mass_P15_01.raw/_HEADER.TXT

# Bruker (PXD036417, PRIDE, CC0) - zipped bundle directory, ~90 MB
python3 scripts/download_fixture.py \
  "https://ftp.pride.ebi.ac.uk/pride/data/archive/2022/11/PXD036417/NQO1-F107C_coi-N2-P_200-0C_3996.d.zip" \
  bruker.d.zip
unzip -q -d corpus/bruker bruker.d.zip
rm bruker.d.zip
test -f corpus/bruker/NQO1-F107C_coi-N2-P_200-0C_3996.d/analysis.tdf
test -f corpus/bruker/NQO1-F107C_coi-N2-P_200-0C_3996.d/analysis.tdf_bin

# Shimadzu QGD (PXD034978, PRIDE, CC0)
python3 scripts/download_fixture.py \
  "https://ftp.pride.ebi.ac.uk/pride/data/archive/2022/08/PXD034978/49_27a__8122021_11.qgd" \
  corpus/shimadzu/PXD034978/49_27a__8122021_11.qgd

# Shimadzu LCD, IT-TOF variant (MTBLS432, MetaboLights/EBI)
python3 scripts/download_fixture.py \
  "https://ftp.ebi.ac.uk/pub/databases/metabolights/studies/public/MTBLS432/FILES/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd" \
  "corpus/shimadzu/MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd"

# Shimadzu LCD, QTOF variant (MSV000084197, MassIVE, CC0). MassIVE
# rate-limits its HTTPS download endpoint per client IP (HTTP 429 on shared
# GitHub runners) and can return an HTTP-success error document, so the
# HTTPS copy is tried first and the FTPS server is the fallback. Either copy
# must match the known SHA-256 before it is used.
lcd_dest=corpus/shimadzu/MSV000084197/20190607_NM16.lcd
lcd_sha256=5934d55437e7c51dc2e53ed683882c032ce10eb3a6cda51c9ed19a56c0c7a49f
lcd_https="https://massive.ucsd.edu/ProteoSAFe/DownloadResultFile?file=f.MSV000084197%2Fraw%2F20190607_NM16.lcd"
lcd_ftps="ftp://massive-ftp.ucsd.edu/v02/MSV000084197/raw/20190607_NM16.lcd"

lcd_matches() {
  echo "$lcd_sha256  $1" | sha256sum --check --status
}

rm -f "$lcd_dest.part"
if python3 scripts/download_fixture.py "$lcd_https" "$lcd_dest.part" && lcd_matches "$lcd_dest.part"; then
  echo "MassIVE LCD fixture downloaded over HTTPS"
else
  echo "::warning::MassIVE HTTPS download failed or returned the wrong file; trying FTPS" >&2
  rm -f "$lcd_dest.part"
  curl --fail --silent --show-error --ssl-reqd --retry 5 --retry-delay 5 \
    --max-time 300 -o "$lcd_dest.part" "$lcd_ftps"
  if ! lcd_matches "$lcd_dest.part"; then
    rm -f "$lcd_dest.part"
    echo "MassIVE FTPS copy does not match SHA-256 $lcd_sha256: $lcd_ftps" >&2
    exit 1
  fi
fi
mv "$lcd_dest.part" "$lcd_dest"

# Agilent QTOF (PXD030293, PRIDE, CC0) - zipped bundle directory,
# ~960 KB zipped; same fixture wired into OpenARaw's own CI, see
# Sigilweaver/OpenARaw#28.
python3 scripts/download_fixture.py \
  "https://ftp.pride.ebi.ac.uk/pride/data/archive/2022/07/PXD030293/180814-Sample19.d.zip" \
  agilent.d.zip
unzip -q -d corpus/agilent agilent.d.zip
rm agilent.d.zip
test -d corpus/agilent/180814-Sample19.d

# SCIEX (PXD022088, PRIDE, CC0) - .wiff + .wiff.scan pair, ~2.5 MB
# total; same fixture wired into OpenSXRaw's own CI, see
# Sigilweaver/OpenSXRaw#33.
python3 scripts/download_fixture.py \
  "https://ftp.pride.ebi.ac.uk/pride/data/archive/2020/12/PXD022088/Rcor2KOESC1.wiff" \
  corpus/sciex/PXD022088/Rcor2KOESC1.wiff
python3 scripts/download_fixture.py \
  "https://ftp.pride.ebi.ac.uk/pride/data/archive/2020/12/PXD022088/Rcor2KOESC1.wiff.scan" \
  corpus/sciex/PXD022088/Rcor2KOESC1.wiff.scan

du -sh corpus
