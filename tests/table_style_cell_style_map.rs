//! A table style's ACAD_ROUNDTRIP_2008_TABLESTYLE_CELLSTYLEMAP survives a DWG save of AutoCAD 2010
//! and later while it still matches the style. An AutoCAD 2018 drawing that went through the
//! 2007 format keeps it on its table styles (the sample drawings: 2 of 2 on one drawing, none on
//! the five that never did), and AutoCAD keeps it on every later save; the DWG writer used to
//! delete it. A map that no longer matches the style is still dropped.

use std::io::Cursor;

use acadrust::objects::{CellStyleMap, DataObject, DataObjectData, ObjectType, TableStyle};
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter};

const KEY: &str = "ACAD_ROUNDTRIP_2008_TABLESTYLE_CELLSTYLEMAP";

fn standard_style(doc: &CadDocument) -> &TableStyle {
    doc.objects
        .values()
        .find_map(|o| match o {
            ObjectType::TableStyle(s) if s.name == "Standard" => Some(s),
            _ => None,
        })
        .expect("Standard table style")
}

/// A drawing read from a DWG whose Standard table style carries the cell style map AutoCAD left.
fn with_map(mutate: impl FnOnce(&mut CellStyleMap)) -> CadDocument {
    let bytes =
        DwgWriter::write_to_vec(&CadDocument::with_version(DxfVersion::AC1032)).expect("write");
    let mut doc = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read");
    let style = standard_style(&doc);
    let handle = style.handle;
    let mut map = CellStyleMap {
        cells: style
            .modern_overrides
            .iter()
            .map(|(_, c)| c.clone())
            .collect(),
    };
    assert!(!map.cells.is_empty(), "the style has cell styles to mirror");
    mutate(&mut map);
    let dictionary = doc.ensure_extension_dictionary(handle);
    let mut object = DataObject::new(DataObjectData::CellStyleMap(map));
    object.handle = doc.allocate_handle();
    object.owner = dictionary;
    object.reactors = vec![dictionary];
    let map_handle = object.handle;
    doc.objects
        .insert(map_handle, ObjectType::DataObject(object));
    let Some(ObjectType::Dictionary(entries)) = doc.objects.get_mut(&dictionary) else {
        panic!("no extension dictionary")
    };
    entries.add_entry(KEY, map_handle);
    doc
}

fn saved_map(doc: &CadDocument) -> Option<CellStyleMap> {
    let bytes = DwgWriter::write_to_vec(doc).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let dictionary = back.extension_dictionary_handle(standard_style(&back).handle)?;
    let ObjectType::Dictionary(entries) = back.objects.get(&dictionary)? else {
        return None;
    };
    let handle: Handle = entries.get(KEY)?;
    match back.objects.get(&handle)? {
        ObjectType::DataObject(DataObject {
            data: DataObjectData::CellStyleMap(map),
            ..
        }) => Some(map.clone()),
        _ => None,
    }
}

#[test]
fn a_matching_map_is_kept_on_an_r2010_plus_save() {
    let doc = with_map(|_| {});
    let expected = standard_style(&doc).modern_overrides.len();
    let map = saved_map(&doc).expect("the cell style map was dropped");
    assert_eq!(map.cells.len(), expected);
}

#[test]
fn a_map_that_no_longer_matches_the_style_is_dropped() {
    let doc = with_map(|map| map.cells[0].name = "Edited elsewhere".to_string());
    assert!(saved_map(&doc).is_none());
}

/// The base "Table" cell style of a table style AutoCAD wrote has the six default borders: all
/// hidden, by block, 0.045 apart. A table style that did not come from a DWG (a DXF has no form
/// for that cell style) is written with them too, not with none.
#[test]
fn the_default_base_cell_style_has_autocads_six_borders() {
    let bytes =
        DwgWriter::write_to_vec(&CadDocument::with_version(DxfVersion::AC1032)).expect("write");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read");
    let style = standard_style(&back)
        .modern_style
        .as_ref()
        .expect("the base cell style");
    let borders = &style.cell_style.borders;
    assert_eq!(
        borders.iter().map(|b| b.index_mask).collect::<Vec<_>>(),
        [1, 2, 4, 8, 16, 32]
    );
    for border in borders {
        assert!(border.border.is_invisible);
        assert_eq!(border.border.color, acadrust::types::Color::ByBlock);
        assert_eq!(
            border.border.line_weight,
            acadrust::types::LineWeight::ByBlock
        );
        assert_eq!(border.border.double_line_spacing, 0.045);
        assert!(border.line_type.is_null());
    }
}
