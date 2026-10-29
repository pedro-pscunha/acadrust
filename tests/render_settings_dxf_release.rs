//! The R2013 flag of a RENDERSETTINGS (`has_predefined`) survives a DXF declared as any release.

use std::io::Cursor;

use acadrust::classes::DxfClass;
use acadrust::objects::{ClassObject, ClassObjectData, ObjectType, RenderSettings};
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn document() -> (CadDocument, Handle) {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    document
        .classes
        .add_or_update(DxfClass::new("RENDERSETTINGS", "AcDbRenderSettings"));
    let handle = document.allocate_handle();
    let owner = document.header.named_objects_dict_handle;
    document.objects.insert(
        handle,
        ObjectType::ClassObject(ClassObject {
            handle,
            owner,
            data: ClassObjectData::RenderSettings(RenderSettings {
                class_version: 2,
                name: "Medium".into(),
                fog_enabled: true,
                backfaces_enabled: true,
                display_index: 3,
                has_predefined: true,
                ..Default::default()
            }),
            ..Default::default()
        }),
    );
    (document, handle)
}

fn settings(document: &CadDocument, handle: Handle) -> RenderSettings {
    match document.objects.get(&handle) {
        Some(ObjectType::ClassObject(ClassObject {
            data: ClassObjectData::RenderSettings(value),
            ..
        })) => value.clone(),
        other => panic!("no render settings: {other:?}"),
    }
}

fn through_dwg(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn the_predefined_flag_survives_a_dxf_declared_as_any_release() {
    let (source, handle) = document();
    let read = through_dwg(&source);
    let wanted = settings(&read, handle);
    assert!(wanted.has_predefined, "the DWG keeps the flag");
    for declared in [DxfVersion::AC1015, DxfVersion::AC1024, DxfVersion::AC1032] {
        let dxf = DxfWriter::new(&read)
            .with_version(declared)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf");
        let mut from_dxf = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(settings(&from_dxf, handle), wanted, "{declared:?}: DXF");
        from_dxf.version = DxfVersion::AC1032;
        assert_eq!(
            settings(&through_dwg(&from_dxf), handle),
            wanted,
            "{declared:?}: DWG"
        );
    }
}
