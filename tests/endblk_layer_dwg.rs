//! The layer of a block's ENDBLK record (DXF 8) is the record's own: it survives a DWG.
use std::io::Cursor;

use acadrust::{DwgReader, DwgWriter, DxfReader, DxfWriter};

fn source(version: &str) -> String {
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\n{version}\n0\nENDSEC\n\
0\nSECTION\n2\nTABLES\n\
0\nTABLE\n2\nLAYER\n70\n3\n\
0\nLAYER\n5\n40\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\n0\n70\n0\n62\n7\n6\nContinuous\n\
0\nLAYER\n5\n41\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\nBeginLayer\n70\n0\n62\n7\n6\nContinuous\n\
0\nLAYER\n5\n42\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\nEndLayer\n70\n0\n62\n7\n6\nContinuous\n\
0\nENDTAB\n\
0\nTABLE\n2\nBLOCK_RECORD\n70\n1\n\
0\nBLOCK_RECORD\n5\n50\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\nDoor\n\
0\nENDTAB\n0\nENDSEC\n\
0\nSECTION\n2\nBLOCKS\n\
0\nBLOCK\n5\n51\n330\n50\n100\nAcDbEntity\n8\nBeginLayer\n100\nAcDbBlockBegin\n2\nDoor\n70\n0\n10\n0\n20\n0\n30\n0\n3\nDoor\n1\n\n\
0\nENDBLK\n5\n52\n330\n50\n100\nAcDbEntity\n8\nEndLayer\n100\nAcDbBlockEnd\n\
0\nENDSEC\n0\nEOF\n"
    )
}

/// The layers of the BLOCK and the ENDBLK records of the block Door in a DXF.
fn layers(bytes: &[u8]) -> Vec<(String, String)> {
    let text = String::from_utf8_lossy(bytes).to_string();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let pairs: Vec<(&str, &str)> = lines
        .chunks(2)
        .filter(|p| p.len() == 2)
        .map(|p| (p[0], p[1]))
        .collect();
    let mut out = Vec::new();
    for (i, p) in pairs.iter().enumerate() {
        if p.0 == "0" && (p.1 == "BLOCK" || p.1 == "ENDBLK") {
            let layer = pairs[i + 1..]
                .iter()
                .take_while(|q| q.0 != "0")
                .find(|q| q.0 == "8")
                .map(|q| q.1.to_string());
            let name = pairs[i + 1..]
                .iter()
                .take_while(|q| q.0 != "0")
                .find(|q| q.0 == "2")
                .map(|q| q.1.to_string());
            if name.as_deref().is_none_or(|n| n == "Door") {
                out.push((p.1.to_string(), layer.unwrap_or_default()));
            }
        }
    }
    out
}

#[test]
fn the_layers_of_block_and_endblk_survive_a_dwg() {
    for version in ["AC1015", "AC1018", "AC1021", "AC1032"] {
        let doc = DxfReader::from_reader(Cursor::new(source(version).into_bytes()))
            .unwrap()
            .read()
            .unwrap();
        let want = vec![
            ("BLOCK".to_string(), "BeginLayer".to_string()),
            ("ENDBLK".to_string(), "EndLayer".to_string()),
        ];
        let door = |bytes: &[u8]| {
            layers(bytes)
                .into_iter()
                .filter(|(k, _)| k == "BLOCK" || k == "ENDBLK")
                .collect::<Vec<_>>()
        };
        let dxf = DxfWriter::new(&doc).write_to_vec().unwrap();
        let in_door = |all: Vec<(String, String)>| {
            all.into_iter()
                .filter(|(_, l)| l.ends_with("Layer"))
                .collect::<Vec<_>>()
        };
        assert_eq!(in_door(door(&dxf)), want, "{version} DXF");
        let back = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        let dxf = DxfWriter::new(&back).write_to_vec().unwrap();
        assert_eq!(in_door(door(&dxf)), want, "{version} DWG");
    }
}
