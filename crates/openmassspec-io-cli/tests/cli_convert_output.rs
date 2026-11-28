//! Integration tests for what `vendor2mzml convert` and `info` do with
//! their paths: a gzip output is a complete gzip stream, a failed write is
//! an error, and an unreadable input is reported as such. The Thermo tests
//! are skipped when the corpus fixture is absent, unless
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

fn assert_io_error(o: &Output, what: &str) {
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{what}: {stderr}");
    assert!(stderr.contains("cannot read"), "{what}: {stderr}");
    assert!(
        !stderr.contains("does not look like a supported vendor format"),
        "{what}: {stderr}"
    );
}

#[test]
fn missing_input_is_reported_as_an_io_error() {
    let dir = scratch_dir("missing");
    let input = dir.join("absent.raw");
    let out = dir.join("out.mzML");
    assert_io_error(
        &run(&["convert".as_ref(), input.as_os_str(), out.as_os_str()]),
        "convert",
    );
    assert_io_error(&run(&["info".as_ref(), input.as_os_str()]), "info");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn unreadable_input_is_reported_as_an_io_error() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch_dir("denied");
    let locked = dir.join("locked");
    std::fs::create_dir(&locked).expect("create locked dir");
    let input = locked.join("sample.raw");
    std::fs::write(&input, b"x").expect("write input");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
        .expect("lock directory");
    // Root (or a filesystem ignoring modes) can still stat the file.
    let can_stat = std::fs::metadata(&input).is_ok();
    let out = dir.join("out.mzML");
    let convert = run(&["convert".as_ref(), input.as_os_str(), out.as_os_str()]);
    let info = run(&["info".as_ref(), input.as_os_str()]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
        .expect("unlock directory");
    let _ = std::fs::remove_dir_all(&dir);
    if can_stat {
        eprintln!("skipping: directory permissions are not enforced here");
        return;
    }
    assert_io_error(&convert, "convert");
    assert_io_error(&info, "info");
}

fn sciex_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/sciex/PXD022088/Rcor2KOESC1.wiff")
}

fn agilent_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/agilent/180814-Sample19.d")
}

/// A format that never states the scan mode converts with no `--allow`:
/// exit 0, a warning count per code on stderr, and `info` names what the
/// format cannot state.
#[test]
fn sciex_converts_without_allow_and_says_what_it_cannot_state() {
    let input = sciex_fixture();
    if skip_missing(&input) {
        return;
    }
    let dir = scratch_dir("sciex-unstated");
    let out = dir.join("out.mzML");
    let o = run(&["convert".as_ref(), input.as_os_str(), out.as_os_str()]);
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("with warning 'unknown_scan_mode'"),
        "{stderr}"
    );
    assert!(
        stderr.contains("with warning 'unstated_by_source'"),
        "{stderr}"
    );
    let text = std::fs::read_to_string(&out).expect("read output");
    assert!(text.contains("openmassspec: warning unstated_by_source"));

    let info = run(&["info".as_ref(), input.as_os_str()]);
    assert!(info.status.success());
    let stdout = String::from_utf8_lossy(&info.stdout);
    assert!(
        stdout.contains("never stated:  unknown_scan_mode"),
        "{stdout}"
    );
    let json = run(&["info".as_ref(), "--json".as_ref(), input.as_os_str()]);
    let stdout = String::from_utf8_lossy(&json.stdout);
    assert!(
        stdout.contains("\"unstated\":[\"unknown_scan_mode\"]"),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn agilent_converts_without_allow_and_flags_the_unit() {
    let input = agilent_fixture();
    if skip_missing(&input) {
        return;
    }
    let dir = scratch_dir("agilent-unstated");
    let out = dir.join("out.mzML");
    let o = run(&["convert".as_ref(), input.as_os_str(), out.as_os_str()]);
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("with warning 'unknown_collision_energy_unit'"),
        "{stderr}"
    );
    // The same file with the check named explicitly is accepted too.
    let o = run(&[
        "convert".as_ref(),
        "--allow".as_ref(),
        "unknown_collision_energy_unit".as_ref(),
        input.as_os_str(),
        out.as_os_str(),
    ]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// An unknown check name is a usage error that lists the valid ones, not a
/// silently ignored flag.
#[test]
fn allow_rejects_an_unknown_check_name() {
    let dir = scratch_dir("bad-allow");
    let o = run(&[
        "convert".as_ref(),
        "--allow".as_ref(),
        "no_such_check".as_ref(),
        "in.raw".as_ref(),
        dir.join("out.mzML").as_os_str(),
    ]);
    assert_eq!(o.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("no_such_check"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--spool-dir` is used: a directory that does not exist stops a
/// conversion large enough to spill.
#[test]
fn spool_dir_option_is_accepted() {
    let input = thermo_fixture();
    if skip_missing(&input) {
        return;
    }
    let dir = scratch_dir("spool-opt");
    let out = dir.join("out.mzML");
    let spool = dir.join("spool");
    std::fs::create_dir(&spool).expect("create spool dir");
    let o = run(&[
        "convert".as_ref(),
        "--spool-dir".as_ref(),
        spool.as_os_str(),
        input.as_os_str(),
        out.as_os_str(),
    ]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let left: Vec<_> = std::fs::read_dir(&spool).expect("read spool dir").collect();
    assert!(left.is_empty(), "spool file left behind: {left:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
