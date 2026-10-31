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
