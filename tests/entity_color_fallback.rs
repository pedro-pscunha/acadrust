//! The ACI index next to a true colour (DXF 62 beside 420) is kept for entities: the DWG entity colour word holds it beside the RGB
//! (ODA specification 2.11, "flags + color index"). A true colour without an index gets the nearest one in a DXF, as before.

use std::io::Cursor;

use acadrust::entities::Line;
use acadrust::types::{Color, DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};

fn line_dxf(groups: &str) -> Vec<u8> {
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n5\n30\n8\n0\n{groups}10\n0\n20\n0\n30\n0\n11\n1\n21\n1\n31\n0\n0\nENDSEC\n0\nEOF\n"
    )
    .into_bytes()
}

fn first_line(doc: &CadDocument) -> acadrust::entities::EntityCommon {
    doc.entities()
        .find_map(|e| match e {
            EntityType::Line(l) => Some(l.common.clone()),
            _ => None,
        })
        .expect("a line")
}

fn read_dxf(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap()
}

fn group_62(doc: &CadDocument) -> String {
    let text = String::from_utf8(DxfWriter::new(doc).write_to_vec().unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let pairs: Vec<(&str, &str)> = lines
        .chunks(2)
        .filter(|p| p.len() == 2)
        .map(|p| (p[0], p[1]))
        .collect();
    let at = pairs
        .iter()
        .position(|p| *p == ("0", "LINE"))
        .expect("a line");
    pairs[at..]
        .iter()
        .take_while(|p| p.0 != "0" || p.1 == "LINE")
        .find(|p| p.0 == "62")
        .map(|p| p.1.to_string())
        .unwrap_or_default()
}

#[test]
fn an_index_beside_a_true_colour_survives_a_dxf_and_a_dwg() {
    // 62 = 7 beside 420 = 0x112233 (the nearest index is 158 or so), in both orders.
    for groups in ["62\n7\n420\n1122867\n", "420\n1122867\n62\n7\n"] {
        let doc = read_dxf(line_dxf(groups));
        let common = first_line(&doc);
        assert_eq!(
            (common.color, common.color_fallback),
            (
                Color::Rgb {
                    r: 0x11,
                    g: 0x22,
                    b: 0x33
                },
                Some(7)
            ),
            "{groups:?}"
        );
        assert_eq!(group_62(&doc), "7", "{groups:?}");
        let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        let common = first_line(&dwg);
        assert_eq!(
            (common.color, common.color_fallback),
            (
                Color::Rgb {
                    r: 0x11,
                    g: 0x22,
                    b: 0x33
                },
                Some(7)
            ),
            "{groups:?} DWG"
        );
        assert_eq!(group_62(&dwg), "7", "{groups:?} DWG");
        let again = read_dxf(DxfWriter::new(&dwg).write_to_vec().unwrap());
        assert_eq!(
            first_line(&again).color_fallback,
            Some(7),
            "{groups:?} DXF again"
        );
    }
}

#[test]
fn a_true_colour_without_an_index_gets_the_nearest_one() {
    let mut line = Line::from_points(Vector3::ZERO, Vector3::new(1.0, 1.0, 0.0));
    line.common.color = Color::Rgb {
        r: 0x11,
        g: 0x22,
        b: 0x33,
    };
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.add_entity(EntityType::Line(line)).unwrap();
    let nearest = Color::Rgb {
        r: 0x11,
        g: 0x22,
        b: 0x33,
    }
    .approximate_index()
    .to_string();
    assert_eq!(group_62(&doc), nearest);
    let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    assert_eq!(first_line(&dwg).color_fallback, None);
    assert_eq!(group_62(&dwg), nearest);
}
