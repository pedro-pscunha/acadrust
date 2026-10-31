use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, EntityType, Handle};
use std::io::Cursor;

fn source() -> Vec<u8> {
    let boundary = "0\nLWPOLYLINE\n5\nA1\n102\n{ACAD_REACTORS\n330\nB1\n102\n}\n100\nAcDbEntity\n8\n0\n100\nAcDbPolyline\n90\n3\n70\n1\n10\n0\n20\n0\n10\n10\n20\n0\n10\n0\n20\n10\n";
    let hatch = |handle| {
        format!("0\nHATCH\n5\n{handle}\n100\nAcDbEntity\n8\n0\n100\nAcDbHatch\n10\n0\n20\n0\n30\n0\n210\n0\n220\n0\n230\n1\n2\nSOLID\n70\n1\n71\n1\n91\n1\n92\n2\n72\n0\n73\n1\n93\n3\n10\n0\n20\n0\n10\n10\n20\n0\n10\n0\n20\n10\n97\n1\n330\nA1\n75\n0\n76\n1\n98\n0\n")
    };
    format!("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{boundary}{}{}0\nENDSEC\n0\nEOF\n", hatch("B1"), hatch("B2")).into_bytes()
}

fn assert_source_relations(document: &CadDocument, stage: &str) {
    let boundary = document.get_entity(Handle::new(0xA1)).unwrap();
    assert_eq!(
        boundary.common().reactors,
        vec![Handle::new(0xB1)],
        "{stage}"
    );
    for handle in [0xB1, 0xB2] {
        let EntityType::Hatch(hatch) = document.get_entity(Handle::new(handle)).unwrap() else {
            panic!("HATCH")
        };
        assert!(hatch.is_associative);
        assert_eq!(hatch.paths[0].boundary_handles, vec![Handle::new(0xA1)]);
    }
}

#[test]
fn new_and_changed_associations_still_receive_reverse_reactors() {
    let mut document = DxfReader::from_reader(Cursor::new(source()))
        .unwrap()
        .read()
        .unwrap();
    let mut boundary = document.get_entity(Handle::new(0xA1)).unwrap().clone();
    boundary.common_mut().handle = Handle::NULL;
    boundary.common_mut().reactors.clear();
    let new_boundary = document.add_entity(boundary).unwrap();
    let mut hatch = document.get_entity(Handle::new(0xB2)).unwrap().clone();
    hatch.common_mut().handle = Handle::NULL;
    let new_hatch = document.add_entity(hatch).unwrap();
    let EntityType::Hatch(changed) = document.get_entity_mut(Handle::new(0xB2)).unwrap() else {
        panic!("HATCH")
    };
    changed.paths[0].boundary_handles = vec![new_boundary];
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    assert!(document
        .get_entity(new_boundary)
        .unwrap()
        .common()
        .reactors
        .is_empty());
    let saved = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let original = &saved
        .get_entity(Handle::new(0xA1))
        .unwrap()
        .common()
        .reactors;
    assert!(original.contains(&Handle::new(0xB1)));
    assert!(original.contains(&new_hatch));
    assert!(!original.contains(&Handle::new(0xB2)));
    assert_eq!(
        saved.get_entity(new_boundary).unwrap().common().reactors,
        vec![Handle::new(0xB2)]
    );
}

#[test]
fn dwg_saves_preserve_missing_source_hatch_reverse_reactors() {
    let mut document = DxfReader::from_reader(Cursor::new(source()))
        .unwrap()
        .read()
        .unwrap();
    assert_source_relations(&document, "DXF import");
    for _ in 0..2 {
        let bytes = DwgWriter::write_to_vec(&document).unwrap();
        assert_source_relations(&document, "caller after DWG write");
        document = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
        assert_source_relations(&document, "DWG reopen");
    }
}
