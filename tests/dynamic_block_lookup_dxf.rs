//! The look-up parameter and the look-up grip of a dynamic block. AutoCAD writes their subclass
//! markers with a capital U (`AcDbBlockLookUpParameter`, `AcDbBlockLookUpGrip`; the action's is
//! `AcDbBlockLookupAction`, and the C++ class names in the CLASSES section have the small u) and
//! the groups of the parameter in the order 303, 304, 94. A marker with another spelling makes
//! AutoCAD refuse the DXF ("a class separator is expected for the class
//! AcDbBlockLookUpParameter"). The reader takes both spellings and both orders.
//!
//! The two connections of an action are written as AutoCAD writes them: the code of the first and
//! its name, then the code of the second and its name (92, 301, 93, 302).

use std::io::Cursor;

use acadrust::objects::{DynamicBlockData, ObjectType};
use acadrust::{CadDocument, DxfReader, DxfWriter};

/// A BLOCKLOOKUPPARAMETER and a BLOCKLOOKUPGRIP in the layout of an AutoCAD DXF, with `marker` the
/// spelling of the markers' "look-up" word and `parameter_groups` the groups of the parameter.
fn source(look_up: &str, parameter_groups: &str) -> String {
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n\
0\nSECTION\n2\nOBJECTS\n\
0\nBLOCKLOOKUPPARAMETER\n5\n8A1\n330\n8A0\n100\nAcDbEvalExpr\n90\n16\n98\n33\n99\n328\n\
100\nAcDbBlockElement\n300\nLookup\n98\n33\n99\n328\n1071\n0\n\
100\nAcDbBlockParameter\n280\n1\n281\n0\n\
100\nAcDbBlock1PtParameter\n1010\n1.5\n1020\n2.5\n1030\n0.0\n93\n17\n170\n0\n171\n0\n\
100\nAcDbBlock{look_up}Parameter\n{parameter_groups}\
0\nBLOCKLOOKUPGRIP\n5\n8A2\n330\n8A0\n100\nAcDbEvalExpr\n90\n17\n98\n33\n99\n328\n\
100\nAcDbBlockElement\n300\nGrip\n98\n33\n99\n328\n1071\n0\n\
100\nAcDbBlockGrip\n91\n18\n92\n19\n1010\n1.5\n1020\n2.5\n1030\n0.0\n280\n0\n93\n-1\n\
100\nAcDbBlock{look_up}Grip\n\
0\nENDSEC\n0\nEOF\n"
    )
}

const AUTOCAD_ORDER: &str = "303\nNAME\n304\nDescription\n94\n7\n";
const EARLIER_ORDER: &str = "94\n7\n303\nNAME\n304\nDescription\n";

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

/// (index, name, description) of the look-up parameter and whether the grip is there.
fn lookup(doc: &CadDocument) -> (Option<(i32, String, String)>, bool) {
    let mut parameter = None;
    let mut grip = false;
    for object in doc.objects.values() {
        if let ObjectType::DynamicBlock(object) = object {
            match &object.data {
                DynamicBlockData::LookupParameter(p) => {
                    parameter =
                        Some((p.index, p.lookup_name.clone(), p.lookup_description.clone()));
                }
                DynamicBlockData::LookupGrip(_) => grip = true,
                _ => {}
            }
        }
    }
    (parameter, grip)
}

fn wanted() -> (Option<(i32, String, String)>, bool) {
    (
        Some((7, "NAME".to_string(), "Description".to_string())),
        true,
    )
}

/// The group codes after the subclass marker `marker` of the first record that has it.
fn codes_after(text: &str, marker: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let at = lines
        .windows(2)
        .position(|w| w[0] == "100" && w[1] == marker)
        .unwrap_or_else(|| panic!("no {marker}"));
    lines[at + 2..]
        .chunks(2)
        .take_while(|p| p[0] != "0" && p[0] != "100")
        .map(|p| p[0].to_string())
        .collect()
}

#[test]
fn the_reader_takes_both_spellings_and_orders() {
    for (look_up, groups) in [
        ("LookUp", AUTOCAD_ORDER),
        ("Lookup", AUTOCAD_ORDER),
        ("LookUp", EARLIER_ORDER),
        ("Lookup", EARLIER_ORDER),
    ] {
        let doc = read(source(look_up, groups).into_bytes());
        assert_eq!(lookup(&doc), wanted(), "{look_up} / {groups:?}");
    }
}

#[test]
fn the_writer_spells_the_markers_and_orders_the_groups_as_autocad_does() {
    let doc = read(source("LookUp", AUTOCAD_ORDER).into_bytes());
    for bridge in [false, true] {
        let writer = DxfWriter::new(&doc);
        let bytes = if bridge {
            writer.for_dwg_bridge()
        } else {
            writer
        }
        .write_to_vec()
        .expect("write dxf");
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        // The C++ class names in the CLASSES section keep the small u; the subclass markers
        // (group 100) do not.
        let markers: Vec<&str> = text
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[0] == "100")
            .map(|w| w[1])
            .collect();
        assert!(
            !markers.contains(&"AcDbBlockLookupParameter")
                && !markers.contains(&"AcDbBlockLookupGrip"),
            "bridge={bridge}"
        );
        assert_eq!(
            codes_after(&text, "AcDbBlockLookUpParameter"),
            ["303", "304", "94"],
            "bridge={bridge}"
        );
        assert!(markers.contains(&"AcDbBlockLookUpGrip"), "bridge={bridge}");
        assert_eq!(lookup(&read(bytes)), wanted(), "bridge={bridge}");
    }
}

/// A BLOCKSTRETCHACTION with two connections, in the layout of an AutoCAD DXF.
fn stretch_source() -> String {
    "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n\
0\nSECTION\n2\nOBJECTS\n\
0\nBLOCKSTRETCHACTION\n5\n8B1\n330\n8A0\n100\nAcDbEvalExpr\n90\n20\n98\n33\n99\n328\n\
100\nAcDbBlockElement\n300\nStretch\n98\n33\n99\n328\n1071\n0\n\
100\nAcDbBlockAction\n70\n0\n71\n0\n1010\n0.0\n1020\n0.0\n1030\n0.0\n\
100\nAcDbBlockStretchAction\n92\n5\n301\nFirst\n93\n6\n302\nSecond\n72\n0\n73\n0\n75\n0\n140\n1.0\n141\n0.0\n280\n0\n\
0\nENDSEC\n0\nEOF\n"
        .to_string()
}

fn connections(doc: &CadDocument) -> Vec<(i32, String)> {
    doc.objects
        .values()
        .find_map(|object| match object {
            ObjectType::DynamicBlock(object) => match &object.data {
                DynamicBlockData::StretchAction(a) => Some(
                    a.connections
                        .iter()
                        .map(|c| (c.code, c.name.clone()))
                        .collect(),
                ),
                _ => None,
            },
            _ => None,
        })
        .expect("a stretch action")
}

#[test]
fn the_connections_of_an_action_are_written_code_then_name() {
    let doc = read(stretch_source().into_bytes());
    let wanted = vec![(5, "First".to_string()), (6, "Second".to_string())];
    assert_eq!(connections(&doc), wanted);
    let bytes = DxfWriter::new(&doc).write_to_vec().expect("write dxf");
    let text = String::from_utf8(bytes.clone()).expect("utf8");
    let codes = codes_after(&text, "AcDbBlockStretchAction");
    assert_eq!(&codes[..4], ["92", "301", "93", "302"], "{codes:?}");
    assert_eq!(connections(&read(bytes)), wanted);
}
