//! A handle in the extended data (group 1005) that names no object of the drawing stays as it is: AutoCAD keeps it, and the null handle
//! reports as "XData Handle Unknown Null" in its AUDIT. Only a handle of an object the writer leaves out becomes null.

use acadrust::entities::{EntityType, Line};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::xdata::{ExtendedDataRecord, XDataValue};
use acadrust::{CadDocument, DxfWriter, Handle};

#[test]
fn a_1005_handle_that_names_no_object_is_written_as_it_is() {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut line = Line::from_points(Vector3::ZERO, Vector3::new(1.0, 0.0, 0.0));
    let mut record = ExtendedDataRecord::new("AcDbBlockRepETag");
    record
        .values
        .push(XDataValue::Handle(Handle::new(0x379CE1)));
    line.common.extended_data.add_record(record);
    document.add_entity(EntityType::Line(line)).unwrap();
    let text = String::from_utf8(DxfWriter::new(&document).write_to_vec().unwrap()).unwrap();
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    let handles: Vec<_> = lines
        .windows(2)
        .filter(|pair| pair[0] == "1005")
        .map(|pair| pair[1])
        .collect();
    assert_eq!(handles, ["379CE1"]);
}
