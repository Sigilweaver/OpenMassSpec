//! Minimal mzML -> openmassspec-core `SpectrumRecord` adapter, used by
//! `vendor2mzml validate` so the conformance harness can be run against
//! arbitrary mzML files (including indexed mzML and gzipped mzML).
//!
//! Only fields the conformance harness inspects are populated:
//! `index`, `native_id`, `ms_level`, `polarity`, `retention_time_sec`,
//! `mz`, `intensity`, and `precursor` (isolation target m/z, selected ion
//! m/z, charge and intensity, and the precursor `spectrumRef`). Other
//! fields are left at their defaults.

use std::cell::RefCell;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::rc::Rc;

use flate2::bufread::GzDecoder;
use mzdata::prelude::*;
use mzdata::spectrum::bindata::ArrayRetrievalError;
use mzdata::spectrum::{IsolationWindowState, RawSpectrum, ScanPolarity};
use mzdata::MzMLReader;

use openmassspec_core::{Polarity, PrecursorInfo, SpectrumRecord, Warning, WarningKind};

/// Returns true if `path` has an extension suggesting an mzML file.
/// Accepts `.mzml`, `.mzML`, and `.mzml.gz` (any case).
pub fn looks_like_mzml(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".mzml") || lower.ends_with(".mzml.gz")
}

/// Gzip files start with these two bytes.
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// `true` when the file at `path` starts with the gzip magic bytes (the
/// file name is not consulted).
fn is_gzip(path: &Path) -> std::io::Result<bool> {
    let mut magic = [0u8; 2];
    let mut file = File::open(path)?;
    let mut filled = 0;
    while filled < magic.len() {
        match file.read(&mut magic[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled == magic.len() && magic == GZIP_MAGIC)
}

/// Open `path` as a byte stream, decompressing gzip.
fn open_stream(path: &Path, gzip: bool) -> std::io::Result<Box<dyn Read>> {
    let file = BufReader::new(File::open(path)?);
    Ok(if gzip {
        Box::new(GzipMembers::new(file))
    } else {
        Box::new(file)
    })
}

/// Reads a gzip file made of one or more concatenated members. Every
/// member's CRC and size trailer is checked as the member ends. Zero bytes
/// after the last member are padding and are ignored (as `gzip` does); any
/// other trailing bytes are an error.
struct GzipMembers<R: BufRead> {
    state: MemberState<R>,
}

enum MemberState<R: BufRead> {
    Member(GzDecoder<R>),
    Done,
}

impl<R: BufRead> GzipMembers<R> {
    fn new(input: R) -> Self {
        Self {
            state: MemberState::Member(GzDecoder::new(input)),
        }
    }
}

impl<R: BufRead> Read for GzipMembers<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let MemberState::Member(decoder) = &mut self.state else {
                return Ok(0);
            };
            let n = decoder.read(buf)?;
            if n > 0 || buf.is_empty() {
                return Ok(n);
            }
            // The member ended with its trailer verified. Decide what follows.
            let MemberState::Member(decoder) =
                std::mem::replace(&mut self.state, MemberState::Done)
            else {
                return Ok(0);
            };
            let mut input = decoder.into_inner();
            let first = input.fill_buf()?.first().copied();
            match first {
                None => return Ok(0),
                Some(0) => loop {
                    let chunk = input.fill_buf()?;
                    if chunk.is_empty() {
                        return Ok(0);
                    }
                    if chunk.iter().any(|&b| b != 0) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "bytes after the gzip stream are not zero padding",
                        ));
                    }
                    let len = chunk.len();
                    input.consume(len);
                },
                Some(b) if b == GZIP_MAGIC[0] => {
                    self.state = MemberState::Member(GzDecoder::new(input));
                }
                Some(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "bytes after the gzip stream are neither another gzip member nor zero \
                         padding",
                    ));
                }
            }
        }
    }
}

/// Reader that remembers the first I/O error it returned. The mzML reader
/// ends its iterator at an I/O or parse error without reporting it, so a
/// truncated or corrupt gzip stream would otherwise look like a short file.
struct ErrorTap<R> {
    inner: R,
    first_error: Rc<RefCell<Option<std::io::Error>>>,
}

impl<R: Read> Read for ErrorTap<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf).inspect_err(|e| {
            let mut slot = self.first_error.borrow_mut();
            if slot.is_none() && e.kind() != std::io::ErrorKind::Interrupted {
                *slot = Some(std::io::Error::new(e.kind(), e.to_string()));
            }
        })
    }
}

/// Hands the same underlying stream to the mzML reader and, once that reader
/// is done, back to the caller to read to the end.
struct SharedStream(Rc<RefCell<Box<dyn Read>>>);

impl Read for SharedStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().read(buf)
    }
}

/// Words a gzip stream failure by what was found.
fn gzip_stream_error(e: &std::io::Error) -> openmassspec_io::Error {
    let found = if e.kind() == std::io::ErrorKind::UnexpectedEof {
        "the file ends inside a gzip member".to_string()
    } else {
        e.to_string()
    };
    openmassspec_io::Error::Mzml(format!("gzip stream error: {found}"))
}

/// Decode an mzML (or mzML.gz) file into a vector of `SpectrumRecord`s.
/// Gzip is recognised by its magic bytes.
pub fn read_mzml_records(path: &Path) -> openmassspec_io::Result<Vec<SpectrumRecord>> {
    let mzml_err = |e: &dyn std::fmt::Display| openmassspec_io::Error::Mzml(e.to_string());
    if is_gzip(path)? {
        let first_error = Rc::new(RefCell::new(None));
        let stream = Rc::new(RefCell::new(open_stream(path, true)?));
        let tap = ErrorTap {
            inner: SharedStream(Rc::clone(&stream)),
            first_error: Rc::clone(&first_error),
        };
        let records = read_records(MzMLReader::new(tap), path, true, Some(&first_error));
        if let Some(e) = first_error.borrow_mut().take() {
            return Err(gzip_stream_error(&e));
        }
        let records = records?;
        // The mzML reader stops at `</spectrumList>`, so the gzip trailer
        // (CRC and size) has not been read. Read the rest of the same stream,
        // which checks the trailer of every member.
        std::io::copy(&mut SharedStream(stream), &mut std::io::sink())
            .map_err(|e| gzip_stream_error(&e))?;
        return Ok(records);
    }
    let reader = MzMLReader::open_path(path).map_err(|e| mzml_err(&e))?;
    read_records(reader, path, false, None)
}

/// Read every spectrum from `reader` and check the count against the file.
fn read_records<R: Read>(
    reader: MzMLReader<R>,
    path: &Path,
    gzip: bool,
    stream_error: Option<&Rc<RefCell<Option<std::io::Error>>>>,
) -> openmassspec_io::Result<Vec<SpectrumRecord>> {
    // The mzML reader ends its iterator at the first parse error (a
    // truncated file, a corrupt payload) without saying so, so the number
    // of spectra read is checked against the count the file declares.
    let declared = reader.spectrum_count_hint();
    let mut out = Vec::new();
    for (i, spectrum) in reader.enumerate() {
        let raw: RawSpectrum = spectrum.into();
        out.push(spectrum_to_record(i, raw)?);
    }
    if stream_error.is_some_and(|slot| slot.borrow().is_some()) {
        // The caller reports the stream error, which explains the short read.
        return Ok(out);
    }
    check_declared_count(declared, out.len(), path, gzip)?;
    Ok(out)
}

/// The file's `<spectrumList count>` must equal the number of spectra read.
/// A file with no `<spectrumList>` at all (chromatograms only) has no count
/// to check, provided it ends properly.
fn check_declared_count(
    declared: Option<u64>,
    read: usize,
    path: &Path,
    gzip: bool,
) -> openmassspec_io::Result<()> {
    let mzml_err = |msg: String| Err(openmassspec_io::Error::Mzml(msg));
    match declared {
        Some(0) if read == 0 => {
            // An empty list: nothing was read, so only the ending shows that
            // the file was not cut off inside or after the list.
            let (_, has_end) = scan_file_markers(path, gzip)?;
            if has_end {
                Ok(())
            } else {
                mzml_err(
                    "the file declares 0 spectra and ends before </mzML>; it is truncated"
                        .to_string(),
                )
            }
        }
        Some(n) if n == read as u64 => Ok(()),
        Some(n) if (read as u64) > n => mzml_err(format!(
            "read {read} spectra but the file declares only {n}"
        )),
        Some(n) => mzml_err(format!(
            "read {read} of the {n} spectra the file declares; the file is truncated or a \
             spectrum could not be parsed"
        )),
        None if read > 0 => mzml_err(format!(
            "the file has a spectrum list with no count, so the {read} spectra read cannot be \
             checked for truncation"
        )),
        None => {
            let (has_list, has_end) = scan_file_markers(path, gzip)?;
            if has_list {
                mzml_err(
                    "the file has a spectrum list with no count, so 0 spectra read cannot be \
                     checked for truncation"
                        .to_string(),
                )
            } else if has_end {
                Ok(())
            } else {
                mzml_err(
                    "the file has no spectrum list and ends before </mzML>; it is truncated"
                        .to_string(),
                )
            }
        }
    }
}

/// Open `path` again and scan it with `scan_markers`. A gzip stream failure is
/// worded like the one from the main read.
fn scan_file_markers(path: &Path, gzip: bool) -> openmassspec_io::Result<(bool, bool)> {
    let stream = open_stream(path, gzip)?;
    scan_markers(stream).map_err(|e| {
        if gzip {
            gzip_stream_error(&e)
        } else {
            e.into()
        }
    })
}

/// Stream through `reader` and report whether it contains a `<spectrumList`
/// start tag and a closing `</mzML>` tag.
fn scan_markers(mut reader: impl Read) -> std::io::Result<(bool, bool)> {
    const LIST: &[u8] = b"<spectrumList";
    const END: &[u8] = b"</mzML>";
    let (mut has_list, mut has_end) = (false, false);
    let mut window: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            return Ok((has_list, has_end));
        }
        window.extend_from_slice(&chunk[..n]);
        has_list |= window.windows(LIST.len()).any(|w| w == LIST);
        has_end |= window.windows(END.len()).any(|w| w == END);
        // Keep a tail so a marker split across two reads is still found.
        let keep = LIST.len().max(END.len()) - 1;
        if window.len() > keep {
            window.drain(..window.len() - keep);
        }
    }
}

/// Decode the m/z and intensity arrays of one spectrum.
///
/// mzML makes `binaryDataArrayList` optional (`minOccurs="0"` on
/// `SpectrumType`), and the openmassspec-core writer omits it for spectra
/// with no peaks. A spectrum carrying neither array is therefore read as
/// an empty spectrum. A spectrum carrying only one of the two arrays, or
/// an array that fails to decode, is an error: the harness must not see a
/// silently truncated spectrum.
fn peak_arrays(s: &RawSpectrum) -> openmassspec_io::Result<(Vec<f64>, Vec<f32>)> {
    let mz = s.arrays.mzs();
    let intensity = s.arrays.intensities();
    let fail = |what: String| {
        openmassspec_io::Error::Mzml(format!(
            "spectrum {:?} (index {}): {what}",
            s.id(),
            s.index()
        ))
    };
    match (mz, intensity) {
        (Ok(mz), Ok(intensity)) => Ok((mz.into_owned(), intensity.into_owned())),
        (Err(ArrayRetrievalError::NotFound(_)), Err(ArrayRetrievalError::NotFound(_))) => {
            Ok((Vec::new(), Vec::new()))
        }
        (Ok(_), Err(ArrayRetrievalError::NotFound(_))) => {
            Err(fail("has an m/z array but no intensity array".into()))
        }
        (Err(ArrayRetrievalError::NotFound(_)), Ok(_)) => {
            Err(fail("has an intensity array but no m/z array".into()))
        }
        (Err(e), _) | (_, Err(e)) => Err(fail(format!("cannot decode binary array: {e}"))),
    }
}

fn spectrum_to_record(
    stream_index: usize,
    s: RawSpectrum,
) -> openmassspec_io::Result<SpectrumRecord> {
    let native_id = s.id().to_string();
    let ms_level = u32::from(s.ms_level());
    let rt_min = s.start_time();
    let rt_sec = rt_min * 60.0;
    let polarity = match s.polarity() {
        ScanPolarity::Positive => Some(Polarity::Positive),
        ScanPolarity::Negative => Some(Polarity::Negative),
        ScanPolarity::Unknown => None,
    };
    let (mz, intensity) = peak_arrays(&s)?;
    let precursor = s.precursor().map(|p| {
        let ion = p.ions.first();
        // The writer emits `isolation window target m/z` without a
        // selectedIonList when only the target is known (e.g. Agilent
        // Q-TOF MS2), so the target is read here too; dropping it would
        // make the harness report a missing precursor.
        let window = &p.isolation_window;
        let target_mz = (!matches!(
            window.flags,
            IsolationWindowState::Unknown | IsolationWindowState::NoIsolation
        ))
        .then_some(f64::from(window.target));
        PrecursorInfo::default()
            .with_target_mz(target_mz)
            .with_selected_mz(ion.map(|x| x.mz))
            .with_charge(ion.and_then(|x| x.charge))
            .with_intensity(ion.map(|x| f64::from(x.intensity)))
            .with_precursor_native_id(p.precursor_id.clone())
    });

    let empty = mz.is_empty();
    let mut record = SpectrumRecord::new(s.index(), stream_index as u32 + 1, native_id)
        .with_ms_level(ms_level)
        .with_polarity(polarity)
        .with_retention_time_sec(rt_sec)
        .with_precursor(precursor)
        .with_peaks(mz, intensity);
    if empty {
        // Core's harness requires the flag on every spectrum with no peaks,
        // and mzML from other tools does not carry it.
        record = record.with_warning(Warning::new(
            WarningKind::EmptySpectrum,
            "the mzML spectrum has no peaks",
        ));
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, ErrorKind, Write};

    use flate2::write::DeflateEncoder;
    use flate2::{Compression, Crc};

    use super::*;

    /// Input buffer sizes the reader is driven with. 1 splits every header
    /// field and every padding block across reads.
    const CAPACITIES: [usize; 6] = [1, 2, 3, 7, 64, 8192];

    /// Optional gzip header fields (RFC 1952, section 2.3.1).
    #[derive(Default, Clone)]
    struct Header {
        extra: Option<&'static [u8]>,
        name: Option<&'static str>,
        comment: Option<&'static str>,
        header_crc: bool,
    }

    /// One gzip member holding `data`, with the header fields in `h`.
    fn member(data: &[u8], h: &Header) -> Vec<u8> {
        let mut flags = 0u8;
        let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 255];
        if let Some(extra) = h.extra {
            flags |= 0x04;
            out.extend((extra.len() as u16).to_le_bytes());
            out.extend(extra);
        }
        if let Some(name) = h.name {
            flags |= 0x08;
            out.extend(name.as_bytes());
            out.push(0);
        }
        if let Some(comment) = h.comment {
            flags |= 0x10;
            out.extend(comment.as_bytes());
            out.push(0);
        }
        if h.header_crc {
            flags |= 0x02;
        }
        out[3] = flags;
        if h.header_crc {
            let mut crc = Crc::new();
            crc.update(&out);
            out.extend((crc.sum() as u16).to_le_bytes());
        }
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(data).expect("deflate write");
        out.extend(enc.finish().expect("deflate finish"));
        let mut crc = Crc::new();
        crc.update(data);
        out.extend(crc.sum().to_le_bytes());
        out.extend((data.len() as u32).to_le_bytes());
        out
    }

    fn plain(data: &[u8]) -> Vec<u8> {
        member(data, &Header::default())
    }

    fn read_all(bytes: &[u8], capacity: usize) -> std::io::Result<Vec<u8>> {
        let input = BufReader::with_capacity(capacity, Cursor::new(bytes.to_vec()));
        let mut out = Vec::new();
        GzipMembers::new(input).read_to_end(&mut out)?;
        Ok(out)
    }

    fn assert_reads(bytes: &[u8], expected: &[u8]) {
        for capacity in CAPACITIES {
            let got = read_all(bytes, capacity)
                .unwrap_or_else(|e| panic!("capacity {capacity}: unexpected error: {e}"));
            assert_eq!(got, expected, "capacity {capacity}");
        }
    }

    fn assert_fails(bytes: &[u8], kind: ErrorKind) -> Vec<String> {
        CAPACITIES
            .iter()
            .map(|&capacity| {
                let e = read_all(bytes, capacity)
                    .expect_err(&format!("capacity {capacity}: expected an error"));
                assert_eq!(e.kind(), kind, "capacity {capacity}: {e}");
                e.to_string()
            })
            .collect()
    }

    #[test]
    fn one_member_with_a_header_split_across_reads() {
        assert_reads(&plain(b"hello gzip"), b"hello gzip");
    }

    #[test]
    fn empty_member_in_the_middle_and_at_the_end() {
        let mut gz = plain(b"abc");
        gz.extend(plain(b""));
        assert_reads(&gz, b"abc");
        gz.extend(plain(b"def"));
        assert_reads(&gz, b"abcdef");
        gz.extend(plain(b""));
        assert_reads(&gz, b"abcdef");
        assert_reads(&plain(b""), b"");
        let mut empties = plain(b"");
        empties.extend(plain(b""));
        assert_reads(&empties, b"");
    }

    #[test]
    fn header_fields_are_skipped() {
        let fields = [
            Header {
                extra: Some(b"XYabcdef"),
                ..Header::default()
            },
            Header {
                name: Some("run1.mzML"),
                ..Header::default()
            },
            Header {
                comment: Some("a comment"),
                ..Header::default()
            },
            Header {
                header_crc: true,
                ..Header::default()
            },
            Header {
                extra: Some(b"AB\x00\x02\x01\x02"),
                name: Some("run1.mzML"),
                comment: Some("a comment"),
                header_crc: true,
            },
        ];
        for h in &fields {
            assert_reads(&member(b"payload", h), b"payload");
            // The same member after another one, so the header is parsed
            // from the middle of the stream.
            let mut gz = plain(b"first ");
            gz.extend(member(b"second", h));
            assert_reads(&gz, b"first second");
        }
    }

    #[test]
    fn a_wrong_header_crc_is_an_error() {
        let mut gz = member(
            b"payload",
            &Header {
                header_crc: true,
                ..Header::default()
            },
        );
        gz[10] ^= 0xff;
        for capacity in CAPACITIES {
            assert!(read_all(&gz, capacity).is_err(), "capacity {capacity}");
        }
    }

    #[test]
    fn zero_padding_of_any_length_is_ignored() {
        for padding in [1usize, 2, 3, 5, 511, 512, 513, 10_000] {
            let mut gz = plain(b"abc");
            gz.extend(vec![0u8; padding]);
            assert_reads(&gz, b"abc");
        }
    }

    #[test]
    fn zero_padding_after_an_empty_member_is_ignored() {
        let mut gz = plain(b"abc");
        gz.extend(plain(b""));
        gz.extend([0u8; 9]);
        assert_reads(&gz, b"abc");
    }

    #[test]
    fn a_partial_padding_block_is_ignored() {
        // 4 zero bytes is less than the 512-byte block `tar` pads to, and
        // more than one read at capacity 1, 2 and 3.
        let mut gz = plain(b"abc");
        gz.extend([0u8; 4]);
        assert_reads(&gz, b"abc");
    }

    #[test]
    fn non_zero_bytes_after_padding_are_an_error() {
        for padding in [1usize, 4, 511, 600] {
            let mut gz = plain(b"abc");
            gz.extend(vec![0u8; padding]);
            gz.push(1);
            for msg in assert_fails(&gz, ErrorKind::InvalidData) {
                assert!(msg.contains("not zero padding"), "padding {padding}: {msg}");
            }
        }
    }

    #[test]
    fn a_member_after_padding_is_an_error() {
        let mut gz = plain(b"abc");
        gz.extend([0u8; 4]);
        gz.extend(plain(b"def"));
        assert_fails(&gz, ErrorKind::InvalidData);
    }

    #[test]
    fn trailing_bytes_that_are_not_a_gzip_member_are_an_error() {
        // Shorter and longer than the 10-byte fixed gzip header.
        for garbage in [&b"x"[..], b"not gzip", b"this is not a gzip member at all"] {
            let mut gz = plain(b"abc");
            gz.extend(garbage);
            for msg in assert_fails(&gz, ErrorKind::InvalidData) {
                assert!(msg.contains("neither another gzip member"), "{msg}");
            }
        }
    }

    #[test]
    fn a_truncated_second_member_is_an_unexpected_eof() {
        let mut gz = plain(b"abc");
        let second = plain(b"def");
        gz.extend(&second[..5]);
        assert_fails(&gz, ErrorKind::UnexpectedEof);
    }

    #[test]
    fn a_missing_trailer_is_an_unexpected_eof() {
        let gz = plain(b"abc");
        assert_fails(&gz[..gz.len() - 8], ErrorKind::UnexpectedEof);
        assert_fails(&gz[..gz.len() - 1], ErrorKind::UnexpectedEof);
    }

    #[test]
    fn a_wrong_crc_is_an_error_that_says_so() {
        let mut gz = plain(b"abc");
        let crc_byte = gz.len() - 8;
        gz[crc_byte] ^= 0xff;
        for msg in assert_fails(&gz, ErrorKind::InvalidInput) {
            assert!(msg.contains("checksum"), "{msg}");
        }
    }

    #[test]
    fn stream_errors_say_what_was_found() {
        let eof = gzip_stream_error(&std::io::Error::from(ErrorKind::UnexpectedEof)).to_string();
        assert!(eof.contains("gzip stream error"), "{eof}");
        assert!(eof.contains("ends inside a gzip member"), "{eof}");

        let crc = gzip_stream_error(&std::io::Error::new(
            ErrorKind::InvalidInput,
            "corrupt gzip stream does not have a matching checksum",
        ))
        .to_string();
        assert!(crc.contains("matching checksum"), "{crc}");
        assert!(!crc.contains("truncated"), "{crc}");

        let padding = gzip_stream_error(&std::io::Error::new(
            ErrorKind::InvalidData,
            "bytes after the gzip stream are not zero padding",
        ))
        .to_string();
        assert!(padding.contains("not zero padding"), "{padding}");
        assert!(!padding.contains("truncated"), "{padding}");
    }
}
