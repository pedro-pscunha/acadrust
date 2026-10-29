//! $PROJECTNAME (group 1) names the project of a drawing, for its xref and image search paths.
//! The DWG header holds it (2004 and later) and the DXF reader reads it, but the DXF writer left
//! it out, so a drawing that went DWG, DXF, DWG came back without its project name.

use std::io::Cursor;

use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

#[test]
fn the_project_name_survives_dxf_to_dwg() {
    for version in [DxfVersion::AC1018, DxfVersion::AC1032] {
        let mut doc = CadDocument::with_version(version);
        doc.header.project_name = "Synthetic".to_string();
        let dxf = DxfWriter::new(&doc).write_to_vec().expect("write dxf");
        let text = String::from_utf8(dxf.clone()).expect("utf-8");
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let at = lines
            .windows(2)
            .position(|w| w[0] == "9" && w[1] == "$PROJECTNAME")
            .unwrap_or_else(|| panic!("{version:?}: no $PROJECTNAME"));
        assert_eq!(&lines[at + 2..at + 4], ["1", "Synthetic"], "{version:?}");
        let bridged = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(bridged.header.project_name, "Synthetic", "{version:?}: DXF");
        let dwg = DwgWriter::write_to_vec(&bridged).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(dwg))
            .read()
            .expect("read dwg");
        assert_eq!(back.header.project_name, "Synthetic", "{version:?}: DWG");
    }
}
