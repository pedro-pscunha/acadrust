//! The cached values of a FIELD survive a DXF declared as any release. A host can write the DXF
//! as R2000 and raise the version of the document before the DWG write.

use std::io::Cursor;

use acadrust::classes::DxfClass;
use acadrust::entities::table::{CellValue, CellValueType, ValueUnitType};
use acadrust::objects::{Field, FieldChildValue, ObjectType};
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn text_value(text: &str, flags: i32) -> CellValue {
    let mut value = CellValue::new();
    value.value_type = CellValueType::String;
    value.raw_type_code = 4;
    value.flags = flags;
    value.text = text.to_string();
    value.unit_type = ValueUnitType::Distance;
    value.raw_unit_type_code = 1;
    value.format = "%lu2%pr2".to_string();
    value.formatted_value = format!("shown {text}");
    value
}

fn document() -> (CadDocument, Handle) {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    document
        .classes
        .add_or_update(DxfClass::new("FIELD", "AcDbField"));
    let handle = document.allocate_handle();
    let owner = document.header.named_objects_dict_handle;
    let mut number = CellValue::new();
    number.value_type = CellValueType::Double;
    number.raw_type_code = 2;
    number.numeric_value = 12.5;
    number.unit_type = ValueUnitType::Area;
    number.raw_unit_type_code = 4;
    number.format = "%lu6".to_string();
    number.formatted_value = "12.50".to_string();
    document.objects.insert(
        handle,
        ObjectType::Field(Field {
            handle,
            owner,
            evaluator_id: "AcObjProp".to_string(),
            code: "%<\\AcObjProp Object(%<\\_ObjId 1>%).Area>%".to_string(),
            value: text_value("cached", 0),
            child_values: vec![FieldChildValue {
                key: "ObjectPropertyValue".to_string(),
                value: number,
            }],
            value_string: "cached".to_string(),
            value_string_length: 6,
            ..Default::default()
        }),
    );
    (document, handle)
}

fn field(document: &CadDocument, handle: Handle) -> Field {
    let Some(ObjectType::Field(value)) = document.objects.get(&handle) else {
        panic!("the field is missing");
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
fn the_cached_values_survive_a_dxf_declared_as_any_release() {
    let (source, handle) = document();
    let read = through_dwg(&source);
    let wanted = field(&read, handle);
    assert_eq!(wanted.value.text, "cached", "the DWG keeps the value");
    assert_eq!(wanted.child_values.len(), 1);
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
        assert_eq!(field(&from_dxf, handle), wanted, "{declared:?}: DXF");
        from_dxf.version = DxfVersion::AC1032;
        assert_eq!(
            field(&through_dwg(&from_dxf), handle),
            wanted,
            "{declared:?}: DWG"
        );
    }
}
