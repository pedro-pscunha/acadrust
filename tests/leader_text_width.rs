//! The annotation size of a LEADER (DXF groups 40 and 41 of AcDbLeader). A DWG up to R2007 stores
//! the height and the width of the annotation box in the LEADER record. A DWG of R2010 and later
//! has no field for them (the size comes from the annotation), and AutoCAD's own DXF of such a
//! drawing has neither group, so a width set in a bridge DXF cannot be kept by an R2010+ DWG.

use std::io::Cursor;

use acadrust::entities::{EntityType, Leader};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn source(version: &str) -> String {
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\n{version}\n0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n\
0\nLEADER\n5\n100\n8\n0\n100\nAcDbLeader\n3\nStandard\n71\n1\n72\n0\n73\n3\n74\n0\n75\n0\n\
40\n2.5\n41\n7.75\n76\n2\n10\n0.0\n20\n0.0\n30\n0.0\n10\n10.0\n20\n5.0\n30\n0.0\n\
0\nENDSEC\n0\nEOF\n"
    )
}

fn leader(doc: &CadDocument) -> &Leader {
    doc.entities()
        .find_map(|e| match e {
            EntityType::Leader(l) => Some(l),
            _ => None,
        })
        .expect("a leader")
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

fn read(text: String) -> CadDocument {
    DxfReader::from_reader(Cursor::new(text.into_bytes()))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn the_dxf_reader_and_writer_keep_the_annotation_size() {
    let doc = read(source("AC1032"));
    assert_eq!(
        (leader(&doc).text_height, leader(&doc).text_width),
        (2.5, 7.75)
    );
    let bytes = DxfWriter::new(&doc)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("dxf");
    let again = read(String::from_utf8(bytes).expect("utf8"));
    assert_eq!(
        (leader(&again).text_height, leader(&again).text_width),
        (2.5, 7.75)
    );
}

#[test]
fn a_dwg_up_to_r2007_keeps_the_annotation_size() {
    for version in ["AC1015", "AC1018", "AC1021"] {
        let back = through_dwg(&read(source(version)));
        assert_eq!(
            (leader(&back).text_height, leader(&back).text_width),
            (2.5, 7.75),
            "{version}"
        );
    }
}

#[test]
fn a_dwg_of_r2010_and_later_has_no_field_for_it() {
    for version in [DxfVersion::AC1024, DxfVersion::AC1032] {
        let mut doc = read(source("AC1032"));
        doc.version = version;
        let back = through_dwg(&doc);
        assert_eq!(leader(&back).text_width, 0.0, "{version:?}");
        // What the DWG does keep.
        assert_eq!(leader(&back).vertices.len(), 2, "{version:?}");
        assert_eq!(leader(&back).vertices[1], Vector3::new(10.0, 5.0, 0.0));
    }
}
