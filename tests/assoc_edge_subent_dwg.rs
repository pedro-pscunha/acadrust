//! The persistent subentity id of an AcDbAssocGeomDependency. The id of an edge
//! (AcDbAssocEdgePersSubentId) holds a class version and two vertex ids, which the DXF writes as
//! three groups 90 and a pre-R2013 DWG stores right after the class name, before the compound
//! flag. A DWG save that left them out made AutoCAD discard the object ("error 67,
//! eDwgObjectImproperlyRead"): 68 objects of a real drawing.

use std::io::Cursor;

use acadrust::classes::DxfClass;
use acadrust::objects::{
    AssocDependency, AssocGeomDependency, AssocPersistentSubentId, AssociativeData,
    AssociativeObject, ObjectType,
};
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn document(version: DxfVersion, class_name: &str, values: Vec<i32>) -> (CadDocument, Handle) {
    let mut document = CadDocument::with_version(version);
    document.classes.add_or_update(DxfClass::new(
        "ACDBASSOCGEOMDEPENDENCY",
        "AcDbAssocGeomDependency",
    ));
    let handle = document.allocate_handle();
    let owner = document.header.named_objects_dict_handle;
    document.objects.insert(
        handle,
        ObjectType::Associative(AssociativeObject {
            handle,
            owner,
            dxf_name: "ACDBASSOCGEOMDEPENDENCY".to_string(),
            cpp_class_name: "AcDbAssocGeomDependency".to_string(),
            data: AssociativeData::GeomDependency(AssocGeomDependency {
                dependency: AssocDependency::default(),
                class_version: 0,
                enabled: true,
                persistent_subent: AssocPersistentSubentId {
                    class_name: class_name.to_string(),
                    class_code: AssocPersistentSubentId::code_for_class_name(class_name)
                        .unwrap_or(0),
                    values,
                    dependent_on_compound_object: false,
                    leading_flag: false,
                },
            }),
            ..Default::default()
        }),
    );
    (document, handle)
}

fn subent(document: &CadDocument, handle: Handle) -> (String, Vec<i32>, bool) {
    let Some(ObjectType::Associative(object)) = document.objects.get(&handle) else {
        panic!("the dependency is missing");
    };
    let AssociativeData::GeomDependency(geom) = &object.data else {
        panic!("not a geometry dependency");
    };
    let subent = &geom.persistent_subent;
    (
        subent.class_name.clone(),
        subent.values.clone(),
        subent.dependent_on_compound_object,
    )
}

fn through_dwg(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn an_edge_id_survives_a_dwg() {
    for version in [DxfVersion::AC1018, DxfVersion::AC1024, DxfVersion::AC1032] {
        let (document, handle) = document(version, "AcDbAssocEdgePersSubentId", vec![0, 9, 10]);
        let back = through_dwg(&document);
        let (name, values, compound) = subent(&back, handle);
        assert_eq!(name, "AcDbAssocEdgePersSubentId", "{version:?}");
        assert_eq!(values, vec![0, 9, 10], "{version:?}");
        assert!(!compound, "{version:?}");
        // Written again from what was read, it stays the same.
        assert_eq!(
            subent(&through_dwg(&back), handle),
            (name, values, compound),
            "{version:?}: second round"
        );
    }
}

#[test]
fn a_single_edge_id_has_no_fields() {
    for version in [DxfVersion::AC1024, DxfVersion::AC1032] {
        let (document, handle) = document(version, "AcDbAssocSingleEdgePersSubentId", vec![]);
        let (name, values, _) = subent(&through_dwg(&document), handle);
        assert_eq!(name, "AcDbAssocSingleEdgePersSubentId", "{version:?}");
        assert!(values.is_empty(), "{version:?}");
    }
}

#[test]
fn an_edge_id_survives_dxf_to_dwg_to_dxf() {
    for version in [DxfVersion::AC1024, DxfVersion::AC1032] {
        let (document, handle) = document(version, "AcDbAssocEdgePersSubentId", vec![0, 9, 10]);
        let dxf = DxfWriter::new(&document).write_to_vec().expect("write dxf");
        let from_dxf = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(
            subent(&from_dxf, handle).1,
            vec![0, 9, 10],
            "{version:?}: DXF"
        );
        let back = through_dwg(&from_dxf);
        assert_eq!(subent(&back, handle).1, vec![0, 9, 10], "{version:?}: DWG");
    }
}
