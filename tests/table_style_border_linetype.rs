//! The borders of a table style's cell styles hold a linetype (a hard pointer). AutoCAD points
//! them at the ByBlock linetype. A DXF holds no linetype for a legacy row border, so a drawing
//! that went DWG, DXF, DWG came back with null handles, and the row borders read from a DWG
//! dropped them.

use std::io::Cursor;

use acadrust::objects::{ObjectType, TableStyle};
use acadrust::types::{DxfVersion, Handle};
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

#[test]
fn a_row_border_linetype_survives_dwg_dxf_dwg() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let continuous = linetype(&doc, "Continuous");
    style_mut(&mut doc).title_row_style.top_border.line_type = continuous;
    let read = through_dwg(&doc);
    let overrides = override_linetypes(&read);
    let title = overrides
        .iter()
        .find(|(k, _)| *k == 1)
        .expect("title override");
    assert_eq!(title.1[0], continuous, "top border of the title row, DWG");
    // A DXF names no linetype of a legacy row border: the DWG writer falls back to ByBlock.
    let again = through_dwg(&through_dxf(&read));
    let by_block = linetype(&again, "ByBlock");
    for (key, handles) in override_linetypes(&again) {
        assert_eq!(handles, vec![by_block; 6], "override {key}");
    }
}
