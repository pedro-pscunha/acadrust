//! The flow direction of a table style (DXF 70) survives a DWG of every version with table styles. Before R2010 it is a field of the object; from R2010
//! it is the "flow direction bottom to top" bit (0x10000) of the merge flags of the Table cell style.

use std::io::Cursor;

use acadrust::objects::{ObjectType, TableFlowDirection, TableStyle};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter};

fn plan_flow(doc: &CadDocument, version: DxfVersion) -> TableFlowDirection {
    let styles: Vec<&TableStyle> = doc
        .objects
        .values()
        .filter_map(|o| match o {
            ObjectType::TableStyle(s) => Some(s),
            _ => None,
        })
        .collect();
    let names: Vec<&str> = styles.iter().map(|s| s.name.as_str()).collect();
    styles
        .iter()
        .find(|s| s.name == "Plan")
        .unwrap_or_else(|| panic!("{version:?}: no table style Plan in {names:?}"))
        .flow_direction
}

fn document(version: DxfVersion, flow: TableFlowDirection) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let standard = doc
        .objects
        .iter()
        .find_map(|(h, o)| matches!(o, ObjectType::TableStyle(_)).then_some(*h))
        .expect("the Standard table style");
    let dict = doc
        .objects
        .iter()
        .find_map(|(h, o)| match o {
            ObjectType::Dictionary(d) if d.entries.iter().any(|(_, e)| *e == standard) => Some(*h),
            _ => None,
        })
        .expect("the table style dictionary");
    let mut style = TableStyle::standard();
    style.handle = doc.allocate_handle();
    style.owner_handle = dict;
    style.name = "Plan".to_string();
    style.flow_direction = flow;
    let handle = style.handle;
    doc.objects.insert(handle, ObjectType::TableStyle(style));
    let Some(ObjectType::Dictionary(d)) = doc.objects.get_mut(&dict) else {
        panic!("no dictionary")
    };
    d.add_entry("Plan", handle);
    doc
}

#[test]
fn the_flow_direction_of_a_table_style_survives_a_dwg_of_every_version() {
    for version in [
        DxfVersion::AC1018,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1027,
        DxfVersion::AC1032,
    ] {
        for flow in [TableFlowDirection::Up, TableFlowDirection::Down] {
            let bytes = DwgWriter::write_to_vec(&document(version, flow)).unwrap();
            let back = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
            assert_eq!(plan_flow(&back, version), flow, "{version:?}");
        }
    }
}
