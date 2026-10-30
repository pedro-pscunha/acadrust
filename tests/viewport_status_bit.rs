//! Group 68 of a viewport is its stacking value, 0 for one that AutoCAD calls inactive. AutoCAD's DXFOUT writes 68 = 0 together with
//! bit 0x8000 ("on") in group 90 and keeps the bit when it reads the file: the reader must not clear a bit group 90 set.

use std::io::Cursor;

use acadrust::entities::{EntityType, Viewport};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DxfReader, DxfWriter};

fn restored(status: i16, is_on: bool) -> Viewport {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut viewport = Viewport::new();
    viewport.id = 2;
    viewport.dxf_status = Some(status);
    viewport.status.is_on = is_on;
    document.add_paper_space_entity(EntityType::Viewport(Box::new(viewport))).unwrap();
    let bytes = DxfWriter::new(&document).write_to_vec().unwrap();
    let read = DxfReader::from_reader(Cursor::new(bytes)).unwrap().read().unwrap();
    let found = read.entities().find_map(|e| match e {
        EntityType::Viewport(v) => Some(v.as_ref().clone()),
        _ => None,
    });
    found.expect("viewport missing")
}

#[test]
fn group_68_zero_keeps_the_on_bit_of_group_90() {
    let v = restored(0, true);
    assert!(v.status.is_on, "bit 0x8000 of group 90 was cleared by group 68 = 0");
    assert!(v.status.to_bits() & 0x8000 != 0);
}

#[test]
fn a_viewport_without_the_bit_stays_without_it_unless_group_68_is_set() {
    assert!(!restored(0, false).status.is_on);
    assert!(restored(3, false).status.is_on);
}
