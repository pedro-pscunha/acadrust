//! Block-owned surfaces keep their SAB and native wire cache through the DWG bridge.
use acadrust::entities::acis::{primitives::build_box, SabWriter};
use acadrust::entities::solid3d::{AcisData, Wire};
use acadrust::entities::{EntityType, Surface, SurfaceKind};
use acadrust::tables::BlockRecord;
use acadrust::types::{Color, DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};
use std::io::Cursor;

fn surface(kind: SurfaceKind) -> Surface {
    let mut surface = Surface::new(kind);
    surface.acis_data =
        AcisData::from_sab(SabWriter::write(&build_box([0.0, 0.0, 0.0], 2.0, 3.0, 4.0)));
    surface.u_isolines = 6;
    surface.v_isolines = 6;
    surface
}

fn with_wires() -> Surface {
    let mut surface = surface(SurfaceKind::Generic);
    surface.acis_data.wireframe_data_present = true;
    surface.acis_data.wireframe_point_present = true;
    surface.acis_data.wireframe_isoline_present = true;
    surface.acis_data.wireframe_isolines = 6;
    surface.point_of_reference = Vector3::new(3.0, 4.0, 5.0);
    surface.wires = (0..12)
        .map(|index| {
            let mut wire = Wire::from_points(vec![
                Vector3::new(index as f64, 1.0, 2.0),
                Vector3::new(index as f64, 3.0, 4.0),
            ]);
            wire.color = Color::Index(2);
            wire.acis_index = index;
            wire.selection_marker = index + 20;
            wire
        })
        .collect();
    surface
}

#[test]
fn block_owned_surface_family_reads_typed_with_sab_attached() {
    for version in [DxfVersion::AC1027, DxfVersion::AC1032] {
        let mut document = CadDocument::with_version(version);
        let mut block = BlockRecord::new("SURFACE_BLOCK");
        block.handle = document.allocate_handle();
        block.block_entity_handle = document.allocate_handle();
        block.block_end_handle = document.allocate_handle();
        let owner = block.handle;
        document.block_records.add(block).unwrap();
        let mut expected = Vec::new();
        for kind in [
            SurfaceKind::Generic,
            SurfaceKind::Plane,
            SurfaceKind::Extruded,
            SurfaceKind::Lofted,
            SurfaceKind::Revolved,
            SurfaceKind::Swept,
            SurfaceKind::Nurb,
        ] {
            let mut source = surface(kind);
            source.common.owner_handle = owner;
            let sab = source.acis_data.sab_data.clone();
            let handle = document
                .add_entity(EntityType::Surface(Box::new(source)))
                .unwrap();
            expected.push((handle, kind, sab));
        }
        let mut bytes = Vec::new();
        DxfWriter::new(&document)
            .for_dwg_bridge()
            .write_to_writer(&mut bytes)
            .unwrap();
        let restored = DxfReader::from_reader(Cursor::new(bytes))
            .unwrap()
            .read()
            .unwrap();
        for (handle, kind, sab) in expected {
            let Some(EntityType::Surface(actual)) = restored.get_entity(handle) else {
                panic!("block surface {kind:?} became untyped in {version:?}");
            };
            assert_eq!(actual.kind, kind);
            assert_eq!(actual.common.owner_handle, owner);
            assert_eq!(actual.acis_data.sab_data, sab);
        }
    }
}

#[test]
fn acds_surface_native_roundtrip_keeps_all_twelve_wires() {
    for version in [DxfVersion::AC1027, DxfVersion::AC1032] {
        let mut document = CadDocument::with_version(version);
        let source = with_wires();
        let handle = document
            .add_entity(EntityType::Surface(Box::new(source.clone())))
            .unwrap();
        let bytes = DwgWriter::write_to_vec(&document).unwrap();
        let restored = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
        let Some(EntityType::Surface(actual)) = restored.get_entity(handle) else {
            panic!("surface missing after native roundtrip");
        };
        assert_eq!(actual.wires, source.wires, "{version:?}");
        assert_eq!(actual.point_of_reference, source.point_of_reference);
        assert_eq!(actual.acis_data.sab_data, source.acis_data.sab_data);
    }
}

#[test]
fn surface_bridge_roundtrip_keeps_wireframe_data() {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let source = with_wires();
    let handle = document
        .add_entity(EntityType::Surface(Box::new(source.clone())))
        .unwrap();
    let mut bytes = Vec::new();
    DxfWriter::new(&document)
        .for_dwg_bridge()
        .write_to_writer(&mut bytes)
        .unwrap();
    let restored = DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap();
    let Some(EntityType::Surface(actual)) = restored.get_entity(handle) else {
        panic!("surface missing after bridge roundtrip");
    };
    assert_eq!(actual.wires, source.wires);
    assert_eq!(actual.point_of_reference, source.point_of_reference);
    assert_eq!(
        actual.acis_data.wireframe_isolines,
        source.acis_data.wireframe_isolines
    );
    assert_eq!(actual.acis_data.sab_data, source.acis_data.sab_data);
}

#[test]
fn private_surface_cache_keeps_revision_material_references_and_xdata() {
    use acadrust::entities::solid3d::{AcisMaterial, Silhouette};
    use acadrust::xdata::{ExtendedDataRecord, XDataValue};
    use acadrust::Handle;
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut source = with_wires();
    source.acis_data.revision.has_guid = true;
    source.acis_data.revision.major = 12345;
    source.acis_data.revision.bytes = [1, 2, 3, 4, 5, 6, 7, 8];
    source.acis_data.materials.push(AcisMaterial {
        array_index: 3,
        absolute_reference: 4,
        material_handle: Some(Handle::new(0xF00)),
    });
    source.silhouettes.push(Silhouette::new(55));
    let mut xdata = ExtendedDataRecord::new("ACME");
    xdata
        .values
        .push(XDataValue::String("surface-user-data".into()));
    source.common.extended_data.add_record(xdata);
    let handle = document
        .add_entity(EntityType::Surface(Box::new(source.clone())))
        .unwrap();
    let mut public = Vec::new();
    DxfWriter::new(&document)
        .write_to_writer(&mut public)
        .unwrap();
    assert!(!String::from_utf8(public)
        .unwrap()
        .contains("AcDbAcadrustSurfaceCache"));
    let mut bridge = Vec::new();
    DxfWriter::new(&document)
        .for_dwg_bridge()
        .write_to_writer(&mut bridge)
        .unwrap();
    let bridge = String::from_utf8(bridge).unwrap().replace("F00", "F01");
    let restored = DxfReader::from_reader(Cursor::new(bridge.into_bytes()))
        .unwrap()
        .read()
        .unwrap();
    let Some(EntityType::Surface(actual)) = restored.get_entity(handle) else {
        panic!("surface missing after bridge roundtrip");
    };
    assert_eq!(actual.acis_data.revision, source.acis_data.revision);
    assert_eq!(
        actual.acis_data.materials[0].material_handle,
        Some(Handle::new(0xF01))
    );
    assert_eq!(actual.acis_data.materials[0].array_index, 3);
    assert_eq!(actual.silhouettes, source.silhouettes);
    assert_eq!(actual.common.extended_data, source.common.extended_data);
}
