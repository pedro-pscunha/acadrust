//! A smoothed polygon mesh keeps which of its VERTEX records are control points (flags 16 + 64)
//! and which are the fitted surface points (flag 64). The DWG writer wrote the flags, but the
//! DWG reader set every vertex to 0, so after a DWG round trip the two kinds no longer differed.

use std::io::Cursor;

use acadrust::entities::{PolygonMeshEntity, PolygonMeshVertex};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, EntityType};

#[test]
fn a_dwg_keeps_the_flags_of_polygon_mesh_vertices() {
    for version in [DxfVersion::AC1018, DxfVersion::AC1032] {
        let mut mesh = PolygonMeshEntity::new();
        mesh.m_vertex_count = 2;
        mesh.n_vertex_count = 2;
        let flags = [80_i16, 80, 64, 64];
        for (i, f) in flags.iter().enumerate() {
            let mut v = PolygonMeshVertex::at(Vector3::new(i as f64, 0.0, 0.0));
            v.flags = *f;
            mesh.vertices.push(v);
        }
        let mut doc = CadDocument::with_version(version);
        doc.add_entity(EntityType::PolygonMesh(mesh))
            .expect("add mesh");
        let dwg = DwgWriter::write_to_vec(&doc).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(dwg))
            .read()
            .expect("read dwg");
        let Some(EntityType::PolygonMesh(m)) = back
            .entities()
            .find(|e| matches!(e, EntityType::PolygonMesh(_)))
        else {
            panic!("{version:?}: no polygon mesh");
        };
        let read: Vec<i16> = m.vertices.iter().map(|v| v.flags).collect();
        assert_eq!(read, flags, "{version:?}");
    }
}
