//! The current layer ($CLAYER) and linetype ($CELTYPE) of a drawing survive the DWG, DXF, DWG
//! bridge. A DWG header holds each as a handle and a DXF as a name; the DWG writer kept the handle
//! only when it named an object of the document, and a handle read from a DXF never does, so every
//! drawing came back with layer 0 and ByLayer current.

use std::io::Cursor;

use acadrust::tables::{Layer, LineType};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, TableEntry};

fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let mut layer = Layer::new("Walls");
    layer.set_handle(doc.allocate_handle());
    doc.header.current_layer_name = "Walls".to_string();
    doc.header.current_layer_handle = layer.handle;
    doc.layers.add(layer).expect("add layer");
    let mut dashed = LineType::new("Dashed");
    dashed.set_handle(doc.allocate_handle());
    doc.header.current_linetype_name = "Dashed".to_string();
    doc.header.current_linetype_handle = dashed.handle;
    doc.line_types.add(dashed).expect("add linetype");
    doc
}

fn current_linetype(doc: &CadDocument) -> String {
    let h = doc.header.current_linetype_handle;
    doc.line_types
        .iter()
        .find(|l| l.handle == h)
        .map_or_else(|| format!("no linetype for {h:?}"), |l| l.name.clone())
}

fn current_layer(doc: &CadDocument) -> String {
    let h = doc.header.current_layer_handle;
    doc.layers
        .iter()
        .find(|l| l.handle == h)
        .map_or_else(|| format!("no layer for {h:?}"), |l| l.name.clone())
}

#[test]
fn a_dwg_reads_back_the_current_layer_name() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let bytes = DwgWriter::write_to_vec(&document(version)).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read dwg");
        assert_eq!(back.header.current_layer_name, "Walls", "{version:?}: name");
        assert_eq!(current_layer(&back), "Walls", "{version:?}: handle");
    }
}

#[test]
fn the_current_layer_survives_dwg_dxf_dwg() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let dwg = DwgWriter::write_to_vec(&document(version)).expect("write dwg");
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
        assert_eq!(current_layer(&back), "Walls", "{version:?}: layer");
        assert_eq!(current_linetype(&back), "Dashed", "{version:?}: linetype");
    }
}
