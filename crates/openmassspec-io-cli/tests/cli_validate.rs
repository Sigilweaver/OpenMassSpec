//! Integration tests for `vendor2mzml validate`.
//!
//! These tests synthesize a tiny mzML on disk using `openmassspec-core`'s
//! own writer, then invoke the compiled `vendor2mzml` binary against it
//! to exercise the end-to-end CLI path (including the mzdata-based
//! mzML reader and the conformance harness).

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::Command;

use openmassspec_core::{write_mzml, CvTerm, Polarity, PrecursorInfo, RunMetadata, SpectrumRecord};
use openmassspec_io::VecSource;

fn bin_path() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by Cargo for integration tests of the
    // binaries in the same crate.
    PathBuf::from(env!("CARGO_BIN_EXE_vendor2mzml"))
}

fn make_metadata() -> RunMetadata {
    RunMetadata::new(
        "synthetic.raw",
        CvTerm::new("MS:1000563", "Thermo RAW format"),
        CvTerm::new("MS:1000768", "Thermo nativeID format"),
        CvTerm::new("MS:1000031", "instrument model"),
        "openmassspec-io-cli-test",
        "0.0.0",
    )
}

fn make_record(index: usize, scan: u32, ms_level: u32, rt_sec: f64) -> SpectrumRecord {
    let precursor = (ms_level >= 2).then(|| {
        PrecursorInfo::default()
            .with_target_mz(150.0)
            .with_selected_mz(150.0)
            .with_isolation_width(2.0)
            .with_charge(2)
            .with_intensity(1.0e5)
            .with_collision_energy(28.0)
            .with_ce_is_nce(true)
            .with_precursor_native_id(format!(
                "controllerType=0 controllerNumber=1 scan={}",
                scan - 1
            ))
            .with_activation(openmassspec_core::Activation::HCD)
    });
    SpectrumRecord::new(
        index,
        scan,
        format!("controllerType=0 controllerNumber=1 scan={scan}"),
    )
    .with_ms_level(ms_level)
    .with_polarity(Polarity::Positive)
    .with_scan_mode(openmassspec_core::ScanMode::Centroid)
    .with_retention_time_sec(rt_sec)
    .with_total_ion_current(60.0)
    .with_base_peak_mz(300.0)
    .with_base_peak_intensity(30.0)
    .with_low_mz(100.0)
    .with_high_mz(300.0)
    .with_precursor(precursor)
    .with_peaks(vec![100.0, 200.0, 300.0], vec![10.0f32, 20.0, 30.0])
}

fn write_good_mzml(path: &std::path::Path) {
    let metadata = make_metadata();
    let records = vec![
        make_record(0, 1, 1, 0.5),
        make_record(1, 2, 2, 0.8),
        make_record(2, 3, 1, 1.2),
    ];
    let mut src = VecSource::new(metadata, records);
    let f = File::create(path).expect("create mzml");
    let mut w = BufWriter::new(f);
    write_mzml(&mut src, &mut w).expect("write mzml");
    w.flush().expect("flush");
}

fn tmp_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("vendor2mzml-validate-{pid}-{nanos}-{name}"));
    p
}

#[test]
fn validate_good_mzml_succeeds() {
    let path = tmp_path("good.mzML");
    write_good_mzml(&path);

    let out = Command::new(bin_path())
        .arg("validate")
        .arg("--json")
        .arg(&path)
        .output()
        .expect("run vendor2mzml");
    let _ = std::fs::remove_file(&path);

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "expected success, got status={:?}, stdout={}, stderr={}",
        out.status.code(),
        stdout,
        stderr
    );
    assert!(stdout.contains("\"ok\":true"), "stdout was: {stdout}");
    assert!(stdout.contains("\"kind\":\"mzML\""), "stdout was: {stdout}");
    assert!(
        stdout.contains("\"spectrum_count\":3"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_bad_mzml_reports_conformance_failure() {
    // Hand-crafted mzML with mismatched mz / intensity array lengths.
    // The mz array has one f64 (8 bytes -> 1 value); the intensity array
    // has two f32s (8 bytes -> 2 values). The conformance harness must
    // flag this as PeakArrayLengthMismatch.
    let path = tmp_path("bad.mzML");
    let mzml = bad_mzml_with_array_length_mismatch();
    std::fs::write(&path, mzml).expect("write bad mzML");

    let out = Command::new(bin_path())
        .arg("validate")
        .arg("--json")
        .arg(&path)
        .output()
        .expect("run vendor2mzml");
    let _ = std::fs::remove_file(&path);

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    assert_eq!(
        out.status.code(),
        Some(3),
        "expected exit code 3, got {:?}, stdout={}, stderr={}",
        out.status.code(),
        stdout,
        stderr
    );
    assert!(stdout.contains("\"ok\":false"), "stdout was: {stdout}");
    assert!(
        stdout.contains("PeakArrayLengthMismatch"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_accepts_empty_spectra_and_target_only_precursors() {
    // The core writer omits binaryDataArrayList for spectra with no peaks
    // (allowed by the mzML schema) and writes only the isolation target
    // when a precursor has no selected-ion m/z (Agilent Q-TOF MS2). Both
    // must read back as valid spectra.
    let mut empty = make_record(0, 1, 1, 0.5);
    empty.mz.clear();
    empty.intensity.clear();
    empty.total_ion_current = None;
    empty.base_peak_mz = None;
    empty.base_peak_intensity = None;
    empty.low_mz = None;
    empty.high_mz = None;
    let mut ms2 = make_record(1, 2, 2, 0.8);
    let pre = ms2.precursor.as_mut().expect("ms2 precursor");
    pre.selected_mz = None;
    pre.charge = None;
    pre.intensity = None;
    pre.precursor_native_id = None;
    let records = vec![empty, ms2, make_record(2, 3, 1, 1.2)];

    let path = tmp_path("empty-and-target-only.mzML");
    let mut src = VecSource::new(make_metadata(), records);
    let mut w = BufWriter::new(File::create(&path).expect("create mzml"));
    write_mzml(&mut src, &mut w).expect("write mzml");
    w.flush().expect("flush");
    drop(w);
    let written = std::fs::read_to_string(&path).expect("read back");
    assert_eq!(
        written.matches("<binaryDataArrayList").count(),
        2,
        "empty spectrum should carry no binaryDataArrayList"
    );

    let out = Command::new(bin_path())
        .arg("validate")
        .arg("--json")
        .arg(&path)
        .output()
        .expect("run vendor2mzml");
    let _ = std::fs::remove_file(&path);

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "expected success, got status={:?}, stdout={}, stderr={}",
        out.status.code(),
        stdout,
        stderr
    );
    assert!(
        stdout.contains("\"spectrum_count\":3"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_rejects_spectrum_with_only_one_array() {
    // A spectrum carrying an m/z array but no intensity array is not an
    // empty spectrum; it must be reported, not read as zero peaks.
    let path = tmp_path("mz-only.mzML");
    let mzml = bad_mzml_with_array_length_mismatch();
    let start = mzml
        .find("          <binaryDataArray encodedLength")
        .and_then(|first| {
            mzml[first + 1..]
                .find("          <binaryDataArray encodedLength")
                .map(|second| first + 1 + second)
        })
        .expect("second binaryDataArray");
    let end = start
        + mzml[start..]
            .find("</binaryDataArray>\n")
            .expect("end of second array")
        + "</binaryDataArray>\n".len();
    let mzml = format!("{}{}", &mzml[..start], &mzml[end..]).replace(
        r#"<binaryDataArrayList count="2">"#,
        r#"<binaryDataArrayList count="1">"#,
    );
    assert!(!mzml.contains("MS:1000515"), "intensity array not removed");
    std::fs::write(&path, mzml).expect("write mz-only mzML");

    let out = Command::new(bin_path())
        .arg("validate")
        .arg("--json")
        .arg(&path)
        .output()
        .expect("run vendor2mzml");
    let _ = std::fs::remove_file(&path);

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit code 1, got {:?}, stdout={}, stderr={}",
        out.status.code(),
        stdout,
        stderr
    );
    assert!(
        stdout.contains("no intensity array") || stderr.contains("no intensity array"),
        "stdout={stdout}, stderr={stderr}"
    );
}

#[test]
fn validate_unknown_input_returns_two() {
    let path = tmp_path("garbage.bin");
    std::fs::write(&path, b"not an mzml and not a vendor file").expect("write");

    let out = Command::new(bin_path())
        .arg("validate")
        .arg(&path)
        .output()
        .expect("run vendor2mzml");
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        out.status.code(),
        Some(2),
        "stdout: {}, stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

// ----- bad-mzml helper -----------------------------------------------

fn bad_mzml_with_array_length_mismatch() -> String {
    // One mz value (one f64 = 8 bytes), two intensity values (two f32 = 8 bytes).
    // base64("\x00\x00\x00\x00\x00\x00\x59\x40") encodes f64 = 100.0 little-endian.
    // base64("\x00\x00\x20\x41\x00\x00\xa0\x41") encodes f32s [10.0, 20.0].
    let mz_one: [u8; 8] = 100.0f64.to_le_bytes();
    let mut int_two = Vec::with_capacity(8);
    int_two.extend_from_slice(&10.0f32.to_le_bytes());
    int_two.extend_from_slice(&20.0f32.to_le_bytes());
    let mz_b64 = base64_encode(&mz_one);
    let int_b64 = base64_encode(&int_two);

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<indexedmzML xmlns="http://psi.hupo.org/ms/mzml">
<mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">
  <cvList count="1">
    <cv id="MS" fullName="PSI-MS" URI="https://www.psidev.info/ms" version="4.1.30"/>
  </cvList>
  <fileDescription>
    <fileContent>
      <cvParam cvRef="MS" accession="MS:1000579" name="MS1 spectrum" value=""/>
    </fileContent>
  </fileDescription>
  <run id="bad" defaultInstrumentConfigurationRef="ic0">
    <spectrumList count="1" defaultDataProcessingRef="dp0">
      <spectrum index="0" id="scan=1" defaultArrayLength="1">
        <cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>
        <cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>
        <scanList count="1">
          <scan>
            <cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="0.5" unitAccession="UO:0000010" unitName="second"/>
          </scan>
        </scanList>
        <binaryDataArrayList count="2">
          <binaryDataArray encodedLength="{mz_len}">
            <cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>
            <cvParam cvRef="MS" accession="MS:1000576" name="no compression" value=""/>
            <cvParam cvRef="MS" accession="MS:1000514" name="m/z array" value="" unitAccession="MS:1000040" unitName="m/z"/>
            <binary>{mz}</binary>
          </binaryDataArray>
          <binaryDataArray encodedLength="{int_len}">
            <cvParam cvRef="MS" accession="MS:1000521" name="32-bit float" value=""/>
            <cvParam cvRef="MS" accession="MS:1000576" name="no compression" value=""/>
            <cvParam cvRef="MS" accession="MS:1000515" name="intensity array" value="" unitAccession="MS:1000131" unitName="number of counts"/>
            <binary>{int}</binary>
          </binaryDataArray>
        </binaryDataArrayList>
      </spectrum>
    </spectrumList>
  </run>
</mzML>
</indexedmzML>
"#,
        mz_len = mz_b64.len(),
        int_len = int_b64.len(),
        mz = mz_b64,
        int = int_b64,
    )
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 2 < data.len() {
        let b = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(T[((b >> 18) & 0x3f) as usize]);
        out.push(T[((b >> 12) & 0x3f) as usize]);
        out.push(T[((b >> 6) & 0x3f) as usize]);
        out.push(T[(b & 0x3f) as usize]);
        i += 3;
    }
    if data.len() - i == 2 {
        let b = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(T[((b >> 18) & 0x3f) as usize]);
        out.push(T[((b >> 12) & 0x3f) as usize]);
        out.push(T[((b >> 6) & 0x3f) as usize]);
        out.push(b'=');
    } else if data.len() - i == 1 {
        let b = (data[i] as u32) << 16;
        out.push(T[((b >> 18) & 0x3f) as usize]);
        out.push(T[((b >> 12) & 0x3f) as usize]);
        out.push(b'=');
        out.push(b'=');
    }
    String::from_utf8(out).unwrap()
}

fn validate_json(path: &std::path::Path) -> (Option<i32>, String) {
    let out = Command::new(bin_path())
        .arg("validate")
        .arg("--json")
        .arg(path)
        .output()
        .expect("run vendor2mzml");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn validate_truncated_mzml_fails_instead_of_reporting_ok() {
    let good = tmp_path("trunc-src.mzML");
    write_good_mzml(&good);
    let bytes = std::fs::read(&good).expect("read good mzML");
    let _ = std::fs::remove_file(&good);

    // Cut inside the spectrumList, after the first spectrum has closed.
    let text = String::from_utf8(bytes).expect("utf-8 mzML");
    let first_end = text.find("</spectrum>").expect("a spectrum") + "</spectrum>".len();
    let cut = first_end + (text.len() - first_end) / 3;
    let path = tmp_path("truncated.mzML");
    std::fs::write(&path, &text.as_bytes()[..cut]).expect("write truncated mzML");

    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_ne!(code, Some(0), "truncated file validated: {stdout}");
    assert!(stdout.contains("\"ok\":false"), "stdout was: {stdout}");
}

#[test]
fn validate_mzml_with_corrupt_payload_fails_instead_of_reporting_ok() {
    let good = tmp_path("corrupt-src.mzML");
    write_good_mzml(&good);
    let text = std::fs::read_to_string(&good).expect("read good mzML");
    let _ = std::fs::remove_file(&good);

    // Damage the second spectrum's m/z payload.
    let second = text
        .match_indices("<binary>")
        .nth(2)
        .expect("a second payload")
        .0;
    let mut damaged = text.clone();
    damaged.replace_range(second + 8..second + 12, "!!!!");
    let path = tmp_path("corrupt.mzML");
    std::fs::write(&path, damaged).expect("write corrupt mzML");

    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_ne!(code, Some(0), "corrupt file validated: {stdout}");
    assert!(stdout.contains("\"ok\":false"), "stdout was: {stdout}");
}

#[test]
fn validate_mzml_without_spectrum_count_fails_instead_of_reporting_ok() {
    let good = tmp_path("nocount-src.mzML");
    write_good_mzml(&good);
    let text = std::fs::read_to_string(&good).expect("read good mzML");
    let _ = std::fs::remove_file(&good);

    let at = text
        .find("<spectrumList count=\"")
        .expect("spectrumList count");
    let end = at + text[at..].find('>').expect("end of spectrumList tag");
    let mut stripped = text.clone();
    stripped.replace_range(at..end, "<spectrumList");
    let path = tmp_path("nocount.mzML");
    std::fs::write(&path, stripped).expect("write mzML without count");

    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_ne!(code, Some(0), "file without a count validated: {stdout}");
    assert!(stdout.contains("\"ok\":false"), "stdout was: {stdout}");
    assert!(
        stdout.contains("spectrum list with no count"),
        "stdout was: {stdout}"
    );
}

fn gzip_bytes(data: &[u8]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    let mut enc = GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data).expect("gzip write");
    enc.finish().expect("gzip finish")
}

fn good_mzml_text() -> String {
    let good = tmp_path("text-src.mzML");
    write_good_mzml(&good);
    let text = std::fs::read_to_string(&good).expect("read good mzML");
    let _ = std::fs::remove_file(&good);
    text
}

#[test]
fn validate_gzipped_mzml_reports_every_spectrum() {
    let path = tmp_path("good.mzML.gz");
    std::fs::write(&path, gzip_bytes(good_mzml_text().as_bytes())).expect("write gz");
    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, Some(0), "stdout was: {stdout}");
    assert!(stdout.contains("\"ok\":true"), "stdout was: {stdout}");
    assert!(
        stdout.contains("\"spectrum_count\":3"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_truncated_gzip_fails_with_a_stream_error() {
    let gz = gzip_bytes(good_mzml_text().as_bytes());
    let path = tmp_path("truncated.mzML.gz");
    std::fs::write(&path, &gz[..gz.len() * 2 / 3]).expect("write truncated gz");
    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(stdout.contains("\"ok\":false"), "stdout was: {stdout}");
    assert!(stdout.contains("gzip stream error"), "stdout was: {stdout}");
}

/// A valid mzML with a chromatogram list and no `<spectrumList>`.
fn chromatogram_only_mzml() -> String {
    let text = good_mzml_text();
    let start = text.find("<spectrumList").expect("spectrumList");
    let end_tag = "</spectrumList>";
    let end = text.find(end_tag).expect("end of spectrumList") + end_tag.len();
    let mut out = text.clone();
    out.replace_range(
        start..end,
        concat!(
            r#"<chromatogramList count="1" defaultDataProcessingRef="dp1">"#,
            r#"<chromatogram index="0" id="TIC" defaultArrayLength="0">"#,
            r#"<cvParam cvRef="MS" accession="MS:1000235" name="total ion current chromatogram"/>"#,
            r#"</chromatogram></chromatogramList>"#
        ),
    );
    out
}

#[test]
fn validate_chromatogram_only_mzml_succeeds() {
    let path = tmp_path("chrom-only.mzML");
    std::fs::write(&path, chromatogram_only_mzml()).expect("write mzML");
    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, Some(0), "stdout was: {stdout}");
    assert!(stdout.contains("\"ok\":true"), "stdout was: {stdout}");
    assert!(
        stdout.contains("\"spectrum_count\":0"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_truncated_chromatogram_only_mzml_fails() {
    let text = chromatogram_only_mzml();
    let cut = text.find("<chromatogramList").expect("chromatogramList");
    let path = tmp_path("chrom-only-truncated.mzML");
    std::fs::write(&path, &text.as_bytes()[..cut]).expect("write mzML");
    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(stdout.contains("\"ok\":false"), "stdout was: {stdout}");
    assert!(
        stdout.contains("ends before </mzML>"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_empty_spectrum_list_without_count_fails() {
    let text = good_mzml_text();
    let start = text.find("<spectrumList").expect("spectrumList");
    let end_tag = "</spectrumList>";
    let end = text.find(end_tag).expect("end of spectrumList") + end_tag.len();
    let mut stripped = text.clone();
    stripped.replace_range(start..end, "<spectrumList></spectrumList>");
    let path = tmp_path("empty-list.mzML");
    std::fs::write(&path, stripped).expect("write mzML");
    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(
        stdout.contains("spectrum list with no count"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_more_spectra_than_declared_says_so() {
    let text = good_mzml_text();
    let at = text.find("<spectrumList count=\"3\"").expect("count 3");
    let mut changed = text.clone();
    changed.replace_range(
        at..at + "<spectrumList count=\"3\"".len(),
        "<spectrumList count=\"2\"",
    );
    let path = tmp_path("overfull.mzML");
    std::fs::write(&path, changed).expect("write mzML");
    let (code, stdout) = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(
        stdout.contains("read 3 spectra but the file declares only 2"),
        "stdout was: {stdout}"
    );
    assert!(!stdout.contains("truncated"), "stdout was: {stdout}");
}

#[test]
fn validate_missing_input_reports_the_io_error() {
    let path = tmp_path("absent.mzML");
    let (code, stdout) = validate_json(&path);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(stdout.contains("\"ok\":false"), "stdout was: {stdout}");
    assert!(stdout.contains("cannot read"), "stdout was: {stdout}");
    let out = Command::new(bin_path())
        .arg("validate")
        .arg(&path)
        .output()
        .expect("run vendor2mzml");
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot read"));
}

fn validate_gz_bytes(name: &str, bytes: &[u8]) -> (Option<i32>, String) {
    let path = tmp_path(name);
    std::fs::write(&path, bytes).expect("write gz");
    let result = validate_json(&path);
    let _ = std::fs::remove_file(&path);
    result
}

#[test]
fn validate_gzip_without_its_trailer_fails() {
    let gz = gzip_bytes(good_mzml_text().as_bytes());
    let (code, stdout) = validate_gz_bytes("no-trailer.mzML.gz", &gz[..gz.len() - 8]);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(stdout.contains("gzip stream error"), "stdout was: {stdout}");
}

#[test]
fn validate_gzip_with_a_wrong_crc_fails() {
    let mut gz = gzip_bytes(good_mzml_text().as_bytes());
    let crc_byte = gz.len() - 8;
    gz[crc_byte] ^= 0xff;
    let (code, stdout) = validate_gz_bytes("bad-crc.mzML.gz", &gz);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(stdout.contains("gzip stream error"), "stdout was: {stdout}");
}

#[test]
fn validate_gzip_with_a_wrong_size_fails() {
    let mut gz = gzip_bytes(good_mzml_text().as_bytes());
    let size_byte = gz.len() - 4;
    gz[size_byte] ^= 0xff;
    let (code, stdout) = validate_gz_bytes("bad-size.mzML.gz", &gz);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(stdout.contains("gzip stream error"), "stdout was: {stdout}");
}

#[test]
fn validate_concatenated_gzip_members_succeed() {
    let text = good_mzml_text();
    let cut = text.find("</spectrum>").expect("a spectrum") + "</spectrum>".len();
    let mut gz = gzip_bytes(&text.as_bytes()[..cut]);
    gz.extend(gzip_bytes(&text.as_bytes()[cut..]));
    let (code, stdout) = validate_gz_bytes("members.mzML.gz", &gz);
    assert_eq!(code, Some(0), "stdout was: {stdout}");
    assert!(
        stdout.contains("\"spectrum_count\":3"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_gzip_with_trailing_zero_padding_succeeds() {
    let mut gz = gzip_bytes(good_mzml_text().as_bytes());
    gz.extend([0u8; 512]);
    let (code, stdout) = validate_gz_bytes("padded.mzML.gz", &gz);
    assert_eq!(code, Some(0), "stdout was: {stdout}");
    assert!(
        stdout.contains("\"spectrum_count\":3"),
        "stdout was: {stdout}"
    );
}

#[test]
fn validate_gzip_followed_by_garbage_fails() {
    let mut gz = gzip_bytes(good_mzml_text().as_bytes());
    gz.extend(b"not a gzip member");
    let (code, stdout) = validate_gz_bytes("garbage-after.mzML.gz", &gz);
    assert_eq!(code, Some(1), "stdout was: {stdout}");
    assert!(stdout.contains("gzip stream error"), "stdout was: {stdout}");
}
