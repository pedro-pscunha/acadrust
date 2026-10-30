//! A named STYLE that describes a shape file (70 bit 1) keeps its name through a DWG.
use std::io::Cursor;

use acadrust::{DwgReader, DwgWriter, DxfReader};

const SOURCE: &str = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nSTYLE\n70\n1\n0\nSTYLE\n5\n60\n100\nAcDbSymbolTableRecord\n100\nAcDbTextStyleTableRecord\n2\nSourceShapes\n70\n5\n40\n0\n41\n1\n50\n0\n71\n0\n42\n12.75\n3\nsource_shapes.shx\n4\n\n0\nSTYLE\n5\n61\n100\nAcDbSymbolTableRecord\n100\nAcDbTextStyleTableRecord\n2\n\n70\n1\n40\n0\n41\n1\n50\n0\n71\n0\n42\n0.2\n3\nltypeshp.shx\n4\n\n0\nENDTAB\n0\nENDSEC\n0\nEOF\n";

#[test]
fn a_named_shape_style_and_the_unnamed_one_keep_their_fields_through_a_dwg() {
    let doc = DxfReader::from_reader(Cursor::new(SOURCE.as_bytes().to_vec()))
        .unwrap()
        .read()
        .unwrap();
    let fields = |doc: &acadrust::CadDocument| {
        doc.text_styles
            .iter()
            .map(|s| {
                (
                    s.name.clone(),
                    s.is_shape_file,
                    s.is_vertical,
                    s.last_height,
                )
            })
            .collect::<Vec<_>>()
    };
    let want = vec![
        ("Standard".to_string(), false, false, 2.5),
        ("SourceShapes".to_string(), true, true, 12.75),
        (String::new(), true, false, 0.2),
    ];
    assert_eq!(fields(&doc), want);
    let back = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    assert_eq!(fields(&back), want);
}
