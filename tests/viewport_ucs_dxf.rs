//! A VIEWPORT entity names its UCS (group 345), the UCS an orthographic UCS is relative to (346)
//! and the orthographic type of the UCS (79). The DXF writer wrote none of the three and the
//! reader had no arm for them, so a layout viewport of a drawing that went DWG, DXF, DWG came
//! back in the world UCS.

use std::io::Cursor;

use acadrust::entities::Viewport;
use acadrust::tables::Ucs;
use acadrust::types::DxfVersion;
use acadrust::{
    CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType, Handle, TableEntry,
};

fn add_ucs(doc: &mut CadDocument, name: &str) -> Handle {
    let mut ucs = Ucs::new(name);
    ucs.set_handle(doc.allocate_handle());
    let handle = ucs.handle;
    doc.ucss.add(ucs).expect("add ucs");
    handle
}

fn document(version: DxfVersion) -> (CadDocument, Handle, Handle) {
    let mut doc = CadDocument::with_version(version);
    let named = add_ucs(&mut doc, "Sheet");
    let base = add_ucs(&mut doc, "SheetBase");
    let mut vp = Viewport::new();
    vp.id = 2;
    vp.width = 160.0;
    vp.height = 100.0;
    vp.ucs_per_viewport = true;
    vp.ucs_handle = named;
    vp.base_ucs_handle = base;
    vp.ucs_ortho_type = 3;
    doc.add_entity_to_layout(EntityType::Viewport(Box::new(vp)), "Layout1")
        .expect("add viewport");
    (doc, named, base)
}

fn check(doc: &CadDocument, named: &str, base: &str, what: &str) {
    let vp = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Viewport(v) if v.width == 160.0 => Some(v.as_ref()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{what}: viewport lost"));
    let name_of = |handle: Handle| {
        doc.ucss
            .iter()
            .find(|u| u.handle == handle)
            .map_or_else(|| format!("no UCS for {handle:?}"), |u| u.name.clone())
    };
    assert_eq!(name_of(vp.ucs_handle), named, "{what}: 345");
    assert_eq!(name_of(vp.base_ucs_handle), base, "{what}: 346");
    assert_eq!(vp.ucs_ortho_type, 3, "{what}: 79");
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let dxf = DxfWriter::new(doc).write_to_vec().expect("write dxf");
    DxfReader::from_reader(Cursor::new(dxf))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn the_viewport_ucs_survives_dxf() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let (doc, _, _) = document(version);
        check(
            &through_dxf(&doc),
            "Sheet",
            "SheetBase",
            &format!("{version:?}"),
        );
    }
}

#[test]
fn the_viewport_ucs_survives_dxf_to_dwg() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let (doc, _, _) = document(version);
        let dwg = DwgWriter::write_to_vec(&through_dxf(&doc)).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(dwg))
            .read()
            .expect("read dwg");
        check(&back, "Sheet", "SheetBase", &format!("{version:?}"));
    }
}
