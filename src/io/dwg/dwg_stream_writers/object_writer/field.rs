use crate::io::dwg::dwg_reference_type::DwgReferenceType;
use crate::objects::{Field, FieldList};

use super::DwgObjectWriter;

impl<'a> DwgObjectWriter<'a> {
    pub(super) fn write_field_object(&mut self, value: &Field) {
        let type_code = self.class_type_code("FIELD", 500);
        let eed = value
            .xdata
            .records()
            .iter()
            .filter_map(|rec| self.encode_xdata_record(rec))
            .collect();
        self.write_common_non_entity_data_eed(
            type_code,
            value.handle,
            value.owner,
            &[],
            &None,
            eed,
        );
        self.writer.write_variable_text(&value.evaluator_id);
        self.writer.write_variable_text(&value.code);
        self.writer.write_bit_long(value.child_fields.len() as i32);
        self.writer
            .write_bit_long(value.referenced_objects.len() as i32);
        if self.version.r2007_pre() {
            self.writer.write_variable_text(value.pre2007_format());
        }
        self.writer.write_bit_long(value.evaluation_option);
        self.writer.write_bit_long(value.filing_option);
        self.writer.write_bit_long(value.state);
        self.writer.write_bit_long(value.evaluation_status);
        self.writer.write_bit_long(value.evaluation_error_code);
        self.writer
            .write_variable_text(&value.evaluation_error_message);
        self.write_table_cad_value(&value.value);
        self.writer.write_variable_text(&value.value_string);
        self.writer.write_bit_long(value.value_string_length);
        self.writer.write_bit_long(value.child_values.len() as i32);
        for item in &value.child_values {
            self.writer.write_variable_text(&item.key);
            self.write_table_cad_value(&item.value);
        }
        for handle in &value.child_fields {
            self.writer
                .write_handle(DwgReferenceType::HardOwnership, handle.value());
        }
        // Soft pointers (DXF group 331), as AutoCAD stores them: as hard pointers it discards the
        // field.
        for handle in &value.referenced_objects {
            self.writer
                .write_handle(DwgReferenceType::SoftPointer, handle.value());
        }
        self.register_object(value.handle);
    }

    pub(super) fn write_field_list(&mut self, value: &FieldList) {
        let type_code = self.class_type_code("FIELDLIST", 500);
        self.write_common_non_entity_data(type_code, value.handle, value.owner, &[], &None);
        self.writer.write_bit_long(value.fields.len() as i32);
        self.writer.write_bit(value.unknown);
        for handle in &value.fields {
            self.writer
                .write_handle(DwgReferenceType::SoftPointer, handle.value());
        }
        self.register_object(value.handle);
    }
}

#[cfg(test)]
mod field_reference_tests {
    use crate::document::CadDocument;
    use crate::io::dwg::dwg_reference_type::DwgReferenceType;
    use crate::io::dwg::dwg_stream_readers::object_reader::DwgObjectReader;
    use crate::io::dwg::dwg_stream_writers::DwgObjectWriter;
    use crate::objects::{Field, ObjectType};
    use crate::types::{DxfVersion, Handle};

    /// The reference types of the handles after the common data of a FIELD with one child field
    /// and one referenced object. The record is written natively (an unchanged DWG read would
    /// reuse the raw record).
    fn stored_references(version: DxfVersion) -> Vec<DwgReferenceType> {
        let mut document = CadDocument::with_version(version);
        document
            .classes
            .add_or_update(crate::classes::DxfClass::new("FIELD", "AcDbField"));
        let handle = document.allocate_handle();
        let field = Field {
            handle,
            owner: document.header.named_objects_dict_handle,
            evaluator_id: "AcObjProp".into(),
            code: "%<\\AcObjProp Object(%<\\_ObjIdx 0>%).Area>%".into(),
            child_fields: vec![Handle::new(0x7001)],
            referenced_objects: vec![Handle::new(0x7002)],
            ..Default::default()
        };
        document.objects.insert(handle, ObjectType::Field(field));
        let (bytes, handles, _, _) = DwgObjectWriter::new(&document).unwrap().write();
        let at = handles
            .iter()
            .find(|(h, _)| *h == handle.value())
            .map(|(_, at)| i64::from(*at))
            .expect("the field is written");
        let reader = DwgObjectReader::new(
            bytes,
            version,
            handles
                .into_iter()
                .map(|(h, at)| (h, i64::from(at)))
                .collect(),
        )
        .unwrap();
        let (type_code, mut merged) = reader.read_record_at(at as usize).unwrap();
        reader.read_common_non_entity_data(&mut merged, type_code);
        (0..2).map(|_| merged.read_typed_handle().1).collect()
    }

    /// AutoCAD stores the child fields of a FIELD as hard owners (DXF 360) and the referenced
    /// objects as soft pointers (DXF 331). With hard pointers for the objects it discards the
    /// field ("error 34, eWrongObjectType"): 2,019 AcObjProp fields of a real drawing.
    #[test]
    fn the_referenced_objects_of_a_field_are_soft_pointers() {
        for version in [DxfVersion::AC1018, DxfVersion::AC1032] {
            assert_eq!(
                stored_references(version),
                vec![
                    DwgReferenceType::HardOwnership,
                    DwgReferenceType::SoftPointer
                ],
                "{version:?}"
            );
        }
    }
}
