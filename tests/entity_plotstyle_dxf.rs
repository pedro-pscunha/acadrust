//! An entity's plot style (DXF groups 380 type and 390 PlotStyleName object) survives DXF and
//! the DXF to DWG bridge, for an ATTRIB as for any entity. The DXF writer wrote group 390 for
//! layers only and the reader had no arm for it, so every entity of a drawing that named a
//! plot style (such as Color_1) lost it.
//!
//! The Autodesk common entity group table defines 390 and no 380, so a DXF that AutoCAD opens
//! carries the plot style by its handle only. The type (380: 1 by block, 2 dictionary default,
//! 3 by object) is private to the DWG bridge writer (`DxfWriter::for_dwg_bridge`); the reader
//! takes both.

use std::io::Cursor;

use acadrust::entities::{EntityType, Insert};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, Handle};

const STYLE: u64 = 0xB0;
const OTHER: u64 = 0xB1;

/// A line and an INSERT with an attribute, each naming the plot style by handle, a line with
/// ByBlock plot style, a line with the dictionary default, and the two plot style placeholders
/// (a Color_1 and a Normal). `standard` writes only the groups the Autodesk reference defines:
/// the handles (390) and no type (380), so the ByBlock and default lines are left out.
fn source(standard: bool) -> String {
    let mut pairs: Vec<(i32, String)> = vec![
        (0, "SECTION".into()),
        (2, "HEADER".into()),
        (9, "$ACADVER".into()),
        (1, "AC1032".into()),
        (0, "ENDSEC".into()),
        (0, "SECTION".into()),
        (2, "BLOCKS".into()),
        (0, "BLOCK".into()),
        (8, "0".into()),
        (2, "LABEL".into()),
        (70, "2".into()),
        (10, "0".into()),
        (20, "0".into()),
        (3, "LABEL".into()),
        (0, "ENDBLK".into()),
        (8, "0".into()),
        (0, "ENDSEC".into()),
        (0, "SECTION".into()),
        (2, "ENTITIES".into()),
    ];
    let line = |handle: &str, end: &str, style: Vec<(i32, String)>| {
        let mut pairs = vec![
            (0, "LINE".to_string()),
            (5, handle.to_string()),
            (100, "AcDbEntity".to_string()),
            (8, "0".to_string()),
        ];
        pairs.extend(style);
        pairs.extend([
            (100, "AcDbLine".to_string()),
            (10, "0".to_string()),
            (20, "0".to_string()),
            (30, "0".to_string()),
            (11, end.to_string()),
            (21, end.to_string()),
            (31, "0".to_string()),
        ]);
        pairs
    };
    let named = |type_group: bool, style: u64| {
        let mut pairs = Vec::new();
        if type_group {
            pairs.push((380, "3".to_string()));
        }
        pairs.push((390, format!("{style:X}")));
        pairs
    };
    pairs.extend(line("100", "5", named(!standard, STYLE)));
    if !standard {
        pairs.extend(line("101", "6", vec![(380, "1".to_string())]));
        pairs.extend(line("102", "7", vec![(380, "2".to_string())]));
    }
    pairs.extend([
        (0, "INSERT".into()),
        (5, "110".into()),
        (8, "0".into()),
        (66, "1".into()),
        (2, "LABEL".into()),
        (10, "0".into()),
        (20, "0".into()),
        (0, "ATTRIB".into()),
        (5, "111".into()),
        (330, "110".into()),
        (100, "AcDbEntity".into()),
        (8, "0".into()),
    ]);
    pairs.extend(named(!standard, OTHER));
    pairs.extend([
        (100, "AcDbText".into()),
        (10, "0".into()),
        (20, "0".into()),
        (40, "1".into()),
        (1, "value".into()),
        (100, "AcDbAttribute".into()),
        (2, "TAG".into()),
        (70, "0".into()),
        (0, "SEQEND".into()),
        (5, "112".into()),
        (330, "110".into()),
        (0, "ENDSEC".into()),
        (0, "SECTION".into()),
        (2, "OBJECTS".into()),
        (0, "ACDBPLACEHOLDER".into()),
        (5, format!("{STYLE:X}")),
        (330, "C".into()),
        (0, "ACDBPLACEHOLDER".into()),
        (5, format!("{OTHER:X}")),
        (330, "C".into()),
        (0, "ENDSEC".into()),
        (0, "EOF".into()),
    ]);
    pairs
        .iter()
        .map(|(code, value)| format!("{code}\n{value}\n"))
        .collect()
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

type Style = (u8, Option<Handle>);

/// (plotstyle_flags, plotstyle_handle) of the lines at `y` (5, 6 and 7) and of the attribute.
fn styles(doc: &CadDocument) -> [Style; 4] {
    let line = |y: f64| {
        doc.entities()
            .find_map(|e| match e {
                EntityType::Line(l) if l.end.y == y => {
                    Some((l.common.plotstyle_flags, l.common.plotstyle_handle))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no line at {y}"))
    };
    let attribute = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Insert(Insert { attributes, .. }) => attributes.first(),
            _ => None,
        })
        .map(|a| (a.common.plotstyle_flags, a.common.plotstyle_handle))
        .expect("no attribute");
    [line(5.0), line(6.0), line(7.0), attribute]
}

/// Every type of plot style: by object, ByBlock, dictionary default and an attribute by object.
fn wanted() -> [Style; 4] {
    [
        (3, Some(Handle::new(STYLE))),
        (1, None),
        (2, None),
        (3, Some(Handle::new(OTHER))),
    ]
}

/// A line the standard source has no ByBlock or default twin of: only the objects keep a style.
fn wanted_named(doc: &CadDocument) -> [Style; 2] {
    let line = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Line(l) if l.end.y == 5.0 => {
                Some((l.common.plotstyle_flags, l.common.plotstyle_handle))
            }
            _ => None,
        })
        .expect("line");
    let attribute = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Insert(Insert { attributes, .. }) => attributes.first(),
            _ => None,
        })
        .map(|a| (a.common.plotstyle_flags, a.common.plotstyle_handle))
        .expect("attribute");
    [line, attribute]
}

fn named() -> [Style; 2] {
    [(3, Some(Handle::new(STYLE))), (3, Some(Handle::new(OTHER)))]
}

/// The group codes of the ENTITIES section of a DXF.
fn entity_codes(bytes: Vec<u8>) -> Vec<i32> {
    let text = String::from_utf8(bytes).expect("utf8");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut codes = Vec::new();
    let mut section = "";
    for pair in lines.chunks(2) {
        let [code, value] = pair else { break };
        let code: i32 = code.parse().expect("group code");
        if code == 2 && section == "?" {
            section = if *value == "ENTITIES" {
                "ENTITIES"
            } else {
                "other"
            };
        } else if code == 0 && *value == "SECTION" {
            section = "?";
        } else if code == 0 && *value == "ENDSEC" {
            section = "";
        } else if section == "ENTITIES" {
            codes.push(code);
        }
    }
    codes
}

#[test]
fn the_dxf_reader_takes_380_and_390() {
    assert_eq!(styles(&read(source(false).into_bytes())), wanted());
}

#[test]
fn the_dxf_reader_takes_390_alone() {
    assert_eq!(wanted_named(&read(source(true).into_bytes())), named());
}

/// The default DXF carries the plot style by its handle and no type group, so a named style
/// survives it.
#[test]
fn a_named_plot_style_survives_the_default_dxf() {
    let doc = read(source(true).into_bytes());
    let bytes = DxfWriter::new(&doc).write_to_vec().expect("write dxf");
    assert_eq!(wanted_named(&read(bytes)), named());
}

#[test]
fn the_default_dxf_has_no_plot_style_type_group_on_an_entity() {
    // ByBlock, dictionary default and named plot styles, as a DWG read gives them.
    let doc = read(source(false).into_bytes());
    assert_eq!(styles(&doc), wanted());
    let codes = entity_codes(DxfWriter::new(&doc).write_to_vec().expect("write dxf"));
    assert!(codes.contains(&390), "the named styles keep group 390");
    assert!(!codes.contains(&380), "group 380 is not an entity group");
}

#[test]
fn plot_styles_survive_the_bridge_dxf() {
    let doc = read(source(false).into_bytes());
    let bytes = DxfWriter::new(&doc)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    assert!(entity_codes(bytes.clone()).contains(&380));
    assert_eq!(styles(&read(bytes)), wanted());
}

#[test]
fn plot_styles_survive_the_dxf_to_dwg_bridge() {
    let doc = read(source(false).into_bytes());
    let bridged = read(
        DxfWriter::new(&doc)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf"),
    );
    let dwg = DwgWriter::write_to_vec(&bridged).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(dwg))
        .read()
        .expect("read dwg");
    assert_eq!(styles(&back), wanted());
}
