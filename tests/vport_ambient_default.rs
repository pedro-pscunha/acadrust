//! The ambient colour of a VPORT record that the source gives none is ByLayer (256), as AutoCAD writes it, not ACI 250.

use std::io::Cursor;

use acadrust::tables::VPort;
use acadrust::types::{Color, DxfVersion};
use acadrust::{CadDocument, DwgReader, DwgWriter};

#[test]
fn a_new_vport_has_a_bylayer_ambient_colour() {
    assert_eq!(VPort::new("*Active").ambient_color, Color::ByLayer);
}

#[test]
fn it_stays_through_a_dwg_save() {
    let doc = CadDocument::with_version(DxfVersion::AC1032);
    let read = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    let vport = read.vports.iter().next().expect("a VPORT");
    assert_eq!(vport.ambient_color, Color::ByLayer);
}
