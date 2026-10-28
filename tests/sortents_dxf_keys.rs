//! Sort keys are opaque ordering values, not new object handles. Unlisted entities retain
//! their own handle as their sort key, so renumbering only listed entries changes draw order.

use std::io::Cursor;

use acadrust::objects::{ObjectType, SortEntitiesTable};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, Handle};

fn source() -> CadDocument {
    let mut dxf = String::from(concat!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n9\n$HANDSEED\n5\n1000\n0\nENDSEC\n",
        "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nBLOCK_RECORD\n5\n1\n70\n1\n",
        "0\nBLOCK_RECORD\n5\n1F\n102\n{ACAD_XDICTIONARY\n360\nB1\n102\n}\n330\n1\n",
        "100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Model_Space\n",
        "0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n",
    ));
    for handle in ["A1", "A2", "A3"] {
        dxf.push_str(&format!(
            "0\nLINE\n5\n{handle}\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n0\n20\n0\n30\n0\n11\n10\n21\n0\n31\n0\n"
        ));
    }
    dxf.push_str(concat!(
        "0\nENDSEC\n0\nSECTION\n2\nOBJECTS\n0\nDICTIONARY\n5\nC\n330\n0\n100\nAcDbDictionary\n",
        "0\nDICTIONARY\n5\nB1\n330\n1F\n100\nAcDbDictionary\n280\n1\n281\n1\n3\nACAD_SORTENTS\n360\nB0\n",
        "0\nSORTENTSTABLE\n5\nB0\n330\nB1\n100\nAcDbSortentsTable\n330\n1F\n331\nA1\n5\nA4\n331\nA3\n5\n10\n",
        "0\nENDSEC\n0\nEOF\n",
    ));
    DxfReader::from_reader(Cursor::new(dxf.into_bytes()))
        .unwrap()
        .read()
        .unwrap()
}

fn table(document: &CadDocument) -> &SortEntitiesTable {
    document
        .objects
        .values()
        .find_map(|object| match object {
            ObjectType::SortEntitiesTable(table) => Some(table),
            _ => None,
        })
        .expect("SORTENTSTABLE in the document")
}

fn dwg_source() -> CadDocument {
    let bytes = DwgWriter::write_to_vec(&source()).unwrap();
    let document = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    assert!(table(&document).raw_dxf_codes.is_none());
    assert_eq!(
        table(&document)
            .entries()
            .map(|entry| (entry.entity_handle.value(), entry.sort_handle.value()))
            .collect::<Vec<_>>(),
        [(0xA1, 0xA4), (0xA3, 0x10)],
        "synthetic DWG must reproduce the exact partial table"
    );
    document
}

#[test]
fn dwg_to_dxf_keeps_exact_sort_keys_without_allocating_new_handles() {
    let document = dwg_source();
    let bytes = DxfWriter::new(&document).write_to_vec().unwrap();
    let back = DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap();
    assert_eq!(
        table(&back)
            .entries()
            .map(|entry| (entry.entity_handle.value(), entry.sort_handle.value()))
            .collect::<Vec<_>>(),
        [(0xA1, 0xA4), (0xA3, 0x10)]
    );
}

#[test]
fn dwg_to_dxf_partial_table_keeps_unlisted_entity_between_low_and_high_keys() {
    let document = dwg_source();
    let bytes = DxfWriter::new(&document).write_to_vec().unwrap();
    let back = DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap();
    let table = table(&back);
    assert!(
        !table.contains(Handle::new(0xA2)),
        "A2 is intentionally unlisted"
    );
    let mut entities = [Handle::new(0xA1), Handle::new(0xA2), Handle::new(0xA3)];
    entities.sort_by_key(|entity| table.get_sort_handle(*entity).unwrap_or(*entity).value());
    assert_eq!(
        entities.map(|handle| handle.value()),
        [0xA3, 0xA2, 0xA1],
        "unlisted A2 must keep its own handle as its key"
    );
}
