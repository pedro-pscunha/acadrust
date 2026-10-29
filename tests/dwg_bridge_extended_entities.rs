//! Entities the DWG writer already writes, kept through the DXF bridge a DWG save goes through
//! (DXF text, DXF reader, DWG writer): an ARCALIGNEDTEXT and a LARGE_RADIAL_DIMENSION (a jogged
//! radius dimension) are in the DWG read back, with their data. This guards the bridge: a host
//! that lists them as "not written to DWG" is wrong.

use std::io::Cursor;

use acadrust::entities::{
    ArcAlignedTextData, Dimension, DimensionLargeRadial, EntityCommon, ExtendedEntity,
    ExtendedEntityData,
};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType, Handle};

fn arc_text() -> ArcAlignedTextData {
    ArcAlignedTextData {
        text: "Arc text".into(),
        font_name: "txt.shx".into(),
        big_font_name: String::new(),
        style_name: "Standard".into(),
        center: Vector3::new(55., 30., 0.),
        radius: 25.,
        x_scale: 1.25,
        text_size: 4.5,
        character_spacing: 0.125,
        offset_from_arc: -2.5,
        right_offset: 0.75,
        left_offset: 1.125,
        start_angle: 30_f64.to_radians(),
        end_angle: 210_f64.to_radians(),
        reverse: true,
        text_direction: 1,
        alignment: 2,
        text_position: 1,
        bold: false,
        italic: false,
        underlined: false,
        character_set: 0,
        pitch_and_family: 0,
        is_shx: true,
        text_color: 7,
        normal: Vector3::UNIT_Z,
        wizard_flag: false,
        arc_handle: Handle::NULL,
    }
}

fn large_radial() -> DimensionLargeRadial {
    let mut d = DimensionLargeRadial::default();
    d.definition_point = Vector3::new(10., 0., 0.);
    d.chord_point = Vector3::new(0., 10., 0.);
    d.base.definition_point = d.definition_point;
    d.base.text_middle_point = Vector3::new(4., 6., 0.);
    d.override_center = Vector3::new(2., 2., 0.);
    d.jog_point = Vector3::new(5., 5., 0.);
    d.jog_angle = 45_f64.to_radians();
    d
}

#[test]
fn arc_aligned_text_and_large_radial_dimension_survive_the_bridge() {
    for version in [DxfVersion::AC1018, DxfVersion::AC1032] {
        let mut doc = CadDocument::with_version(version);
        doc.add_entity(EntityType::Extended(Box::new(ExtendedEntity {
            common: EntityCommon::default(),
            data: ExtendedEntityData::ArcAlignedText(arc_text()),
        })))
        .unwrap();
        doc.add_entity(EntityType::Dimension(Box::new(Dimension::LargeRadial(
            large_radial(),
        ))))
        .unwrap();

        let dxf = DxfWriter::new(&doc).write_to_vec().expect("write dxf");
        let bridged = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        let dwg = DwgWriter::write_to_vec(&bridged).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(dwg))
            .read()
            .expect("read dwg");

        let arcs: Vec<_> = back
            .entities()
            .filter_map(|e| match e {
                EntityType::Extended(x) => match &x.data {
                    ExtendedEntityData::ArcAlignedText(a) => Some(a),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 1, "{version:?}: ARCALIGNEDTEXT");
        assert_eq!(arcs[0].text, "Arc text", "{version:?}");
        assert_eq!(arcs[0].center, Vector3::new(55., 30., 0.), "{version:?}");
        assert!((arcs[0].start_angle - 30_f64.to_radians()).abs() < 1e-9);

        let radials: Vec<_> = back
            .entities()
            .filter_map(|e| match e {
                EntityType::Dimension(d) => match d.as_ref() {
                    Dimension::LargeRadial(r) => Some(r),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(radials.len(), 1, "{version:?}: LARGE_RADIAL_DIMENSION");
        let r = radials[0];
        assert_eq!(r.chord_point, Vector3::new(0., 10., 0.), "{version:?}");
        assert_eq!(r.override_center, Vector3::new(2., 2., 0.), "{version:?}");
        assert_eq!(r.jog_point, Vector3::new(5., 5., 0.), "{version:?}");
        assert!(
            (r.jog_angle - 45_f64.to_radians()).abs() < 1e-9,
            "{version:?}"
        );
    }
}
