//! A 2D POLYLINE's default start and end widths (groups 40 and 41 of the POLYLINE record) stay
//! with the polyline: AutoCAD keeps them in a DWG, and a host such as CADCraft writes them next
//! to the widths of every vertex. The DXF reader bakes the effective width into each vertex (a
//! VERTEX without 40/41 inherits the default), but it also kept the default at 0 once any vertex
//! carried a width of its own, so the default was lost through DXF to DWG.

use std::io::Cursor;

use acadrust::entities::Polyline2D;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};

/// A POLYLINE with the default widths 2.5 and 1.0 and three vertices; `vertex` is the width groups
/// of each VERTEX.
fn source(vertex: &str) -> String {
    let vertices: String = [(0, 0), (10, 0), (10, 10)]
        .iter()
        .map(|(x, y)| format!("0\nVERTEX\n8\n0\n10\n{x}.0\n20\n{y}.0\n30\n0.0\n{vertex}70\n0\n"))
        .collect();
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n\
0\nPOLYLINE\n5\n100\n8\n0\n66\n1\n10\n0.0\n20\n0.0\n30\n0.0\n40\n2.5\n41\n1.0\n70\n0\n{vertices}\
0\nSEQEND\n8\n0\n\
0\nENDSEC\n0\nEOF\n"
    )
}

fn read(text: &str) -> CadDocument {
    DxfReader::from_reader(Cursor::new(text.as_bytes().to_vec()))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn polyline(doc: &CadDocument) -> &Polyline2D {
    doc.entities()
        .find_map(|e| match e {
            EntityType::Polyline2D(p) => Some(p),
            _ => None,
        })
        .expect("a 2D polyline")
}

fn widths(p: &Polyline2D) -> ((f64, f64), Vec<(f64, f64)>) {
    (
        (p.start_width, p.end_width),
        p.vertices
            .iter()
            .map(|v| (v.start_width, v.end_width))
            .collect(),
    )
}

/// DXF to the DWG bridge and back: the document a DWG reader gives.
fn through_the_bridge(doc: &CadDocument) -> CadDocument {
    let dxf = DxfWriter::new(doc)
        .keep_heavy_polylines()
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    let dwg = DwgWriter::write_to_vec(&read_bytes(dxf)).expect("write dwg");
    DwgReader::from_stream(Cursor::new(dwg))
        .read()
        .expect("read dwg")
}

fn read_bytes(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn the_default_widths_stay_when_the_vertices_carry_none() {
    let back = through_the_bridge(&read(&source("")));
    assert_eq!(
        widths(polyline(&back)),
        ((2.5, 1.0), vec![(2.5, 1.0); 3]),
        "the vertices inherit the default"
    );
}

#[test]
fn the_default_widths_stay_when_every_vertex_carries_its_own() {
    let doc = read(&source("40\n2.5\n41\n1.0\n"));
    assert_eq!(widths(polyline(&doc)), ((2.5, 1.0), vec![(2.5, 1.0); 3]));
    let back = through_the_bridge(&doc);
    assert_eq!(widths(polyline(&back)), ((2.5, 1.0), vec![(2.5, 1.0); 3]));
}

#[test]
fn a_vertex_that_is_explicitly_zero_wide_stays_zero_wide() {
    // A default of 2.5 would give such a vertex a width in a reader that treats 0 as "use the
    // default", so the default is not kept for this polyline (a vertex written with width 0
    // under a non-zero default carries the group, so it reads as zero wide again).
    let doc = read(&source("40\n0.0\n41\n0.0\n"));
    let back = through_the_bridge(&doc);
    let (_, vertices) = widths(polyline(&back));
    assert_eq!(vertices, vec![(0.0, 0.0); 3]);
}

#[test]
fn a_zero_wide_vertex_under_a_default_is_written_with_its_zero_width() {
    // A polyline a DWG gave: default widths and a vertex of no width.
    let mut doc = read(&source("40\n2.5\n41\n1.0\n"));
    let Some(EntityType::Polyline2D(p)) = doc
        .entities_mut()
        .find(|e| matches!(e, EntityType::Polyline2D(_)))
    else {
        panic!("no polyline")
    };
    p.vertices[1].start_width = 0.0;
    p.vertices[1].end_width = 0.0;
    let dxf = DxfWriter::new(&doc)
        .keep_heavy_polylines()
        .write_to_vec()
        .expect("write dxf");
    let back = read_bytes(dxf);
    let (_, vertices) = widths(polyline(&back));
    assert_eq!(vertices[1], (0.0, 0.0));
    assert_eq!(vertices[0], (2.5, 1.0));
}
