//! The class table names the rotate action of a dynamic block `AcDbBlockRotateAction`, as AutoCAD's own CLASS record does; its objects
//! carry the subclass marker `AcDbBlockRotationAction`, a different name. A class table with the marker's name makes AutoCAD load the
//! object as a proxy.

use acadrust::classes::DxfClassCollection;

#[test]
fn the_rotate_action_class_has_the_cpp_name_autocad_gives_it() {
    let mut classes = DxfClassCollection::new();
    classes.update_defaults();
    let class = classes.get_by_name("BLOCKROTATEACTION").expect("the class");
    assert_eq!(class.cpp_class_name, "AcDbBlockRotateAction");
}

/// A class record read from a DXF replaces the default of the same name: its proxy flags, application and C++ name are the file's.
#[test]
fn a_class_of_the_file_takes_the_place_of_the_default() {
    use acadrust::{types::DxfVersion, CadDocument, DxfReader, DxfWriter};
    use std::io::Cursor;

    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    document.classes.clear();
    let mut class = acadrust::classes::DxfClass::new("TABLESTYLE", "AcDbTableStyle");
    class.proxy_flags = acadrust::classes::ProxyFlags(4095);
    class.application_name = "ObjectDBX Classes".into();
    document.classes.add_or_update(class);
    let text = DxfWriter::new(&document).write_to_vec().expect("write");
    let back = DxfReader::from_reader(Cursor::new(text))
        .expect("open")
        .read()
        .expect("read");
    let class = back.classes.get_by_name("TABLESTYLE").expect("class");
    assert_eq!(class.proxy_flags.0, 4095);
}
