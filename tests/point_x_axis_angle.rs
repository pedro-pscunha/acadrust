//! The x-axis angle of a POINT is held in radians. A DWG stores it in radians (a BD), a DXF in
//! degrees (group 50), so the DXF reader and writer convert.

use std::io::Cursor;

use acadrust::entities::Point;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};

const ANGLE: f64 = 0.5; // radians, 28.6478897565 degrees

fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let mut point = Point::new();
    point.location = Vector3::new(1.0, 2.0, 0.0);
    point.x_axis_angle = ANGLE;
    doc.add_entity(EntityType::Point(point)).unwrap();
    doc
}

fn angle(doc: &CadDocument) -> f64 {
    doc.entities()
        .find_map(|entity| match entity {
            EntityType::Point(point) => Some(point.x_axis_angle),
            _ => None,
        })
        .expect("a point")
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(
        DwgWriter::write_to_vec(doc).expect("write dwg"),
    ))
    .read()
    .expect("read dwg")
}

fn through_dxf(doc: &CadDocument) -> (String, CadDocument) {
    let text =
        String::from_utf8(DxfWriter::new(doc).write_to_vec().expect("write dxf")).expect("utf8");
    let read = DxfReader::from_reader(Cursor::new(text.clone().into_bytes()))
        .expect("reader")
        .read()
        .expect("read dxf");
    (text, read)
}

/// The value of the group 50 of the POINT in a DXF.
fn group_50(dxf: &str) -> f64 {
    let lines: Vec<&str> = dxf.lines().map(str::trim).collect();
    let at = lines
        .iter()
        .position(|l| *l == "AcDbPoint")
        .expect("a POINT")
        - 1;
    lines[at..]
        .chunks(2)
        .take_while(|pair| pair[0] != "0")
        .find(|pair| pair[0] == "50")
        .and_then(|pair| pair[1].parse().ok())
        .expect("a group 50")
}

#[test]
fn a_dwg_keeps_the_angle_in_radians() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        assert_eq!(
            angle(&through_dwg(&document(version))),
            ANGLE,
            "{version:?}"
        );
    }
}

#[test]
fn a_dxf_holds_degrees_and_reads_radians() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let (text, read) = through_dxf(&document(version));
        assert!(
            (group_50(&text) - ANGLE.to_degrees()).abs() < 1e-9,
            "{version:?}"
        );
        assert!((angle(&read) - ANGLE).abs() < 1e-12, "{version:?}");
    }
}

#[test]
fn a_dwg_to_dxf_gives_degrees_and_back_to_dwg_gives_radians() {
    let read = through_dwg(&document(DxfVersion::AC1032));
    let (text, from_dxf) = through_dxf(&read);
    assert!((group_50(&text) - ANGLE.to_degrees()).abs() < 1e-9);
    assert!((angle(&through_dwg(&from_dxf)) - ANGLE).abs() < 1e-12);
}
