//! The ambient colour of a viewport that the source gives none is ByLayer (256), as AutoCAD's DWG reads it, not ACI 0.

use std::io::Cursor;

use acadrust::entities::{EntityType, Viewport};
use acadrust::types::{Color, DxfVersion};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn ambient(doc: &CadDocument) -> Color {
    let found = doc.entities().find_map(|e| match e {
        EntityType::Viewport(v) => Some(v.ambient_color),
        _ => None,
    });
    found.expect("viewport missing")
}

fn document() -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.add_paper_space_entity(EntityType::Viewport(Box::new(Viewport::new()))).unwrap();
    doc
}

#[test]
fn a_new_viewport_has_a_bylayer_ambient_colour() {
    assert_eq!(Viewport::new().ambient_color, Color::ByLayer);
}

#[test]
fn it_stays_through_dxf_and_dwg() {
    let doc = document();
    let dxf = DxfWriter::new(&doc).write_to_vec().unwrap();
    let from_dxf = DxfReader::from_reader(Cursor::new(dxf)).unwrap().read().unwrap();
    assert_eq!(ambient(&from_dxf), Color::ByLayer);
    let dwg = DwgWriter::write_to_vec(&doc).unwrap();
    let from_dwg = DwgReader::from_stream(Cursor::new(dwg)).read().unwrap();
    assert_eq!(ambient(&from_dwg), Color::ByLayer);
}
