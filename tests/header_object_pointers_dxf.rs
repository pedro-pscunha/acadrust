//! Header variables that hold a pointer to an object: the current material ($CMATERIAL, group
//! 347), the visual styles of interference objects, interference viewports and 3D drag
//! ($INTERFEREOBJVS 345, $INTERFEREVPVS 346, $DRAGVS 349) and the plot style of new objects
//! ($CEPSNTYPE 380 with the handle in 390). A DWG header holds each as a handle. The DWG reader
//! discarded three of them, the DWG writer wrote them null, and the DXF writer and reader left
//! all of them out, so a drawing that went DWG, DXF, DWG came back without them.

use std::io::Cursor;

use acadrust::objects::{Material, ObjectType, VisualStyle};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, Handle};

/// Add the object `make(handle, owner)` builds to the dictionary `dictionary` under `name` and
/// return its handle.
fn add_to_dictionary(
    doc: &mut CadDocument,
    dictionary: Handle,
    name: &str,
    make: impl FnOnce(Handle, Handle) -> ObjectType,
) -> Handle {
    let handle = doc.allocate_handle();
    doc.objects.insert(handle, make(handle, dictionary));
    let Some(ObjectType::Dictionary(dict)) = doc.objects.get_mut(&dictionary) else {
        panic!("no dictionary {dictionary:?}");
    };
    dict.add_entry(name, handle);
    handle
}

fn visual_style(description: &'static str) -> impl FnOnce(Handle, Handle) -> ObjectType {
    move |handle, owner| {
        let mut style = VisualStyle::new();
        style.handle = handle;
        style.owner = owner;
        style.description = description.to_string();
        ObjectType::VisualStyle(style)
    }
}

/// A document whose header points at a material and at three visual styles.
fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let materials = doc.header.acad_material_dict_handle;
    let styles = doc.header.acad_visualstyle_dict_handle;
    doc.header.current_material_handle =
        add_to_dictionary(&mut doc, materials, "Brick", |handle, owner| {
            let mut material = Material::new();
            material.handle = handle;
            material.owner = owner;
            material.name = "Brick".to_string();
            ObjectType::Material(material)
        });
    doc.header.interference_object_visual_style_handle = add_to_dictionary(
        &mut doc,
        styles,
        "InterferenceObjects",
        visual_style("interference objects"),
    );
    doc.header.interference_viewport_visual_style_handle = add_to_dictionary(
        &mut doc,
        styles,
        "InterferenceViewport",
        visual_style("interference viewport"),
    );
    doc.header.drag_visual_style_handle =
        add_to_dictionary(&mut doc, styles, "Drag", visual_style("drag"));
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

fn material_name(doc: &CadDocument) -> String {
    match doc.objects.get(&doc.header.current_material_handle) {
        Some(ObjectType::Material(m)) => m.name.clone(),
        other => format!("not a material: {other:?}"),
    }
}

fn style_description(doc: &CadDocument, handle: Handle) -> String {
    match doc.objects.get(&handle) {
        Some(ObjectType::VisualStyle(s)) => s.description.clone(),
        other => format!("not a visual style: {other:?}"),
    }
}

fn check_pointers(doc: &CadDocument, what: &str) {
    let h = &doc.header;
    assert_eq!(material_name(doc), "Brick", "{what}: CMATERIAL");
    assert_eq!(
        style_description(doc, h.interference_object_visual_style_handle),
        "interference objects",
        "{what}: INTERFEREOBJVS"
    );
    assert_eq!(
        style_description(doc, h.interference_viewport_visual_style_handle),
        "interference viewport",
        "{what}: INTERFEREVPVS"
    );
    assert_eq!(
        style_description(doc, h.drag_visual_style_handle),
        "drag",
        "{what}: DRAGVS"
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
fn the_dxf_header_carries_the_object_pointers_with_their_group_codes() {
    let doc = document(DxfVersion::AC1032);
    let dxf =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf-8");
    for (name, code, handle) in [
        ("$CMATERIAL", "347", doc.header.current_material_handle),
        (
            "$INTERFEREOBJVS",
            "345",
            doc.header.interference_object_visual_style_handle,
        ),
        (
            "$INTERFEREVPVS",
            "346",
            doc.header.interference_viewport_visual_style_handle,
        ),
        ("$DRAGVS", "349", doc.header.drag_visual_style_handle),
    ] {
        assert_eq!(
            header_variable(&dxf, name),
            vec![code.to_string(), format!("{:X}", handle.value())],
            "{name}"
        );
    }
}

#[test]
fn the_object_pointers_survive_dxf() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1024, DxfVersion::AC1032] {
        check_pointers(&through_dxf(&document(version)), &format!("{version:?}"));
    }
}

#[test]
fn a_dwg_keeps_the_object_pointers() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1024, DxfVersion::AC1032] {
        check_pointers(&through_dwg(&document(version)), &format!("{version:?}"));
    }
}

#[test]
fn the_object_pointers_survive_dxf_to_dwg() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1024, DxfVersion::AC1032] {
        let bridged = through_dxf(&document(version));
        check_pointers(&through_dwg(&bridged), &format!("{version:?}"));
    }
}

#[test]
fn a_dangling_pointer_is_written_null() {
    let mut doc = document(DxfVersion::AC1032);
    doc.header.current_material_handle = Handle::new(0x7FFF_0000);
    doc.header.drag_visual_style_handle = Handle::new(0x7FFF_0001);
    let back = through_dwg(&doc);
    assert!(back.header.current_material_handle.is_null(), "CMATERIAL");
    assert!(back.header.drag_visual_style_handle.is_null(), "DRAGVS");
    assert_eq!(
        style_description(&back, back.header.interference_object_visual_style_handle),
        "interference objects"
    );
}

/// The ACAD_PLOTSTYLENAME placeholder every drawing holds.
fn plot_style_placeholder(doc: &CadDocument) -> Handle {
    let Some(ObjectType::DictionaryWithDefault(dict)) =
        doc.objects.get(&doc.header.acad_plotstylename_dict_handle)
    else {
        panic!("no ACAD_PLOTSTYLENAME dictionary");
    };
    dict.default_handle
}

#[test]
fn the_plot_style_of_new_objects_survives_dxf_to_dwg() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let mut doc = CadDocument::with_version(version);
        let placeholder = plot_style_placeholder(&doc);
        doc.header.current_plotstyle_type = 3;
        doc.header.current_plotstyle_handle = placeholder;
        let dxf = String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf"))
            .expect("utf-8");
        assert_eq!(
            header_variable(&dxf, "$CEPSNTYPE"),
            vec![
                "380".to_string(),
                "3".to_string(),
                "390".to_string(),
                format!("{:X}", placeholder.value())
            ],
            "{version:?}: DXF"
        );
        let bridged = through_dxf(&doc);
        assert_eq!(
            bridged.header.current_plotstyle_type, 3,
            "{version:?}: DXF type"
        );
        assert_eq!(
            bridged.header.current_plotstyle_handle, placeholder,
            "{version:?}: DXF"
        );
        let back = through_dwg(&bridged);
        assert_eq!(
            back.header.current_plotstyle_type, 3,
            "{version:?}: DWG type"
        );
        assert!(
            matches!(
                back.objects.get(&back.header.current_plotstyle_handle),
                Some(ObjectType::PlaceHolder(_))
            ),
            "{version:?}: CPSNID names {:?}",
            back.header.current_plotstyle_handle
        );
    }
}

#[test]
fn a_plot_style_type_other_than_by_object_writes_no_handle() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.header.current_plotstyle_type = 2;
    let dxf =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf-8");
    assert_eq!(
        header_variable(&dxf, "$CEPSNTYPE"),
        vec!["380".to_string(), "2".to_string()]
    );
}
