//! The MTEXT background groups of a DXF, as AutoCAD writes them (its DXFOUT of DP Inc and Planeta-Toulouse, 167 records):
//! with fill flag 1 (the fill's own colour) the colour index 63 (the nearest index of a true colour), the true colour 421 when
//! there is one, the box scale 45 and the transparency 441; with flag 2 alone (the drawing window colour) nothing after 90.
//! AutoCAD LT 2025 discards a DXF that has another form ("Não foi possível analisar o marcador de objeto incorporado",
//! "Falta código de grupo DXF: 90"). The DWG stores the fill the same way (only with bit 1, or bit 0x10 from R2018).

use acadrust::entities::MText;
use acadrust::types::{Color, DxfVersion, Vector3};
use acadrust::{CadDocument, DxfWriter, EntityType};

/// The groups after group 90 of the MTEXT, up to the next one that is no background group.
fn background(flags: i32, color: Color) -> Vec<(i32, String)> {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut mtext = MText::with_value("Fill", Vector3::new(0.0, 0.0, 0.0));
    mtext.background_fill_flags = flags;
    mtext.background_scale = 1.5;
    mtext.background_color = color;
    doc.add_entity(EntityType::MText(Box::new(mtext))).unwrap();
    let text = String::from_utf8(DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let pairs: Vec<(i32, String)> = lines
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[c, v]| (c.trim().parse().unwrap_or(-1), v.trim().to_string()))
        .collect();
    let start = pairs.iter().position(|(c, v)| *c == 0 && v == "MTEXT").unwrap();
    let rest = &pairs[start + 1..];
    let at = rest.iter().position(|(c, _)| *c == 90).unwrap();
    rest[at + 1..]
        .iter()
        .take_while(|(c, _)| matches!(c, 63 | 421 | 45 | 441))
        .cloned()
        .collect()
}

#[test]
fn a_fill_with_its_own_true_colour_writes_index_colour_scale_and_transparency() {
    let groups = background(1, Color::Rgb { r: 200, g: 200, b: 200 });
    let codes: Vec<i32> = groups.iter().map(|(c, _)| *c).collect();
    assert_eq!(codes, [63, 421, 45, 441]);
    assert_eq!(groups[0].1, "9", "the nearest index colour of RGB 200,200,200");
    assert_eq!(groups[1].1, "13158600");
}

#[test]
fn a_fill_with_an_index_colour_writes_no_true_colour() {
    let codes: Vec<i32> = background(3, Color::from_index(5)).iter().map(|(c, _)| *c).collect();
    assert_eq!(codes, [63, 45, 441]);
}

#[test]
fn a_fill_with_the_window_colour_writes_nothing_after_its_flags() {
    assert_eq!(background(2, Color::from_index(5)), []);
}
