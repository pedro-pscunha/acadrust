//! A BLOCK and its ENDBLK are entities with a common data of their own: each keeps its colour (62) and linetype scale (48), independent
//! of the other, through the DXF reader, the DWG and the DXF writer. The ENDBLK's were dropped: the reader read its handle and layer only,
//! and the writer wrote the layer only.

use std::io::Cursor;

use acadrust::entities::EntityType;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const SOURCE: &str = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n70\n1\n0\nLAYER\n5\n40\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\nBoundary\n70\n0\n62\n7\n6\nContinuous\n0\nENDTAB\n\
0\nTABLE\n2\nBLOCK_RECORD\n70\n1\n\
0\nBLOCK_RECORD\n5\n50\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\nSourceBlock\n70\n0\n0\nENDTAB\n0\nENDSEC\n\
0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n5\n51\n330\n50\n100\nAcDbEntity\n8\n0\n48\n2.5\n62\n3\n100\nAcDbBlockBegin\n2\nSourceBlock\n70\n0\n\
10\n0\n20\n0\n30\n0\n3\nSourceBlock\n0\nLINE\n5\n52\n330\n50\n100\nAcDbEntity\n8\n0\n100\nAcDbLine\n10\n0\n20\n0\n30\n0\n11\n10\n21\n0\n31\n0\n\
0\nENDBLK\n5\n53\n330\n50\n100\nAcDbEntity\n8\nBoundary\n48\n3.75\n62\n5\n100\nAcDbBlockEnd\n0\nENDSEC\n0\nEOF\n";

/// ((BLOCK linetype scale, colour index), (ENDBLK layer, linetype scale, colour index)) of SourceBlock.
type Pair = ((f64, u16), (String, f64, u16));

fn pair(doc: &CadDocument) -> Pair {
    let record = doc.block_records.get("SourceBlock").expect("the block");
    let block = doc
        .get_entity(record.block_entity_handle)
        .and_then(|e| {
            if let EntityType::Block(b) = e {
                Some(b)
            } else {
                None
            }
        })
        .expect("BLOCK");
    let end = doc
        .get_entity(record.block_end_handle)
        .and_then(|e| {
            if let EntityType::BlockEnd(b) = e {
                Some(b)
            } else {
                None
            }
        })
        .expect("ENDBLK");
    (
        (
            block.common.linetype_scale,
            block.common.color.index().unwrap_or(u16::MAX),
        ),
        (
            end.common.layer.clone(),
            end.common.linetype_scale,
            end.common.color.index().unwrap_or(u16::MAX),
        ),
    )
}

fn expected() -> Pair {
    ((2.5, 3), ("Boundary".into(), 3.75, 5))
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn the_dxf_reader_keeps_both() {
    assert_eq!(pair(&read(SOURCE.as_bytes().to_vec())), expected());
}

#[test]
fn the_dxf_writer_keeps_both_and_so_does_the_bridge() {
    let doc = read(SOURCE.as_bytes().to_vec());
    for bridge in [false, true] {
        let writer = DxfWriter::new(&doc);
        let bytes = if bridge {
            writer.for_dwg_bridge()
        } else {
            writer
        }
        .write_to_vec()
        .expect("write dxf");
        assert_eq!(pair(&read(bytes)), expected(), "bridge={bridge}");
    }
}

#[test]
fn every_dwg_release_keeps_both() {
    let doc = read(SOURCE.as_bytes().to_vec());
    for version in [
        DxfVersion::AC1015,
        DxfVersion::AC1018,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1027,
        DxfVersion::AC1032,
    ] {
        let mut copy = doc.clone();
        copy.version = version;
        let bytes = DwgWriter::write_to_vec(&copy).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read dwg");
        assert_eq!(pair(&back), expected(), "{version:?}");
    }
}
