//! The `$HANDSEED` of the DXF form of a drawing (the DWG bridge reads a DWG through it) is the first handle past the ones the writer
//! gives, plus the one it pads with. A VERTEX or SEQEND that has a handle keeps it, so it takes none: counting one per SEQEND, or one
//! per vertex, raised the seed of a drawing by that many each time the DWG was saved and read again.

use std::io::Cursor;

use acadrust::entities::{AttributeEntity, EntityType, Insert, Polyline2D, Vertex2D};
use acadrust::tables::BlockRecord;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfWriter};

fn dwg_round_trip(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read")
}

/// `$HANDSEED` of the DXF form the DWG bridge reads.
fn bridge_seed(document: &CadDocument) -> u64 {
    let mut text = Vec::new();
    DxfWriter::new(document)
        .for_dwg_bridge()
        .keep_heavy_polylines()
        .write_to_writer(&mut text)
        .expect("dxf");
    let text = String::from_utf8_lossy(&text);
    let mut lines = text.lines().map(str::trim);
    while let Some(line) = lines.next() {
        if line == "$HANDSEED" {
            lines.next();
            return u64::from_str_radix(lines.next().expect("the seed"), 16).expect("hex");
        }
    }
    panic!("no $HANDSEED")
}

fn attributed_insert() -> CadDocument {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut record = BlockRecord::new("K");
    record.handle = document.allocate_handle();
    record.block_entity_handle = document.allocate_handle();
    record.block_end_handle = document.allocate_handle();
    document.block_records.add(record).unwrap();
    let mut insert = Insert::new("K", Vector3::new(5.0, 5.0, 0.0));
    insert.attributes = vec![
        AttributeEntity::new("A".into(), "one".into()),
        AttributeEntity::new("B".into(), "two".into()),
    ];
    document.add_entity(EntityType::Insert(insert)).unwrap();
    document
}

fn heavy_polyline() -> CadDocument {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut polyline = Polyline2D::new();
    for x in [0.0, 5.0, 9.0] {
        polyline
            .vertices
            .push(Vertex2D::new(Vector3::new(x, 1.0, 0.0)));
    }
    document
        .add_entity(EntityType::Polyline2D(polyline))
        .unwrap();
    document
}

/// After a DWG round trip every child has its handle: the DXF form's seed is the DWG's own next handle and the pad, not more.
#[test]
fn children_that_have_a_handle_take_none_from_the_seed() {
    for (what, document) in [
        ("an INSERT with attributes", attributed_insert()),
        ("a heavy polyline", heavy_polyline()),
    ] {
        let back = dwg_round_trip(&document);
        let native = back.header.handle_seed;
        assert_eq!(
            bridge_seed(&back),
            native + 1,
            "{what}: the DXF seed is one past the DWG's own (the pad)"
        );
        // And again: a second round trip does not move it.
        let again = dwg_round_trip(&back);
        assert_eq!(
            again.header.handle_seed, native,
            "{what}: the DWG's own seed after a second round trip"
        );
        assert_eq!(
            bridge_seed(&again),
            native + 1,
            "{what}: the DXF seed after a second round trip"
        );
    }
}

/// A child without a handle is given one by the writer, and the seed is above it.
#[test]
fn children_without_a_handle_still_count() {
    for (what, document) in [
        ("an INSERT with attributes", attributed_insert()),
        ("a heavy polyline", heavy_polyline()),
    ] {
        let next = document.header.handle_seed.max(document.next_handle());
        let with_children = bridge_seed(&document);
        // Two attributes and the SEQEND, or three vertices and the SEQEND, are four handles; none of them has one yet.
        let given = if what.starts_with("an INSERT") { 3 } else { 4 };
        assert!(
            with_children >= next + given,
            "{what}: seed {with_children:X} for next {next:X} and {given} handles to give"
        );
    }
}
