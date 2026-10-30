//! The xref flags (16 dependent, 32 resolved) and names of the symbols of an xref through a DWG.
use std::io::Cursor;

use acadrust::{DwgReader, DwgWriter, DxfReader, DxfWriter};

const SYMBOLS: [(&str, &str, &str); 6] = [
    ("LTYPE", "xref|DASHED", "AcDbLinetypeTableRecord"),
    ("LAYER", "xref|Walls", "AcDbLayerTableRecord"),
    ("VIEW", "xref|Camera", "AcDbViewTableRecord"),
    ("UCS", "xref|Survey", "AcDbUCSTableRecord"),
    ("VPORT", "xref|Grid", "AcDbViewportTableRecord"),
    ("APPID", "xref|App", "AcDbRegAppTableRecord"),
];

fn source(version: &str, flags: i64) -> String {
    let mut text = format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\n{version}\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n"
    );
    for (i, (kind, name, class)) in SYMBOLS.iter().enumerate() {
        text.push_str(&format!(
            "0\nTABLE\n2\n{kind}\n70\n1\n0\n{kind}\n5\n{:X}\n100\nAcDbSymbolTableRecord\n100\n{class}\n2\n{name}\n70\n{flags}\n",
            0x50 + i
        ));
        text.push_str(match *kind {
            "LTYPE" => "3\nsource\n72\n65\n73\n0\n40\n0\n",
            "LAYER" => "62\n7\n6\nxref|DASHED\n",
            "VIEW" => "40\n10\n41\n15\n10\n1\n20\n2\n",
            "UCS" => "10\n1\n20\n2\n30\n3\n11\n1\n21\n0\n31\n0\n12\n0\n22\n1\n32\n0\n",
            "VPORT" => "10\n0\n20\n0\n11\n1\n21\n1\n12\n1\n22\n2\n40\n10\n41\n1.5\n",
            _ => "",
        });
        text.push_str("0\nENDTAB\n");
    }
    text.push_str("0\nENDSEC\n0\nEOF\n");
    text
}

fn flags_of(bytes: &[u8]) -> Vec<(String, String)> {
    let text = String::from_utf8_lossy(bytes).to_string();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let pairs: Vec<(&str, &str)> = lines
        .chunks(2)
        .filter(|p| p.len() == 2)
        .map(|p| (p[0], p[1]))
        .collect();
    let mut out = Vec::new();
    for (i, p) in pairs.iter().enumerate() {
        if p.0 == "2" && p.1.starts_with("xref|") {
            let f = pairs[i + 1..]
                .iter()
                .take_while(|q| q.0 != "0")
                .find(|q| q.0 == "70")
                .map(|q| q.1.to_string())
                .unwrap_or("-".into());
            out.push((p.1.to_string(), f));
        }
    }
    out
}

fn through_dwg(version: &str, flags: i64) -> Vec<(String, String)> {
    let doc = DxfReader::from_reader(Cursor::new(source(version, flags).into_bytes()))
        .unwrap()
        .read()
        .unwrap();
    let back = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    flags_of(&DxfWriter::new(&back).write_to_vec().unwrap())
}

fn flag(rows: &[(String, String)], name: &str) -> Option<String> {
    rows.iter().find(|(n, _)| n == name).map(|(_, f)| f.clone())
}

/// A layer or a linetype that depends on an xref keeps its resolved bit as the views do (before R2007 apart, after it as one).
#[test]
fn the_xref_flags_of_a_layer_and_a_linetype_follow_the_dwg_layout() {
    for name in ["xref|Walls", "xref|DASHED"] {
        assert_eq!(
            flag(&through_dwg("AC1018", 16), name).as_deref(),
            Some("16"),
            "{name} R2004"
        );
        assert_eq!(
            flag(&through_dwg("AC1018", 48), name).as_deref(),
            Some("48"),
            "{name} R2004"
        );
        for version in ["AC1021", "AC1032"] {
            assert_eq!(
                flag(&through_dwg(version, 48), name).as_deref(),
                Some("48"),
                "{name} {version}"
            );
        }
    }
}

/// The name of an xref-dependent application keeps its `|`, and its flags are kept as for the other symbols.
#[test]
fn an_xref_dependent_application_keeps_its_name_and_flags() {
    for flags in [16, 48] {
        let doc = DxfReader::from_reader(Cursor::new(source("AC1032", flags).into_bytes()))
            .unwrap()
            .read()
            .unwrap();
        let dxf = DxfWriter::new(&doc).write_to_vec().unwrap();
        assert_eq!(flag(&flags_of(&dxf), "xref|App"), Some(flags.to_string()));
    }
    assert_eq!(
        flag(&through_dwg("AC1018", 16), "xref|App").as_deref(),
        Some("16")
    );
    assert_eq!(
        flag(&through_dwg("AC1018", 48), "xref|App").as_deref(),
        Some("48")
    );
    for version in ["AC1021", "AC1032"] {
        assert_eq!(
            flag(&through_dwg(version, 16), "xref|App").as_deref(),
            Some("48"),
            "{version}"
        );
    }
}

/// Before R2007 a DWG holds the dependent bit and the xref index apart, so a view, UCS or viewport that depends on an xref without
/// being resolved (16) stays so. From R2007 it holds one number (0, or 256 for a resolved dependent entry; a record with another
/// value is refused by AutoCAD), so a dependent entry reads back as resolved (48).
#[test]
fn the_xref_flags_of_a_view_ucs_and_viewport_follow_the_dwg_layout() {
    for (kind, name) in [
        ("VIEW", "xref|Camera"),
        ("UCS", "xref|Survey"),
        ("VPORT", "xref|Grid"),
    ] {
        assert_eq!(
            flag(&through_dwg("AC1018", 16), name).as_deref(),
            Some("16"),
            "{kind} R2004"
        );
        assert_eq!(
            flag(&through_dwg("AC1018", 48), name).as_deref(),
            Some("48"),
            "{kind} R2004"
        );
        for version in ["AC1021", "AC1032"] {
            assert_eq!(
                flag(&through_dwg(version, 16), name).as_deref(),
                Some("48"),
                "{kind} {version}"
            );
        }
    }
}
