//! INSERT-owned ATTRIBs retain their XDATA through both file formats.

use std::io::Cursor;

use acadrust::entities::EntityType;
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
    let attribute = &insert.attributes[0];
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
