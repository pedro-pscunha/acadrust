//! The thickness of an ATTRIB and of an ATTDEF (DXF 39, a field of the TEXT data of a DWG)
//! survives a DWG and a DXF round trip.

use std::io::Cursor;

use acadrust::entities::{AttributeDefinition, AttributeEntity, Insert};
use acadrust::tables::BlockRecord;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};

const ATTRIB_THICKNESS: f64 = 3.5;
const ATTDEF_THICKNESS: f64 = 2.25;

/// An INSERT with one ATTRIB and, beside it, an ATTDEF, both with a thickness.
fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let mut record = BlockRecord::new("TAGGED");
    record.handle = doc.allocate_handle();
    record.block_entity_handle = doc.allocate_handle();
    record.block_end_handle = doc.allocate_handle();
    doc.block_records.add(record).unwrap();

    let mut insert = Insert::new("TAGGED", Vector3::new(5.0, 5.0, 0.0));
    insert.common.handle = doc.allocate_handle();
    let mut attribute = AttributeEntity::new("TAG".into(), "value".into());
    attribute.common.handle = doc.allocate_handle();
    attribute.common.owner_handle = insert.common.handle;
    attribute.thickness = ATTRIB_THICKNESS;
    insert.attributes.push(attribute);
    insert.seqend_handle = Some(doc.allocate_handle());
    doc.add_entity(EntityType::Insert(insert)).unwrap();

    let mut definition = AttributeDefinition::new("TAG".into(), "Prompt".into(), "default".into());
    definition.thickness = ATTDEF_THICKNESS;
    doc.add_entity(EntityType::AttributeDefinition(definition))
        .unwrap();
    doc
}

/// (thickness of the ATTRIB, thickness of the ATTDEF) of a document.
fn thicknesses(doc: &CadDocument) -> (Option<f64>, Option<f64>) {
    let mut attrib = None;
    let mut attdef = None;
    for entity in doc.entities() {
        match entity {
            EntityType::Insert(insert) => {
                attrib = insert.attributes.first().map(|a| a.thickness);
            }
            EntityType::AttributeDefinition(definition) => attdef = Some(definition.thickness),
            _ => {}
        }
    }
    (attrib, attdef)
}

const WANTED: (Option<f64>, Option<f64>) = (Some(ATTRIB_THICKNESS), Some(ATTDEF_THICKNESS));

#[test]
fn a_dwg_keeps_the_thickness_of_attributes() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1032] {
        let bytes = DwgWriter::write_to_vec(&document(version)).expect("write dwg");
        let read = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read dwg");
        assert_eq!(thicknesses(&read), WANTED, "{version:?}");
    }
}

#[test]
fn a_dxf_keeps_the_thickness_of_attributes() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        for bridge in [false, true] {
            let source = document(version);
            let writer = DxfWriter::new(&source);
            let writer = if bridge {
                writer.for_dwg_bridge()
            } else {
                writer
            };
            let dxf = writer.write_to_vec().expect("write dxf");
            let read = DxfReader::from_reader(Cursor::new(dxf))
                .expect("reader")
                .read()
                .expect("read dxf");
            assert_eq!(thicknesses(&read), WANTED, "{version:?} bridge {bridge}");
        }
    }
}

#[test]
fn a_dwg_to_dxf_to_dwg_keeps_the_thickness_of_attributes() {
    let bytes = DwgWriter::write_to_vec(&document(DxfVersion::AC1032)).expect("write dwg");
    let read = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let dxf = DxfWriter::new(&read)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    let from_dxf = DxfReader::from_reader(Cursor::new(dxf))
        .expect("reader")
        .read()
        .expect("read dxf");
    let again = DwgReader::from_stream(Cursor::new(
        DwgWriter::write_to_vec(&from_dxf).expect("write dwg"),
    ))
    .read()
    .expect("read dwg");
    assert_eq!(thicknesses(&again), WANTED);
}
