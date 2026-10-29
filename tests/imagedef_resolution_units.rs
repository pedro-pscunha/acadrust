//! An IMAGEDEF's resolution units (DXF group 281: 0 none, 2 centimeters, 5 inches) survive the
//! DXF reader as well as the DXF and DWG writers. The reader had no arm for 281, so an image
//! definition read from DXF always came back with no units. A code other than those three (AutoCAD
//! stores 3 for some images) is kept as stored, not read as "none".

use std::io::Cursor;

use acadrust::objects::{ImageDefinition, ObjectType, ResolutionUnit};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

/// The resolution unit codes of the document's image definitions, by file name.
fn units(doc: &CadDocument) -> Vec<i32> {
    let mut found: Vec<(String, i32)> = doc
        .objects
        .values()
        .filter_map(|o| match o {
            ObjectType::ImageDefinition(d) => {
                Some((d.file_name.clone(), d.resolution_unit.to_code()))
            }
            _ => None,
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found.into_iter().map(|(_, u)| u).collect()
}

fn with_definitions() -> CadDocument {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    for (name, unit) in [
        ("a.png", ResolutionUnit::Centimeters),
        ("b.png", ResolutionUnit::Inches),
        ("c.png", ResolutionUnit::None),
        ("d.png", ResolutionUnit::from_code(3)),
    ] {
        let handle = doc.allocate_handle();
        let mut def = ImageDefinition::new(name);
        def.handle = handle;
        def.owner = doc.header.named_objects_dict_handle;
        def.size_in_pixels = (100, 50);
        def.pixel_size = (0.01, 0.01);
        def.resolution_unit = unit;
        doc.objects.insert(handle, ObjectType::ImageDefinition(def));
    }
    doc
}

fn wanted() -> [i32; 4] {
    [2, 5, 0, 3]
}

#[test]
fn the_dxf_reader_takes_group_281() {
    let text = "  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  1\nAC1015\n  0\nENDSEC\n\
  0\nSECTION\n  2\nOBJECTS\n\
  0\nIMAGEDEF\n  5\nB0\n330\nC\n100\nAcDbRasterImageDef\n 90\n     0\n  1\na.png\n 10\n100.0\n 20\n50.0\n 11\n0.01\n 21\n0.01\n280\n     1\n281\n     2\n\
  0\nIMAGEDEF\n  5\nB1\n330\nC\n100\nAcDbRasterImageDef\n 90\n     0\n  1\nb.png\n 10\n100.0\n 20\n50.0\n 11\n0.01\n 21\n0.01\n280\n     1\n281\n     5\n\
  0\nIMAGEDEF\n  5\nB2\n330\nC\n100\nAcDbRasterImageDef\n 90\n     0\n  1\nc.png\n 10\n100.0\n 20\n50.0\n 11\n0.01\n 21\n0.01\n280\n     1\n281\n     3\n\
  0\nENDSEC\n  0\nEOF\n";
    let doc = DxfReader::from_reader(Cursor::new(text.as_bytes().to_vec()))
        .expect("reader")
        .read()
        .expect("read");
    assert_eq!(units(&doc), [2, 5, 3]);
    assert_eq!(ResolutionUnit::from_code(3).to_code(), 3);
}

#[test]
fn units_survive_dxf() {
    let bytes = DxfWriter::new(&with_definitions())
        .write_to_vec()
        .expect("write dxf");
    let back = DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf");
    assert_eq!(units(&back), wanted());
}

#[test]
fn units_survive_dwg() {
    let bytes = DwgWriter::write_to_vec(&with_definitions()).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    assert_eq!(units(&back), wanted());
}

#[test]
fn units_survive_the_dxf_to_dwg_bridge() {
    let dxf = DxfWriter::new(&with_definitions())
        .write_to_vec()
        .expect("write dxf");
    let doc = DxfReader::from_reader(Cursor::new(dxf))
        .expect("reader")
        .read()
        .expect("read dxf");
    let bytes = DwgWriter::write_to_vec(&doc).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    assert_eq!(units(&back), wanted());
}
