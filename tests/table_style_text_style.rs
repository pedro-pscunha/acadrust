//! The text style that a TABLESTYLE cell style names (group 7 of the data, title and header
//! cell styles) survives DXF to DWG to DXF, by handle and by name.

use std::io::Cursor;

use acadrust::objects::{ObjectType, TableStyle};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

/// A drawing with the text styles Standard and Notes, whose table style Header cell style uses
/// Notes (the groups of a cell style in the order data, title, header).
fn source(version: &str) -> String {
    let cell = |style: &str, height: &str| {
        format!(
            "7\n{style}\n140\n{height}\n170\n5\n62\n0\n63\n7\n283\n0\n90\n512\n91\n0\n1\n\n\
274\n-2\n284\n1\n64\n0\n275\n-2\n285\n1\n65\n0\n276\n-2\n286\n1\n66\n0\n\
277\n-2\n287\n1\n67\n0\n278\n-2\n288\n1\n68\n0\n279\n-2\n289\n1\n69\n0\n"
        )
    };
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\n{version}\n0\nENDSEC\n\
0\nSECTION\n2\nTABLES\n\
0\nTABLE\n2\nSTYLE\n5\n3\n100\nAcDbSymbolTable\n70\n2\n\
0\nSTYLE\n5\n11\n100\nAcDbSymbolTableRecord\n100\nAcDbTextStyleTableRecord\n2\nStandard\n70\n0\n40\n0.0\n41\n1.0\n50\n0.0\n71\n0\n42\n2.5\n3\ntxt\n4\n\n\
0\nSTYLE\n5\n12\n100\nAcDbSymbolTableRecord\n100\nAcDbTextStyleTableRecord\n2\nNotes\n70\n0\n40\n0.0\n41\n1.0\n50\n0.0\n71\n0\n42\n2.5\n3\ntxt\n4\n\n\
0\nENDTAB\n0\nENDSEC\n\
0\nSECTION\n2\nOBJECTS\n\
0\nDICTIONARY\n5\nC\n330\n0\n100\nAcDbDictionary\n281\n1\n3\nACAD_TABLESTYLE\n350\n20\n\
0\nDICTIONARY\n5\n20\n330\nC\n100\nAcDbDictionary\n281\n1\n3\nSchedule\n350\n21\n\
0\nTABLESTYLE\n5\n21\n330\n20\n100\nAcDbTableStyle\n280\n0\n3\nSchedule\n70\n0\n71\n0\n40\n0.06\n41\n0.06\n280\n0\n281\n0\n{}{}{}\
0\nENDSEC\n0\nEOF\n",
        cell("Standard", "0.18"),
        cell("Standard", "0.25"),
        cell("Notes", "0.30"),
    )
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn style(doc: &CadDocument) -> &TableStyle {
    doc.objects
        .values()
        .find_map(|o| match o {
            ObjectType::TableStyle(s) if s.name == "Schedule" => Some(s),
            _ => None,
        })
        .expect("Schedule table style")
}

/// (name, handle) of the text style that the Header cell style of "Schedule" uses.
fn header_text_style(doc: &CadDocument) -> (String, u64) {
    let row = &style(doc).header_row_style;
    let handle = row.text_style_handle.map_or(0, |h| h.value());
    (row.text_style_name.clone(), handle)
}

fn notes_handle(doc: &CadDocument) -> u64 {
    doc.text_styles.get("Notes").expect("Notes").handle.value()
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn the_dxf_reader_keeps_the_text_style_of_a_cell_style() {
    let doc = read(source("AC1032").into_bytes());
    assert_eq!(header_text_style(&doc).0, "Notes");
    assert_eq!(style(&doc).data_row_style.text_style_name, "Standard");
}

#[test]
fn the_text_style_of_a_cell_style_survives_dxf_to_dwg_to_dxf() {
    for version in ["AC1018", "AC1021", "AC1024", "AC1032"] {
        let doc = read(source(version).into_bytes());
        let back = through_dwg(&doc);
        let (name, handle) = header_text_style(&back);
        assert_eq!(name, "Notes", "{version}: DWG name");
        assert_eq!(handle, notes_handle(&back), "{version}: DWG handle");
        assert_eq!(
            style(&back).data_row_style.text_style_name,
            "Standard",
            "{version}: data row"
        );
        // And on to a bridge DXF and a second DWG.
        let dxf = DxfWriter::new(&back)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf");
        let again = through_dwg(&read(dxf));
        let (name, handle) = header_text_style(&again);
        assert_eq!(name, "Notes", "{version}: second DWG name");
        assert_eq!(handle, notes_handle(&again), "{version}: second DWG handle");
    }
}

#[test]
fn the_dwg_version_is_the_one_asked() {
    assert_eq!(
        read(source("AC1032").into_bytes()).version,
        DxfVersion::AC1032
    );
}

#[test]
fn a_style_known_by_name_only_gets_its_handle_in_a_dwg() {
    // Built in code: the cell style has the name, not the handle.
    for version in [DxfVersion::AC1018, DxfVersion::AC1032] {
        let mut doc = CadDocument::with_version(version);
        let mut notes = acadrust::tables::TextStyle::new("Notes");
        notes.handle = doc.allocate_handle();
        doc.text_styles.add(notes).expect("add style");
        for object in doc.objects.values_mut() {
            if let ObjectType::TableStyle(style) = object {
                style.header_row_style.text_style_name = "Notes".to_string();
                style.header_row_style.text_style_handle = None;
            }
        }
        let back = through_dwg(&doc);
        let row = back
            .objects
            .values()
            .find_map(|o| match o {
                ObjectType::TableStyle(s) if s.name == "Standard" => Some(&s.header_row_style),
                _ => None,
            })
            .expect("Standard table style");
        assert_eq!(row.text_style_name, "Notes", "{version:?}");
        assert_eq!(
            row.text_style_handle.map(|h| h.value()),
            Some(notes_handle(&back)),
            "{version:?}"
        );
    }
}
