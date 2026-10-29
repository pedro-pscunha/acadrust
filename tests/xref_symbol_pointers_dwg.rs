//! A symbol an external reference brings in (a layer, linetype, text style or dimension style
//! named `<xref>|<name>`) holds a hard pointer to the block record of the xref in a DWG. A DXF has
//! no group for it, so a drawing that went DWG, DXF, DWG came back with the pointer null on every
//! xref layer and linetype, and the record named a dependent symbol of no xref.

use std::io::Cursor;

use acadrust::tables::{BlockRecord, DimStyle, Layer, LineType, TextStyle};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, Handle, TableEntry};

/// A document with the xref block "SITE" and one dependent symbol of each kind, without pointers.
fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let mut block = BlockRecord::new("SITE");
    block.set_handle(doc.allocate_handle());
    block.flags.is_xref = true;
    block.xref_path = "site.dwg".to_string();
    doc.block_records.add(block).expect("add xref block");

    let mut layer = Layer::new("SITE|Walls");
    layer.set_handle(doc.allocate_handle());
    layer.flags.xref_dependent = true;
    doc.layers.add(layer).expect("add layer");
    let mut linetype = LineType::new("SITE|Dashed");
    linetype.set_handle(doc.allocate_handle());
    linetype.xref_dependent = true;
    doc.line_types.add(linetype).expect("add linetype");
    let mut style = TextStyle::new("SITE|Notes");
    style.set_handle(doc.allocate_handle());
    style.xref_dependent = true;
    doc.text_styles.add(style).expect("add text style");
    let mut dim = DimStyle::new("SITE|Dim");
    dim.set_handle(doc.allocate_handle());
    dim.xref_dependent = true;
    doc.dim_styles.add(dim).expect("add dim style");
    doc
}

fn check(doc: &CadDocument, xref: Handle, what: &str) {
    let xref_of = |name: &str| {
        (
            doc.layers.get(name).map(|l| l.xref_block_record_handle),
            doc.line_types.get(name).map(|l| l.xref_block_record_handle),
            doc.text_styles
                .get(name)
                .map(|s| s.xref_block_record_handle),
            doc.dim_styles.get(name).map(|d| d.xref_handle),
        )
    };
    assert_eq!(xref_of("SITE|Walls").0, Some(xref), "{what}: layer");
    assert_eq!(xref_of("SITE|Dashed").1, Some(xref), "{what}: linetype");
    assert_eq!(xref_of("SITE|Notes").2, Some(xref), "{what}: text style");
    assert_eq!(xref_of("SITE|Dim").3, Some(xref), "{what}: dimension style");
}

#[test]
fn the_dwg_writer_points_xref_symbols_at_their_xref_block() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let doc = document(version);
        let dwg = DwgWriter::write_to_vec(&doc).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(dwg))
            .read()
            .expect("read dwg");
        let block = back
            .block_records
            .iter()
            .find(|b| b.name == "SITE")
            .expect("xref block");
        check(&back, block.handle, &format!("{version:?}"));
    }
}

#[test]
fn xref_symbols_keep_their_xref_block_through_dwg_dxf_dwg() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let doc = document(version);
        let dwg = DwgWriter::write_to_vec(&doc).expect("write dwg");
        let read = DwgReader::from_stream(Cursor::new(dwg))
            .read()
            .expect("read dwg");
        let dxf = DxfWriter::new(&read).write_to_vec().expect("write dxf");
        let bridged = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        let again = DwgWriter::write_to_vec(&bridged).expect("write dwg again");
        let back = DwgReader::from_stream(Cursor::new(again))
            .read()
            .expect("read dwg again");
        let block = back
            .block_records
            .iter()
            .find(|b| b.name == "SITE")
            .expect("xref block");
        check(&back, block.handle, &format!("{version:?}"));
    }
}

#[test]
fn a_symbol_named_with_a_bar_of_no_xref_keeps_a_null_pointer() {
    let mut doc = document(DxfVersion::AC1032);
    let mut layer = Layer::new("OTHER|Walls");
    layer.set_handle(doc.allocate_handle());
    doc.layers.add(layer).expect("add layer");
    let dwg = DwgWriter::write_to_vec(&doc).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(dwg))
        .read()
        .expect("read dwg");
    assert!(back
        .layers
        .get("OTHER|Walls")
        .expect("layer")
        .xref_block_record_handle
        .is_null());
}
