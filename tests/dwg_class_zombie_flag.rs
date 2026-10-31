//! The zombie flag of a class table record (group code 280) is the file's: AutoCAD keeps it through a save, whether or not
//! an object uses the class. A class with no instance was marked a zombie on a DWG save.

use acadrust::classes::DxfClass;
use acadrust::{types::DxfVersion, CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

fn zombie_flags(zombie: bool) -> bool {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut class = DxfClass::new("SPATIAL_FILTER", "AcDbSpatialFilter");
    class.was_zombie = zombie;
    document.classes.add_from_file(class);
    let bytes = DwgWriter::write_to_vec(&document).expect("write");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read");
    back.classes
        .get_by_name("SPATIAL_FILTER")
        .expect("the class")
        .was_zombie
}

#[test]
fn an_unused_class_keeps_the_zombie_flag_the_file_gave_it() {
    assert!(!zombie_flags(false), "a class that was not a zombie");
    assert!(zombie_flags(true), "a class that was a zombie");
}

/// The flag of a class that objects use: MLEADERSTYLE has an instance in every document.
fn used_class_flag(from_file: Option<bool>) -> (bool, i32) {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    if let Some(zombie) = from_file {
        let mut class = DxfClass::new("MLEADERSTYLE", "AcDbMLeaderStyle");
        class.was_zombie = zombie;
        document.classes.add_from_file(class);
    }
    let bytes = DwgWriter::write_to_vec(&document).expect("write");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read");
    let class = back.classes.get_by_name("MLEADERSTYLE").expect("the class");
    (class.was_zombie, class.instance_count)
}

#[test]
fn a_used_class_keeps_the_zombie_flag_the_file_gave_it() {
    let (zombie, instances) = used_class_flag(Some(true));
    assert!(instances > 0, "the class is used");
    assert!(zombie, "a used class that was a zombie in the file");
    assert!(!used_class_flag(Some(false)).0, "a used class that was not");
}

#[test]
fn a_generated_used_class_is_not_a_zombie() {
    let (zombie, instances) = used_class_flag(None);
    assert!(instances > 0, "the class is used");
    assert!(!zombie);
}
