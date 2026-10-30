//! The values behind a table's override masks (DXF 93 to 96) survive an ASCII DXF round trip. A text DXF types groups 280 to 289 as
//! numbers, so the title and header suppression (280, 281) and the row fill switches (283) of the table-wide overrides were dropped.

use std::io::Cursor;

use acadrust::entities::table::{LegacyBorderOverrides, LegacyTableStyleOverride, Table};
use acadrust::types::{Color, DxfVersion, LineWeight, Vector3};
use acadrust::{CadDocument, DxfReader, DxfWriter, EntityType};

fn table_of(doc: &CadDocument) -> Table {
    doc.entities()
        .find_map(|e| match e {
            EntityType::Table(t) => Some((**t).clone()),
            _ => None,
        })
        .expect("a table")
}

#[test]
fn the_override_values_of_a_table_survive_a_dxf_round_trip() {
    let mut table = Table::new(Vector3::ZERO, 2, 2);
    table.override_flag = true;
    table.legacy_style_override = Some(LegacyTableStyleOverride {
        flags: 0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x100,
        title_suppressed: Some(true),
        header_suppressed: Some(true),
        flow_direction: Some(1),
        horizontal_cell_margin: Some(0.2),
        vertical_cell_margin: Some(0.3),
        row_fill_none: vec![true],
        ..LegacyTableStyleOverride::default()
    });
    table.override_border_color = true;
    table.legacy_border_colors = Some(LegacyBorderOverrides {
        flags: 3,
        values: vec![Color::Index(1), Color::Index(3)],
    });
    table.override_border_line_weight = true;
    table.legacy_border_line_weights = Some(LegacyBorderOverrides {
        flags: 1,
        values: vec![LineWeight::from_value(50)],
    });
    table.override_border_visibility = true;
    table.legacy_border_visibility = Some(LegacyBorderOverrides {
        flags: 1,
        values: vec![false],
    });
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.add_entity(EntityType::Table(Box::new(table.clone())))
        .unwrap();
    let bytes = DxfWriter::new(&doc).write_to_vec().unwrap();
    let back = table_of(
        &DxfReader::from_reader(Cursor::new(bytes))
            .unwrap()
            .read()
            .unwrap(),
    );
    assert_eq!(back.legacy_style_override, table.legacy_style_override);
    assert_eq!(back.legacy_border_colors, table.legacy_border_colors);
    assert_eq!(
        back.legacy_border_line_weights,
        table.legacy_border_line_weights
    );
    assert_eq!(
        back.legacy_border_visibility,
        table.legacy_border_visibility
    );
}
