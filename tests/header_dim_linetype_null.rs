//! $DIMLTYPE, $DIMLTEX1 and $DIMLTEX2 name a linetype in a DXF and hold its handle in a DWG. A
//! drawing from AutoCAD holds a null handle in all three, and the DWG reader left the default name
//! ByBlock, so the DXF said ByBlock and the DWG writer wrote the handle of the ByBlock linetype
//! where the source had a null.

use std::io::Cursor;

use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, Handle};

/// A document with the null dimension linetype handles AutoCAD writes.
fn document() -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.header.dim_linetype_name = String::new();
    doc.header.dim_linetype1_name = String::new();
    doc.header.dim_linetype2_name = String::new();
    doc.header.dim_linetype_handle = Handle::NULL;
    doc.header.dim_linetype1_handle = Handle::NULL;
    doc.header.dim_linetype2_handle = Handle::NULL;
    doc
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let dwg = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(dwg))
        .read()
        .expect("read dwg")
}

fn check_null(doc: &CadDocument, what: &str) {
    let h = &doc.header;
    for (variable, name, handle) in [
        ("DIMLTYPE", &h.dim_linetype_name, h.dim_linetype_handle),
        ("DIMLTEX1", &h.dim_linetype1_name, h.dim_linetype1_handle),
        ("DIMLTEX2", &h.dim_linetype2_name, h.dim_linetype2_handle),
    ] {
        assert_eq!(name, "", "{what}: {variable} name");
        assert!(handle.is_null(), "{what}: {variable} handle {handle:?}");
    }
}

#[test]
fn a_dwg_keeps_the_null_dimension_linetypes() {
    check_null(&through_dwg(&document()), "DWG");
}

#[test]
fn null_dimension_linetypes_survive_dwg_dxf_dwg() {
    let read = through_dwg(&document());
    let dxf = DxfWriter::new(&read).write_to_vec().expect("write dxf");
    let text = String::from_utf8(dxf.clone()).expect("utf-8");
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    for variable in ["$DIMLTYPE", "$DIMLTEX1", "$DIMLTEX2"] {
        let at = lines
            .windows(2)
            .position(|w| w[0].trim() == "9" && w[1] == variable)
            .unwrap_or_else(|| panic!("no {variable}"));
        assert_eq!(lines[at + 2].trim(), "6", "{variable} group");
        assert_eq!(lines[at + 3].trim(), "", "{variable} value");
    }
    let bridged = DxfReader::from_reader(Cursor::new(dxf))
        .expect("reader")
        .read()
        .expect("read dxf");
    check_null(&through_dwg(&bridged), "DWG, DXF, DWG");
}

#[test]
fn a_named_dimension_linetype_still_resolves() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.header.dim_linetype_name = "ByBlock".to_string();
    let back = through_dwg(&doc);
    assert_eq!(back.header.dim_linetype_name, "ByBlock");
    assert!(!back.header.dim_linetype_handle.is_null());
}
