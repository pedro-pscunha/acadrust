//! INSERT-owned ATTRIBs retain their XDATA through both file formats.

use std::io::Cursor;

use acadrust::entities::{AttributeEntity, EntityType};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const DXF: &str = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nAPPID\n70\n1\n0\nAPPID\n2\nACME\n70\n0\n0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n8\n0\n2\nLABEL\n70\n2\n10\n0\n20\n0\n3\nLABEL\n0\nENDBLK\n8\n0\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nINSERT\n5\n30\n8\n0\n66\n1\n2\nLABEL\n10\n0\n20\n0\n0\nATTRIB\n5\n31\n330\n30\n100\nAcDbEntity\n8\n0\n100\nAcDbText\n10\n0\n20\n0\n40\n1\n1\nvalue\n100\nAcDbAttribute\n2\nTAG\n70\n0\n1001\nACME\n1000\nattribute-xdata\n1005\n31\n0\nSEQEND\n5\n32\n330\n30\n0\nENDSEC\n0\nEOF\n";

fn assert_attrib_xdata(document: &CadDocument, stage: &str) {
    let insert = document
        .entities()
        .find_map(|e| match e {
            EntityType::Insert(i) => Some(i),
            _ => None,
        })
        .unwrap();
    assert_attribute_xdata(&insert.attributes[0], stage);
}

fn assert_attribute_xdata(attribute: &AttributeEntity, stage: &str) {
    let data = attribute
        .common
        .extended_data
        .get_record("ACME")
        .unwrap_or_else(|| panic!("{stage}: ATTRIB has no ACME XDATA"));
    assert_eq!(
        data.values,
        vec![
            acadrust::xdata::XDataValue::String("attribute-xdata".into()),
            acadrust::xdata::XDataValue::Handle(attribute.common.handle),
        ],
        "{stage}"
    );
}

fn assert_nested_attribute_xdata(document: &CadDocument, stage: &str) {
    let block = document
        .block_records
        .get("OUTER")
        .expect("OUTER block definition");
    let insert = block
        .entity_handles
        .iter()
        .filter_map(|h| document.get_entity(*h))
        .find_map(|e| match e {
            EntityType::Insert(insert) if insert.block_name == "LABEL" => Some(insert),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{stage}: nested LABEL INSERT is not in OUTER"));
    assert_eq!(
        insert.common.owner_handle, block.handle,
        "{stage}: nested INSERT must stay owned by OUTER"
    );
    assert!(
        !document
            .block_records
            .iter()
            .any(|b| b.handle == insert.common.handle),
        "{stage}: fixture INSERT handle collides with an allocated BLOCK_RECORD"
    );
    let attribute = insert
        .attributes
        .first()
        .unwrap_or_else(|| panic!("{stage}: nested INSERT lost its ATTRIB"));
    assert!(
        !document
            .entities()
            .any(|e| e.common().handle == attribute.common.handle),
        "{stage}: nested ATTRIB handle collides with another entity"
    );
    assert_attribute_xdata(attribute, stage);
}

#[test]
fn attribute_xdata_on_insert_inside_block_definition_survives_roundtrips() {
    // Explicit block records and owners distinguish the nested INSERT from a model-space one.
    let dxf = concat!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n9\n$HANDSEED\n5\nB000\n0\nENDSEC\n",
        "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nAPPID\n5\nA010\n70\n1\n0\nAPPID\n5\nA011\n2\nACME\n70\n0\n0\nENDTAB\n",
        "0\nTABLE\n2\nBLOCK_RECORD\n5\nA000\n70\n4\n0\nBLOCK_RECORD\n5\nA001\n2\n*Model_Space\n70\n0\n0\nBLOCK_RECORD\n5\nA002\n2\n*Paper_Space\n70\n0\n0\nBLOCK_RECORD\n5\nA003\n2\nLABEL\n70\n0\n0\nBLOCK_RECORD\n5\nA004\n2\nOUTER\n70\n0\n0\nENDTAB\n0\nENDSEC\n",
        "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n5\nA100\n330\nA003\n8\n0\n2\nLABEL\n70\n2\n10\n0\n20\n0\n3\nLABEL\n0\nENDBLK\n5\nA101\n330\nA003\n8\n0\n",
        "0\nBLOCK\n5\nA102\n330\nA004\n8\n0\n2\nOUTER\n70\n0\n10\n0\n20\n0\n3\nOUTER\n",
        "0\nINSERT\n5\n1300\n330\nA004\n8\n0\n66\n1\n2\nLABEL\n10\n0\n20\n0\n0\nATTRIB\n5\n1301\n330\n1300\n100\nAcDbEntity\n8\n0\n100\nAcDbText\n10\n0\n20\n0\n40\n1\n1\nvalue\n100\nAcDbAttribute\n2\nTAG\n70\n0\n1001\nACME\n1000\nattribute-xdata\n1005\n1301\n0\nSEQEND\n5\n1302\n330\n1300\n",
        "0\nENDBLK\n5\nA103\n330\nA004\n8\n0\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nINSERT\n5\nA104\n330\nA001\n8\n0\n2\nOUTER\n10\n0\n20\n0\n0\nENDSEC\n0\nEOF\n",
    );
    assert_eq!(
        dxf.matches("0\nINSERT\n").count(),
        2,
        "fixture must contain a nested INSERT and the model-space OUTER reference"
    );
    let document = DxfReader::from_reader(Cursor::new(dxf.as_bytes()))
        .unwrap()
        .read()
        .unwrap();
    assert_nested_attribute_xdata(&document, "DXF input");
    let dxf = DxfWriter::new(&document).write_to_vec().unwrap();
    let read = DxfReader::from_reader(Cursor::new(dxf))
        .unwrap()
        .read()
        .unwrap();
    assert_nested_attribute_xdata(&read, "DXF round trip");
    let dwg = DwgWriter::write_to_vec(&document).unwrap();
    let read = DwgReader::from_stream(Cursor::new(dwg)).read().unwrap();
    assert_nested_attribute_xdata(&read, "DWG round trip");
    let dxf = DxfWriter::new(&read).write_to_vec().unwrap();
    let read = DxfReader::from_reader(Cursor::new(dxf))
        .unwrap()
        .read()
        .unwrap();
    assert_nested_attribute_xdata(&read, "DWG to DXF");
}

#[test]
fn insert_attribute_xdata_survives_dxf_and_dwg_roundtrips() {
    let document = DxfReader::from_reader(Cursor::new(DXF.as_bytes()))
        .unwrap()
        .read()
        .unwrap();
    assert_attrib_xdata(&document, "DXF input");
    let dxf = DxfWriter::new(&document).write_to_vec().unwrap();
    let read = DxfReader::from_reader(Cursor::new(dxf))
        .unwrap()
        .read()
        .unwrap();
    assert_attrib_xdata(&read, "DXF round trip");
    let dwg = DwgWriter::write_to_vec(&document).unwrap();
    let read = DwgReader::from_stream(Cursor::new(dwg)).read().unwrap();
    assert_attrib_xdata(&read, "DWG round trip");
    let dxf = DxfWriter::new(&read).write_to_vec().unwrap();
    let read = DxfReader::from_reader(Cursor::new(dxf))
        .unwrap()
        .read()
        .unwrap();
    assert_attrib_xdata(&read, "DWG to DXF");
}
