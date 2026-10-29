//! The borders of a table style's cell styles hold a linetype (a hard pointer). AutoCAD points
//! them at the ByBlock linetype. A DXF holds no linetype for a legacy row border, so a drawing
//! that went DWG, DXF, DWG came back with null handles, and the row borders read from a DWG
//! dropped them.
//!
//! The DXF Reference has no TABLESTYLE group for a border's linetype or its override flags, so the
//! DWG bridge (`DxfWriter::for_dwg_bridge`) writes private groups for them (340 to 345, the
//! linetype of each border, and 94 to 99, its override flags) and a normal DXF does not.

use std::io::Cursor;

use acadrust::objects::{ObjectType, TableBorderPropertyFlags, TableStyle};
use acadrust::tables::LineType;
use acadrust::types::{Color, DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn style_mut(doc: &mut CadDocument) -> &mut TableStyle {
    doc.objects
        .values_mut()
        .find_map(|o| match o {
            ObjectType::TableStyle(s) if s.name == "Standard" => Some(s),
            _ => None,
        })
        .expect("Standard table style")
}

fn style(doc: &CadDocument) -> &TableStyle {
    doc.objects
        .values()
        .find_map(|o| match o {
            ObjectType::TableStyle(s) if s.name == "Standard" => Some(s),
            _ => None,
        })
        .expect("Standard table style")
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let dwg = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(dwg))
        .read()
        .expect("read dwg")
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let dxf = DxfWriter::new(doc).write_to_vec().expect("write dxf");
    DxfReader::from_reader(Cursor::new(dxf))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn through_bridge(doc: &CadDocument) -> CadDocument {
    let dxf = DxfWriter::new(doc)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    DxfReader::from_reader(Cursor::new(dxf))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn linetype(doc: &CadDocument, name: &str) -> Handle {
    doc.line_types.get(name).expect(name).handle
}

/// The linetypes of the six borders of each cell style override, by override key.
fn override_linetypes(doc: &CadDocument) -> Vec<(i32, Vec<Handle>)> {
    style(doc)
        .modern_overrides
        .iter()
        .map(|(key, s)| {
            (
                *key,
                s.cell_style.borders.iter().map(|g| g.line_type).collect(),
            )
        })
        .collect()
}

#[test]
fn a_dwg_written_from_a_style_without_linetypes_points_its_borders_at_byblock() {
    let back = through_dwg(&CadDocument::with_version(DxfVersion::AC1032));
    let by_block = linetype(&back, "ByBlock");
    let all = override_linetypes(&back);
    assert_eq!(all.len(), 3, "title, header and data overrides");
    for (key, handles) in all {
        assert_eq!(handles, vec![by_block; 6], "override {key}");
    }
}

#[test]
fn the_borders_of_a_dwg_style_keep_their_linetype_in_the_row_styles() {
    let back = through_dwg(&CadDocument::with_version(DxfVersion::AC1032));
    let by_block = linetype(&back, "ByBlock");
    let s = style(&back);
    assert_eq!(s.title_row_style.top_border.line_type, by_block);
    assert_eq!(s.data_row_style.right_border.line_type, by_block);
}

/// The name of the linetype `handle` is in `doc`.
fn linetype_name(doc: &CadDocument, handle: Handle) -> String {
    doc.line_types
        .iter()
        .find(|line_type| line_type.handle == handle)
        .map_or_else(|| format!("no linetype {handle:?}"), |l| l.name.clone())
}

/// The linetype (by name) and override flags of the top border of the title row override.
fn title_top(doc: &CadDocument) -> (String, TableBorderPropertyFlags) {
    let (_, title) = style(doc)
        .modern_overrides
        .iter()
        .find(|(key, _)| *key == 1)
        .expect("title override");
    let grid = &title.cell_style.borders[0];
    (
        linetype_name(doc, grid.line_type),
        grid.border.property_flags,
    )
}

/// A drawing whose title row has a dashed top border with the linetype override.
fn with_dashed_border() -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut dashed = LineType::dashed();
    dashed.handle = doc.allocate_handle();
    let handle = dashed.handle;
    doc.line_types.add(dashed).expect("add linetype");
    let border = &mut style_mut(&mut doc).title_row_style.top_border;
    border.line_type = handle;
    border.property_flags = TableBorderPropertyFlags::LINE_TYPE;
    doc
}

#[test]
fn a_dwg_keeps_a_dashed_border() {
    let read = through_dwg(&with_dashed_border());
    assert_eq!(
        title_top(&read),
        ("Dashed".to_string(), TableBorderPropertyFlags::LINE_TYPE)
    );
}

#[test]
fn a_border_linetype_and_its_override_survive_the_bridge() {
    let read = through_dwg(&with_dashed_border());
    let again = through_dwg(&through_bridge(&read));
    assert_eq!(
        title_top(&again),
        ("Dashed".to_string(), TableBorderPropertyFlags::LINE_TYPE)
    );
    // The other borders stay by block, with no override.
    let by_block = linetype(&again, "ByBlock");
    let (_, title) = style(&again)
        .modern_overrides
        .iter()
        .find(|(key, _)| *key == 1)
        .expect("title override");
    for grid in &title.cell_style.borders[1..] {
        assert_eq!(grid.line_type, by_block);
    }
}

#[test]
fn a_border_linetype_follows_a_renumbered_linetype() {
    let read = through_dwg(&with_dashed_border());
    let dashed = linetype(&read, "Dashed");
    let dxf = String::from_utf8(
        DxfWriter::new(&read)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf"),
    )
    .expect("utf8");
    // A host that renumbers handles changes the LTYPE record and every pointer to it.
    let moved = format!("{:X}", dashed.value() + 0x6000);
    let from = format!("{:X}", dashed.value());
    let lines: Vec<&str> = dxf.lines().collect();
    let mut out = Vec::new();
    for pair in lines.chunks(2) {
        let code: i32 = pair[0].trim().parse().unwrap_or(0);
        let pointer = matches!(code, 5 | 330 | 340..=369 | 390);
        out.push(pair[0].to_string());
        if let Some(value) = pair.get(1) {
            out.push(if pointer && value.trim() == from {
                moved.clone()
            } else {
                value.to_string()
            });
        }
    }
    let back = DxfReader::from_reader(Cursor::new(out.join("\r\n").into_bytes()))
        .expect("reader")
        .read()
        .expect("read dxf");
    assert_eq!(
        linetype(&back, "Dashed").value(),
        dashed.value() + 0x6000,
        "the host moved the linetype"
    );
    assert_eq!(title_top(&through_dwg(&back)).0, "Dashed");
}

/// The group codes of every TABLESTYLE object of a DXF.
fn table_style_codes(bytes: Vec<u8>) -> Vec<Vec<i32>> {
    let text = String::from_utf8(bytes).expect("utf8");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut records: Vec<Vec<i32>> = Vec::new();
    let mut in_style = false;
    for pair in lines.chunks(2) {
        let [code, value] = pair else { break };
        let code: i32 = code.parse().expect("group code");
        if code == 0 {
            in_style = *value == "TABLESTYLE";
            if in_style {
                records.push(Vec::new());
            }
        } else if in_style {
            records.last_mut().expect("record").push(code);
        }
    }
    records
}

#[test]
fn the_default_dxf_has_no_private_table_style_groups() {
    let read = through_dwg(&with_dashed_border());
    let default = table_style_codes(DxfWriter::new(&read).write_to_vec().expect("write dxf"));
    assert!(!default.is_empty(), "no TABLESTYLE");
    for record in &default {
        assert!(
            record.iter().all(|c| !matches!(*c, 94..=99 | 340..=345)),
            "{record:?}"
        );
    }
    let bridge = table_style_codes(
        DxfWriter::new(&read)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf"),
    );
    assert!(bridge.iter().any(|r| r.contains(&340) && r.contains(&94)));
    // A default DXF read again and written again stays free of them too.
    let again = table_style_codes(
        DxfWriter::new(&through_bridge(&read))
            .write_to_vec()
            .expect("write dxf"),
    );
    for record in &again {
        assert!(
            record.iter().all(|c| !matches!(*c, 94..=99 | 340..=345)),
            "replayed: {record:?}"
        );
    }
}

#[test]
fn a_default_dxf_still_gives_by_block_borders() {
    let read = through_dwg(&with_dashed_border());
    let again = through_dwg(&through_dxf(&read));
    let by_block = linetype(&again, "ByBlock");
    for (key, handles) in override_linetypes(&again) {
        assert_eq!(handles, vec![by_block; 6], "override {key}");
    }
}

// ── true colors ────────────────────────────────────────────────────────────

const TEXT: Color = Color::Rgb {
    r: 187,
    g: 130,
    b: 157,
};
const FILL: Color = Color::Rgb {
    r: 12,
    g: 99,
    b: 255,
};
const BORDER: Color = Color::Rgb {
    r: 250,
    g: 200,
    b: 10,
};

/// A drawing whose title row has a true text color, fill color and top border color.
fn with_true_colors() -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let row = &mut style_mut(&mut doc).title_row_style;
    row.text_color = TEXT;
    row.fill_color = FILL;
    row.top_border.color = BORDER;
    doc
}

fn title_colors(doc: &CadDocument) -> (Color, Color, Color) {
    let row = &style(doc).title_row_style;
    (row.text_color, row.fill_color, row.top_border.color)
}

fn private_color_groups(bytes: Vec<u8>) -> Vec<Vec<i32>> {
    table_style_codes(bytes)
        .into_iter()
        .map(|record| {
            record
                .into_iter()
                .filter(|code| (420..=427).contains(code))
                .collect()
        })
        .collect()
}

#[test]
fn the_default_dxf_has_no_true_color_groups_in_a_table_style() {
    let doc = with_true_colors();
    let records = private_color_groups(DxfWriter::new(&doc).write_to_vec().expect("write dxf"));
    assert!(!records.is_empty(), "no TABLESTYLE");
    for record in records {
        assert!(record.is_empty(), "{record:?}");
    }
    // The ACI colors stay, as the nearest index of each.
    let back = through_dxf(&doc);
    assert_eq!(
        title_colors(&back),
        (
            Color::from_index(TEXT.approximate_index()),
            Color::from_index(FILL.approximate_index()),
            Color::from_index(BORDER.approximate_index())
        )
    );
}

#[test]
fn the_bridge_dxf_keeps_the_true_colors_of_a_table_style() {
    let doc = with_true_colors();
    let bytes = DxfWriter::new(&doc)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    assert!(private_color_groups(bytes).iter().any(|r| r.len() == 3));
    assert_eq!(title_colors(&through_bridge(&doc)), (TEXT, FILL, BORDER));
}

#[test]
fn a_style_read_from_a_bridge_has_no_true_color_groups_in_a_default_dxf() {
    // The bridge style is written again from its raw groups.
    let read = through_bridge(&with_true_colors());
    assert_eq!(title_colors(&read), (TEXT, FILL, BORDER));
    let records = private_color_groups(DxfWriter::new(&read).write_to_vec().expect("write dxf"));
    assert!(!records.is_empty(), "no TABLESTYLE");
    for record in records {
        assert!(record.is_empty(), "{record:?}");
    }
}
