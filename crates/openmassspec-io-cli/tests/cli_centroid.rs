//! Integration test for `vendor2mzml convert --centroid` / `info --centroid`,
//! exercised against a real vendor fixture. A test is skipped (with a
//! message) when the fixture is absent, unless `OPENMASSSPEC_REQUIRE_CORPUS=1`,
//! which turns a missing fixture into a failure.

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

/// `(spectra, total points, profile terms, centroid terms)` of an mzML file.
fn mzml_counts(text: &str) -> (usize, usize, usize, usize) {
    let mut points = 0;
    let mut spectra = 0;
    for part in text.split("<spectrum ").skip(1) {
        spectra += 1;
        let key = "defaultArrayLength=\"";
        let at = part.find(key).expect("defaultArrayLength") + key.len();
        let end = part[at..].find('"').expect("closing quote");
        points += part[at..at + end]
            .parse::<usize>()
            .expect("numeric defaultArrayLength");
    }
    (
        spectra,
        points,
        text.matches(r#"accession="MS:1000128""#).count(),
        text.matches(r#"accession="MS:1000127""#).count(),
    )
}

fn convert_text(input: &std::path::Path, tag: &str, flags: &[&str]) -> String {
    let out = std::env::temp_dir().join(format!("vendor2mzml-{tag}-{}.mzML", std::process::id()));
    let output = Command::new(bin_path())
        .arg("convert")
        .arg(input)
        .arg(&out)
        .args(flags)
        .output()
        .expect("run vendor2mzml convert");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::fs::read_to_string(&out).expect("read output mzML");
    let _ = std::fs::remove_file(&out);
    text
}

#[test]
fn thermo_profile_scans_are_written_as_dense_profile() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }
    let text = convert_text(&input, "thermo-profile", &[]);
    let (spectra, points, profile, centroid) = mzml_counts(&text);
    assert_eq!(spectra, 3047);
    // Every spectrum is labelled one way or the other.
    assert_eq!(profile + centroid, spectra);
    assert!(profile > 0, "no profile spectra exported");
    // The reader's centroid lists hold 199,722 points in this file; its
    // profile data holds over a million. A profile-labelled export below
    // half a million points is a peak list tagged as profile.
    assert!(
        points > 500_000,
        "only {points} points: profile data dropped?"
    );
}

#[test]
fn centroid_flag_picks_peaks_from_thermo_profile_scans() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }
    let text = convert_text(&input, "thermo-centroid-pick", &["--centroid"]);
    let (spectra, peaks, profile, centroid) = mzml_counts(&text);
    assert_eq!(profile, 0);
    assert_eq!(centroid, spectra);
    // Picking peaks from the profile data lands within a few percent of the
    // 199,722 peaks in the reader's own centroid lists. Re-centroiding
    // those lists keeps only 66,200.
    assert!(
        (180_000..220_000).contains(&peaks),
        "{peaks} peaks after --centroid"
    );
}
