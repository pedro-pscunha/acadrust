//! The extrusion direction (normal, groups 210, 220 and 230) of an ATTRIB and of an ATTDEF
//! survives a DXF read, a DWG and the DXF to DWG bridge. The DXF reader took the normal of an
//! ATTDEF and dropped the one of an ATTRIB, so every attribute of a tilted or flipped INSERT
//! came out in the XY plane.

use std::io::Cursor;

use acadrust::entities::EntityType;
use acadrust::types::Vector3;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const TILTED: Vector3 = Vector3 {
    x: 0.6,
    y: 0.0,
    z: 0.8,
};
const FLIPPED: Vector3 = Vector3 {
    x: 0.0,
    y: 0.0,
    z: -1.0,
};

/// A block with two ATTDEFs (TILT and FLIP) and an INSERT of it with two ATTRIBs.
fn source() -> String {
    let normal = |n: Vector3| format!("210\n{}\n220\n{}\n230\n{}\n", n.x, n.y, n.z);
    let attdef = |tag: &str, y: i32, n: Vector3| {
        format!(
            "0\nATTDEF\n5\n{:X}\n8\n0\n100\nAcDbText\n10\n0.0\n20\n{y}.0\n30\n0.0\n40\n2.5\n1\nvalue\n{}100\nAcDbAttributeDefinition\n3\nPrompt\n2\n{tag}\n70\n0\n",
            0x300 + y,
            normal(n)
        )
    };
    let attrib = |tag: &str, y: i32, n: Vector3| {
        format!(
            "0\nATTRIB\n5\n{:X}\n8\n0\n100\nAcDbText\n10\n0.0\n20\n{y}.0\n30\n0.0\n40\n2.5\n1\nvalue\n{}100\nAcDbAttribute\n2\n{tag}\n70\n0\n",
            0x400 + y,
            normal(n)
        )
    };
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n\
0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n8\n0\n2\nTAGS\n70\n2\n10\n0.0\n20\n0.0\n30\n0.0\n3\nTAGS\n{}{}0\nENDBLK\n8\n0\n\
0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n\
0\nINSERT\n5\n200\n8\n0\n66\n1\n2\nTAGS\n10\n0.0\n20\n0.0\n30\n0.0\n{}{}0\nSEQEND\n5\n201\n8\n0\n\
0\nENDSEC\n0\nEOF\n",
        attdef("TILT", 1, TILTED),
        attdef("FLIP", 2, FLIPPED),
        attrib("TILT", 1, TILTED),
        attrib("FLIP", 2, FLIPPED),
    )
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

/// The normals of the ATTRIBs (TILT, FLIP) of the INSERT and of the ATTDEFs (TILT, FLIP) of the
/// block.
fn normals(doc: &CadDocument) -> ([Vector3; 2], [Vector3; 2]) {
    let by_tag = |tags: Vec<(String, Vector3)>| {
        let find = |tag: &str| {
            tags.iter()
                .find(|(t, _)| t == tag)
                .map(|(_, n)| *n)
                .unwrap_or_else(|| panic!("no {tag}"))
        };
        [find("TILT"), find("FLIP")]
    };
    let attribs = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Insert(insert) => Some(
                insert
                    .attributes
                    .iter()
                    .map(|a| (a.tag.clone(), a.normal))
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .expect("an INSERT");
    let definitions: Vec<(String, Vector3)> = doc
        .block_records
        .get("TAGS")
        .expect("block TAGS")
        .entity_handles
        .iter()
        .filter_map(|h| match doc.get_entity(*h) {
            Some(EntityType::AttributeDefinition(d)) => Some((d.tag.clone(), d.normal)),
            _ => None,
        })
        .collect();
    (by_tag(attribs), by_tag(definitions))
}

fn close(a: Vector3, b: Vector3) -> bool {
    (a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12 && (a.z - b.z).abs() < 1e-12
}

fn check(doc: &CadDocument, what: &str) {
    let (attribs, definitions) = normals(doc);
    for (found, wanted) in attribs
        .iter()
        .chain(&definitions)
        .zip([TILTED, FLIPPED, TILTED, FLIPPED])
    {
        assert!(close(*found, wanted), "{what}: {found:?} is not {wanted:?}");
    }
}

#[test]
fn the_dxf_reader_keeps_the_normals() {
    check(&read(source().into_bytes()), "DXF");
}

#[test]
fn the_normals_survive_a_dxf_write() {
    let doc = read(source().into_bytes());
    for bridge in [false, true] {
        let writer = DxfWriter::new(&doc);
        let bytes = if bridge {
            writer.for_dwg_bridge()
        } else {
            writer
        }
        .write_to_vec()
        .expect("write dxf");
        check(&read(bytes), &format!("DXF again, bridge={bridge}"));
    }
}

#[test]
fn the_normals_survive_the_dwg() {
    let doc = read(source().into_bytes());
    let bytes = DwgWriter::write_to_vec(&doc).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    check(&back, "DWG");
    // And on to a bridge DXF and a second DWG.
    let dxf = DxfWriter::new(&back)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    let again = DwgWriter::write_to_vec(&read(dxf)).expect("write dwg");
    check(
        &DwgReader::from_stream(Cursor::new(again))
            .read()
            .expect("read dwg"),
        "second DWG",
    );
}
