//! Integration test for `vendor2mzml convert --centroid` / `info --centroid`,
//! exercised against real vendor fixtures. A test is skipped (with a message)
//! when its fixture is absent, unless `OPENMASSSPEC_REQUIRE_CORPUS=1`, which
//! turns a missing fixture into a failure.

use std::path::PathBuf;
use std::process::Command;

/// `true` when the fixture is missing and the test should be skipped.
/// Panics instead when `OPENMASSSPEC_REQUIRE_CORPUS=1`.
fn skip_missing(path: &std::path::Path) -> bool {
    if path.exists() {
        return false;
    }
    assert!(
        std::env::var("OPENMASSSPEC_REQUIRE_CORPUS").as_deref() != Ok("1"),
        "OPENMASSSPEC_REQUIRE_CORPUS=1 but fixture is missing: {}",
        path.display()
    );
    eprintln!("skipping: corpus not present at {}", path.display());
    true
}

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_vendor2mzml"))
}

fn thermo_fixture() -> PathBuf {
    // Repo-root `corpus/` first (populated by ci.yml's "Download corpus
    // fixtures for smoke tests" step, Linux only - see
    // Sigilweaver/OpenMassSpec#24); old sibling-checkout path as a
    // local-dev fallback.
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../corpus/thermo/PXD068962_Q_Exactive_UHMR_insource-CID.raw"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../SpecLance/corpus/thermo/PXD068962_Q_Exactive_UHMR_insource-CID.raw"),
    ];
    candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

#[test]
fn convert_centroid_flag_produces_no_profile_spectra() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }
    let out = std::env::temp_dir().join(format!(
        "vendor2mzml-cli-centroid-{}.mzML",
        std::process::id()
    ));

    let status = Command::new(bin_path())
        .arg("convert")
        .arg(&input)
        .arg(&out)
        .arg("--centroid")
        .status()
        .expect("run vendor2mzml convert --centroid");
    assert!(status.success(), "convert --centroid exited non-zero");

    let text = std::fs::read_to_string(&out).expect("read output mzML");
    let _ = std::fs::remove_file(&out);
    assert!(
        !text.contains(r#"accession="MS:1000128""#),
        "output still contains a profile spectrum cvParam after --centroid"
    );
    assert!(
        text.contains(r#"accession="MS:1000127""#),
        "output has no centroid spectrum cvParam at all"
    );
}

#[test]
fn info_centroid_flag_runs_successfully() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }

    let out = Command::new(bin_path())
        .arg("info")
        .arg(&input)
        .arg("--centroid")
        .arg("--json")
        .output()
        .expect("run vendor2mzml info --centroid");
    assert!(
        out.status.success(),
        "info --centroid exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"spectrum_count\":"),
        "missing spectrum_count in info --json output: {stdout}"
    );
}

fn sciex_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/sciex/PXD022088/Rcor2KOESC1.wiff")
}

/// `(spectra, total peaks, profile terms, centroid terms)` of an mzML file.
fn mzml_counts(text: &str) -> (usize, usize, usize, usize) {
    let mut peaks = 0;
    let mut spectra = 0;
    for part in text.split("<spectrum ").skip(1) {
        spectra += 1;
        let key = "defaultArrayLength=\"";
        let at = part.find(key).expect("defaultArrayLength") + key.len();
        let end = part[at..].find('"').unwrap();
        peaks += part[at..at + end].parse::<usize>().unwrap();
    }
    (
        spectra,
        peaks,
        text.matches(r#"accession="MS:1000128""#).count(),
        text.matches(r#"accession="MS:1000127""#).count(),
    )
}

fn convert(input: &std::path::Path, tag: &str, flags: &[&str]) -> (std::process::Output, String) {
    let out = std::env::temp_dir().join(format!("vendor2mzml-{tag}-{}.mzML", std::process::id()));
    let output = Command::new(bin_path())
        .arg("convert")
        .arg(input)
        .arg(&out)
        .args(flags)
        .output()
        .expect("run vendor2mzml convert");
    let text = std::fs::read_to_string(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    (output, text)
}

#[test]
fn thermo_profile_scans_are_written_as_dense_profile() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }
    let (out, text) = convert(&input, "thermo-profile", &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let (spectra, peaks, profile, centroid) = mzml_counts(&text);
    // Every spectrum is labelled one way or the other. Reading this file's
    // scans as centroid lists gives 199,722 points; exporting profile data
    // gives 1,157,028. A profile-labelled export with fewer than half a
    // million points would be the old peak-list-labelled-profile output.
    assert_eq!(profile + centroid, spectra);
    assert_eq!(spectra, 3047);
    assert!(profile > 0, "no profile spectra exported");
    assert!(
        peaks > 500_000,
        "only {peaks} points: profile data dropped?"
    );
}

#[test]
fn centroid_flag_keeps_the_peaks_of_thermo_profile_scans() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }
    let (out, text) = convert(&input, "thermo-centroid-keep", &["--centroid"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let (spectra, peaks, profile, centroid) = mzml_counts(&text);
    assert_eq!(profile, 0);
    assert_eq!(centroid, spectra);
    // Centroiding the profile data lands within a few percent of the
    // 199,722 peaks the reader's own centroid lists hold. The old
    // re-centroiding of those lists kept 66,200.
    assert!(
        (180_000..220_000).contains(&peaks),
        "{peaks} peaks after --centroid"
    );
}

#[test]
fn sciex_without_scan_mode_fails_unless_allowed() {
    let input = sciex_fixture();
    if skip_missing(&input) {
        return;
    }
    let (out, _) = convert(&input, "sciex-strict", &["--strict"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("scan mode is unknown"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let (out, text) = convert(&input, "sciex-allow", &["--allow", "unknown_scan_mode"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let (spectra, _, profile, centroid) = mzml_counts(&text);
    assert_eq!(
        (profile, centroid),
        (0, 0),
        "must not assert a representation"
    );
    assert_eq!(
        text.matches("openmassspec: warning unknown_scan_mode")
            .count(),
        spectra
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown_scan_mode"));
}

#[test]
fn sciex_centroid_with_unknown_scan_mode_leaves_peaks_unchanged() {
    let input = sciex_fixture();
    if skip_missing(&input) {
        return;
    }
    let (_, plain) = convert(&input, "sciex-plain", &["--allow", "unknown_scan_mode"]);
    let (out, cent) = convert(
        &input,
        "sciex-cent",
        &["--allow", "unknown_scan_mode", "--centroid"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(mzml_counts(&plain).1, mzml_counts(&cent).1, "peaks changed");
    let (out, _) = convert(&input, "sciex-cent-strict", &["--centroid", "--strict"]);
    assert_eq!(out.status.code(), Some(1));
}
