//! The current multileader style (CMLEADERSTYLE) has no field in the DWG header stream: AutoCAD keeps it as the DICTIONARYVAR
//! `CMLEADERSTYLE` of the variable dictionary (a DWG of 2004 or later; a DWG whose header handle slot after the visual style
//! dictionary is null, as AutoCAD 2018's is). A read gives it to the header, and a DWG save writes the header's name back.

use std::io::Cursor;

use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn dwg_round_trip(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read")
}

#[test]
fn a_dwg_save_writes_the_name_of_the_header_in_the_variable_dictionary() {
    for version in [
        DxfVersion::AC1018,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1027,
        DxfVersion::AC1032,
    ] {
        let mut document = CadDocument::with_version(version);
        document.header.current_mleader_style_name = "Callout".into();
        let back = dwg_round_trip(&document);
        assert_eq!(
            back.drawing_variable("CMLEADERSTYLE"),
            Some("Callout"),
            "{version:?}"
        );
        assert_eq!(
            back.header.current_mleader_style_name, "Callout",
            "{version:?}"
        );
    }
}

#[test]
fn a_changed_header_name_replaces_the_value_of_the_variable() {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    assert!(document.set_drawing_variable("CMLEADERSTYLE", "Callout"));
    document.header.current_mleader_style_name = "Notes".into();
    let back = dwg_round_trip(&document);
    assert_eq!(back.drawing_variable("CMLEADERSTYLE"), Some("Notes"));
    assert_eq!(back.header.current_mleader_style_name, "Notes");
}

#[test]
fn the_standard_style_keeps_the_spelling_of_the_variable() {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    assert!(document.set_drawing_variable("CMLEADERSTYLE", "STANDARD"));
    let back = dwg_round_trip(&document);
    assert_eq!(back.drawing_variable("CMLEADERSTYLE"), Some("STANDARD"));
    assert_eq!(back.header.current_mleader_style_name, "Standard");
}

#[test]
fn a_default_name_creates_no_variable() {
    let document = CadDocument::with_version(DxfVersion::AC1032);
    let back = dwg_round_trip(&document);
    assert_eq!(back.drawing_variable("CMLEADERSTYLE"), None);
}

#[test]
fn a_release_without_the_variable_dictionary_writes_nothing_new() {
    let mut document = CadDocument::with_version(DxfVersion::AC1015);
    document.header.current_mleader_style_name = "Callout".into();
    let back = dwg_round_trip(&document);
    assert_eq!(back.drawing_variable("CMLEADERSTYLE"), None);
}

#[test]
fn a_dxf_without_the_header_variable_gets_the_name_from_the_variable_dictionary() {
    // AutoCAD's own DXF has no $CMLEADERSTYLE; the DICTIONARYVAR is the only place of the name.
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    assert!(document.set_drawing_variable("CMLEADERSTYLE", "Callout"));
    let text = DxfWriter::new(&document).write_to_vec().expect("write");
    assert!(!String::from_utf8_lossy(&text).contains("$CMLEADERSTYLE"));
    let back = DxfReader::from_reader(Cursor::new(text))
        .expect("open")
        .read()
        .expect("read");
    assert_eq!(back.header.current_mleader_style_name, "Callout");
}

/// The Citycon drawing AutoCAD 2010 saved with the multileader style `Linha_MEDIO` current.
#[test]
#[ignore = "needs /tmp/cad/rt7/Citycon-Augusto_Miele.dwg (an AutoCAD file)"]
fn an_autocad_dwg_gives_its_current_multileader_style() {
    let bytes = std::fs::read("/tmp/cad/rt7/Citycon-Augusto_Miele.dwg").expect("the AutoCAD DWG");
    let document = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read");
    assert_eq!(document.version, DxfVersion::AC1024);
    assert_eq!(
        document.drawing_variable("CMLEADERSTYLE"),
        Some("Linha_MEDIO")
    );
    assert_eq!(document.header.current_mleader_style_name, "Linha_MEDIO");
    let again = dwg_round_trip(&document);
    assert_eq!(again.header.current_mleader_style_name, "Linha_MEDIO");
}
