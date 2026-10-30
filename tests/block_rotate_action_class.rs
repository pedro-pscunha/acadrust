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
