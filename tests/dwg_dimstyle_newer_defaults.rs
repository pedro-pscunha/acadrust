//! A DWG before 2007 has no DIMFXL or DIMJOGANG in its dimension styles: a style read from it
//! gets AutoCAD's defaults (1.0 and 45 degrees), as `DimStyle::new` has them. The reader left
//! them at zero, so the DXF writer wrote 49 = 0 and a jog angle of 5 degrees (its clamp of 0).

use acadrust::tables::DimStyle;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, TableEntry};

fn read_back(version: DxfVersion) -> DimStyle {
    let mut doc = CadDocument::with_version(version);
    let mut dim = DimStyle::new("MyDim");
    dim.set_handle(doc.allocate_handle());
    doc.dim_styles.add(dim).unwrap();
    let bytes = DwgWriter::write_to_vec(&doc).expect("write dwg");
    let loaded = DwgReader::from_stream(std::io::Cursor::new(bytes)).read().expect("read dwg");
    let style = loaded.dim_styles.iter().find(|s| s.name == "MyDim").expect("MyDim").clone();
    style
}

#[test]
fn a_dwg_before_2007_gives_the_newer_style_fields_their_defaults() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1018] {
        let style = read_back(version);
        assert_eq!(style.dimjogang, std::f64::consts::FRAC_PI_4, "{version:?} DIMJOGANG");
        assert_eq!(style.dimfxl, 1.0, "{version:?} DIMFXL");
    }
}

#[test]
fn a_dwg_from_2007_keeps_the_stored_values() {
    let style = read_back(DxfVersion::AC1021);
    assert_eq!(style.dimjogang, std::f64::consts::FRAC_PI_4);
    assert_eq!(style.dimfxl, 1.0);
}
