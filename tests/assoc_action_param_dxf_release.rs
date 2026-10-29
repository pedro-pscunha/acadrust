//! The version of an AcDbAssocActionParam survives a DXF declared as any release. A host can
//! write the DXF as R2000 and raise the version of the document before the DWG write.

use std::io::Cursor;

use acadrust::classes::DxfClass;
use acadrust::objects::{AssocActionParam, AssociativeData, AssociativeObject, ObjectType};
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn document() -> (CadDocument, Handle) {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    document
        .classes
        .add_or_update(DxfClass::new("ASSOCACTIONPARAM", "AcDbAssocActionParam"));
    let handle = document.allocate_handle();
    let owner = document.header.named_objects_dict_handle;
    document.objects.insert(
        handle,
        ObjectType::Associative(AssociativeObject {
            handle,
            owner,
            dxf_name: "ASSOCACTIONPARAM".to_string(),
            cpp_class_name: "AcDbAssocActionParam".to_string(),
            data: AssociativeData::ActionParam(AssocActionParam {
                is_r2013: 1,
                version: 7,
                name: "Param".to_string(),
            }),
            ..Default::default()
        }),
    );
    (document, handle)
}

fn param(document: &CadDocument, handle: Handle) -> AssocActionParam {
    let Some(ObjectType::Associative(object)) = document.objects.get(&handle) else {
        panic!("the parameter is missing");
    };
    let AssociativeData::ActionParam(value) = &object.data else {
        panic!("not an action parameter");
    };
    value.clone()
}

fn through_dwg(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn the_version_survives_a_dxf_declared_as_any_release() {
    let (source, handle) = document();
    let read = through_dwg(&source);
    let wanted = param(&read, handle);
    assert_eq!(wanted.version, 7, "the DWG keeps the version");
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
        assert_eq!(param(&from_dxf, handle), wanted, "{declared:?}: DXF");
        from_dxf.version = DxfVersion::AC1032;
        assert_eq!(
            param(&through_dwg(&from_dxf), handle),
            wanted,
            "{declared:?}: DWG"
        );
    }
}
