//! A dimension's group 12 point (`insertion_point`) is an OCS point of the entity's frame; a DWG stores the X and Y of its WCS point.
//! AutoCAD measured: normal (0, 0.6, 0.8) and OCS point (4, 5, 6) are stored as WCS (-4, -0.4) and read back as OCS (4, 0.32, -0.24).

use std::io::Cursor;

use acadrust::entities::{Dimension, DimensionAligned, EntityType};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter};

fn through_dwg(normal: Vector3, point: Vector3, version: DxfVersion) -> Vector3 {
    let mut dimension =
        DimensionAligned::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0));
    dimension.base.normal = normal;
    dimension.base.insertion_point = point;
    let mut document = CadDocument::with_version(version);
    document
        .add_entity(EntityType::Dimension(Box::new(Dimension::Aligned(
            dimension,
        ))))
        .unwrap();
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let read = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let point = read
        .entities()
        .find_map(|e| match e {
            EntityType::Dimension(d) => Some(d.base().insertion_point),
            _ => None,
        })
        .expect("the dimension");
    point
}

fn close(a: Vector3, b: Vector3) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9 && (a.z - b.z).abs() < 1e-9
}

#[test]
fn the_clone_point_of_a_tilted_dimension_is_converted_between_the_frames() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1027] {
        let read = through_dwg(
            Vector3::new(0.0, 0.6, 0.8),
            Vector3::new(4.0, 5.0, 6.0),
            version,
        );
        assert!(
            close(read, Vector3::new(4.0, 0.32, -0.24)),
            "{version:?}: {read:?}"
        );
    }
}

#[test]
fn the_clone_point_of_a_dimension_in_the_world_frame_is_unchanged() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1027] {
        let read = through_dwg(
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(4.0, 5.0, 0.0),
            version,
        );
        assert!(
            close(read, Vector3::new(4.0, 5.0, 0.0)),
            "{version:?}: {read:?}"
        );
    }
}

#[test]
fn the_clone_point_of_a_dimension_in_the_flipped_world_frame_survives() {
    // Normal (0, 0, -1) has the X axis (-1, 0, 0): the stored WCS X is the OCS X negated, and it reads back as the OCS point.
    for version in [DxfVersion::AC1015, DxfVersion::AC1027] {
        let read = through_dwg(
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(4.0, 5.0, 0.0),
            version,
        );
        assert!(
            close(read, Vector3::new(4.0, 5.0, 0.0)),
            "{version:?}: {read:?}"
        );
    }
}
