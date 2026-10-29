//! The UCS header variables. A DWG header holds the current UCS ($UCSNAME), the UCS the
//! orthographic settings are relative to ($UCSBASE, $UCSORTHOREF) and their paper space twins as
//! handles of UCS table entries, and a DXF holds each as a name (group 2). The DWG reader
//! discarded the handles, the DWG writer wrote them null, and the DXF writer left the names, the
//! orthographic view type and the orthographic origins out, so a drawing that went DWG, DXF, DWG
//! came back with the world UCS.

use std::io::Cursor;

use acadrust::tables::Ucs;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, Handle, TableEntry};

fn add_ucs(doc: &mut CadDocument, name: &str, origin: Vector3) -> Handle {
    let mut ucs = Ucs::new(name);
    ucs.origin = origin;
    ucs.set_handle(doc.allocate_handle());
    let handle = ucs.handle;
    doc.ucss.add(ucs).expect("add ucs");
    handle
}

/// A document whose header names six different UCS entries, one for each pointer.
fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let plan = add_ucs(&mut doc, "Plan", Vector3::new(1.0, 2.0, 0.0));
    let site = add_ucs(&mut doc, "Site", Vector3::new(3.0, 4.0, 0.0));
    let tower = add_ucs(&mut doc, "Tower", Vector3::new(5.0, 6.0, 0.0));
    let sheet = add_ucs(&mut doc, "Sheet", Vector3::new(7.0, 8.0, 0.0));
    let sheet_base = add_ucs(&mut doc, "SheetBase", Vector3::new(9.0, 1.0, 0.0));
    let sheet_ref = add_ucs(&mut doc, "SheetRef", Vector3::new(2.0, 3.0, 0.0));
    let h = &mut doc.header;
    h.model_space_ucs_name = "Plan".to_string();
    h.model_space_ucs_handle = plan;
    h.ucs_base = "Site".to_string();
    h.ucs_base_handle = site;
    h.ucs_ortho_ref_name = "Tower".to_string();
    h.ucs_ortho_ref = tower;
    h.ucs_ortho_view = 3;
    h.ucs_ortho_origins[0] = Vector3::new(0.5, 1.5, 2.5);
    h.ucs_ortho_origins[5] = Vector3::new(-1.0, -2.0, -3.0);
    h.paper_space_ucs_name = "Sheet".to_string();
    h.paper_space_ucs_handle = sheet;
    h.paper_space_ucs_origin = Vector3::new(7.0, 8.0, 0.0);
    h.paper_space_ucs_x_axis = Vector3::new(0.0, 1.0, 0.0);
    h.paper_space_ucs_y_axis = Vector3::new(-1.0, 0.0, 0.0);
    h.paper_ucs_base = "SheetBase".to_string();
    h.paper_ucs_base_handle = sheet_base;
    h.paper_ucs_ortho_ref_name = "SheetRef".to_string();
    h.paper_ucs_ortho_ref = sheet_ref;
    h.paper_ucs_ortho_view = 5;
    h.paper_ucs_ortho_origins[2] = Vector3::new(4.5, 5.5, 6.5);
    doc
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let dxf = DxfWriter::new(doc).write_to_vec().expect("write dxf");
    DxfReader::from_reader(Cursor::new(dxf))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let dwg = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(dwg))
        .read()
        .expect("read dwg")
}

/// The name of the UCS table entry `handle` names.
fn ucs_name(doc: &CadDocument, handle: Handle) -> String {
    doc.ucss
        .iter()
        .find(|u| u.handle == handle)
        .map_or_else(|| format!("no UCS for {handle:?}"), |u| u.name.clone())
}

/// A DXF names the UCS entries; the handles come when the DWG writer resolves the names.
fn check_ucs(doc: &CadDocument, what: &str, handles: bool) {
    let h = &doc.header;
    for (variable, name, handle, expected) in [
        (
            "UCSNAME",
            &h.model_space_ucs_name,
            h.model_space_ucs_handle,
            "Plan",
        ),
        ("UCSBASE", &h.ucs_base, h.ucs_base_handle, "Site"),
        (
            "UCSORTHOREF",
            &h.ucs_ortho_ref_name,
            h.ucs_ortho_ref,
            "Tower",
        ),
        (
            "PUCSNAME",
            &h.paper_space_ucs_name,
            h.paper_space_ucs_handle,
            "Sheet",
        ),
        (
            "PUCSBASE",
            &h.paper_ucs_base,
            h.paper_ucs_base_handle,
            "SheetBase",
        ),
        (
            "PUCSORTHOREF",
            &h.paper_ucs_ortho_ref_name,
            h.paper_ucs_ortho_ref,
            "SheetRef",
        ),
    ] {
        assert_eq!(name, expected, "{what}: {variable} name");
        if handles {
            assert_eq!(ucs_name(doc, handle), expected, "{what}: {variable} handle");
        }
    }
    assert_eq!(h.ucs_ortho_view, 3, "{what}: UCSORTHOVIEW");
    assert_eq!(
        h.ucs_ortho_origins[0],
        Vector3::new(0.5, 1.5, 2.5),
        "{what}: UCSORGTOP"
    );
    assert_eq!(
        h.ucs_ortho_origins[5],
        Vector3::new(-1.0, -2.0, -3.0),
        "{what}: UCSORGBACK"
    );
    assert_eq!(h.paper_ucs_ortho_view, 5, "{what}: PUCSORTHOVIEW");
    assert_eq!(
        h.paper_ucs_ortho_origins[2],
        Vector3::new(4.5, 5.5, 6.5),
        "{what}: PUCSORGLEFT"
    );
    assert_eq!(
        h.paper_space_ucs_origin,
        Vector3::new(7.0, 8.0, 0.0),
        "{what}: PUCSORG"
    );
    assert_eq!(
        h.paper_space_ucs_x_axis,
        Vector3::new(0.0, 1.0, 0.0),
        "{what}: PUCSXDIR"
    );
    assert_eq!(
        h.paper_space_ucs_y_axis,
        Vector3::new(-1.0, 0.0, 0.0),
        "{what}: PUCSYDIR"
    );
}

/// The lines of the HEADER section that follow `$NAME`, up to the next variable.
fn header_variable(dxf: &str, name: &str) -> Vec<String> {
    let lines: Vec<&str> = dxf.lines().map(str::trim).collect();
    let start = lines
        .windows(2)
        .position(|w| w[0] == "9" && w[1] == name)
        .unwrap_or_else(|| panic!("no {name} in the HEADER"))
        + 2;
    let end = lines[start..]
        .chunks(2)
        .position(|pair| pair[0] == "9" || pair[0] == "0")
        .map_or(lines.len(), |n| start + 2 * n);
    lines[start..end].iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn the_dxf_header_names_the_ucs_entries() {
    let doc = document(DxfVersion::AC1032);
    let dxf =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf-8");
    for (variable, name) in [
        ("$UCSNAME", "Plan"),
        ("$UCSBASE", "Site"),
        ("$UCSORTHOREF", "Tower"),
        ("$PUCSNAME", "Sheet"),
        ("$PUCSBASE", "SheetBase"),
        ("$PUCSORTHOREF", "SheetRef"),
    ] {
        assert_eq!(
            header_variable(&dxf, variable),
            vec!["2", name],
            "{variable}"
        );
    }
    assert_eq!(header_variable(&dxf, "$UCSORTHOVIEW"), vec!["70", "3"]);
    assert_eq!(header_variable(&dxf, "$PUCSORTHOVIEW"), vec!["70", "5"]);
}

#[test]
fn the_ucs_header_survives_dxf() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        check_ucs(
            &through_dxf(&document(version)),
            &format!("{version:?}"),
            false,
        );
    }
}

#[test]
fn a_dwg_keeps_the_ucs_header() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        check_ucs(
            &through_dwg(&document(version)),
            &format!("{version:?}"),
            true,
        );
    }
}

#[test]
fn the_ucs_header_survives_dxf_to_dwg() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let bridged = through_dxf(&document(version));
        check_ucs(&through_dwg(&bridged), &format!("{version:?}"), true);
    }
}

#[test]
fn a_ucs_name_that_names_no_entry_is_written_null() {
    let mut doc = document(DxfVersion::AC1032);
    doc.header.model_space_ucs_name = "Gone".to_string();
    doc.header.model_space_ucs_handle = Handle::new(0x7FFF_0000);
    let back = through_dwg(&doc);
    assert!(back.header.model_space_ucs_handle.is_null());
    assert_eq!(back.header.model_space_ucs_name, "");
    assert_eq!(ucs_name(&back, back.header.ucs_base_handle), "Site");
}
