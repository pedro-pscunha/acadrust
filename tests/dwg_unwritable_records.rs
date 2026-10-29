//! `DwgWriter::write_to_vec_omitting_unwritable` leaves out a record it cannot write instead of
//! failing the save. A DXF has no DWG bytes for an AutoCAD Architecture entity or object (or for
//! any record of a class acadrust does not decode), and bytes of another DWG version do not fit
//! the one being written; `write_to_vec` fails on the first such record so that DWG data does not
//! disappear unnoticed, and the omitting variant says which records it left out.

use std::io::Cursor;

use acadrust::classes::arch::ArchObject;
use acadrust::entities::{EntityType, ExtendedEntityData, Line};
use acadrust::objects::ObjectType;
use acadrust::types::{DxfVersion, Handle, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader};

/// A DXF with a line, an AutoCAD Architecture entity and an AutoCAD Architecture object: each
/// has DXF groups only.
const SOURCE: &str = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n\
0\nLINE\n5\n100\n8\n0\n10\n0.0\n20\n0.0\n30\n0.0\n11\n5.0\n21\n5.0\n31\n0.0\n\
0\nAECBWALL\n5\n101\n8\n0\n90\n1\n\
0\nENDSEC\n\
0\nSECTION\n2\nOBJECTS\n\
0\nAEC_WALL_STYLE\n5\n200\n330\n0\n90\n1\n\
0\nENDSEC\n0\nEOF\n";

fn source() -> CadDocument {
    DxfReader::from_reader(Cursor::new(SOURCE.as_bytes().to_vec()))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn arch_entities(doc: &CadDocument) -> usize {
    doc.entities()
        .filter(|e| {
            matches!(e, EntityType::Extended(x) if matches!(x.data, ExtendedEntityData::Arch(_)))
        })
        .count()
}

fn arch_objects(doc: &CadDocument) -> usize {
    doc.objects
        .values()
        .filter(|o| matches!(o, ObjectType::Arch(_)))
        .count()
}

#[test]
fn records_that_came_from_dxf_are_left_out_and_reported() {
    let doc = source();
    assert_eq!((arch_entities(&doc), arch_objects(&doc)), (1, 1));
    assert!(
        DwgWriter::write_to_vec(&doc).is_err(),
        "the plain write still fails"
    );

    let (bytes, mut omitted) = DwgWriter::write_to_vec_omitting_unwritable(&doc)
        .expect("one unwritable record stopped the save");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    assert_eq!((arch_entities(&back), arch_objects(&back)), (0, 0));
    assert_eq!(
        back.entities()
            .filter(|e| matches!(e, EntityType::Line(_)))
            .count(),
        1,
        "the rest of the drawing is written"
    );

    omitted.sort_by_key(|o| o.handle);
    assert_eq!(omitted, {
        let mut report = DwgWriter::omitted_records(&doc);
        report.sort_by_key(|o| o.handle);
        report
    });
    let names: Vec<&str> = omitted.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["AECBWALL", "AEC_WALL_STYLE"]);
    assert!(omitted.iter().all(|o| !o.reason.is_empty()));
}

#[test]
fn bytes_of_another_dwg_version_are_left_out_too() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1027);
    let handle = doc.allocate_handle();
    let mut object = ArchObject::new(
        "AEC_WALL_STYLE",
        handle,
        doc.header.named_objects_dict_handle,
    );
    object.raw_dwg_data = Some(vec![0; 16]);
    object.raw_dwg_version = Some(DxfVersion::AC1032);
    doc.objects.insert(handle, ObjectType::Arch(object));

    assert!(DwgWriter::write_to_vec(&doc).is_err());
    let (_, omitted) = DwgWriter::write_to_vec_omitting_unwritable(&doc)
        .expect("a record of another DWG version stopped the save");
    assert_eq!(omitted.len(), 1);
    assert_eq!(omitted[0].handle, handle);
}

#[test]
fn a_drawing_with_nothing_unwritable_reports_nothing() {
    assert!(DwgWriter::omitted_records(&CadDocument::new()).is_empty());
}

/// A line, a layer and a dictionary that name the record the save leaves out as a reactor and as
/// their plot style.
fn referring_to_an_omitted_record() -> (CadDocument, Handle) {
    let mut doc = CadDocument::new();
    let root = doc.header.named_objects_dict_handle;
    let omitted = doc.allocate_handle();
    doc.objects.insert(
        omitted,
        ObjectType::Arch(ArchObject::new("AEC_WALL_STYLE", omitted, root)),
    );
    let mut line = Line::from_points(Vector3::ZERO, Vector3::new(5.0, 5.0, 0.0));
    line.common.reactors = vec![omitted, root];
    line.common.plotstyle_flags = 3;
    line.common.plotstyle_handle = Some(omitted);
    doc.add_entity(EntityType::Line(line)).expect("add line");
    doc.layers
        .iter_mut()
        .next()
        .expect("layer 0")
        .plotstyle_handle = omitted;
    let Some(ObjectType::Dictionary(dictionary)) = doc.objects.get_mut(&root) else {
        panic!("no root dictionary")
    };
    dictionary.reactors = vec![omitted, root];
    (doc, omitted)
}

#[test]
fn what_is_left_names_no_omitted_record() {
    let (doc, omitted) = referring_to_an_omitted_record();
    let (bytes, report) = DwgWriter::write_to_vec_omitting_unwritable(&doc)
        .expect("an omitted record stopped the save");
    assert_eq!(report.len(), 1);
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");

    let line = back
        .entities()
        .find_map(|e| match e {
            EntityType::Line(line) => Some(line),
            _ => None,
        })
        .expect("the line is written");
    assert!(!line.common.reactors.contains(&omitted), "line reactors");
    assert!(
        line.common
            .reactors
            .contains(&doc.header.named_objects_dict_handle),
        "a reactor that is written stays"
    );
    // A plot style by object that lost its object is the plot style of its layer again.
    assert_eq!(
        (line.common.plotstyle_flags, line.common.plotstyle_handle),
        (0, None)
    );
    assert_ne!(
        back.layers.iter().next().expect("layer 0").plotstyle_handle,
        omitted,
        "layer plot style"
    );
    let Some(ObjectType::Dictionary(root)) =
        back.objects.get(&doc.header.named_objects_dict_handle)
    else {
        panic!("no root dictionary")
    };
    assert!(!root.reactors.contains(&omitted), "dictionary reactors");
}
