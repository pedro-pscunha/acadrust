//! DIMCLRD, DIMCLRE, DIMCLRT and DIMTFILLCLR true colors. A DWG of AutoCAD 2004 and later keeps a
//! true color per dimension style; AutoCAD's DXF DIMSTYLE record has only the ACI groups
//! (176, 177, 178, 70), so a true color came back from DXF as its nearest ACI color. acadrust
//! writes private groups next to them for DXF R2004 and later (the DWG's CMC colors exist from
//! then): 421 DIMCLRD, 422 DIMCLRE, 423 DIMCLRT, 424 DIMTFILLCLR, each a packed 0xRRGGBB like
//! group 420. They are private, so only the DWG bridge writer (`DxfWriter::for_dwg_bridge`)
//! writes them: a DXF that AutoCAD opens holds only the groups the DXF Reference defines.

use std::io::Cursor;

use acadrust::tables::DimStyle;
use acadrust::types::{Color, DxfVersion};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const D: Color = Color::Rgb {
    r: 187,
    g: 130,
    b: 157,
};
const E: Color = Color::Rgb { r: 1, g: 2, b: 3 };
const T: Color = Color::Rgb {
    r: 250,
    g: 200,
    b: 10,
};
const FILL: Color = Color::Rgb {
    r: 12,
    g: 99,
    b: 255,
};

fn styled(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let mut style = DimStyle::new("Tint");
    style.dimclrd = D.approximate_index();
    style.dimclrd_true_color = Some(D);
    style.dimclre = E.approximate_index();
    style.dimclre_true_color = Some(E);
    style.dimclrt = T.approximate_index();
    style.dimclrt_true_color = Some(T);
    style.dimtfill = 2;
    style.dimtfillclr = FILL.approximate_index();
    style.dimtfillclr_true_color = Some(FILL);
    // An ACI-colored style stays ACI.
    let mut plain = DimStyle::new("Plain");
    plain.dimclrd = 3;
    plain.dimclre = 4;
    plain.dimclrt = 5;
    doc.dim_styles.add(style).expect("add style");
    doc.dim_styles.add(plain).expect("add style");
    doc
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

/// DIMTFILLCLR is a DIMSTYLE field of DWG R2007 and later only.
fn check(doc: &CadDocument, version: DxfVersion, what: &str) {
    let s = doc
        .dim_styles
        .get("Tint")
        .unwrap_or_else(|| panic!("{what}: style lost"));
    assert_eq!(s.dimclrd_true_color, Some(D), "{what}: DIMCLRD");
    assert_eq!(s.dimclre_true_color, Some(E), "{what}: DIMCLRE");
    assert_eq!(s.dimclrt_true_color, Some(T), "{what}: DIMCLRT");
    if version >= DxfVersion::AC1021 {
        assert_eq!(s.dimtfillclr_true_color, Some(FILL), "{what}: DIMTFILLCLR");
        assert_eq!(
            s.dimtfillclr,
            FILL.approximate_index(),
            "{what}: DIMTFILLCLR ACI"
        );
    }
    assert_eq!(s.dimclrd, D.approximate_index(), "{what}: DIMCLRD ACI");
    let p = doc
        .dim_styles
        .get("Plain")
        .unwrap_or_else(|| panic!("{what}: Plain lost"));
    assert_eq!(
        (p.dimclrd, p.dimclre, p.dimclrt),
        (3, 4, 5),
        "{what}: Plain ACI"
    );
    assert_eq!(
        (
            p.dimclrd_true_color,
            p.dimclre_true_color,
            p.dimclrt_true_color,
            p.dimtfillclr_true_color
        ),
        (None, None, None, None),
        "{what}: Plain true colors"
    );
}

#[test]
fn true_colors_survive_dxf() {
    for version in [DxfVersion::AC1018, DxfVersion::AC1024, DxfVersion::AC1032] {
        check(
            &through_dxf(&styled(version)),
            version,
            &format!("{version:?}"),
        );
    }
}

#[test]
fn true_colors_survive_dxf_to_dwg_to_dxf() {
    for version in [DxfVersion::AC1018, DxfVersion::AC1032] {
        let bridged = through_dxf(&styled(version));
        let dwg = through_dwg(&bridged);
        check(&dwg, version, &format!("{version:?}: DWG"));
        check(
            &through_dxf(&dwg),
            version,
            &format!("{version:?}: DXF again"),
        );
    }
}

/// The group codes of every DIMSTYLE record in `bytes`.
fn dimstyle_codes(bytes: Vec<u8>) -> Vec<Vec<i32>> {
    let text = String::from_utf8(bytes).expect("utf8");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut records: Vec<Vec<i32>> = Vec::new();
    let mut in_style = false;
    for pair in lines.chunks(2) {
        let [code, value] = pair else { break };
        let code: i32 = code.parse().expect("group code");
        if code == 0 {
            in_style = *value == "DIMSTYLE";
            if in_style {
                records.push(Vec::new());
            }
        } else if in_style {
            records.last_mut().expect("record").push(code);
        }
    }
    records
}

#[test]
fn the_default_dxf_gets_no_private_groups() {
    for version in [
        DxfVersion::AC1015,
        DxfVersion::AC1018,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1032,
    ] {
        let bytes = DxfWriter::new(&styled(version))
            .write_to_vec()
            .expect("write");
        let records = dimstyle_codes(bytes);
        assert!(records.len() >= 2, "{version:?}: no DIMSTYLE records");
        for record in records {
            assert!(
                record.iter().all(|c| !matches!(*c, 421 | 422 | 423 | 424)),
                "{version:?}: {record:?}"
            );
        }
    }
}

#[test]
fn the_bridge_dxf_writes_the_true_color_groups() {
    let bytes = DxfWriter::new(&styled(DxfVersion::AC1032))
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write");
    let tint = dimstyle_codes(bytes)
        .into_iter()
        .find(|r| r.contains(&421))
        .expect("a record with the true colors");
    assert!([421, 422, 423, 424].iter().all(|c| tint.contains(c)));
}
