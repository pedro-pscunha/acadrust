//! The arc point (16) of a 2-line angular DIMENSION is a 2D point in the DWG; its Z is the elevation of the common dimension data, the
//! one the text midpoint (11) has ("Z-coord for the ECS points (11, 12, 16)"). A DWG gives the arc point that Z back. An arc point whose
//! Z differs from the text midpoint's has no place in the format.

use std::io::Cursor;

use acadrust::entities::{Dimension, EntityType};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader};

fn source(text_z: f64, arc_z: f64) -> String {
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nDIMENSION\n5\n200\n100\nAcDbEntity\n8\n0\n\
100\nAcDbDimension\n10\n5\n20\n5\n30\n{text_z}\n11\n4\n21\n4\n31\n{text_z}\n70\n34\n100\nAcDb2LineAngularDimension\n\
13\n0\n23\n0\n33\n0\n14\n10\n24\n0\n34\n0\n15\n0\n25\n0\n35\n0\n16\n6\n26\n6\n36\n{arc_z}\n0\nENDSEC\n0\nEOF\n"
    )
}

fn arc_z(doc: &CadDocument) -> (f64, f64) {
    let dimension = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Dimension(d) => Some(d),
            _ => None,
        })
        .expect("a dimension");
    match &**dimension {
        Dimension::Angular2Ln(d) => (d.dimension_arc.z, d.base.text_middle_point.z),
        other => panic!("{other:?}"),
    }
}

fn through_dwg(text: String, version: DxfVersion) -> CadDocument {
    let mut doc = DxfReader::from_reader(Cursor::new(text.into_bytes()))
        .expect("reader")
        .read()
        .expect("read dxf");
    doc.version = version;
    let bytes = DwgWriter::write_to_vec(&doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn the_arc_point_of_a_plane_at_elevation_two_keeps_its_z_through_the_dwg() {
    for version in [
        DxfVersion::AC1015,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1032,
    ] {
        assert_eq!(
            arc_z(&through_dwg(source(2.0, 2.0), version)),
            (2.0, 2.0),
            "{version:?}"
        );
    }
}

#[test]
fn an_arc_point_at_another_height_than_the_text_has_no_place_in_the_dwg() {
    // The format holds one elevation: the arc point takes the text midpoint's.
    assert_eq!(
        arc_z(&through_dwg(source(2.0, 5.0), DxfVersion::AC1032)),
        (2.0, 2.0)
    );
}
