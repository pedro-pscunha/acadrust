//! An xdata handle (1005) that is null keeps its place as the null handle, and one that names nothing in the document stays as the file
//! gave it (AutoCAD keeps it; see `xdata_handle_to_missing_object`). AutoCAD's dimension overrides (`ACAD` DSTYLE) are `1070 <variable>` then the
//! value, so a dropped `1005 0` after `1070 340` (DIMTXSTY) shifted every later pair.

use std::io::Cursor;

use acadrust::entities::EntityType;
use acadrust::types::Handle;
use acadrust::xdata::XDataValue;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const DXF: &str = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nAPPID\n70\n1\n0\nAPPID\n2\nACME\n70\n0\n0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n5\n30\n8\n0\n10\n0\n20\n0\n30\n0\n11\n1\n21\n1\n31\n0\n1001\nACME\n1070\n340\n1005\n0\n1070\n341\n1005\nFFFF\n1070\n41\n1040\n2.5\n0\nENDSEC\n0\nEOF\n";

fn values(document: &CadDocument, stage: &str) -> Vec<XDataValue> {
    let line = document
        .entities()
        .find_map(|e| match e {
            EntityType::Line(l) => Some(l),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{stage}: no LINE"));
    line.common
        .extended_data
        .get_record("ACME")
        .unwrap_or_else(|| panic!("{stage}: LINE has no ACME xdata"))
        .values
        .clone()
}

fn dxf(document: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(document).write_to_vec().unwrap();
    DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap()
}

#[test]
fn null_and_dangling_xdata_handles_keep_their_place() {
    let document = DxfReader::from_reader(Cursor::new(DXF.as_bytes()))
        .unwrap()
        .read()
        .unwrap();
    let want = vec![
        XDataValue::Integer16(340),
        XDataValue::Handle(Handle::NULL),
        XDataValue::Integer16(341),
        XDataValue::Handle(Handle::new(0xFFFF)),
        XDataValue::Integer16(41),
        XDataValue::Real(2.5),
    ];
    assert_eq!(values(&dxf(&document), "DXF"), want, "DXF");
    let dwg = DwgWriter::write_to_vec(&document).unwrap();
    let read = DwgReader::from_stream(Cursor::new(dwg)).read().unwrap();
    assert_eq!(values(&dxf(&read), "DWG"), want, "DWG then DXF");
}
