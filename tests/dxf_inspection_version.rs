use std::io::Cursor;

use acadrust::entities::Line;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DxfReader, DxfWriter, EntityType};

fn document(version: DxfVersion) -> CadDocument {
    let mut document = CadDocument::with_version(version);
    document.add_entity(EntityType::Line(Line::from_points(
        Vector3::new(-0.0, 0.12345678901234568, 0.0),
        Vector3::new(2.0, 3.0, 4.0),
    ))).unwrap();
    document
}

#[test]
fn binary_inspection_keeps_the_document_release_and_exact_numeric_bits() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1021, DxfVersion::AC1032] {
        let source = document(version);
        let mut bytes = Vec::new();
        DxfWriter::new_binary(&source).write_to_writer(&mut bytes).unwrap();
        let decoded = DxfReader::from_reader(Cursor::new(bytes)).unwrap().read().unwrap();
        assert_eq!(decoded.version, version, "inspection cannot compare a different release");
        let line = decoded.entities().find_map(|entity| if let EntityType::Line(line) = entity { Some(line) } else { None }).unwrap();
        assert_eq!(line.start.x.to_bits(), (-0.0_f64).to_bits());
        assert_eq!(line.start.y.to_bits(), 0.12345678901234568_f64.to_bits());
    }
}

#[test]
fn ordinary_binary_export_keeps_its_importer_compatibility_promotion() {
    let source = document(DxfVersion::AC1018);
    let mut bytes = Vec::new();
    DxfWriter::new_binary(&source).write_to_writer(&mut bytes).unwrap();
    let decoded = DxfReader::from_reader(Cursor::new(bytes)).unwrap().read().unwrap();
    assert_eq!(decoded.version, DxfVersion::AC1024);
}
