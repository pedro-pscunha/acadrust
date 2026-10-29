//! Reform a single DXF record from the layout of one release to the layout of another.

use std::io::Cursor;

use crate::io::dxf::writer::{DxfTextWriter, SectionWriter};
use crate::io::dxf::DxfReader;
use crate::types::DxfVersion;

/// The records whose DXF form depends on the release, and the section that holds them.
fn section_of(kind: &str) -> Option<&'static str> {
    match kind {
        // Objects: the visual style has three layouts, the constraint group two, and the
        // history steps carry embedded DWG entities encoded with the layout of a release.
        "VISUALSTYLE"
        | "ACDBASSOC2DCONSTRAINTGROUP"
        | "ACSH_SWEEP_CLASS"
        | "ACSH_EXTRUSION_CLASS"
        | "ACSH_LOFT_CLASS"
        | "ACSH_REVOLVE_CLASS" => Some("OBJECTS"),
        // Entities with embedded DWG entities.
        "EXTRUDEDSURFACE" | "LOFTEDSURFACE" | "REVOLVEDSURFACE" | "SWEPTSURFACE" => {
            Some("ENTITIES")
        }
        _ => None,
    }
}

/// Reform one raw DXF record (`tags`: every group, `0 <kind>` first) written for release `from`
/// into the form of release `to`: the record is read with the reader of `from` and written with
/// the writer of `to`.
///
/// It covers the kinds whose DXF form depends on the release: `VISUALSTYLE` (three layouts),
/// `ACDBASSOC2DCONSTRAINTGROUP` (inline nodes below R2013, a registry from R2013), and the
/// records with embedded DWG entities (`ACSH_SWEEP_CLASS`, `ACSH_EXTRUSION_CLASS`,
/// `ACSH_LOFT_CLASS`, `ACSH_REVOLVE_CLASS`, `EXTRUDEDSURFACE`, `LOFTEDSURFACE`,
/// `REVOLVEDSURFACE`, `SWEPTSURFACE`), whose bytes (groups 310) are decoded with the DWG layout of
/// `from` and encoded with that of `to`. Handles stay as given; the host remaps them. Returns
/// `None` for another kind, and when the record does not read as that kind.
pub fn reform_object(
    kind: &str,
    tags: &[(i32, String)],
    from: DxfVersion,
    to: DxfVersion,
) -> Option<Vec<(i32, String)>> {
    let kind = kind.to_ascii_uppercase();
    let section = section_of(&kind)?;
    let mut text = format!("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\n{}\n0\nENDSEC\n", from.to_dxf_string());
    text.push_str(&format!("0\nSECTION\n2\n{section}\n"));
    for (code, value) in tags {
        text.push_str(&format!("{code}\n{value}\n"));
    }
    text.push_str("0\nENDSEC\n0\nEOF\n");

    let mut reader = DxfReader::from_reader(Cursor::new(text.into_bytes())).ok()?;
    let document = reader.read_records_only().ok()?;
    if document.entities.is_empty() && document.objects.is_empty() {
        return None;
    }

    let mut buffer: Vec<u8> = Vec::new();
    {
        let mut stream = DxfTextWriter::new(&mut buffer);
        let mut writer = SectionWriter::new(&mut stream, 0x100, 0x100);
        writer.set_version(to);
        writer.write_single_record(&document).ok()?;
    }
    let written = String::from_utf8(buffer).ok()?;
    let mut lines = written.split('\n');
    let mut out: Vec<(i32, String)> = Vec::new();
    while let Some(code) = lines.next() {
        let Ok(code) = code.trim().parse::<i32>() else {
            continue;
        };
        let value = lines.next()?.trim_end_matches('\r').to_string();
        out.push((code, value));
    }
    // An object is written inside its section.
    if out.first().is_some_and(|(code, value)| *code == 0 && value == "SECTION") {
        out.drain(..2);
    }
    if out.last().is_some_and(|(code, value)| *code == 0 && value == "ENDSEC") {
        out.pop();
    }
    out.first().filter(|(code, _)| *code == 0)?;
    Some(out)
}
