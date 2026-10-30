//! The text flow direction of a multileader's context (DXF 172) survives DXF and DWG of every version: 1 horizontal, 3 vertical,
//! 6 by style (ODA specification 20.4.86), and a value outside those as the file gave it. The enum took 5 for "by style" and read any
//! other value as horizontal.

use std::io::Cursor;

use acadrust::entities::multileader::FlowDirectionType;
use acadrust::entities::{EntityType, MultiLeader};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

fn drawing(version: DxfVersion, flow: FlowDirectionType) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    let mut leader = MultiLeader::with_text(
        "Leader",
        Vector3::new(3.0, 3.0, 0.0),
        vec![Vector3::ZERO, Vector3::new(2.0, 2.0, 0.0)],
    );
    leader.context.text_flow_direction = flow;
    doc.add_entity(EntityType::MultiLeader(Box::new(leader)))
        .unwrap();
    doc
}

fn flow(doc: &CadDocument) -> FlowDirectionType {
    doc.entities()
        .find_map(|entity| match entity {
            EntityType::MultiLeader(leader) => Some(leader.context.text_flow_direction),
            _ => None,
        })
        .expect("a multileader")
}

#[test]
fn the_text_flow_direction_survives_dxf_and_dwg() {
    let values = [
        (FlowDirectionType::Horizontal, 1),
        (FlowDirectionType::Vertical, 3),
        (FlowDirectionType::ByStyle, 6),
        (FlowDirectionType::Other(2), 2),
    ];
    for (direction, value) in values {
        assert_eq!(direction.value(), value);
        assert_eq!(FlowDirectionType::from(value), direction);
        for version in [
            DxfVersion::AC1018,
            DxfVersion::AC1021,
            DxfVersion::AC1027,
            DxfVersion::AC1032,
        ] {
            let doc = drawing(version, direction);
            let dxf =
                DxfReader::from_reader(Cursor::new(DxfWriter::new(&doc).write_to_vec().unwrap()))
                    .unwrap()
                    .read()
                    .unwrap();
            let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
                .read()
                .unwrap();
            // The path of a CADCraft save: a DXF read, then written as a DWG.
            let both = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&dxf).unwrap()))
                .read()
                .unwrap();
            assert_eq!(flow(&both), direction, "{version:?} DXF then DWG");
            assert_eq!(flow(&dxf), direction, "{version:?} DXF");
            assert_eq!(flow(&dwg), direction, "{version:?} DWG");
        }
    }
}
