//! DIMPOST keeps its stored value through DXF (an empty prefix/suffix is not the default "<>"),
//! and DIMTXTDIRECTION is DIMSTYLE group 294, the code AutoCAD uses in a DIMSTYLE record and in
//! DSTYLE overrides; files older acadrust wrote it as 295, which the reader still takes.

use std::io::Cursor;

use acadrust::tables::DimStyle;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc).write_to_vec().expect("write dxf");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

/// The (code, value) pairs of the DIMSTYLE record called `name` in DXF `text`.
fn record(text: &str, name: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let pairs: Vec<(String, String)> = lines
        .chunks(2)
        .map(|p| {
            (
                p[0].trim().to_string(),
                p.get(1).map_or(String::new(), |v| v.trim().to_string()),
            )
        })
        .collect();
    let at = pairs
        .iter()
        .position(|(c, v)| c == "2" && v == name)
        .unwrap_or_else(|| panic!("no style {name}"));
    pairs[at..]
        .iter()
        .take_while(|(c, _)| c != "0")
        .cloned()
        .collect()
}

/// `text` with the group `from` of the DIMSTYLE record `name` written as `to`.
fn recode(text: &str, name: &str, from: &str, to: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut inside = false;
    for pair in lines.chunks(2) {
        let (code, value) = (pair[0].trim(), pair.get(1).map_or("", |v| v.trim()));
        if code == "0" {
            inside = false;
        }
        if code == "2" && value == name {
            inside = true;
        }
        out.push(if inside && code == from {
            format!("{to:>3}")
        } else {
            pair[0].to_string()
        });
        out.extend(pair.get(1).map(|v| v.to_string()));
    }
    out.join("\r\n")
}

#[test]
fn dimpost_keeps_its_stored_value() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let mut doc = CadDocument::with_version(version);
        for (name, post) in [("Empty", ""), ("Default", "<>"), ("Suffix", "<> mm")] {
            let mut style = DimStyle::new(name);
            style.dimpost = post.to_string();
            doc.dim_styles.add(style).expect("add style");
        }
        let back = through_dxf(&doc);
        for (name, post) in [("Empty", ""), ("Default", "<>"), ("Suffix", "<> mm")] {
            let style = back
                .dim_styles
                .get(name)
                .unwrap_or_else(|| panic!("{version:?}: {name} lost"));
            assert_eq!(style.dimpost, post, "{version:?}: {name}");
        }
    }
}

#[test]
fn dimtxtdirection_is_group_294() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut style = DimStyle::new("Flip");
    style.dimtxtdirection = true;
    doc.dim_styles.add(style).expect("add style");
    let text =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf8");
    let record = record(&text, "Flip");
    assert!(
        record.contains(&("294".to_string(), "1".to_string())),
        "no group 294: {record:?}"
    );
    assert!(
        record.iter().all(|(c, _)| c != "295"),
        "group 295 written: {record:?}"
    );
    assert!(
        through_dxf(&doc)
            .dim_styles
            .get("Flip")
            .expect("style")
            .dimtxtdirection
    );
}

#[test]
fn dimtxtdirection_reads_294_and_295_and_reaches_the_dwg() {
    let doc = {
        let mut doc = CadDocument::with_version(DxfVersion::AC1032);
        let mut style = DimStyle::new("Flip");
        style.dimtxtdirection = true;
        doc.dim_styles.add(style).expect("add style");
        doc
    };
    let text =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf8");
    for code in ["294", "295"] {
        // The same file with DIMTXTDIRECTION at `code`.
        let swapped = recode(&text, "Flip", "294", code);
        assert!(record(&swapped, "Flip").contains(&(code.to_string(), "1".to_string())));
        let read = DxfReader::from_reader(Cursor::new(swapped.into_bytes()))
            .expect("reader")
            .read()
            .expect("read");
        assert!(
            read.dim_styles.get("Flip").expect("style").dimtxtdirection,
            "group {code}"
        );
        let bytes = DwgWriter::write_to_vec(&read).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read dwg");
        assert!(
            back.dim_styles.get("Flip").expect("style").dimtxtdirection,
            "DWG, group {code}"
        );
    }
}

#[test]
fn the_header_keeps_dimtxtdirection() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.header.dim_text_direction = true;
    assert!(through_dxf(&doc).header.dim_text_direction);
}
