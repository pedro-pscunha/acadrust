//! The "has attribute definitions" flag (bit 2) of a BLOCK_RECORD in a DWG: AutoCAD sets it on an ordinary block that holds an ATTDEF
//! and never on the records of model and paper space, whatever they hold.

use std::io::Cursor;

use acadrust::entities::{AttributeDefinition, EntityType};
use acadrust::tables::BlockRecord;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter};

fn attdef() -> EntityType {
    EntityType::AttributeDefinition(AttributeDefinition::new("TAG".into(), "Prompt".into(), "x".into()))
}

fn roundtrip(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    // An ATTDEF in model space, one in paper space, and one in an ordinary block.
    doc.add_entity(attdef()).unwrap();
    doc.add_paper_space_entity(attdef()).unwrap();
    let mut record = BlockRecord::new("Tagged");
    record.handle = doc.allocate_handle();
    record.block_entity_handle = doc.allocate_handle();
    record.block_end_handle = doc.allocate_handle();
    let handle = doc.add_entity(attdef()).unwrap();
    record.entity_handles.push(handle);
    doc.block_records.add(record).unwrap();
    let bytes = DwgWriter::write_to_vec(&doc).expect("DWG write failed");
    DwgReader::from_stream(Cursor::new(bytes)).read().expect("DWG read failed")
}

#[test]
fn the_records_of_model_and_paper_space_never_have_the_attribute_bit() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1032] {
        let read = roundtrip(version);
        let flag = |name: &str| read.block_records.get(name).map(|r| r.flags.has_attributes);
        assert_eq!(flag("*Model_Space"), Some(false), "{version:?}: *Model_Space");
        assert_eq!(flag("*Paper_Space"), Some(false), "{version:?}: *Paper_Space");
        assert_eq!(flag("Tagged"), Some(true), "{version:?}: an ordinary block with an ATTDEF");
    }
}
