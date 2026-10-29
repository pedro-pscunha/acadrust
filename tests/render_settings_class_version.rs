//! The class version of a render settings object is the one of AutoCAD's DXF (group 90) and of its
//! DWG: an R2018 DWG stores 2 in a MENTALRAYRENDERSETTINGS and in a RAPIDRTRENDERSETTINGS. A save
//! that stored one more made AutoCAD discard the object ("error 67, eDwgObjectImproperlyRead")
//! and AUDIT then removed the ACAD_RENDER_ACTIVE_SETTINGS entry.

use std::io::Cursor;

use acadrust::classes::DxfClass;
use acadrust::objects::{
    ClassObject, ClassObjectData, MentalRayRenderSettings, ObjectType, RapidRtRenderSettings,
    RenderSettings,
};
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn settings(class_version: i32) -> RenderSettings {
    RenderSettings {
        class_version,
        name: "Medium".into(),
        fog_enabled: true,
        backfaces_enabled: true,
        environment_image_enabled: true,
        display_index: 3,
        ..Default::default()
    }
}

/// A drawing with a MENTALRAYRENDERSETTINGS and a RAPIDRTRENDERSETTINGS, both of class version 2.
fn document(version: DxfVersion) -> (CadDocument, Handle, Handle) {
    let mut document = CadDocument::with_version(version);
    for (name, class) in [
        ("MENTALRAYRENDERSETTINGS", "AcDbMentalRayRenderSettings"),
        ("RAPIDRTRENDERSETTINGS", "AcDbRapidRTRenderSettings"),
    ] {
        document.classes.add_or_update(DxfClass::new(name, class));
    }
    let owner = document.header.named_objects_dict_handle;
    let (mental, rapid) = (document.allocate_handle(), document.allocate_handle());
    for (handle, data) in [
        (
            mental,
            ClassObjectData::MentalRayRenderSettings(MentalRayRenderSettings {
                base: settings(2),
                version: 1,
                ..Default::default()
            }),
        ),
        (
            rapid,
            ClassObjectData::RapidRtRenderSettings(RapidRtRenderSettings {
                base: settings(2),
                version: 1,
                ..Default::default()
            }),
        ),
    ] {
        document.objects.insert(
            handle,
            ObjectType::ClassObject(ClassObject {
                handle,
                owner,
                data,
                ..Default::default()
            }),
        );
    }
    (document, mental, rapid)
}

fn class_versions(document: &CadDocument, mental: Handle, rapid: Handle) -> (i32, i32) {
    let version = |handle: Handle| match document.objects.get(&handle) {
        Some(ObjectType::ClassObject(ClassObject {
            data: ClassObjectData::MentalRayRenderSettings(value),
            ..
        })) => value.base.class_version,
        Some(ObjectType::ClassObject(ClassObject {
            data: ClassObjectData::RapidRtRenderSettings(value),
            ..
        })) => value.base.class_version,
        other => panic!("no render settings at {handle:?}: {other:?}"),
    };
    (version(mental), version(rapid))
}

#[test]
fn a_dwg_keeps_the_class_version_of_render_settings() {
    for version in [DxfVersion::AC1027, DxfVersion::AC1032] {
        let (document, mental, rapid) = document(version);
        let mut current = document;
        // Read and write twice: the version must not move on the way.
        for round in 0..2 {
            let bytes = DwgWriter::write_to_vec(&current).expect("write dwg");
            current = DwgReader::from_stream(Cursor::new(bytes))
                .read()
                .expect("read dwg");
            assert_eq!(
                class_versions(&current, mental, rapid),
                (2, 2),
                "{version:?} round {round}"
            );
        }
    }
}

#[test]
fn the_class_version_of_a_dxf_follows_the_dwg() {
    for version in [DxfVersion::AC1027, DxfVersion::AC1032] {
        let (document, mental, rapid) = document(version);
        let dxf = DxfWriter::new(&document).write_to_vec().expect("write dxf");
        let from_dxf = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(
            class_versions(&from_dxf, mental, rapid),
            (2, 2),
            "{version:?}: DXF"
        );
        let bytes = DwgWriter::write_to_vec(&from_dxf).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read dwg");
        assert_eq!(
            class_versions(&back, mental, rapid),
            (2, 2),
            "{version:?}: DWG"
        );
    }
}

/// CADCraft's path: an R2018 DWG is read, written as a bridge DXF of an older release (R2000),
/// read again, its version raised to R2018, and written as a DWG. The class version must be the
/// same on every leg, whatever release the DXF is.
#[test]
fn the_class_version_survives_a_dxf_of_another_release() {
    let (document, mental, rapid) = document(DxfVersion::AC1032);
    let bytes = DwgWriter::write_to_vec(&document).expect("write dwg");
    let read = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    assert_eq!(class_versions(&read, mental, rapid), (2, 2), "DWG read");
    for release in [DxfVersion::AC1015, DxfVersion::AC1027, DxfVersion::AC1032] {
        let dxf = DxfWriter::new(&read)
            .with_version(release)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf");
        let mut from_dxf = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(
            class_versions(&from_dxf, mental, rapid),
            (2, 2),
            "{release:?}: DXF read"
        );
        from_dxf.version = DxfVersion::AC1032;
        let bytes = DwgWriter::write_to_vec(&from_dxf).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read dwg");
        assert_eq!(
            class_versions(&back, mental, rapid),
            (2, 2),
            "{release:?}: DWG written as R2018"
        );
    }
}

/// The DXF that acadrust writes for an R2018 drawing reaches a host that rewrites it as an R2000
/// DXF (its header says AC1015) with the same groups, and acadrust reads that.
#[test]
fn the_class_version_survives_a_host_that_declares_an_older_release() {
    let (document, mental, rapid) = document(DxfVersion::AC1032);
    let bytes = DwgWriter::write_to_vec(&document).expect("write dwg");
    let read = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let dxf = DxfWriter::new(&read)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    let text = String::from_utf8(dxf).expect("utf8");
    assert!(text.contains("AC1032"), "an R2018 DXF");
    let declared = text.replacen("AC1032", "AC1015", 1);
    let mut from_dxf = DxfReader::from_reader(Cursor::new(declared.into_bytes()))
        .expect("reader")
        .read()
        .expect("read dxf");
    assert_eq!(from_dxf.version, DxfVersion::AC1015);
    assert_eq!(class_versions(&from_dxf, mental, rapid), (2, 2), "DXF read");
    from_dxf.version = DxfVersion::AC1032;
    let bytes = DwgWriter::write_to_vec(&from_dxf).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    assert_eq!(class_versions(&back, mental, rapid), (2, 2), "DWG as R2018");
}
