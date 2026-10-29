//! A proxy object names its class by the position of the class in the CLASSES section of the
//! file (group 91: the first class is 500, the next 501, and so on). A document registers its
//! default classes first, so the number of a class in the document is another number than its
//! position in the file. The proxy must keep naming the same class.

use std::io::Cursor;

use acadrust::objects::{ObjectType, ProxyObject};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const NAME: &str = "ACDBASSOCPERSSUBENTMANAGER";

fn dxf(proxy_class: i32) -> Vec<u8> {
    let mut pairs: Vec<(i32, String)> = vec![
        (0, "SECTION".into()),
        (2, "HEADER".into()),
        (9, "$ACADVER".into()),
        (1, "AC1032".into()),
        (0, "ENDSEC".into()),
        (0, "SECTION".into()),
        (2, "CLASSES".into()),
    ];
    for (dxf_name, cpp_name) in [
        ("ZZ_CLASS_A", "ZzClassA"),
        ("ZZ_CLASS_B", "ZzClassB"),
        (NAME, "AcDbAssocPersSubentManager"),
    ] {
        pairs.extend([
            (0, "CLASS".into()),
            (1, dxf_name.into()),
            (2, cpp_name.into()),
            (3, "ObjectDBX Classes".into()),
            (90, "0".into()),
            (91, "1".into()),
            (280, "0".into()),
            (281, "0".into()),
        ]);
    }
    pairs.extend([
        (0, "ENDSEC".into()),
        (0, "SECTION".into()),
        (2, "OBJECTS".into()),
        (0, "ACAD_PROXY_OBJECT".into()),
        (5, "ABCD".into()),
        (330, "C".into()),
        (100, "AcDbProxyObject".into()),
        (90, "499".into()),
        (91, proxy_class.to_string()),
        (0, "ENDSEC".into()),
        (0, "EOF".into()),
    ]);
    pairs
        .into_iter()
        .map(|(code, value)| format!("{code}\n{value}\n"))
        .collect::<String>()
        .into_bytes()
}

fn proxy(document: &CadDocument) -> &ProxyObject {
    document
        .objects
        .values()
        .find_map(|object| match object {
            ObjectType::ProxyObject(proxy) => Some(proxy),
            _ => None,
        })
        .expect("the proxy object")
}

fn class_number(document: &CadDocument) -> i32 {
    i32::from(
        document
            .classes
            .get_by_name(NAME)
            .expect("the class is registered")
            .class_number,
    )
}

#[test]
fn a_proxy_keeps_the_class_the_file_names_by_position() {
    let document = DxfReader::from_reader(Cursor::new(dxf(502)))
        .expect("reader")
        .read()
        .expect("read dxf");
    let number = class_number(&document);
    assert_ne!(number, 502, "the document numbers its default classes first");
    assert_eq!(proxy(&document).class_id, number, "after the DXF read");

    let bytes = DwgWriter::write_to_vec(&document).expect("write dwg");
    let read = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let read_proxy = proxy(&read);
    assert_eq!(read_proxy.dxf_subclass, NAME, "after the DWG round trip");
    assert_eq!(read_proxy.class_id, class_number(&read));
}

#[test]
fn a_proxy_keeps_its_class_through_the_dxf_writer() {
    let document = DxfReader::from_reader(Cursor::new(dxf(502)))
        .expect("reader")
        .read()
        .expect("read dxf");
    let bytes = DxfWriter::new(&document).write_to_vec().expect("write dxf");
    let read = DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read written dxf");
    assert_eq!(proxy(&read).class_id, class_number(&read));
}

#[test]
fn a_proxy_whose_number_names_another_class_is_not_written() {
    let mut document = DxfReader::from_reader(Cursor::new(dxf(502)))
        .expect("reader")
        .read()
        .expect("read dxf");
    let handle = proxy(&document).handle;
    let Some(ObjectType::ProxyObject(object)) = document.objects.get_mut(&handle) else {
        panic!("the proxy object");
    };
    object.dxf_subclass = NAME.to_string();
    object.class_id = 502;
    assert!(DwgWriter::write_to_vec(&document).is_err());
}

#[test]
fn a_proxy_naming_no_class_of_the_file_fails_the_read() {
    let result = DxfReader::from_reader(Cursor::new(dxf(503)))
        .expect("reader")
        .read();
    assert!(result.is_err(), "position 503 names no class");
}
