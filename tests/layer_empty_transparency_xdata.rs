//! A layer whose AcCmTransparency XDATA has no value group (1071) is still a layer: it keeps its
//! record and its XDATA as read, through DXF and the DXF to DWG bridge.

use std::io::Cursor;

use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

/// A DXF whose layer `Empty` carries an AcCmTransparency record followed by `tail` groups, and
/// which has a layer `After` behind it when `after`.
fn source(tail: &[(i32, &str)], after: bool) -> String {
    let layer =
        |handle: &'static str, name: &'static str, color: &'static str| -> Vec<(i32, &str)> {
            vec![
                (0, "LAYER"),
                (5, handle),
                (100, "AcDbSymbolTableRecord"),
                (100, "AcDbLayerTableRecord"),
                (2, name),
                (70, "0"),
                (62, color),
                (6, "Continuous"),
            ]
        };
    let mut pairs: Vec<(i32, &str)> = vec![
        (0, "SECTION"),
        (2, "HEADER"),
        (9, "$ACADVER"),
        (1, "AC1032"),
        (0, "ENDSEC"),
        (0, "SECTION"),
        (2, "TABLES"),
        (0, "TABLE"),
        (2, "APPID"),
        (70, "1"),
        (0, "APPID"),
        (2, "AcCmTransparency"),
        (70, "0"),
        (0, "ENDTAB"),
        (0, "TABLE"),
        (2, "LAYER"),
        (70, "4"),
    ];
    pairs.extend(layer("50", "0", "7"));
    pairs.extend(layer("51", "Plain", "3"));
    pairs.extend(layer("52", "Empty", "4"));
    pairs.push((1001, "AcCmTransparency"));
    pairs.extend_from_slice(tail);
    if after {
        pairs.extend(layer("53", "After", "5"));
    }
    pairs.extend_from_slice(&[(0, "ENDTAB"), (0, "ENDSEC"), (0, "EOF")]);
    pairs.iter().map(|(c, v)| format!("{c}\n{v}\n")).collect()
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn check(doc: &CadDocument, after: bool, what: &str) {
    let names: &[&str] = if after {
        &["Plain", "Empty", "After"]
    } else {
        &["Plain", "Empty"]
    };
    for name in names {
        assert!(doc.layers.get(name).is_some(), "{what}: layer {name} lost");
    }
    let empty = doc.layers.get("Empty").unwrap();
    assert!(empty.transparency.is_opaque(), "{what}");
    assert!(
        doc.object_xdata(empty.handle)
            .is_some_and(|x| x.get_record("AcCmTransparency").is_some()),
        "{what}: the AcCmTransparency record of Empty is gone"
    );
}

const TAILS: [&[(i32, &str)]; 7] = [
    &[],
    &[(1070, "5")],
    &[(1000, "text")],
    &[(1071, "33554687")],
    &[(1071, "")],
    &[(1071, "x")],
    // The layer standard's description behind an empty transparency record.
    &[(1001, "AcAecLayerStandard"), (1000, ""), (1000, "note")],
];

#[test]
fn the_dxf_reader_keeps_every_layer_and_the_record() {
    for tail in TAILS {
        for after in [false, true] {
            check(
                &read(source(tail, after).into_bytes()),
                after,
                &format!("read {tail:?} {after}"),
            );
        }
    }
}

#[test]
fn the_layers_survive_dxf_and_dwg() {
    for tail in TAILS {
        let doc = read(source(tail, true).into_bytes());
        let again = read(DxfWriter::new(&doc).write_to_vec().expect("write dxf"));
        check(&again, true, &format!("DXF {tail:?}"));
        let bytes = DwgWriter::write_to_vec(&again).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read dwg");
        for name in ["Plain", "Empty", "After"] {
            assert!(
                back.layers.get(name).is_some(),
                "DWG {tail:?}: layer {name} lost"
            );
        }
    }
}

/// A DWG block of XDATA cannot be empty (its size is the list's terminator when 0), so a record
/// with no values is left out of the layer's EED: the DWG writer wrote a size of 0 and the
/// application handle after it, which made the whole layer record unreadable.
#[test]
fn an_empty_record_does_not_corrupt_the_layer_in_a_dwg() {
    for tail in TAILS {
        let doc = read(source(tail, true).into_bytes());
        let bytes = DwgWriter::write_to_vec(&doc).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(bytes.clone()))
            .read()
            .unwrap_or_else(|e| panic!("{tail:?}: the DWG does not read: {e}"));
        for name in ["Plain", "Empty", "After"] {
            assert!(
                back.layers.get(name).is_some(),
                "{tail:?}: layer {name} lost"
            );
        }
    }
}
