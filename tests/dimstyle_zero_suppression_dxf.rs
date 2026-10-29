//! The R2010 dimension zero-suppression variables (DIMMZF, DIMMZS, DIMALTMZF, DIMALTMZS) in a
//! DIMSTYLE record and in the HEADER section survive the DXF bridge. A DWG keeps them per style;
//! AutoCAD's DXF Reference has no group code for them, so acadrust writes its own:
//! 51 DIMMZF, 52 DIMALTMZF, 300 DIMMZS, 301 DIMALTMZS (free in a DIMSTYLE record). They are
//! private, so only the DWG bridge writer (`DxfWriter::for_dwg_bridge`) writes them.

use std::io::Cursor;

use acadrust::tables::DimStyle;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn styled(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let mut style = DimStyle::new("Zeros");
    style.dimmzf = 100.0;
    style.dimaltmzf = 75.5;
    style.dimmzs = "none".to_string();
    style.dimaltmzs = "alt".to_string();
    doc.dim_styles.add(style).expect("add style");
    let h = &mut doc.header;
    h.dim_mzf = 12.5;
    h.dim_mzs = "mz".to_string();
    h.dim_alt_mzf = 7.5;
    h.dim_alt_mzs = "amz".to_string();
    doc
}

fn check_style(doc: &CadDocument, what: &str) {
    let s = doc
        .dim_styles
        .get("Zeros")
        .unwrap_or_else(|| panic!("{what}: style lost"));
    assert_eq!(s.dimmzf, 100.0, "{what}: DIMMZF");
    assert_eq!(s.dimaltmzf, 75.5, "{what}: DIMALTMZF");
    assert_eq!(s.dimmzs, "none", "{what}: DIMMZS");
    assert_eq!(s.dimaltmzs, "alt", "{what}: DIMALTMZS");
}

fn check_header(doc: &CadDocument, what: &str) {
    let h = &doc.header;
    assert_eq!(h.dim_mzf, 12.5, "{what}: $DIMMZF");
    assert_eq!(h.dim_mzs, "mz", "{what}: $DIMMZS");
    assert_eq!(h.dim_alt_mzf, 7.5, "{what}: $DIMALTMZF");
    assert_eq!(h.dim_alt_mzs, "amz", "{what}: $DIMALTMZS");
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn dimstyle_records_keep_them_through_dxf() {
    for version in [DxfVersion::AC1024, DxfVersion::AC1027, DxfVersion::AC1032] {
        check_style(&through_dxf(&styled(version)), &format!("{version:?}"));
    }
}

#[test]
fn the_header_keeps_them_through_dxf() {
    for version in [DxfVersion::AC1024, DxfVersion::AC1032] {
        check_header(&through_dxf(&styled(version)), &format!("{version:?}"));
    }
}

#[test]
fn dwg_to_dxf_to_dwg_keeps_them() {
    let doc = through_dxf(&styled(DxfVersion::AC1032));
    let bytes = DwgWriter::write_to_vec(&doc).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    check_style(&back, "DWG");
    check_header(&back, "DWG");
}

#[test]
fn a_style_without_the_groups_reads_as_autocads_default() {
    // AutoCAD's own DXF never carries them; a DWG made from such a style holds 100, as the
    // drawings AutoCAD saves do.
    let bytes = DxfWriter::new(&styled(DxfVersion::AC1032))
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    let text = String::from_utf8(bytes).expect("utf8");
    let stripped: Vec<&str> = {
        let lines: Vec<&str> = text.lines().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i + 1 < lines.len() {
            if matches!(lines[i].trim(), "51" | "52" | "300" | "301") {
                i += 2;
                continue;
            }
            out.push(lines[i]);
            out.push(lines[i + 1]);
            i += 2;
        }
        out
    };
    let doc = DxfReader::from_reader(Cursor::new(stripped.join("\r\n").into_bytes()))
        .expect("reader")
        .read()
        .expect("read");
    let s = doc.dim_styles.get("Zeros").expect("style");
    assert_eq!((s.dimmzf, s.dimaltmzf), (100.0, 100.0));
    assert_eq!((s.dimmzs.as_str(), s.dimaltmzs.as_str()), ("", ""));
}

/// The group codes of every DIMSTYLE record in `bytes`.
fn dimstyle_codes(bytes: Vec<u8>) -> Vec<Vec<i32>> {
    let text = String::from_utf8(bytes).expect("utf8");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut records: Vec<Vec<i32>> = Vec::new();
    let mut in_style = false;
    for pair in lines.chunks(2) {
        let [code, value] = pair else { break };
        let code: i32 = code.parse().expect("group code");
        if code == 0 {
            in_style = *value == "DIMSTYLE";
            if in_style {
                records.push(Vec::new());
            }
        } else if in_style {
            records.last_mut().expect("record").push(code);
        }
    }
    records
}

#[test]
fn the_default_dxf_gets_no_private_dimstyle_groups() {
    for version in [
        DxfVersion::AC1018,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1032,
    ] {
        let bytes = DxfWriter::new(&styled(version))
            .write_to_vec()
            .expect("write");
        let records = dimstyle_codes(bytes);
        assert!(records.len() >= 2, "{version:?}: no DIMSTYLE records");
        for record in records {
            assert!(
                record.iter().all(|c| !matches!(*c, 51 | 52 | 300 | 301)),
                "{version:?}: {record:?}"
            );
        }
    }
}

#[test]
fn the_bridge_dxf_writes_the_zero_suppression_groups() {
    let bytes = DxfWriter::new(&styled(DxfVersion::AC1032))
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write");
    let zeros = dimstyle_codes(bytes)
        .into_iter()
        .find(|r| r.contains(&300))
        .expect("a record with the groups");
    assert!([51, 52, 300, 301].iter().all(|c| zeros.contains(c)));
}
