//! End-to-end smoke test: detect each vendor format and write a tiny
//! mzML to a tempfile, asserting it is non-empty and starts with the
//! mzML preamble. Each branch is skipped silently when the
//! corresponding corpus path is not present so the test stays green on
//! a vanilla checkout.

use std::fs;
use std::path::PathBuf;

/// Returns the first candidate that exists on disk, or the first
/// candidate (the repo-root `corpus/` location) if none do - so the
/// existing `!input.exists()` skip-and-log check in `smoke`/
/// `centroid_smoke`/`stream_matches_collect` still prints a sensible
/// path when nothing is present.
///
/// Candidates are always given repo-root-first, sibling-checkout-second:
/// the repo-root `corpus/` path is the one `.github/workflows/ci.yml`'s
/// "Download corpus fixtures for smoke tests" step populates (Linux
/// only - see Sigilweaver/OpenMassSpec#24); the sibling-checkout path is
/// a local-dev fallback for machines with the relevant corpus checked
/// out next to this repo.
fn first_existing(candidates: &[PathBuf]) -> PathBuf {
    candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn thermo_fixture() -> PathBuf {
    first_existing(&[
        manifest_dir().join("../../corpus/thermo/PXD068962_Q_Exactive_UHMR_insource-CID.raw"),
        manifest_dir()
            .join("../../../SpecLance/corpus/thermo/PXD068962_Q_Exactive_UHMR_insource-CID.raw"),
    ])
}

fn waters_fixture() -> PathBuf {
    first_existing(&[
        manifest_dir().join("../../corpus/waters/molecular_mass_P15_01.raw"),
        manifest_dir().join("../../../SpecLance/corpus/waters/PXD058812/molecular_mass_P15_01.raw"),
    ])
}

fn bruker_fixture() -> PathBuf {
    first_existing(&[
        manifest_dir().join("../../corpus/bruker/NQO1-F107C_coi-N2-P_200-0C_3996.d"),
        manifest_dir().join(
            "../../../OpenTimsTDF/re/artifacts/cache/pride/PXD036417/NQO1-F107C_coi-N2-P_200-0C_3996.d",
        ),
    ])
}

fn shimadzu_qgd_fixture() -> PathBuf {
    first_existing(&[
        manifest_dir().join("../../corpus/shimadzu/PXD034978/49_27a__8122021_11.qgd"),
        manifest_dir().join("../../../Data/SZRaw/PXD034978/49_27a__8122021_11.qgd"),
    ])
}

fn shimadzu_lcd_ittof_fixture() -> PathBuf {
    first_existing(&[
        manifest_dir()
            .join("../../corpus/shimadzu/MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd"),
        manifest_dir()
            .join("../../../Data/SZRaw/MTBLS432/6-wk_HZ_CC_male_12_65__30min_pos-neg_43.lcd"),
    ])
}

fn shimadzu_lcd_qtof_fixture() -> PathBuf {
    first_existing(&[
        manifest_dir().join("../../corpus/shimadzu/MSV000084197/20190607_NM16.lcd"),
        manifest_dir().join("../../../Data/SZRaw/MSV000084197/20190607_NM16.lcd"),
    ])
}

fn agilent_fixture() -> PathBuf {
    first_existing(&[
        manifest_dir().join("../../corpus/agilent/180814-Sample19.d"),
        manifest_dir().join("../../../OpenARaw/corpus/PXD030293/180814-Sample19.d"),
    ])
}

/// The SCIEX reader needs the sibling `.wiff.scan` file present next to
/// the `.wiff` path returned here, not just the path itself - see the
/// download step in ci.yml, which fetches both.
fn sciex_fixture() -> PathBuf {
    first_existing(&[
        manifest_dir().join("../../corpus/sciex/PXD022088/Rcor2KOESC1.wiff"),
        manifest_dir().join("../../../OpenSXRaw/corpus/PXD022088/Rcor2KOESC1.wiff"),
    ])
}

/// Derive a filesystem-safe, per-input-file component for temp output
/// names. Needed because several vendors (e.g. Shimadzu's three on-disk
/// variants) share one `VendorFormat::name()`, so `name()` + pid alone
/// is not unique enough once more than one smoke test exists per
/// vendor. Tests run in parallel by default, and two tests racing on
/// the same temp path caused spurious "No such file or directory"
/// failures before this was added.
fn input_tag(input: &std::path::Path) -> String {
    input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("input")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn smoke(input: PathBuf) {
    if !input.exists() {
        eprintln!("skipping {}: corpus not present", input.display());
        return;
    }
    let det = openmassspec_io::detect_format(&input).expect("detect");
    let out = std::env::temp_dir().join(format!(
        "msio-smoke-{}-{}-{}.mzML",
        det.format.name(),
        input_tag(&input),
        std::process::id()
    ));
    openmassspec_io::convert_to_mzml(det, &out, false).expect("convert");
    let bytes = fs::read(&out).expect("read");
    assert!(
        bytes.len() > 4096,
        "mzML suspiciously small: {}",
        bytes.len()
    );
    let head = std::str::from_utf8(&bytes[..256.min(bytes.len())]).unwrap_or("");
    assert!(head.contains("<?xml"), "missing xml preamble");
    assert!(
        head.contains("mzML") || bytes.windows(4).any(|w| w == b"mzML"),
        "missing mzML root tag"
    );
    let _ = fs::remove_file(&out);
}

#[test]
fn thermo_smoke() {
    smoke(thermo_fixture());
}

#[test]
fn waters_smoke() {
    smoke(waters_fixture());
}

#[test]
fn bruker_smoke() {
    smoke(bruker_fixture());
}

#[test]
fn shimadzu_qgd_smoke() {
    smoke(shimadzu_qgd_fixture());
}

#[test]
fn shimadzu_lcd_ittof_smoke() {
    smoke(shimadzu_lcd_ittof_fixture());
}

#[test]
fn shimadzu_lcd_qtof_smoke() {
    smoke(shimadzu_lcd_qtof_fixture());
}

#[test]
fn agilent_smoke() {
    smoke(agilent_fixture());
}

#[test]
fn sciex_smoke() {
    smoke(sciex_fixture());
}

/// After `convert_to_mzml_centroided`, no spectrum in the output should
/// still be tagged profile mode - every profile spectrum was centroided,
/// and every already-centroid spectrum passed through unchanged. This
/// holds regardless of the input file's actual mode mix, so it's a
/// meaningful assertion even against real-world corpus data.
fn centroid_smoke(input: PathBuf) {
    if !input.exists() {
        eprintln!("skipping {}: corpus not present", input.display());
        return;
    }
    let det = openmassspec_io::detect_format(&input).expect("detect");
    let out = std::env::temp_dir().join(format!(
        "msio-centroid-smoke-{}-{}-{}.mzML",
        det.format.name(),
        input_tag(&input),
        std::process::id()
    ));
    openmassspec_io::convert_to_mzml_centroided(det, &out, false, None).expect("convert");
    let text = fs::read_to_string(&out).expect("read");
    assert!(
        !text.contains(r#"accession="MS:1000128""#),
        "output still contains a profile spectrum cvParam after centroiding"
    );
    assert!(
        text.contains(r#"accession="MS:1000127""#),
        "output has no centroid spectrum cvParam at all"
    );
    let _ = fs::remove_file(&out);
}

#[test]
fn thermo_centroid_smoke() {
    centroid_smoke(thermo_fixture());
}

#[test]
fn waters_centroid_smoke() {
    centroid_smoke(waters_fixture());
}

#[test]
fn bruker_centroid_smoke() {
    centroid_smoke(bruker_fixture());
}

#[test]
fn shimadzu_centroid_smoke() {
    // The QTOF variant is already centroid; MS1000127 should still be
    // present (pass-through), same invariant as the other vendors' tests.
    centroid_smoke(shimadzu_lcd_qtof_fixture());
}

#[test]
fn agilent_centroid_smoke() {
    centroid_smoke(agilent_fixture());
}

#[test]
fn sciex_centroid_smoke() {
    centroid_smoke(sciex_fixture());
}

/// `stream()`/`metadata_only()` must agree with `collect()` on both the
/// records visited and the run metadata returned, for every vendor. This
/// is the property issue #3 asked for: a lazy path that doesn't buffer the
/// whole run into a `Vec`, without silently diverging from the existing
/// two-pass API.
fn stream_matches_collect(input: PathBuf) {
    if !input.exists() {
        eprintln!("skipping {}: corpus not present", input.display());
        return;
    }
    let det = openmassspec_io::detect_format(&input).expect("detect");

    let (collected, collect_meta) = openmassspec_io::collect(det.clone()).expect("collect");

    let mut streamed = Vec::new();
    let stream_meta = openmassspec_io::stream(det.clone(), |rec| {
        streamed.push(rec);
        Ok(())
    })
    .expect("stream");

    assert_eq!(streamed.len(), collected.len(), "record count mismatch");
    for (a, b) in streamed.iter().zip(collected.iter()) {
        assert_eq!(a.native_id, b.native_id);
        assert_eq!(a.index, b.index);
        assert_eq!(a.mz, b.mz);
        assert_eq!(a.intensity, b.intensity);
    }

    let meta_only = openmassspec_io::metadata_only(det).expect("metadata_only");
    assert_eq!(meta_only.instrument.name, collect_meta.instrument.name);
    assert_eq!(meta_only.source_file_name, collect_meta.source_file_name);
    assert_eq!(stream_meta.instrument.name, collect_meta.instrument.name);
}

#[test]
fn thermo_stream_matches_collect() {
    stream_matches_collect(thermo_fixture());
}

#[test]
fn waters_stream_matches_collect() {
    stream_matches_collect(waters_fixture());
}

#[test]
fn bruker_stream_matches_collect() {
    stream_matches_collect(bruker_fixture());
}

#[test]
fn shimadzu_stream_matches_collect() {
    stream_matches_collect(shimadzu_qgd_fixture());
}

#[test]
fn agilent_stream_matches_collect() {
    stream_matches_collect(agilent_fixture());
}

#[test]
fn sciex_stream_matches_collect() {
    stream_matches_collect(sciex_fixture());
}
