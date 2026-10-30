//! The style overrides of a table cell read from an R2010+ DWG are written as the cell's DXF override groups (alignment 170, content
//! colour 64, ...). The writer took the cell's override flags as the DXF bits whatever their origin, so the property flags of an
//! R2010+ cell were read as other properties (a background colour as a left border colour) and the real overrides were left out.

use std::io::Cursor;

use acadrust::entities::table::{legacy_override_properties, Table};
use acadrust::entities::CellStyle;
use acadrust::types::{Color, DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfWriter, EntityType};

#[test]
fn the_overrides_of_an_r2010_cell_style_are_written_as_dxf_override_groups() {
    let mut table = Table::new(Vector3::ZERO, 1, 1);
    table.set_cell_text(0, 0, "x");
    let mut style = CellStyle::new();
    style.override_flags = 0x01 | 0x08;
    style.legacy_override_bits = true;
    style.property_flags |= legacy_override_properties(style.override_flags);
    style.alignment = 5;
    style.content_color = Color::Index(2);
    table.rows[0].cells[0].style = Some(style);
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.add_entity(EntityType::Table(Box::new(table))).unwrap();
    let read = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    let text = String::from_utf8(DxfWriter::new(&read).write_to_vec().unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let pairs: Vec<(&str, &str)> = lines
        .chunks(2)
        .filter(|p| p.len() == 2)
        .map(|p| (p[0], p[1]))
        .collect();
    // The groups of the first cell: from its 171 on, the table being the first ACAD_TABLE.
    let table = pairs
        .iter()
        .position(|p| *p == ("0", "ACAD_TABLE"))
        .expect("a table");
    let cell = table
        + pairs[table..]
            .iter()
            .position(|p| p.0 == "171")
            .expect("a cell");
    let groups = &pairs[cell..(cell + 60).min(pairs.len())];
    assert!(groups.contains(&("170", "5")), "alignment: {groups:?}");
    assert!(groups.contains(&("64", "2")), "content colour: {groups:?}");
}
