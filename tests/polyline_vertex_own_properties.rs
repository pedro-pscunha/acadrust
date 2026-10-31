//! A VERTEX that is ByLayer under a polyline of another colour keeps its own properties on a save: AutoCAD does not give it the owner's,
//! and its AUDIT reports a vertex whose colour, linetype or linetype scale differ from the owner's without repairing it.

use std::io::Cursor;

use acadrust::entities::{EntityType, Polyline2D, Vertex2D};
use acadrust::types::{Color, DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn document() -> (CadDocument, acadrust::Handle) {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut polyline = Polyline2D::new();
    polyline.common.color = Color::Index(12);
    polyline.common.linetype = "CONTINUOUS".to_string();
    polyline.common.linetype_scale = 0.02;
    for x in [0.0, 5.0] {
        let mut vertex = Vertex2D::new(Vector3::new(x, 1.0, 0.0));
        // The vertex own data: ByLayer, no linetype, a scale of 1.
        let mut own = acadrust::entities::EntityCommon::new();
        own.color = Color::ByLayer;
        own.linetype_scale = 1.0;
        vertex.common = Some(Box::new(own));
        polyline.vertices.push(vertex);
    }
    let handle = document
        .add_entity(EntityType::Polyline2D(polyline))
        .unwrap();
    (document, handle)
}

fn vertices(document: &CadDocument, handle: acadrust::Handle) -> Vec<(Color, f64)> {
    let EntityType::Polyline2D(polyline) = document.get_entity(handle).unwrap() else {
        panic!("a 2D polyline")
    };
    polyline
        .vertices
        .iter()
        .map(|v| {
            v.common.as_ref().map_or(
                (polyline.common.color, polyline.common.linetype_scale),
                |c| (c.color, c.linetype_scale),
            )
        })
        .collect()
}

#[test]
fn a_bylayer_vertex_keeps_its_color_and_scale_through_a_dwg_save() {
    let (document, handle) = document();
    let back = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&document).unwrap()))
        .read()
        .unwrap();
    let handle = back
        .entities()
        .find_map(|e| matches!(e, EntityType::Polyline2D(_)).then(|| e.common().handle))
        .unwrap_or(handle);
    assert_eq!(vertices(&back, handle), vec![(Color::ByLayer, 1.0); 2]);
}

#[test]
fn a_bylayer_vertex_keeps_its_color_and_scale_through_a_dxf_save() {
    let (document, _) = document();
    let text = DxfWriter::new(&document).write_to_vec().unwrap();
    let back = DxfReader::from_reader(Cursor::new(text))
        .unwrap()
        .read()
        .unwrap();
    let handle = back
        .entities()
        .find_map(|e| matches!(e, EntityType::Polyline2D(_)).then(|| e.common().handle))
        .unwrap();
    assert_eq!(vertices(&back, handle), vec![(Color::ByLayer, 1.0); 2]);
}
