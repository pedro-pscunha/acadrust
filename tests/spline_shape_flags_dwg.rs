//! A DWG spline has no field for the planar (DXF 70 bit 8) flag; the DXF writer gives it from the geometry, as AutoCAD
//! writes it in a DXF (the linear bit, 16, has no measurable rule: a flag a DXF gave is kept).

use std::io::Cursor;

use acadrust::entities::Spline;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};

fn spline(points: &[(f64, f64, f64)]) -> CadDocument {
    let mut spline = Spline::new();
    spline.degree = 1;
    spline.control_points = points.iter().map(|p| Vector3::new(p.0, p.1, p.2)).collect();
    spline.knots = (0..points.len() + 2).map(|i| i as f64).collect();
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.add_entity(EntityType::Spline(spline)).unwrap();
    doc
}

fn flags(doc: &CadDocument) -> i16 {
    let text = String::from_utf8(DxfWriter::new(doc).write_to_vec().unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let pairs: Vec<(&str, &str)> = lines
        .chunks(2)
        .filter(|p| p.len() == 2)
        .map(|p| (p[0], p[1]))
        .collect();
    let at = pairs
        .iter()
        .position(|p| *p == ("0", "SPLINE"))
        .expect("a spline");
    pairs[at..]
        .iter()
        .find(|p| p.0 == "70")
        .expect("flags")
        .1
        .parse()
        .unwrap()
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap()))
        .read()
        .unwrap()
}

#[test]
fn a_spline_read_from_a_dwg_has_the_planar_flag_its_geometry_gives() {
    // In a plane (also on a line): planar.
    assert_eq!(
        flags(&through_dwg(&spline(&[
            (0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0),
            (3.0, 3.0, 3.0)
        ]))) & 8,
        8
    );
    assert_eq!(
        flags(&through_dwg(&spline(&[
            (0.0, 0.0, 0.0),
            (1.0, 2.0, 0.0),
            (3.0, 0.0, 0.0),
            (4.0, 3.0, 0.0)
        ]))) & 8,
        8
    );
    // In space: not planar.
    assert_eq!(
        flags(&through_dwg(&spline(&[
            (0.0, 0.0, 0.0),
            (1.0, 2.0, 1.0),
            (3.0, 0.0, 5.0),
            (4.0, 3.0, 2.0)
        ]))) & 8,
        0
    );
}

#[test]
fn a_flag_a_dxf_gave_is_kept_in_a_dxf() {
    let mut doc = spline(&[
        (0.0, 0.0, 0.0),
        (1.0, 2.0, 1.0),
        (3.0, 0.0, 5.0),
        (4.0, 3.0, 2.0),
    ]);
    for entity in doc.entities_mut() {
        if let EntityType::Spline(spline) = entity {
            spline.flags.linear = true;
        }
    }
    let dxf = DxfReader::from_reader(Cursor::new(DxfWriter::new(&doc).write_to_vec().unwrap()))
        .unwrap()
        .read()
        .unwrap();
    assert_eq!(flags(&dxf) & 24, 16);
}
