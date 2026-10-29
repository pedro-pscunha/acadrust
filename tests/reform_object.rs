//! `reform_object` rewrites one raw record from the form of one release to the form of another.

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
use acadrust::{reform_object, CadDocument, DxfWriter};

type Tags = Vec<(i32, String)>;

fn fixture(name: &str) -> Tags {
    let path = format!(
        "{}/tests/fixtures/reform/{name}.dxf-records",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).expect("fixture");
    let mut lines = text.lines();
    let mut tags = Vec::new();
    while let Some(code) = lines.next() {
        let value = lines.next().expect("value");
        tags.push((code.trim().parse().expect("code"), value.to_string()));
    }
    tags
}

/// The groups differ only in number formatting and padding.
fn same(left: &Tags, right: &Tags) -> Result<(), String> {
    let number = |value: &str| value.trim().parse::<f64>().ok();
    for (index, (a, b)) in left.iter().zip(right).enumerate() {
        let equal = a.0 == b.0
            && (a.1.trim() == b.1.trim()
                || matches!((number(&a.1), number(&b.1)), (Some(x), Some(y)) if (x - y).abs() <= 1e-9 * x.abs().max(1.0)));
        if !equal {
            return Err(format!("group {index}: {a:?} against {b:?}"));
        }
    }
    if left.len() != right.len() {
        return Err(format!("{} groups against {}", left.len(), right.len()));
    }
    Ok(())
}

/// Without the `102` blocks (reactors, extension dictionary) and the XDATA, which AutoCAD writes
/// differently for the two releases of one file.
fn core(tags: &Tags) -> Tags {
    let mut out = Tags::new();
    let mut inside = false;
    for tag in tags {
        if tag.0 == 1001 {
            break;
        }
        if tag.0 == 102 {
            inside = tag.1.starts_with('{');
            continue;
        }
        if !inside {
            out.push(tag.clone());
        }
    }
    out
}

#[test]
fn a_constraint_group_follows_the_release_like_autocad() {
    let kind = "ACDBASSOC2DCONSTRAINTGROUP";
    let (v2010, v2018) = (
        fixture("constraint-group-2010"),
        fixture("constraint-group-2018"),
    );
    let down = reform_object(kind, &v2018, DxfVersion::AC1032, DxfVersion::AC1024).unwrap();
    same(&core(&down), &core(&v2010)).unwrap();
    let up = reform_object(kind, &v2010, DxfVersion::AC1024, DxfVersion::AC1032).unwrap();
    same(&core(&up), &core(&v2018)).unwrap();
}

/// The extended properties of a long visual style that AutoCAD fills in on an upgrade are its
/// defaults, which the reader fills in too, so the records are equal.
#[test]
fn a_visual_style_follows_the_release_like_autocad() {
    for name in ["visualstyle-a", "visualstyle-b"] {
        let (v2010, v2018) = (
            fixture(&format!("{name}-2010")),
            fixture(&format!("{name}-2018")),
        );
        let down = reform_object(
            "VISUALSTYLE",
            &v2018,
            DxfVersion::AC1032,
            DxfVersion::AC1024,
        )
        .unwrap();
        same(&core(&down), &core(&v2010))
            .unwrap_or_else(|error| panic!("{name} 2018 to 2010: {error}"));
        let up = reform_object(
            "VISUALSTYLE",
            &v2010,
            DxfVersion::AC1024,
            DxfVersion::AC1032,
        )
        .unwrap();
        same(&core(&up), &core(&v2018))
            .unwrap_or_else(|error| panic!("{name} 2010 to 2018: {error}"));
    }
}

#[test]
fn another_kind_is_not_reformed() {
    assert!(reform_object(
        "LINE",
        &[(0, "LINE".into())],
        DxfVersion::AC1032,
        DxfVersion::AC1024
    )
    .is_none());
}

// ---- embedded entities --------------------------------------------------------------------

fn line(offset: f64) -> EmbeddedEntity {
    EmbeddedEntity::Line(Line::from_points(
        Vector3::new(offset, 0.0, 0.0),
        Vector3::new(offset + 5.0, 2.5, 0.0),
    ))
}

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

fn history(document: &mut CadDocument, operation: SolidHistoryOperation) {
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .expect("add solid");
    document
        .create_solid_history(entity, operation)
        .expect("create history");
}

/// Every record kind with embedded entities, built in a document of the given release.
fn documents(version: DxfVersion) -> Vec<(&'static str, CadDocument)> {
    let mut out = Vec::new();
    for profile in [polyline as fn(f64) -> EmbeddedEntity, spline] {
        let mut kinds: Vec<(&'static str, CadDocument)> = Vec::new();
        let mut document = CadDocument::with_version(version);
        let mut value = Surface::new(SurfaceKind::Extruded);
        if let SurfaceData::Extruded {
            sweep_entity,
            sweep_vector,
            ..
        } = &mut value.surface_data
        {
            *sweep_entity = Some(profile(3.0));
            *sweep_vector = Vector3::new(0.0, 0.0, 4.0);
        }
        surface(&mut document, value);
        kinds.push(("EXTRUDEDSURFACE", document));

        let mut document = CadDocument::with_version(version);
        let mut value = Surface::new(SurfaceKind::Lofted);
        if let SurfaceData::Lofted {
            cross_section_entities,
            guide_entities,
            ..
        } = &mut value.surface_data
        {
            *cross_section_entities = vec![profile(2.0), profile(4.0)];
            *guide_entities = vec![line(0.0)];
        }
        surface(&mut document, value);
        kinds.push(("LOFTEDSURFACE", document));

        let mut document = CadDocument::with_version(version);
        let mut value = Surface::new(SurfaceKind::Revolved);
        if let SurfaceData::Revolved {
            revolve_entity,
            axis_vector,
            ..
        } = &mut value.surface_data
        {
            *revolve_entity = Some(profile(1.0));
            *axis_vector = Vector3::UNIT_Z;
        }
        surface(&mut document, value);
        kinds.push(("REVOLVEDSURFACE", document));

        let mut document = CadDocument::with_version(version);
        let mut value = Surface::new(SurfaceKind::Swept);
        if let SurfaceData::Swept {
            sweep_entity,
            path_entity,
            ..
        } = &mut value.surface_data
        {
            *sweep_entity = Some(profile(1.5));
            *path_entity = Some(line(0.0));
        }
        surface(&mut document, value);
        kinds.push(("SWEPTSURFACE", document));

        let mut document = CadDocument::with_version(version);
        history(
            &mut document,
            SolidHistoryOperation::Sweep(SolidHistorySweep {
                base: SolidHistoryNodeBase::new(1),
                sweep_entity: Some(profile(1.5)),
                path_entity: Some(line(0.0)),
                ..Default::default()
            }),
        );
        kinds.push(("ACSH_SWEEP_CLASS", document));

        let mut document = CadDocument::with_version(version);
        history(
            &mut document,
            SolidHistoryOperation::Extrusion(SolidHistorySweep {
                base: SolidHistoryNodeBase::new(1),
                sweep_entity: Some(profile(2.5)),
                ..Default::default()
            }),
        );
        kinds.push(("ACSH_EXTRUSION_CLASS", document));

        let mut document = CadDocument::with_version(version);
        history(
            &mut document,
            SolidHistoryOperation::Loft(SolidHistoryLoft {
                base: SolidHistoryNodeBase::new(1),
                cross_sections: vec![profile(2.0), profile(4.0)],
                guides: vec![line(0.0)],
                ..Default::default()
            }),
        );
        kinds.push(("ACSH_LOFT_CLASS", document));

        let mut document = CadDocument::with_version(version);
        history(
            &mut document,
            SolidHistoryOperation::Revolve(SolidHistoryRevolve {
                base: SolidHistoryNodeBase::new(1),
                sweep_entity: Some(profile(2.0)),
                ..Default::default()
            }),
        );
        kinds.push(("ACSH_REVOLVE_CLASS", document));
        out.extend(kinds);
    }
    out
}

/// The first record of `kind` in the DXF a document gives at a release.
fn record(document: &CadDocument, version: DxfVersion, kind: &str) -> Tags {
    let bytes = DxfWriter::new(document)
        .with_version(version)
        .write_to_vec()
        .expect("write dxf");
    let text = String::from_utf8(bytes).expect("utf8");
    let mut lines = text.lines();
    let mut tags = Tags::new();
    let mut inside = false;
    while let Some(code) = lines.next() {
        let value = lines.next().expect("value");
        let code: i32 = code.trim().parse().expect("code");
        if code == 0 {
            if inside {
                break;
            }
            inside = value == kind;
        }
        if inside {
            tags.push((code, value.to_string()));
        }
    }
    assert!(!tags.is_empty(), "{kind} is not in the DXF at {version:?}");
    tags
}

#[test]
fn embedded_entities_follow_the_release() {
    for (kind, document) in documents(DxfVersion::AC1027) {
        let from = record(&document, DxfVersion::AC1027, kind);
        let native_target = record(&document, DxfVersion::AC1018, kind);
        // The bytes are decoded with the layout of R2013 and encoded with that of R2004: the
        // record is the one the writer gives at R2004.
        let down = reform_object(kind, &from, DxfVersion::AC1027, DxfVersion::AC1018)
            .unwrap_or_else(|| panic!("{kind} is reformed"));
        assert_ne!(down, from, "{kind}: the bytes change with the layout");
        same(&down, &native_target)
            .unwrap_or_else(|error| panic!("{kind} R2013 to R2004: {error}"));
        // Back up and down again keeps the R2004 record (what R2004 cannot hold is already gone).
        let up = reform_object(kind, &down, DxfVersion::AC1018, DxfVersion::AC1027).unwrap();
        let again = reform_object(kind, &up, DxfVersion::AC1027, DxfVersion::AC1018).unwrap();
        same(&again, &down).unwrap_or_else(|error| panic!("{kind} back and forth: {error}"));
    }
}
