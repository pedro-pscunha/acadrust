//! A LEADER's annotation box (40 height, 41 width) and hookline flag (75) through the DXF reader, the DWG and the DXF writer.
//! The DWG holds the box through R2007 only: a later release has no field for it (the reader reads none, and the record that follows
//! it, the annotation and style handles, stays in step), so the leader gets the box of its annotation back. The hookline flag has a bit
//! in the R13/R14 arrowhead type only; later it is derived from the vertices and the horizontal direction.

use std::io::Cursor;

use acadrust::entities::EntityType;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

/// A text leader whose last segment is steep (60 degrees from the horizontal direction), so a hookline is drawn.
const SOURCE: &str = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLEADER\n5\n200\n100\nAcDbEntity\n8\n0\n\
100\nAcDbLeader\n3\nStandard\n71\n1\n72\n0\n73\n0\n74\n1\n75\n1\n40\n3.5\n41\n7.25\n76\n3\n10\n0\n20\n0\n30\n0\n10\n3\n20\n0\n30\n0\n10\n4.5\n20\n2.598076211353316\n30\n0\n\
210\n0\n220\n0\n230\n1\n211\n1\n221\n0\n231\n0\n212\n0\n222\n0\n232\n0\n213\n0\n223\n0\n233\n0\n0\nENDSEC\n0\nEOF\n";

/// (annotation height 40, width 41, hookline flag 75, hookline direction 74).
fn fields(doc: &CadDocument) -> (f64, f64, bool, i16) {
    let leader = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Leader(l) => Some(l),
            _ => None,
        })
        .expect("a leader");
    (
        leader.text_height,
        leader.text_width,
        leader.hookline_enabled,
        leader.hookline_direction.to_value(),
    )
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn through_dwg(doc: &CadDocument, version: DxfVersion) -> CadDocument {
    let mut copy = doc.clone();
    copy.version = version;
    let bytes = DwgWriter::write_to_vec(&copy).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn the_dxf_reader_and_writer_keep_the_box_and_the_hookline() {
    let doc = read(SOURCE.as_bytes().to_vec());
    assert_eq!(fields(&doc), (3.5, 7.25, true, 1));
    let bytes = DxfWriter::new(&doc).write_to_vec().expect("write dxf");
    assert_eq!(fields(&read(bytes)), (3.5, 7.25, true, 1));
}

#[test]
fn a_release_through_r2007_holds_the_box() {
    let doc = read(SOURCE.as_bytes().to_vec());
    for version in [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1021] {
        let (height, width, _, direction) = fields(&through_dwg(&doc, version));
        assert_eq!((height, width, direction), (3.5, 7.25, 1), "{version:?}");
    }
}

#[test]
fn a_later_release_has_no_field_for_the_box_and_keeps_the_record_in_step() {
    let doc = read(SOURCE.as_bytes().to_vec());
    for version in [DxfVersion::AC1024, DxfVersion::AC1027, DxfVersion::AC1032] {
        let back = through_dwg(&doc, version);
        let (height, width, hookline, direction) = fields(&back);
        // The box is not in the record: the leader gets its annotation's (none here) and the style's text height.
        assert_eq!(width, 0.0, "{version:?}");
        assert!(
            height != 3.5,
            "{version:?}: the box height cannot come back from a record that does not hold it"
        );
        // The fields after the box still read right, so the box is not read where another field is.
        assert_eq!(direction, 1, "{version:?}");
        // The hookline flag is derived: 60 degrees to the horizontal direction hooks.
        assert!(hookline, "{version:?}");
    }
}

#[test]
fn the_derived_hookline_flag_is_off_for_a_shallow_last_segment() {
    // The last segment 10 degrees from the horizontal direction: no hookline is drawn.
    let shallow = SOURCE.replace("10\n4.5\n20\n2.598076211353316", "10\n6.9\n20\n0.5");
    let back = through_dwg(&read(shallow.into_bytes()), DxfVersion::AC1032);
    assert!(!fields(&back).2);
}
