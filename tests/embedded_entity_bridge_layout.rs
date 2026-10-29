//! The embedded DWG entities of a SOLID history step or a surface keep their value through a
//! bridge DXF whose header a host rewrites as another release (R2000) before it reads the file,
//! and raises afterwards. The bytes are encoded with the DWG layout of the document, and the DXF
//! records that release.

use std::io::Cursor;

use acadrust::entities::solid3d::Solid3D;
use acadrust::entities::{
    EmbeddedEntity, EntityType, Line, LwPolyline, LwVertex, Spline, Surface, SurfaceData,
    SurfaceKind,
};
use acadrust::objects::{
    SolidHistoryLoft, SolidHistoryNodeBase, SolidHistoryOperation, SolidHistoryRevolve,
    SolidHistorySweep,
};
use acadrust::types::{DxfVersion, Handle, Vector2, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn line(offset: f64) -> EmbeddedEntity {
    EmbeddedEntity::Line(Line::from_points(
        Vector3::new(offset, 0.0, 0.0),
        Vector3::new(offset + 5.0, 2.5, 0.0),
    ))
}

/// A polyline whose vertices carry ids: the DWG stores them from R2010 on, so the bytes depend on
/// the release of the layout.
fn polyline(size: f64) -> EmbeddedEntity {
    let mut polyline = LwPolyline::new();
    for (index, (x, y)) in [(0.0, 0.0), (size, 0.0), (size, size)]
        .into_iter()
        .enumerate()
    {
        let mut vertex = LwVertex::new(Vector2::new(x, y));
        vertex.vertex_id = 11 + index as i32;
        polyline.add_vertex(vertex);
    }
    EmbeddedEntity::LwPolyline(polyline)
}

/// A spline: the DWG stores more of it from R2013 on.
fn spline(size: f64) -> EmbeddedEntity {
    EmbeddedEntity::Spline(Spline::from_control_points(
        3,
        vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(size, 1.0, 0.0),
            Vector3::new(2.0 * size, -1.0, 0.0),
            Vector3::new(3.0 * size, 0.0, 0.0),
        ],
    ))
}

fn surface(document: &mut CadDocument, surface: Surface) -> Handle {
    document
        .add_entity(EntityType::Surface(Box::new(surface)))
        .expect("add surface")
}

fn history(document: &mut CadDocument, operation: SolidHistoryOperation) -> Handle {
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .expect("add solid");
    document
        .create_solid_history(entity, operation)
        .expect("create history");
    entity
}

/// The embedded entities of a surface, in record order.
fn surface_entities(document: &CadDocument, handle: Handle) -> Vec<EmbeddedEntity> {
    let Some(EntityType::Surface(surface)) = document.get_entity(handle) else {
        panic!("the surface is missing");
    };
    match &surface.surface_data {
        SurfaceData::Extruded { sweep_entity, .. } => sweep_entity.iter().cloned().collect(),
        SurfaceData::Lofted {
            cross_section_entities,
            guide_entities,
            path_entity,
            ..
        } => cross_section_entities
            .iter()
            .chain(guide_entities)
            .chain(path_entity)
            .cloned()
            .collect(),
        SurfaceData::Revolved { revolve_entity, .. } => revolve_entity.iter().cloned().collect(),
        SurfaceData::Swept {
            sweep_entity,
            path_entity,
            ..
        } => sweep_entity.iter().chain(path_entity).cloned().collect(),
        other => panic!("unexpected surface data {other:?}"),
    }
}

/// The embedded entities of the first step of a solid history.
fn history_entities(document: &CadDocument, handle: Handle) -> Vec<EmbeddedEntity> {
    let operations = document
        .solid_history_operations(handle)
        .expect("the history");
    match &operations[0] {
        SolidHistoryOperation::Sweep(value) | SolidHistoryOperation::Extrusion(value) => value
            .sweep_entity
            .iter()
            .chain(&value.path_entity)
            .cloned()
            .collect(),
        SolidHistoryOperation::Loft(value) => value
            .cross_sections
            .iter()
            .chain(&value.guides)
            .chain(&value.path_entity)
            .cloned()
            .collect(),
        SolidHistoryOperation::Revolve(value) => value.sweep_entity.iter().cloned().collect(),
        other => panic!("unexpected history step {other:?}"),
    }
}

fn through_dwg(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

/// DWG -> bridge DXF declared as R2000 -> read -> version raised -> DWG, for DWGs of R2013 and
/// R2018: the embedded entities are the same on every leg.
fn check(
    kind: &str,
    build: impl Fn(&mut CadDocument) -> Handle,
    entities: impl Fn(&CadDocument, Handle) -> Vec<EmbeddedEntity>,
    expected: usize,
) {
    for version in [DxfVersion::AC1027, DxfVersion::AC1032] {
        let mut document = CadDocument::with_version(version);
        let handle = build(&mut document);
        // The DXF is written from the native document: a record read from a DWG keeps its raw
        // bytes, and the bridge writes those instead of the entities. A DWG read normalizes the
        // entities, so it also gives the value every leg must keep.
        assert_eq!(
            entities(&document, handle).len(),
            expected,
            "{kind} {version:?}: the entities"
        );
        let wanted = entities(&through_dwg(&document), handle);
        assert_eq!(
            wanted.len(),
            expected,
            "{kind} {version:?}: the DWG keeps them"
        );
        let dxf = DxfWriter::new(&document)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf");
        // The first occurrence is the $ACADVER of the header.
        let declared =
            String::from_utf8(dxf)
                .expect("utf8")
                .replacen(version.to_dxf_string(), "AC1015", 1);
        let mut from_dxf = DxfReader::from_reader(Cursor::new(declared.into_bytes()))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(
            entities(&from_dxf, handle),
            wanted,
            "{kind} {version:?}: DXF"
        );
        from_dxf.version = version;
        assert_eq!(
            entities(&through_dwg(&from_dxf), handle),
            wanted,
            "{kind} {version:?}: DWG"
        );
    }
}

#[test]
fn an_extruded_surface_keeps_its_profile() {
    check(
        "EXTRUDEDSURFACE",
        |document| {
            let mut value = Surface::new(SurfaceKind::Extruded);
            if let SurfaceData::Extruded {
                sweep_entity,
                sweep_vector,
                ..
            } = &mut value.surface_data
            {
                *sweep_entity = Some(spline(3.0));
                *sweep_vector = Vector3::new(0.0, 0.0, 4.0);
            }
            surface(document, value)
        },
        surface_entities,
        1,
    );
}

#[test]
fn a_lofted_surface_keeps_its_sections() {
    check(
        "LOFTEDSURFACE",
        |document| {
            let mut value = Surface::new(SurfaceKind::Lofted);
            if let SurfaceData::Lofted {
                cross_section_entities,
                guide_entities,
                ..
            } = &mut value.surface_data
            {
                *cross_section_entities = vec![spline(2.0), polyline(4.0)];
                *guide_entities = vec![line(0.0)];
            }
            surface(document, value)
        },
        surface_entities,
        3,
    );
}

#[test]
fn a_revolved_surface_keeps_its_profile() {
    check(
        "REVOLVEDSURFACE",
        |document| {
            let mut value = Surface::new(SurfaceKind::Revolved);
            if let SurfaceData::Revolved {
                revolve_entity,
                axis_vector,
                ..
            } = &mut value.surface_data
            {
                *revolve_entity = Some(spline(1.0));
                *axis_vector = Vector3::UNIT_Z;
            }
            surface(document, value)
        },
        surface_entities,
        1,
    );
}

#[test]
fn a_swept_surface_keeps_its_profile_and_path() {
    check(
        "SWEPTSURFACE",
        |document| {
            let mut value = Surface::new(SurfaceKind::Swept);
            if let SurfaceData::Swept {
                sweep_entity,
                path_entity,
                ..
            } = &mut value.surface_data
            {
                *sweep_entity = Some(spline(1.5));
                *path_entity = Some(line(0.0));
            }
            surface(document, value)
        },
        surface_entities,
        2,
    );
}

#[test]
fn a_swept_history_step_keeps_its_profile_and_path() {
    check(
        "ACSH_SWEEP_CLASS",
        |document| {
            history(
                document,
                SolidHistoryOperation::Sweep(SolidHistorySweep {
                    base: SolidHistoryNodeBase::new(1),
                    sweep_entity: Some(spline(1.5)),
                    path_entity: Some(line(0.0)),
                    ..Default::default()
                }),
            )
        },
        history_entities,
        2,
    );
}

#[test]
fn an_extruded_history_step_keeps_its_profile() {
    check(
        "ACSH_EXTRUSION_CLASS",
        |document| {
            history(
                document,
                SolidHistoryOperation::Extrusion(SolidHistorySweep {
                    base: SolidHistoryNodeBase::new(1),
                    sweep_entity: Some(spline(2.5)),
                    ..Default::default()
                }),
            )
        },
        history_entities,
        1,
    );
}

#[test]
fn a_lofted_history_step_keeps_its_sections() {
    check(
        "ACSH_LOFT_CLASS",
        |document| {
            history(
                document,
                SolidHistoryOperation::Loft(SolidHistoryLoft {
                    base: SolidHistoryNodeBase::new(1),
                    cross_sections: vec![spline(2.0), polyline(4.0)],
                    guides: vec![line(0.0)],
                    ..Default::default()
                }),
            )
        },
        history_entities,
        3,
    );
}

#[test]
fn a_revolved_history_step_keeps_its_profile() {
    check(
        "ACSH_REVOLVE_CLASS",
        |document| {
            history(
                document,
                SolidHistoryOperation::Revolve(SolidHistoryRevolve {
                    base: SolidHistoryNodeBase::new(1),
                    sweep_entity: Some(spline(2.0)),
                    ..Default::default()
                }),
            )
        },
        history_entities,
        1,
    );
}
