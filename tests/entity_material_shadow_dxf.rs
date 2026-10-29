//! The material (group 347, a hard pointer to a MATERIAL object) and the shadow mode (group 284:
//! 0 casts and receives, 1 casts, 2 receives, 3 ignores) of an entity are common groups of every
//! graphical object in R2007 and later. The DXF reader had no arm for either, so a material or
//! a shadow mode set on any entity was lost from DXF to DWG.

use std::io::Cursor;

use acadrust::entities::{Leader, Line, Viewport};
use acadrust::objects::{Material, ObjectType};
use acadrust::types::{DxfVersion, Handle, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};

/// A drawing with a MATERIAL object and a line, a viewport and a leader that use it and have the
/// shadow mode 2 (receives shadows).
fn document(version: DxfVersion) -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(version);
    let dictionary = doc.header.acad_material_dict_handle;
    let material = doc.allocate_handle();
    let mut object = Material::new();
    object.handle = material;
    object.owner = dictionary;
    object.name = "Brick".to_string();
    doc.objects.insert(material, ObjectType::Material(object));
    let Some(ObjectType::Dictionary(entries)) = doc.objects.get_mut(&dictionary) else {
        panic!("no material dictionary")
    };
    entries.add_entry("Brick", material);

    let mut line = Line::from_points(Vector3::ZERO, Vector3::new(5.0, 5.0, 0.0));
    let mut viewport = Viewport::new();
    viewport.id = 2;
    viewport.width = 160.0;
    viewport.height = 100.0;
    let mut leader = Leader::new();
    leader.vertices = vec![Vector3::ZERO, Vector3::new(3.0, 2.0, 0.0)];
    for common in [&mut line.common, &mut viewport.common, &mut leader.common] {
        common.material_flags = 3;
        common.material_handle = Some(material);
        common.shadow_flags = 2;
    }
    doc.add_entity(EntityType::Line(line)).expect("add line");
    doc.add_entity_to_layout(EntityType::Viewport(Box::new(viewport)), "Layout1")
        .expect("add viewport");
    doc.add_entity(EntityType::Leader(leader))
        .expect("add leader");
    (doc, material)
}

/// (material flags, material, shadow flags) of the line, the viewport and the leader.
fn materials(doc: &CadDocument) -> Vec<(u8, Option<Handle>, u8)> {
    let mut found = Vec::new();
    for entity in doc.entities() {
        if matches!(
            entity,
            EntityType::Line(_) | EntityType::Viewport(_) | EntityType::Leader(_)
        ) {
            let common = entity.common();
            if matches!(entity, EntityType::Viewport(v) if v.width != 160.0) {
                continue;
            }
            found.push((
                common.material_flags,
                common.material_handle,
                common.shadow_flags,
            ));
        }
    }
    found
}

fn wanted(material: Handle) -> Vec<(u8, Option<Handle>, u8)> {
    vec![(3, Some(material), 2); 3]
}

fn through_dxf(doc: &CadDocument, bridge: bool) -> CadDocument {
    let writer = DxfWriter::new(doc);
    let bytes = if bridge {
        writer.for_dwg_bridge()
    } else {
        writer
    }
    .write_to_vec()
    .expect("write dxf");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn a_dwg_keeps_them() {
    let (doc, material) = document(DxfVersion::AC1032);
    assert_eq!(materials(&through_dwg(&doc)), wanted(material));
}

#[test]
fn dxf_keeps_them() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1032] {
        let (doc, material) = document(version);
        assert_eq!(
            materials(&through_dxf(&doc, false)),
            wanted(material),
            "{version:?}"
        );
    }
}

#[test]
fn dxf_to_dwg_to_dxf_keeps_them() {
    let (doc, material) = document(DxfVersion::AC1032);
    let bridged = through_dxf(&doc, true);
    assert_eq!(materials(&bridged), wanted(material), "DXF");
    let back = through_dwg(&bridged);
    assert_eq!(materials(&back), wanted(material), "DWG");
    assert_eq!(materials(&through_dxf(&back, false)), wanted(material));
}

#[test]
fn an_older_dxf_has_neither_group() {
    let (doc, _) = document(DxfVersion::AC1018);
    let text =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf8");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let start = lines
        .iter()
        .position(|l| *l == "ENTITIES")
        .expect("ENTITIES");
    let codes: Vec<&str> = lines[start..].chunks(2).map(|p| p[0]).collect();
    assert!(
        !codes.contains(&"347") && !codes.contains(&"284"),
        "{codes:?}"
    );
}
