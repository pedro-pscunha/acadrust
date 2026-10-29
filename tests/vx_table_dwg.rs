//! The VX table (viewport entity headers) of a DWG of any release survives a DWG write and the
//! DXF of a DWG bridge. AutoCAD keeps it in R2018 drawings (Civil 3D drawings have one).

use std::io::Cursor;

use acadrust::tables::VxTableRecord;
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DwgReadOptions, DwgReader, DwgWriter, DxfReader, DxfWriter};

/// Three records chained by previous entry, as Toulouse has them: the first has no viewport.
fn document(version: DxfVersion) -> CadDocument {
    let mut document = CadDocument::with_version(version);
    let first = document.allocate_handle();
    let second = document.allocate_handle();
    let third = document.allocate_handle();
    let mut a = VxTableRecord::new("");
    a.handle = first;
    a.is_on = true;
    let mut b = VxTableRecord::new("1");
    b.handle = second;
    b.is_on = true;
    b.viewport = Handle::new(0x7001);
    b.previous_entry = third;
    let mut c = VxTableRecord::new("");
    c.handle = third;
    c.is_on = false;
    c.viewport = Handle::new(0x7002);
    for record in [a, b, c] {
        document.vx_table.add_allow_duplicate(record);
    }
    document.vx_control_entries = vec![first, second, third];
    document
}

/// (name, "on", viewport, previous entry) of each record, in table order.
fn records(document: &CadDocument) -> Vec<(String, bool, u64, u64)> {
    let mut records: Vec<_> = document
        .vx_table
        .iter()
        .map(|record| {
            (
                record.handle.value(),
                record.name.clone(),
                record.is_on,
                record.viewport.value(),
                record.previous_entry.value(),
            )
        })
        .collect();
    records.sort();
    records
        .into_iter()
        .map(|(_, name, on, viewport, previous)| (name, on, viewport, previous))
        .collect()
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DwgReader::from_stream_with_options(Cursor::new(bytes), DwgReadOptions::failsafe())
        .read()
        .expect("read dwg")
}

#[test]
fn a_dwg_keeps_the_vx_table_of_every_release() {
    for version in [
        DxfVersion::AC1015,
        DxfVersion::AC1018,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1027,
        DxfVersion::AC1032,
    ] {
        let source = document(version);
        let read = read(DwgWriter::write_to_vec(&source).expect("write dwg"));
        assert_eq!(records(&read), records(&source), "{version:?}");
        assert_eq!(
            read.vx_table.handle(),
            source.vx_table.handle(),
            "{version:?}"
        );
    }
}

#[test]
fn a_document_without_vx_records_writes_no_vx_table_from_r2004() {
    for version in [DxfVersion::AC1018, DxfVersion::AC1032] {
        let source = CadDocument::with_version(version);
        let read = read(DwgWriter::write_to_vec(&source).expect("write dwg"));
        assert!(read.vx_table.iter().next().is_none(), "{version:?}");
    }
}

#[test]
fn the_dxf_of_a_bridge_carries_the_vx_table() {
    let source = document(DxfVersion::AC1032);
    let read = read(DwgWriter::write_to_vec(&source).expect("write dwg"));
    let dxf = DxfWriter::new(&read)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    let from_dxf = DxfReader::from_reader(Cursor::new(dxf.clone()))
        .expect("reader")
        .read()
        .expect("read dxf");
    assert_eq!(records(&from_dxf), records(&source));
    assert_eq!(from_dxf.vx_table.handle(), source.vx_table.handle());
    assert_eq!(from_dxf.vx_control_entries, source.vx_control_entries);
    let again = self::read(DwgWriter::write_to_vec(&from_dxf).expect("write dwg"));
    assert_eq!(records(&again), records(&source));
    // A normal DXF has no VX table: AutoCAD's DXFOUT writes none.
    let public = DxfWriter::new(&read).write_to_vec().expect("write dxf");
    assert!(!String::from_utf8(public).unwrap().contains("ACADRUST_VX"));
}

/// The file the VX records come from, when it is on this machine.
#[test]
fn toulouse_keeps_its_four_vx_records() {
    let Ok(bytes) = std::fs::read("/tmp/cad/rt7/Planeta-Toulouse.dwg") else {
        return;
    };
    let source = read(bytes);
    assert_eq!(source.vx_table.iter().count(), 4);
    let written = DwgWriter::write_to_vec(&source).expect("write dwg");
    assert_eq!(records(&read(written)), records(&source));
}
