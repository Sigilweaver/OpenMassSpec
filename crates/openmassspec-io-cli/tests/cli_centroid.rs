//! Integration test for `vendor2mzml convert --centroid` / `info --centroid`,
//! exercised against a real vendor fixture (skipped silently when the
//! corpus path is not present, matching `openmassspec-io`'s own
//! `vendor2mzml.rs` test convention).

use std::path::PathBuf;
use std::process::Command;

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_vendor2mzml"))
}

fn thermo_fixture() -> PathBuf {
    PathBuf::from("../../../SpecLance/corpus/thermo/PXD068962_Q_Exactive_UHMR_insource-CID.raw")
}

#[test]
fn convert_centroid_flag_produces_no_profile_spectra() {
    let input = thermo_fixture();
    if !input.exists() {
        eprintln!("skipping: corpus not present at {}", input.display());
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
    if !input.exists() {
        eprintln!("skipping: corpus not present at {}", input.display());
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
