//! Integration test for where `vendor2mzml convert` puts the writer's
//! spool file.
//!
//! The mzML writer holds encoded spectra until the source is exhausted and
//! spills them to a temporary file past 64 MiB. `convert` must put that
//! file in the output file's directory, not the system temp dir. The test
//! converts a fixture whose spectra exceed 64 MiB with `TMPDIR` pointing at
//! a directory that does not exist: the conversion only succeeds if the
//! spool goes next to the output. Skipped when the corpus fixture is not
//! present, matching the other corpus-backed CLI tests.

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_vendor2mzml"))
}

fn bruker_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/bruker/NQO1-F107C_coi-N2-P_200-0C_3996.d")
}

/// `<spectrumList count>`, and the `index` attribute of every `<spectrum>`
/// in document order.
fn spectrum_list(path: &Path) -> (usize, Vec<usize>) {
    fn attr(line: &str, name: &str) -> usize {
        let key = format!(" {name}=\"");
        let start = line.find(&key).expect("attribute present") + key.len();
        let end = start + line[start..].find('"').expect("closing quote");
        line[start..end].parse().expect("integer attribute")
    }
    let mut count = None;
    let mut indexes = Vec::new();
    for line in BufReader::new(File::open(path).unwrap()).lines() {
        let line = line.unwrap();
        let line = line.trim_start();
        if line.starts_with("<spectrumList ") {
            count = Some(attr(line, "count"));
        } else if line.starts_with("<spectrum ") {
            indexes.push(attr(line, "index"));
        }
    }
    (count.expect("<spectrumList> present"), indexes)
}

#[test]
fn convert_spools_next_to_the_output_not_in_tmpdir() {
    let input = bruker_fixture();
    if !input.exists() {
        eprintln!("skipping: corpus not present at {}", input.display());
        return;
    }
    let out_dir =
        std::env::temp_dir().join(format!("vendor2mzml-cli-spool-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.join("run.mzML");
    let missing_tmp = out_dir.join("no-such-tmpdir");

    let result = Command::new(bin_path())
        .env("TMPDIR", &missing_tmp)
        .args(["convert", "--indexed"])
        .arg(&input)
        .arg(&out)
        .output()
        .expect("spawn vendor2mzml");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "convert failed: {stderr}");

    // Precondition: the spectra are larger than the 64 MiB in-memory
    // limit, so the writer did spill to a file.
    let size = fs::metadata(&out).unwrap().len();
    assert!(size > 64 << 20, "fixture output is only {size} bytes");

    // The spool file is gone; only the output is left.
    let entries: Vec<_> = fs::read_dir(&out_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("run.mzML")]);

    let (count, indexes) = spectrum_list(&out);
    assert!(count > 0);
    assert_eq!(count, indexes.len());
    assert!(indexes.iter().enumerate().all(|(i, &idx)| i == idx));

    fs::remove_dir_all(&out_dir).unwrap();
}
