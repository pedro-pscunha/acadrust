//! A layer holds a hard pointer to a plot style (group 390) and, from AutoCAD 2007, one to a
//! material (group 347). The DXF writer wrote the plot style and no material, and the DXF reader
//! read neither, so every layer of a drawing that went DWG, DXF, DWG came back without both.

use std::io::Cursor;

use acadrust::objects::{Material, ObjectType};
use acadrust::tables::Layer;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, Handle, TableEntry};

/// A document with a layer "Walls" that points at a material and at the plot style placeholder.
fn document(version: DxfVersion) -> (CadDocument, Handle, Handle) {
    let mut doc = CadDocument::with_version(version);
    let materials = doc.header.acad_material_dict_handle;
    let material = doc.allocate_handle();
    let mut object = Material::new();
    object.handle = material;
    object.owner = materials;
    object.name = "Concrete".to_string();
    doc.objects.insert(material, ObjectType::Material(object));
    let Some(ObjectType::Dictionary(dict)) = doc.objects.get_mut(&materials) else {
        panic!("no ACAD_MATERIAL dictionary");
    };
    dict.add_entry("Concrete", material);
    let Some(ObjectType::DictionaryWithDefault(styles)) =
        doc.objects.get(&doc.header.acad_plotstylename_dict_handle)
    else {
        panic!("no ACAD_PLOTSTYLENAME dictionary");
    };
    let plot_style = styles.default_handle;
    let mut layer = Layer::new("Walls");
    layer.set_handle(doc.allocate_handle());
    layer.material = material;
    layer.plotstyle_handle = plot_style;
    doc.layers.add(layer).expect("add layer");
    (doc, material, plot_style)
}

fn walls(doc: &CadDocument) -> &Layer {
    doc.layers.get("Walls").expect("layer Walls")
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

/// The group codes and values of the LAYER record named `name`.
fn layer_record(dxf: &str, name: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = dxf.lines().map(str::trim).collect();
    let mut at = 0;
    while at + 1 < lines.len() {
        if lines[at] == "0" && lines[at + 1] == "LAYER" {
            let end = (at + 2..lines.len() - 1)
                .step_by(2)
                .find(|&i| lines[i] == "0")
                .unwrap_or(lines.len() - 1);
            let record: Vec<(String, String)> = lines[at + 2..end]
                .chunks(2)
                .filter(|pair| pair.len() == 2)
                .map(|pair| (pair[0].to_string(), pair[1].to_string()))
                .collect();
            if record.iter().any(|(c, v)| c == "2" && v == name) {
                return record;
            }
            at = end;
        } else {
            at += 1;
        }
    }
    panic!("no LAYER record {name}");
}

#[test]
fn the_dxf_layer_record_holds_the_material_and_plot_style_pointers() {
    let (doc, material, plot_style) = document(DxfVersion::AC1032);
    let dxf =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf-8");
    let record = layer_record(&dxf, "Walls");
    let value = |code: &str| {
        record
            .iter()
            .find(|(c, _)| c == code)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(value("347"), Some(format!("{:X}", material.value())), "347");
    assert_eq!(
        value("390"),
        Some(format!("{:X}", plot_style.value())),
        "390"
    );
}

#[test]
fn the_layer_pointers_survive_dxf() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1032] {
        let (doc, material, plot_style) = document(version);
        let layer = through_dxf(&doc);
        assert_eq!(walls(&layer).material, material, "{version:?}: material");
        assert_eq!(
            walls(&layer).plotstyle_handle,
            plot_style,
            "{version:?}: plot style"
        );
    }
}

#[test]
fn the_layer_pointers_survive_dxf_to_dwg() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1032] {
        let (doc, _, _) = document(version);
        let back = through_dwg(&through_dxf(&doc));
        let layer = walls(&back);
        assert!(
            matches!(
                back.objects.get(&layer.material),
                Some(ObjectType::Material(m)) if m.name == "Concrete"
            ),
            "{version:?}: material {:?}",
            layer.material
        );
        assert!(
            matches!(
                back.objects.get(&layer.plotstyle_handle),
                Some(ObjectType::PlaceHolder(_))
            ),
            "{version:?}: plot style {:?}",
            layer.plotstyle_handle
        );
    }
}

#[test]
fn a_material_the_file_does_not_hold_is_not_written() {
    let (mut doc, _, _) = document(DxfVersion::AC1032);
    doc.layers.get_mut("Walls").expect("layer").material = Handle::new(0x7FFF_0000);
    let dxf =
        String::from_utf8(DxfWriter::new(&doc).write_to_vec().expect("write dxf")).expect("utf-8");
    assert!(layer_record(&dxf, "Walls").iter().all(|(c, _)| c != "347"));
}

#[test]
fn a_dwg_layer_does_not_point_at_a_plot_style_the_document_lacks() {
    let (mut doc, _, _) = document(DxfVersion::AC1032);
    doc.layers.get_mut("Walls").expect("layer").plotstyle_handle = Handle::new(0x7FFF_0000);
    let back = through_dwg(&doc);
    assert!(walls(&back).plotstyle_handle.is_null());
}
