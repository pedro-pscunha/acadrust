//! A DIMENSION's presentation groups survive the DXF reader, the DWG and the DXF writer: the dimension version (280, held by an R2010 or
//! later DWG as the byte before the common data), the undocumented group 73 and the flipped arrows 74 and 75 (held by an R2007 or later
//! DWG), and the attachment point (71), line spacing style (72) and factor (41) (held by an R2000 or later DWG). An older release has no
//! place for what a later one added, so those groups come back as their defaults there: the format rule, not a loss.

use std::io::Cursor;

use acadrust::entities::EntityType;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const SOURCE: &str = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nDIMENSION\n5\n200\n100\nAcDbEntity\n8\n0\n\
100\nAcDbDimension\n280\n1\n10\n0\n20\n3\n30\n0\n11\n5\n21\n3\n31\n0\n70\n32\n71\n9\n72\n2\n41\n1.375\n73\n1\n74\n1\n75\n1\n\
100\nAcDbAlignedDimension\n13\n0\n23\n0\n33\n0\n14\n10\n24\n0\n34\n0\n50\n0\n100\nAcDbRotatedDimension\n0\nENDSEC\n0\nEOF\n";

/// (version 280, undocumented 73, flip arrow 1, flip arrow 2, attachment point 71, line spacing style 72, line spacing factor 41).
type Fields = (u8, bool, bool, bool, i16, i16, f64);

fn fields(doc: &CadDocument) -> Fields {
    let dimension = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Dimension(d) => Some(d),
            _ => None,
        })
        .expect("a dimension");
    let base = dimension.base();
    (
        base.version,
        base.dwg_unknown_bit,
        base.flip_arrow1,
        base.flip_arrow2,
        base.attachment_point as i16,
        base.line_spacing_style,
        base.line_spacing_factor,
    )
}

fn read_dxf(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn through_dwg(doc: &CadDocument, version: DxfVersion) -> CadDocument {
    let mut copy = doc.clone();
    copy.version = version;
    let bytes = DwgWriter::write_to_vec(&copy).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn the_dxf_reader_keeps_the_presentation_groups() {
    assert_eq!(
        fields(&read_dxf(SOURCE.as_bytes().to_vec())),
        (1, true, true, true, 9, 2, 1.375)
    );
}

#[test]
fn each_dwg_release_holds_what_its_layout_has_a_place_for() {
    let doc = read_dxf(SOURCE.as_bytes().to_vec());
    // R2010 and later: all of them.
    for version in [DxfVersion::AC1024, DxfVersion::AC1032] {
        assert_eq!(
            fields(&through_dwg(&doc, version)),
            (1, true, true, true, 9, 2, 1.375),
            "{version:?}"
        );
    }
    // R2007: no version byte, the rest.
    assert_eq!(
        fields(&through_dwg(&doc, DxfVersion::AC1021)),
        (0, true, true, true, 9, 2, 1.375)
    );
    // R2000 and R2004: the attachment point and the line spacing only.
    for version in [DxfVersion::AC1015, DxfVersion::AC1018] {
        assert_eq!(
            fields(&through_dwg(&doc, version)),
            (0, false, false, false, 9, 2, 1.375),
            "{version:?}"
        );
    }
}

#[test]
fn the_dxf_writer_writes_the_groups_its_release_has() {
    let doc = read_dxf(SOURCE.as_bytes().to_vec());
    let groups = |version: DxfVersion, bridge: bool| {
        let mut copy = doc.clone();
        copy.version = version;
        let writer = DxfWriter::new(&copy).with_version(version);
        let bytes = if bridge {
            writer.for_dwg_bridge()
        } else {
            writer
        }
        .write_to_vec()
        .expect("write dxf");
        let text = String::from_utf8(bytes).expect("utf-8");
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let start = lines
            .windows(2)
            .position(|w| w == ["0", "DIMENSION"])
            .expect("a DIMENSION");
        let end = start
            + 2
            + lines[start + 2..]
                .chunks(2)
                .position(|w| w[0] == "0")
                .map_or(lines.len(), |p| p * 2);
        let record = &lines[start + 2..end];
        let value = |code: &str| {
            record
                .chunks(2)
                .find(|w| w[0] == code)
                .map(|w| w[1].to_string())
        };
        ["280", "73", "74", "75"].map(value)
    };
    let some = |v: &str| Some(v.to_string());
    assert_eq!(
        groups(DxfVersion::AC1032, false),
        [some("1"), some("1"), some("1"), some("1")]
    );
    assert_eq!(
        groups(DxfVersion::AC1021, false),
        [None, some("1"), some("1"), some("1")]
    );
    assert_eq!(groups(DxfVersion::AC1015, false), [None, None, None, None]);
    // A bridge carries what the DWG of its document holds.
    assert_eq!(
        groups(DxfVersion::AC1032, true),
        [some("1"), some("1"), some("1"), some("1")]
    );
}

#[test]
fn a_dwg_saved_as_dxf_keeps_them_again() {
    let doc = through_dwg(&read_dxf(SOURCE.as_bytes().to_vec()), DxfVersion::AC1032);
    let bytes = DxfWriter::new(&doc)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    assert_eq!(fields(&read_dxf(bytes)), (1, true, true, true, 9, 2, 1.375));
}
