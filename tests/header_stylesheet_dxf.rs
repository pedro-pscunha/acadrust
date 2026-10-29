//! $STYLESHEET (group 1, as the DXF Reference and AutoCAD have it) names the plot style table file
//! of a drawing. The DWG header holds it
//! and the DXF reader reads it, but the DXF writer left it out, so a drawing that went DWG, DXF,
//! DWG came back without its plot style table.

use std::io::Cursor;

use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

#[test]
fn the_plot_style_table_name_survives_dxf_to_dwg() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let mut doc = CadDocument::with_version(version);
        doc.header.stylesheet = "monochrome.ctb".to_string();
        let dxf = DxfWriter::new(&doc).write_to_vec().expect("write dxf");
        let text = String::from_utf8(dxf.clone()).expect("utf-8");
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let at = lines
            .windows(2)
            .position(|w| w[0] == "9" && w[1] == "$STYLESHEET")
            .unwrap_or_else(|| panic!("{version:?}: no $STYLESHEET"));
        assert_eq!(
            &lines[at + 2..at + 4],
            ["1", "monochrome.ctb"],
            "{version:?}"
        );
        let bridged = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(
            bridged.header.stylesheet, "monochrome.ctb",
            "{version:?}: DXF"
        );
        let dwg = DwgWriter::write_to_vec(&bridged).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(dwg))
            .read()
            .expect("read dwg");
        assert_eq!(back.header.stylesheet, "monochrome.ctb", "{version:?}: DWG");
    }
}

/// An earlier acadrust build wrote group 7, which AutoCAD refuses ("error 50 in the header"): the
/// reader still takes it.
#[test]
fn the_reader_takes_the_earlier_group_7_too() {
    for group in ["1", "7"] {
        let text = format!(
            "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n9\n$STYLESHEET\n{group}\nmonochrome.ctb\n0\nENDSEC\n0\nEOF\n"
        );
        let doc = DxfReader::from_reader(Cursor::new(text.into_bytes()))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(doc.header.stylesheet, "monochrome.ctb", "group {group}");
    }
}
