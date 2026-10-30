//! Two records of a DXF that share a handle are both kept: the reader gives the later one a handle of its own and says so.
use std::io::Cursor;

use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, EntityType};

fn source() -> Vec<u8> {
    "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n\
0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nBLOCK_RECORD\n70\n1\n0\nBLOCK_RECORD\n5\n50\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\nDoor\n0\nENDTAB\n0\nENDSEC\n\
0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n5\n51\n330\n50\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\nDoor\n70\n0\n10\n0\n20\n0\n30\n0\n3\nDoor\n1\n\n\
0\nCIRCLE\n5\n100\n330\n50\n100\nAcDbEntity\n8\n0\n100\nAcDbCircle\n10\n0\n20\n0\n30\n0\n40\n2\n\
0\nENDBLK\n5\n52\n330\n50\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n\
0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n\
0\nLINE\n5\n100\n8\n0\n100\nAcDbEntity\n100\nAcDbLine\n10\n0\n20\n0\n30\n0\n11\n1\n21\n1\n31\n0\n\
0\nENDSEC\n0\nEOF\n"
        .as_bytes()
        .to_vec()
}

fn count(doc: &CadDocument) -> (usize, usize) {
    let lines = doc
        .entities()
        .filter(|e| matches!(e, EntityType::Line(_)))
        .count();
    let circles = doc
        .entities()
        .filter(|e| matches!(e, EntityType::Circle(_)))
        .count();
    (lines, circles)
}

#[test]
fn records_with_one_handle_are_both_kept_and_the_reader_says_so() {
    let doc = DxfReader::from_reader(Cursor::new(source()))
        .unwrap()
        .read()
        .unwrap();
    assert_eq!(count(&doc), (1, 1));
    let mut handles: Vec<_> = doc.entities().map(|e| e.common().handle).collect();
    handles.sort();
    handles.dedup();
    assert_eq!(handles.len(), 2, "each has a handle of its own");
    let notes: Vec<String> = doc
        .notifications
        .iter()
        .map(|n| n.message.clone())
        .collect();
    assert!(
        notes
            .iter()
            .any(|n| n.contains("100") && n.contains("handle")),
        "{notes:?}"
    );
}

#[test]
fn both_records_survive_a_dwg() {
    let doc = DxfReader::from_reader(Cursor::new(source()))
        .unwrap()
        .read()
        .unwrap();
    let back = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    assert_eq!(count(&back), (1, 1));
}

#[test]
fn two_objects_with_one_handle_are_both_kept() {
    use acadrust::objects::ObjectType;
    let group = |description: &str| {
        format!("0\nGROUP\n5\n60\n330\nC\n100\nAcDbGroup\n300\n{description}\n70\n0\n71\n1\n")
    };
    let dxf = format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nOBJECTS\n{}{}0\nENDSEC\n0\nEOF\n",
        group("first"),
        group("second")
    );
    let doc = DxfReader::from_reader(Cursor::new(dxf.into_bytes()))
        .unwrap()
        .read()
        .unwrap();
    let mut descriptions: Vec<String> = doc
        .objects
        .values()
        .filter_map(|o| match o {
            ObjectType::Group(g) => Some(g.description.clone()),
            _ => None,
        })
        .collect();
    descriptions.sort();
    assert_eq!(descriptions, ["first", "second"]);
    assert!(doc
        .notifications
        .iter()
        .any(|n| n.message.contains("object has the handle 60")));
}
