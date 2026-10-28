//! Native edits must include unlisted block members; imported key values stay exact until an edit.

use std::io::Cursor;

use acadrust::entities::{EntityType, Line};
use acadrust::objects::{ObjectType, SortEntitiesTable};
use acadrust::{CadDocument, DxfReader, DxfWriter, Handle};

fn native() -> (CadDocument, SortEntitiesTable, [Handle; 3]) {
    let mut document = CadDocument::new();
    let handles = [0.0, 1.0, 2.0].map(|x| {
        document
            .add_entity(EntityType::Line(Line::from_coords(
                x, 0.0, 0.0, x, 1.0, 0.0,
            )))
            .unwrap()
    });
    let mut table = SortEntitiesTable::for_block(document.header.model_space_block_handle);
    table.handle = document.allocate_handle();
    table.owner_handle = document.header.named_objects_dict_handle;
    (document, table, handles)
}

fn assert_export_order(
    mut document: CadDocument,
    table: SortEntitiesTable,
    handles: [Handle; 3],
    expected: [Handle; 3],
) {
    document
        .objects
        .insert(table.handle, ObjectType::SortEntitiesTable(table));
    let bytes = DxfWriter::new(&document).write_to_vec().unwrap();
    let back = DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap();
    let table = back
        .objects
        .values()
        .find_map(|o| match o {
            ObjectType::SortEntitiesTable(table) => Some(table),
            _ => None,
        })
        .unwrap();
    let mut actual = handles;
    actual.sort_by_key(|h| (table.effective_sort_handle(*h).value(), h.value()));
    assert_eq!(
        actual, expected,
        "exported native edit must include unlisted entities"
    );
    let mut keys: Vec<_> = table.entries().map(|e| e.sort_handle.value()).collect();
    keys.sort_unstable();
    let mut identities = handles.map(|h| h.value());
    identities.sort_unstable();
    assert_eq!(
        keys, identities,
        "an edited table is a permutation of the full block's handles"
    );
}

#[test]
fn empty_native_table_bring_to_front_moves_past_unlisted_entities() {
    let (document, mut table, [a, b, c]) = native();
    table
        .bring_to_front(a, document.block_records.get("*Model_Space").unwrap())
        .unwrap();
    assert_export_order(document, table, [a, b, c], [b, c, a]);
}

#[test]
fn empty_native_table_send_to_back_moves_before_unlisted_entities() {
    let (document, mut table, [a, b, c]) = native();
    table
        .send_to_back(c, document.block_records.get("*Model_Space").unwrap())
        .unwrap();
    assert_export_order(document, table, [a, b, c], [c, a, b]);
}

#[test]
fn native_move_above_accepts_an_unlisted_reference_without_key_collisions() {
    let (document, mut table, [a, b, c]) = native();
    assert!(table
        .move_above(c, a, document.block_records.get("*Model_Space").unwrap())
        .is_some());
    assert_export_order(document, table, [a, b, c], [a, c, b]);
}

#[test]
fn native_front_edit_handles_extreme_imported_keys_without_overflow_or_ties() {
    let (document, mut table, [a, b, c]) = native();
    table.add_entry(a, Handle::new(u64::MAX));
    table.add_entry(b, Handle::NULL);
    table
        .bring_to_front(c, document.block_records.get("*Model_Space").unwrap())
        .unwrap();
    assert_export_order(document, table, [a, b, c], [b, a, c]);
}

#[test]
fn native_edits_reject_wrong_blocks_and_missing_references_without_mutation() {
    let (document, mut table, [a, _, _]) = native();
    let block = document.block_records.get("*Model_Space").unwrap();
    let mut wrong = block.clone();
    wrong.handle = Handle::NULL;
    let before = table.clone();
    assert!(table.bring_to_front(a, &wrong).is_none());
    assert_eq!(table, before);
    assert!(table.move_above(a, Handle::new(0xDEAD), block).is_none());
    assert_eq!(table, before);
}

#[test]
fn native_move_below_accepts_an_unlisted_reference_without_key_collisions() {
    let (document, mut table, [a, b, c]) = native();
    assert!(table
        .move_below(a, c, document.block_records.get("*Model_Space").unwrap())
        .is_some());
    assert_export_order(document, table, [a, b, c], [b, a, c]);
}
