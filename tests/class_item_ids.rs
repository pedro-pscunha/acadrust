//! The class records of a DWG keep their item class id (498 for an entity class, 499 for an
//! object class) through the DXF bridge and back.

use std::io::Cursor;

use acadrust::classes::DxfClass;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const CLASSES: [(&str, &str, i16); 4] = [
    ("AECS_DISP_ENTITY_ONE", "AecsDispEntityOne", 498),
    ("AECS_DISP_ENTITY_TWO", "AecsDispEntityTwo", 498),
    ("AECS_DISP_OBJECT_ONE", "AecsDispObjectOne", 499),
    ("AECS_DISP_OBJECT_TWO", "AecsDispObjectTwo", 499),
];

fn item_ids(document: &CadDocument) -> Vec<(String, i16, bool)> {
    CLASSES
        .iter()
        .map(|(name, _, _)| {
            let class = document.classes.get_by_name(name).expect("the class");
            (name.to_string(), class.item_class_id, class.is_an_entity)
        })
        .collect()
}

fn through_dwg(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn item_class_ids_survive_the_dxf_bridge() {
    for version in [DxfVersion::AC1027, DxfVersion::AC1032] {
        let mut document = CadDocument::with_version(version);
        for (name, cpp, item) in CLASSES {
            let mut class = DxfClass::new(name, cpp);
            class.application_name = "AecArchBase80".to_string();
            class.item_class_id = item;
            class.is_an_entity = item == 498;
            document.classes.add_or_update(class);
        }
        let source = through_dwg(&document);
        let wanted = item_ids(&source);
        for (name, _, item) in CLASSES {
            assert!(
                wanted.contains(&(name.to_string(), item, item == 498)),
                "{version:?}: the DWG keeps {name}"
            );
        }
        for dxf_version in [DxfVersion::AC1015, version] {
            let dxf = DxfWriter::new(&source)
                .with_version(dxf_version)
                .for_dwg_bridge()
                .write_to_vec()
                .expect("write dxf");
            let mut from_dxf = DxfReader::from_reader(Cursor::new(dxf))
                .expect("reader")
                .read()
                .expect("read dxf");
            assert_eq!(
                item_ids(&from_dxf),
                wanted,
                "{version:?}/{dxf_version:?}: DXF"
            );
            from_dxf.version = version;
            assert_eq!(
                item_ids(&through_dwg(&from_dxf)),
                wanted,
                "{version:?}/{dxf_version:?}: DWG"
            );
        }
    }
}
