//! An IMAGEDEF_REACTOR names the image it belongs to twice, as its owner (the first group 330) and
//! after the subclass marker (the second 330). The DXF reader took both groups for the owner, so
//! the second, which the writer left null for a reactor read from a DWG, wiped the owner out, and
//! a drawing that went DWG, DXF, DWG came back with every reactor owned by nothing.

use std::io::Cursor;

use acadrust::entities::Line;
use acadrust::objects::{ImageDefinitionReactor, ObjectType};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType, Handle};

/// A document with a reactor owned by an entity (a line stands in for the image).
fn document(version: DxfVersion) -> (CadDocument, Handle, Handle) {
    let mut doc = CadDocument::with_version(version);
    let mut line = Line::from_coords(0.0, 0.0, 0.0, 1.0, 1.0, 0.0);
    line.common.handle = doc.allocate_handle();
    let image = line.common.handle;
    doc.add_entity(EntityType::Line(line)).expect("add entity");
    let reactor = doc.allocate_handle();
    doc.objects.insert(
        reactor,
        ObjectType::ImageDefinitionReactor(ImageDefinitionReactor {
            handle: reactor,
            owner: image,
            image_handle: image,
        }),
    );
    (doc, reactor, image)
}

fn owner_of(doc: &CadDocument, reactor: Handle) -> Handle {
    match doc.objects.get(&reactor) {
        Some(ObjectType::ImageDefinitionReactor(r)) => r.owner,
        other => panic!("no reactor {reactor:?}: {other:?}"),
    }
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let dwg = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(dwg))
        .read()
        .expect("read dwg")
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let dxf = DxfWriter::new(doc).write_to_vec().expect("write dxf");
    DxfReader::from_reader(Cursor::new(dxf))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn a_dwg_keeps_the_owner_of_a_reactor() {
    let (doc, reactor, image) = document(DxfVersion::AC1032);
    assert_eq!(owner_of(&through_dwg(&doc), reactor), image);
}

#[test]
fn the_dxf_names_the_image_in_both_groups() {
    let (doc, reactor, image) = document(DxfVersion::AC1032);
    let read = through_dwg(&doc);
    let dxf =
        String::from_utf8(DxfWriter::new(&read).write_to_vec().expect("write dxf")).expect("utf-8");
    let lines: Vec<&str> = dxf.lines().map(str::trim).collect();
    let at = lines
        .windows(2)
        .position(|w| w[0] == "5" && w[1] == format!("{:X}", reactor.value()))
        .expect("reactor record");
    let record: Vec<&str> = lines[at..]
        .chunks(2)
        .take_while(|pair| pair[0] != "0")
        .map(|pair| pair[1])
        .collect();
    let image = format!("{:X}", image.value());
    assert_eq!(
        record.iter().filter(|v| **v == image).count(),
        2,
        "the owner and the image in {record:?}"
    );
}

#[test]
fn the_owner_of_a_reactor_survives_dwg_dxf_dwg() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1032] {
        let (doc, reactor, image) = document(version);
        let bridged = through_dxf(&through_dwg(&doc));
        assert_eq!(owner_of(&bridged, reactor), image, "{version:?}: DXF");
        assert_eq!(
            owner_of(&through_dwg(&bridged), reactor),
            image,
            "{version:?}: DWG"
        );
    }
}
