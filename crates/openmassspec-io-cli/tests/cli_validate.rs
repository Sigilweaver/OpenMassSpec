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
    RunMetadata {
        source_file_name: "synthetic.raw".into(),
        source_file_format: CvTerm::new("MS:1000563", "Thermo RAW format"),
        native_id_format: CvTerm::new("MS:1000768", "Thermo nativeID format"),
        instrument: CvTerm::new("MS:1000031", "instrument model"),
        instrument_serial_number: None,
        software_name: "openmassspec-io-cli-test".into(),
        software_version: "0.0.0".into(),
        start_timestamp: None,
        mobility_array_kind: None,
        analyzers: Vec::new(),
        acquisition_software_name: None,
        acquisition_software_version: None,
    }
}

fn make_record(index: usize, scan: u32, ms_level: u32, rt_sec: f64) -> SpectrumRecord {
    let mz = vec![100.0, 200.0, 300.0];
    let intensity = vec![10.0f32, 20.0, 30.0];
    let precursor = if ms_level >= 2 {
        Some(PrecursorInfo {
            target_mz: Some(150.0),
            selected_mz: Some(150.0),
            isolation_width: Some(2.0),
            charge: Some(2),
            intensity: Some(1.0e5),
            collision_energy: Some(28.0),
            ce_is_nce: true,
            precursor_native_id: Some(format!(
                "controllerType=0 controllerNumber=1 scan={}",
                scan - 1
            )),
            activation: Some(openmassspec_core::Activation::HCD),
            analyzer: None,
            ccs: None,
        })
    } else {
        None
    };
    SpectrumRecord {
        index,
        scan_number: scan,
        native_id: format!("controllerType=0 controllerNumber=1 scan={scan}"),
        ms_level,
        polarity: Some(Polarity::Positive),
        scan_mode: Some(openmassspec_core::ScanMode::Centroid),
        analyzer: None,
        filter: None,
        retention_time_sec: rt_sec,
        total_ion_current: Some(60.0),
        base_peak_mz: Some(300.0),
        base_peak_intensity: Some(30.0),
        low_mz: Some(100.0),
        high_mz: Some(300.0),
        ion_injection_time_ms: None,
        inv_mobility: None,
        faims_cv: None,
        precursor,
        mz,
        intensity,
        inv_mobility_per_peak: None,
    }
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
