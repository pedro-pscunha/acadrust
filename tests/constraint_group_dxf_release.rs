//! The nodes of an AcDbAssoc2dConstraintGroup (id, status, connections) keep their values when
//! the DXF that carries them is declared as another release than the one it was written for. A
//! host may rewrite the DXF and its header; the layout of a node in the file (the status before or
//! after the connections, the nodes inline or in a registry) is told by the groups, not by the
//! declared release. A reader that took it from the release scrambled the node ids and dropped the
//! connections ("error 151, eOutOfRange" in AutoCAD).

use std::io::Cursor;

use acadrust::classes::DxfClass;
use acadrust::objects::{
    Assoc2dConstraintGroup, AssocAction, AssocConstraintNode, AssocConstraintNodeData,
    AssociativeData, AssociativeObject, ObjectType,
};
use acadrust::types::{DxfVersion, Handle, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const NEXT_NODE_ID: i32 = 16790;

fn node(id: i32, status: u8, connections: &[i32], active: bool) -> AssocConstraintNode {
    AssocConstraintNode {
        node_id: id,
        status,
        connections: connections.to_vec(),
        class_name: "AcFixedConstraint".to_string(),
        registry_flag: true,
        data: AssocConstraintNodeData::Geometrical {
            owner_id: 0,
            is_implied: false,
            is_active: active,
        },
    }
}

fn document(version: DxfVersion) -> (CadDocument, Handle) {
    let mut document = CadDocument::with_version(version);
    document.classes.add_or_update(DxfClass::new(
        "ASSOC2DCONSTRAINTGROUP",
        "AcDbAssoc2dConstraintGroup",
    ));
    let handle = document.allocate_handle();
    let owner = document.header.named_objects_dict_handle;
    document.objects.insert(
        handle,
        ObjectType::Associative(AssociativeObject {
            handle,
            owner,
            dxf_name: "ASSOC2DCONSTRAINTGROUP".to_string(),
            cpp_class_name: "AcDbAssoc2dConstraintGroup".to_string(),
            data: AssociativeData::ConstraintGroup(Assoc2dConstraintGroup {
                action: AssocAction {
                    class_version: 2,
                    ..Default::default()
                },
                version: 2,
                next_node_id: NEXT_NODE_ID,
                nodes: vec![
                    AssocConstraintNode {
                        node_id: 7,
                        connections: vec![11, 12],
                        ..Default::default()
                    },
                    node(11, 3, &[12, 7], true),
                    node(12, 5, &[11], false),
                    node(4000, 1, &[], true),
                    AssocConstraintNode {
                        node_id: 4001,
                        status: 2,
                        connections: vec![4000],
                        class_name: "AcConstrainedCircle".to_string(),
                        registry_flag: true,
                        data: AssocConstraintNodeData::Circle {
                            geometry_dependency: Handle::new(0x30),
                            geometry_node_id: 5,
                            center: Vector3::new(1.0, 2.0, 0.0),
                            normal: Vector3::UNIT_Z,
                            direction: Vector3::UNIT_X,
                            radius: 3.0,
                            start_parameter: 0.25,
                            end_parameter: 6.0,
                            reserved: 2.5,
                        },
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        }),
    );
    (document, handle)
}

/// (node id, status, connections, class name) of each node, and the next node id.
fn nodes(document: &CadDocument, handle: Handle) -> (Vec<(i32, u8, Vec<i32>, String)>, i32) {
    let Some(ObjectType::Associative(object)) = document.objects.get(&handle) else {
        panic!("the group is missing");
    };
    let AssociativeData::ConstraintGroup(group) = &object.data else {
        panic!("not a constraint group");
    };
    (
        group
            .nodes
            .iter()
            .map(|n| {
                (
                    n.node_id,
                    n.status,
                    n.connections.clone(),
                    n.class_name.clone(),
                )
            })
            .collect(),
        group.next_node_id,
    )
}

fn through_dwg(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn the_nodes_survive_a_dxf_declared_as_another_release() {
    for (written, written_name) in [
        (DxfVersion::AC1015, "AC1015"),
        (DxfVersion::AC1024, "AC1024"),
        (DxfVersion::AC1032, "AC1032"),
    ] {
        let (source, handle) = document(written);
        let read = through_dwg(&source);
        let wanted = nodes(&read, handle);
        assert_eq!(wanted.0.len(), 5, "{written_name}: the DWG keeps the nodes");
        let dxf = DxfWriter::new(&read)
            .with_version(written)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf");
        let text = String::from_utf8(dxf).expect("utf8");
        for (declared, declared_name) in [
            (DxfVersion::AC1015, "AC1015"),
            (DxfVersion::AC1024, "AC1024"),
            (DxfVersion::AC1032, "AC1032"),
        ] {
            let rewritten = text.replacen(written_name, declared_name, 1);
            let mut from_dxf = DxfReader::from_reader(Cursor::new(rewritten.into_bytes()))
                .expect("reader")
                .read()
                .expect("read dxf");
            assert_eq!(from_dxf.version, declared);
            assert_eq!(
                nodes(&from_dxf, handle),
                wanted,
                "written {written_name}, declared {declared_name}: DXF"
            );
            from_dxf.version = DxfVersion::AC1032;
            assert_eq!(
                nodes(&through_dwg(&from_dxf), handle),
                wanted,
                "written {written_name}, declared {declared_name}: DWG as R2018"
            );
        }
    }
}
