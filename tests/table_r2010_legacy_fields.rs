//! The legacy fields of an ACAD_TABLE (DXF 90, the table-wide overrides 93, the cell rotation 145, auto-fit 174 and the cell edge
//! flags 172) survive a DWG of R2010 and later, where the table is stored as table content (ODA specification 20.4.96.2): the value
//! flags are the short after the content, the rotation and auto-scale are properties of the cell style, the margins and the flow
//! direction are properties of the table's base cell style, and the edges with overrides are the borders of the cell style.

use std::io::Cursor;

use acadrust::entities::table::{LegacyTableStyleOverride, Table};
use acadrust::types::{Color, DxfVersion, LineWeight, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};

fn table_of(doc: &CadDocument) -> Table {
    doc.entities()
        .find_map(|entity| match entity {
            EntityType::Table(table) => Some((**table).clone()),
            _ => None,
        })
        .expect("a table")
}

/// A table as a DXF holds it, every legacy field at a non-default value.
fn from_dxf(version: DxfVersion) -> CadDocument {
    let mut table = Table::new(Vector3::ZERO, 2, 2);
    table.set_cell_text(0, 0, "a");
    table.set_cell_text(0, 1, "b");
    table.set_cell_text(1, 0, "c");
    table.set_cell_text(1, 1, "d");
    table.value_flags = 6;
    table.override_flag = true;
    table.legacy_style_override = Some(LegacyTableStyleOverride {
        flags: 0x4 | 0x8 | 0x10,
        flow_direction: Some(1),
        horizontal_cell_margin: Some(0.2),
        vertical_cell_margin: Some(0.3),
        ..LegacyTableStyleOverride::default()
    });
    for (row, cells) in table.rows.iter_mut().enumerate() {
        for (column, cell) in cells.cells.iter_mut().enumerate() {
            cell.rotation = 0.25 * (1 + row * 2 + column) as f64;
            cell.auto_fit = (row + column) % 2 == 0;
            cell.edge_flags = 1;
            let mut style = acadrust::entities::CellStyle::new();
            style.override_flags = 0x40 | 0x400;
            style.property_flags |=
                acadrust::entities::table::legacy_override_properties(style.override_flags);
            style.legacy_override_bits = true;
            style.top_border.color = Color::Index(1);
            style.top_border.line_weight = LineWeight::from_value(50);
            cell.style = Some(style);
        }
    }
    let mut doc = CadDocument::with_version(version);
    doc.add_entity(EntityType::Table(Box::new(table))).unwrap();
    let bytes = DxfWriter::new(&doc).write_to_vec().unwrap();
    DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap()
}

type Fields = (
    i32,
    Option<(i32, Option<i16>, Option<f64>, Option<f64>)>,
    Vec<(f64, bool, u8)>,
);

fn fields(table: &Table) -> Fields {
    (
        table.value_flags,
        table.legacy_style_override.as_ref().map(|o| {
            (
                o.flags & (0x4 | 0x8 | 0x10),
                o.flow_direction,
                o.horizontal_cell_margin,
                o.vertical_cell_margin,
            )
        }),
        table
            .rows
            .iter()
            .flat_map(|row| row.cells.iter())
            .map(|cell| (cell.rotation, cell.auto_fit, cell.edge_flags))
            .collect(),
    )
}

#[test]
fn the_legacy_fields_of_a_table_survive_a_dwg_from_r2010() {
    for version in [DxfVersion::AC1024, DxfVersion::AC1027, DxfVersion::AC1032] {
        let source = fields(&table_of(&from_dxf(version)));
        assert_eq!(source.0, 6);
        let doc = from_dxf(version);
        let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        assert_eq!(fields(&table_of(&dwg)), source, "{version:?}");
        // And a second DWG of that.
        let again = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&dwg).unwrap()))
            .read()
            .unwrap();
        assert_eq!(fields(&table_of(&again)), source, "{version:?} twice");
    }
}

/// A table with a block cell as a DXF holds it: type 2, the block record (340), the scale (144) and an attribute value.
fn block_cell_from_dxf(version: DxfVersion) -> CadDocument {
    use acadrust::entities::table::{CellContent, CellType, TableAttribute, TableCellContentType};
    let mut doc = CadDocument::with_version(version);
    let mut record = acadrust::tables::BlockRecord::new("Logo");
    record.handle = doc.allocate_handle();
    let block = record.handle;
    doc.block_records.add(record).unwrap();
    let mut table = Table::new(Vector3::ZERO, 1, 2);
    table.set_cell_text(0, 0, "a");
    let cell = &mut table.rows[0].cells[1];
    cell.cell_type = CellType::Block;
    cell.block_scale = 2.0;
    let mut content = CellContent::new();
    content.content_type = TableCellContentType::Block;
    content.block_handle = Some(block);
    content.attributes.push(TableAttribute {
        definition_handle: acadrust::Handle::new(0x4202),
        value: "Acme".into(),
        index: 1,
    });
    cell.contents = vec![content];
    doc.add_entity(EntityType::Table(Box::new(table))).unwrap();
    let bytes = DxfWriter::new(&doc).write_to_vec().unwrap();
    DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap()
}

#[test]
fn a_block_cell_survives_a_dwg_from_r2010() {
    use acadrust::entities::table::CellType;
    for version in [
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1027,
        DxfVersion::AC1032,
    ] {
        let doc = block_cell_from_dxf(version);
        let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        let cell = &table_of(&dwg).rows[0].cells[1];
        assert_eq!(cell.cell_type, CellType::Block, "{version:?}");
        assert_eq!(cell.block_scale, 2.0, "{version:?}");
        let content = &cell.contents[0];
        assert!(content.block_handle.is_some(), "{version:?}");
        assert_eq!(content.attributes[0].value, "Acme", "{version:?}");
    }
}
