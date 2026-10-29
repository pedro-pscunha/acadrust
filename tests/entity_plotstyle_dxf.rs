//! An entity's plot style (DXF groups 380 type and 390 PlotStyleName object) survives DXF and
//! the DXF to DWG bridge, for an ATTRIB as for any entity. The DXF writer wrote group 390 for
//! layers only and the reader had no arm for it, so every entity of a drawing that named a
//! plot style (such as Color_1) lost it.

use std::io::Cursor;

use acadrust::entities::{EntityType, Insert};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, Handle};

const STYLE: u64 = 0xB0;
const OTHER: u64 = 0xB1;

/// A line and an INSERT with an attribute, each naming the plot style by handle, a line with
/// ByBlock plot style, and the two plot style placeholders (a Color_1 and a Normal).
fn source() -> String {
    let pairs: Vec<(i32, String)> = vec![
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
        (0, "LINE".into()),
        (5, "100".into()),
        (100, "AcDbEntity".into()),
        (8, "0".into()),
        (380, "3".into()),
        (390, format!("{STYLE:X}")),
        (100, "AcDbLine".into()),
        (10, "0".into()),
        (20, "0".into()),
        (30, "0".into()),
        (11, "5".into()),
        (21, "5".into()),
        (31, "0".into()),
        (0, "LINE".into()),
        (5, "101".into()),
        (100, "AcDbEntity".into()),
        (8, "0".into()),
        (380, "1".into()),
        (100, "AcDbLine".into()),
        (10, "0".into()),
        (20, "0".into()),
        (30, "0".into()),
        (11, "6".into()),
        (21, "6".into()),
        (31, "0".into()),
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
        (380, "3".into()),
        (390, format!("{OTHER:X}")),
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
    ];
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

/// (plotstyle_flags, plotstyle_handle) of the line at `y` and of the attribute.
fn styles(doc: &CadDocument) -> [(u8, Option<Handle>); 3] {
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
    [line(5.0), line(6.0), attribute]
}

fn wanted() -> [(u8, Option<Handle>); 3] {
    [
        (3, Some(Handle::new(STYLE))),
        (1, None),
        (3, Some(Handle::new(OTHER))),
    ]
}

#[test]
fn the_dxf_reader_takes_380_and_390() {
    assert_eq!(styles(&read(source().into_bytes())), wanted());
}

#[test]
fn plot_styles_survive_dxf() {
    let doc = read(source().into_bytes());
    let again = read(DxfWriter::new(&doc).write_to_vec().expect("write dxf"));
    assert_eq!(styles(&again), wanted());
}

#[test]
fn plot_styles_survive_the_dxf_to_dwg_bridge() {
    let doc = read(source().into_bytes());
    let bridged = read(DxfWriter::new(&doc).write_to_vec().expect("write dxf"));
    let dwg = DwgWriter::write_to_vec(&bridged).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(dwg))
        .read()
        .expect("read dwg");
    assert_eq!(styles(&back), wanted());
}
