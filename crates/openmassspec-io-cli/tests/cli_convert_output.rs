//! Integration tests for the output file of `vendor2mzml convert`: a gzip
//! output is a complete gzip stream, and a failed write is an error. A test
//! is skipped (with a message) when the corpus fixture is absent, unless
//! `OPENMASSSPEC_REQUIRE_CORPUS=1`, which turns a missing fixture into a
//! failure.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_vendor2mzml"))
}

fn thermo_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../corpus/thermo/PXD068962_Q_Exactive_UHMR_insource-CID.raw")
}

/// `true` when the fixture is missing and the test should be skipped.
/// Panics instead when `OPENMASSSPEC_REQUIRE_CORPUS=1`.
fn skip_missing(path: &Path) -> bool {
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

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vendor2mzml-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn run(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("run vendor2mzml")
}

#[test]
fn gzip_output_is_a_complete_gzip_stream() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }
    let dir = scratch_dir("gz");
    let plain = dir.join("out.mzML");
    let gz = dir.join("out.mzML.gz");
    for out in [&plain, &gz] {
        let o = run(&["convert".as_ref(), input.as_os_str(), out.as_os_str()]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(&gz).expect("open gz output"))
        .read_to_end(&mut decoded)
        .expect("gzip output must decode to the end (trailer present)");
    assert_eq!(decoded, std::fs::read(&plain).expect("read plain output"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn convert_to_a_full_device_is_an_error() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }
    if !Path::new("/dev/full").exists() {
        eprintln!("skipping: /dev/full not present");
        return;
    }
    let dir = scratch_dir("full");
    for name in ["out.mzML", "out.mzML.gz"] {
        let out = dir.join(name);
        std::os::unix::fs::symlink("/dev/full", &out).expect("symlink /dev/full");
        let o = run(&["convert".as_ref(), input.as_os_str(), out.as_os_str()]);
        assert_eq!(o.status.code(), Some(1), "{name}");
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("error"),
            "{name}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
