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

/// The DXF writer keeps the name of a shape-file style as the file gave it: a named one stays named, an unnamed one stays unnamed.
#[test]
fn a_shape_style_keeps_its_name_in_a_dxf() {
    let doc = DxfReader::from_reader(Cursor::new(SOURCE.as_bytes().to_vec()))
        .unwrap()
        .read()
        .unwrap();
    let text = String::from_utf8(acadrust::DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let pairs: Vec<(&str, &str)> = lines
        .chunks(2)
        .filter(|p| p.len() == 2)
        .map(|p| (p[0], p[1]))
        .collect();
    let mut shape_names = Vec::new();
    for (i, p) in pairs.iter().enumerate() {
        if *p == ("0", "STYLE") {
            let record: Vec<&(&str, &str)> =
                pairs[i + 1..].iter().take_while(|q| q.0 != "0").collect();
            let flags: i16 = record
                .iter()
                .find(|q| q.0 == "70")
                .and_then(|q| q.1.parse().ok())
                .unwrap_or(0);
            if flags & 1 != 0 {
                shape_names.push(
                    record
                        .iter()
                        .find(|q| q.0 == "2")
                        .map(|q| q.1.to_string())
                        .unwrap_or_default(),
                );
            }
        }
    }
    shape_names.sort();
    assert_eq!(shape_names, ["".to_string(), "SourceShapes".to_string()]);
}
