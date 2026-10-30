//! A solid HATCH (450 = 0) keeps the gradient fields its record holds (451, 460, 461, 452, 462, 453, the 463/63/421 colours and 470): the
//! DWG holds them whether or not the fill is a gradient, and the DXF writer left them out unless it was one. They do not turn the solid
//! hatch into a gradient.

use std::io::Cursor;

use acadrust::entities::EntityType;
use acadrust::types::Color;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const SOURCE: &str = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nHATCH\n5\n200\n100\nAcDbEntity\n8\n0\n\
100\nAcDbHatch\n10\n0\n20\n0\n30\n0\n210\n0\n220\n0\n230\n1\n2\nSOLID\n70\n1\n71\n0\n91\n1\n92\n2\n72\n0\n73\n1\n93\n4\n10\n0\n20\n0\n10\n10\n20\n0\n\
10\n10\n20\n10\n10\n0\n20\n10\n97\n0\n75\n0\n76\n1\n98\n0\n450\n0\n451\n0\n460\n0.25\n461\n0.75\n452\n1\n462\n0.6\n453\n2\n463\n0\n63\n3\n421\n66051\n\
463\n1\n63\n5\n421\n263430\n470\nSPHERICAL\n0\nENDSEC\n0\nEOF\n";

/// (enabled, solid, angle, shift, single colour, tint, [value and true colour of each colour], name).
type Held = (
    bool,
    bool,
    f64,
    f64,
    bool,
    f64,
    Vec<(f64, Option<(u8, u8, u8)>)>,
    String,
);

fn held(doc: &CadDocument) -> Held {
    let hatch = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Hatch(h) => Some(h),
            _ => None,
        })
        .expect("a hatch");
    let g = &hatch.gradient_color;
    (
        g.enabled,
        hatch.is_solid,
        g.angle,
        g.shift,
        g.is_single_color,
        g.color_tint,
        g.colors
            .iter()
            .map(|c| {
                (
                    c.value,
                    if let Color::Rgb { r, g, b } = c.color {
                        Some((r, g, b))
                    } else {
                        None
                    },
                )
            })
            .collect(),
        g.name.clone(),
    )
}

fn expected() -> Held {
    (
        false,
        true,
        0.25,
        0.75,
        true,
        0.6,
        vec![(0.0, Some((1, 2, 3))), (1.0, Some((4, 5, 6)))],
        "SPHERICAL".into(),
    )
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn the_dxf_reader_keeps_the_inactive_gradient() {
    assert_eq!(held(&read(SOURCE.as_bytes().to_vec())), expected());
}

#[test]
fn the_dxf_writer_writes_it_back_and_the_dwg_holds_it() {
    let doc = read(SOURCE.as_bytes().to_vec());
    for bridge in [false, true] {
        let writer = DxfWriter::new(&doc);
        let bytes = if bridge {
            writer.for_dwg_bridge()
        } else {
            writer
        }
        .write_to_vec()
        .expect("write dxf");
        assert_eq!(held(&read(bytes)), expected(), "bridge={bridge}");
    }
    let bytes = DwgWriter::write_to_vec(&doc).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    assert_eq!(held(&back), expected(), "DWG");
    let again = DxfWriter::new(&back)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    assert_eq!(held(&read(again)), expected(), "DWG then DXF");
}

#[test]
fn a_hatch_that_never_had_a_gradient_still_writes_none() {
    let plain = SOURCE
        .split("450\n0\n")
        .next()
        .expect("a prefix")
        .to_string()
        + "0\nENDSEC\n0\nEOF\n";
    let doc = read(plain.into_bytes());
    let text =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf-8");
    assert!(
        !text.lines().map(str::trim).any(|l| l == "450"),
        "a gradient block for a plain solid hatch"
    );
}
