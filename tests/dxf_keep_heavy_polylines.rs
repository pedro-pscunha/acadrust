//! A plain 2D heavy polyline stays a POLYLINE (with its VERTEX records and SEQEND) when the DXF
//! writer is asked to keep it, through the DXF reader and the DWG writer. By default the DXF
//! writer down-saves it to an LWPOLYLINE (issue #63); AutoCAD SAVE keeps a 2D polyline as a 2D
//! polyline, and a host that bridges a DWG through DXF needs that.

use std::io::Cursor;

use acadrust::entities::{EntityType, Polyline2D, Vertex2D};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn document(version: DxfVersion) -> CadDocument {
    let mut pl = Polyline2D::new();
    pl.elevation = 2.0;
    pl.close();
    let mut v0 = Vertex2D::new(Vector3::new(0.0, 0.0, 2.0));
    v0.bulge = 0.5;
    pl.add_vertex(v0);
    let mut v1 = Vertex2D::new(Vector3::new(10.0, 0.0, 2.0));
    v1.start_width = 1.0;
    v1.end_width = 0.25;
    pl.add_vertex(v1);
    pl.add_vertex(Vertex2D::new(Vector3::new(10.0, 10.0, 2.0)));
    let mut doc = CadDocument::with_version(version);
    doc.add_entity(EntityType::Polyline2D(pl))
        .expect("add polyline");
    doc
}

fn kinds(doc: &CadDocument) -> (usize, usize) {
    let heavy = doc
        .entities()
        .filter(|e| matches!(e, EntityType::Polyline2D(_)))
        .count();
    let light = doc
        .entities()
        .filter(|e| matches!(e, EntityType::LwPolyline(_)))
        .count();
    (heavy, light)
}

fn record_count(text: &str, name: &str) -> usize {
    text.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .chunks(2)
        .filter(|p| p[0] == "0" && p.get(1) == Some(&name))
        .count()
}

#[test]
fn the_default_still_down_saves() {
    let doc = document(DxfVersion::AC1032);
    let text =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write")).expect("utf8");
    assert_eq!(
        (
            record_count(&text, "POLYLINE"),
            record_count(&text, "LWPOLYLINE")
        ),
        (0, 1)
    );
}

#[test]
fn the_option_keeps_polyline_vertex_and_seqend_records() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let doc = document(version);
        let text = String::from_utf8(
            DxfWriter::new(&doc)
                .keep_heavy_polylines()
                .write_to_vec()
                .expect("write"),
        )
        .expect("utf8");
        assert_eq!(
            (
                record_count(&text, "POLYLINE"),
                record_count(&text, "VERTEX"),
                record_count(&text, "SEQEND"),
                record_count(&text, "LWPOLYLINE")
            ),
            (1, 3, 1, 0),
            "{version:?}"
        );
    }
}

#[test]
fn a_kept_polyline_stays_a_polyline_2d_through_dxf_and_dwg() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1032] {
        let dxf = DxfWriter::new(&document(version))
            .keep_heavy_polylines()
            .write_to_vec()
            .expect("write dxf");
        let bridged = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(kinds(&bridged), (1, 0), "{version:?}: after the DXF reader");
        let dwg = DwgWriter::write_to_vec(&bridged).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(dwg))
            .read()
            .expect("read dwg");
        assert_eq!(kinds(&back), (1, 0), "{version:?}: after the DWG");
        let Some(EntityType::Polyline2D(pl)) = back
            .entities()
            .find(|e| matches!(e, EntityType::Polyline2D(_)))
        else {
            panic!("no polyline");
        };
        assert!(pl.is_closed(), "{version:?}");
        assert_eq!(pl.vertices.len(), 3, "{version:?}");
        assert_eq!(pl.vertices[0].bulge, 0.5, "{version:?}");
        assert_eq!(
            (pl.vertices[1].start_width, pl.vertices[1].end_width),
            (1.0, 0.25),
            "{version:?}"
        );
        assert_eq!(pl.elevation, 2.0, "{version:?}");
    }
}
