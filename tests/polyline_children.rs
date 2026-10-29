//! The VERTEX and SEQEND records of a POLYLINE keep their identity and data: their handles (an
//! XRECORD group 340 names them), layer, color, lineweight, XDATA and extension dictionary. They
//! are the children of a 2D polyline, a 3D polyline, a polygon mesh and a polyface mesh. The DXF
//! reader kept some of that for a 3D polyline and nothing for a 2D one, and the writers generated
//! new child handles with the polyline's layer and no data.

use std::io::Cursor;

use acadrust::types::{Color, DxfVersion, Handle};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};

const POLYLINE: u64 = 0x100;
const SEQEND: u64 = 0x190;
const DICTIONARY: u64 = 0x1A0;
const XRECORD: u64 = 0x1B0;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Polyline2D,
    Polyline3D,
    PolygonMesh,
    PolyfaceMesh,
}

const KINDS: [Kind; 4] = [
    Kind::Polyline2D,
    Kind::Polyline3D,
    Kind::PolygonMesh,
    Kind::PolyfaceMesh,
];

/// The groups of one VERTEX (or face record) `handle`: the first has a layer, a color, XDATA and
/// an extension dictionary, the second has nothing of its own, the third a color and a lineweight.
fn vertex(kind: Kind, index: usize, handle: u64, geometry: &str) -> String {
    let subclass = match kind {
        Kind::Polyline2D => "AcDb2dVertex",
        Kind::Polyline3D => "AcDb3dPolylineVertex",
        Kind::PolygonMesh => "AcDbPolygonMeshVertex",
        Kind::PolyfaceMesh => "AcDbPolyFaceMeshVertex",
    };
    let own = match index {
        0 => format!(
            "102\n{{ACAD_XDICTIONARY\n360\n{DICTIONARY:X}\n102\n}}\n330\n{POLYLINE:X}\n100\nAcDbEntity\n8\nVertexLayer\n62\n2\n100\nAcDbVertex\n100\n{subclass}\n{geometry}1001\nVENDOR\n1000\nvertex data\n"
        ),
        2 => format!(
            "330\n{POLYLINE:X}\n100\nAcDbEntity\n8\n0\n62\n3\n370\n35\n100\nAcDbVertex\n100\n{subclass}\n{geometry}"
        ),
        _ => format!(
            "330\n{POLYLINE:X}\n100\nAcDbEntity\n8\n0\n100\nAcDbVertex\n100\n{subclass}\n{geometry}"
        ),
    };
    format!("0\nVERTEX\n5\n{handle:X}\n{own}")
}

fn source(kind: Kind) -> String {
    let (flags, head) = match kind {
        Kind::Polyline2D => (0, "100\nAcDb2dPolyline\n"),
        Kind::Polyline3D => (8, "100\nAcDb3dPolyline\n"),
        Kind::PolygonMesh => (16, "100\nAcDbPolygonMesh\n71\n2\n72\n2\n"),
        Kind::PolyfaceMesh => (64, "100\nAcDbPolyFaceMesh\n71\n3\n72\n1\n"),
    };
    let point = |x: i32, y: i32| format!("10\n{x}.0\n20\n{y}.0\n30\n0.0\n");
    let mut children = String::new();
    let count = match kind {
        Kind::PolygonMesh => 4,
        _ => 3,
    };
    for index in 0..count {
        let handle = 0x101 + index as u64;
        let vflags = match kind {
            Kind::Polyline2D => "70\n0\n",
            Kind::Polyline3D => "70\n32\n",
            Kind::PolygonMesh => "70\n64\n",
            Kind::PolyfaceMesh => "70\n192\n",
        };
        let geometry = format!("{}{vflags}", point(index as i32 * 10, index as i32));
        children.push_str(&vertex(kind, index, handle, &geometry));
    }
    if matches!(kind, Kind::PolyfaceMesh) {
        // The face record: it is a child too, with a color and the vertex indices 1, 2, 3.
        children.push_str("0\nVERTEX\n5\n104\n330\n100\n100\nAcDbEntity\n8\n0\n62\n5\n100\nAcDbFaceRecord\n10\n0.0\n20\n0.0\n30\n0.0\n70\n128\n71\n1\n72\n2\n73\n3\n74\n0\n");
    }
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n\
0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nAPPID\n5\n9\n100\nAcDbSymbolTable\n70\n2\n\
0\nAPPID\n5\n1C0\n100\nAcDbSymbolTableRecord\n100\nAcDbRegAppTableRecord\n2\nACAD\n70\n0\n\
0\nAPPID\n5\n1C1\n100\nAcDbSymbolTableRecord\n100\nAcDbRegAppTableRecord\n2\nVENDOR\n70\n0\n\
0\nENDTAB\n0\nENDSEC\n\
0\nSECTION\n2\nENTITIES\n\
0\nPOLYLINE\n5\n{POLYLINE:X}\n100\nAcDbEntity\n8\n0\n{head}66\n1\n10\n0.0\n20\n0.0\n30\n0.0\n70\n{flags}\n\
{children}\
0\nSEQEND\n5\n{SEQEND:X}\n330\n{POLYLINE:X}\n100\nAcDbEntity\n8\nSeqLayer\n1001\nVENDOR\n1000\nseqend data\n\
0\nENDSEC\n\
0\nSECTION\n2\nOBJECTS\n\
0\nDICTIONARY\n5\n{DICTIONARY:X}\n330\n101\n100\nAcDbDictionary\n281\n1\n\
0\nXRECORD\n5\n{XRECORD:X}\n330\nC\n100\nAcDbXrecord\n280\n1\n340\n101\n340\n{SEQEND:X}\n340\n103\n\
0\nENDSEC\n0\nEOF\n"
    )
}

fn read(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

/// What a child record keeps, whichever entity type holds it.
#[derive(Debug, PartialEq)]
struct Child {
    handle: u64,
    /// The effective layer.
    layer: String,
    color: Color,
    lineweight: i16,
    vendor_xdata: bool,
    xdictionary: Option<u64>,
}

fn summary(
    common: Option<&acadrust::entities::EntityCommon>,
    handle: Handle,
    layer: &str,
) -> Child {
    Child {
        handle: handle.value(),
        layer: layer.to_string(),
        color: common.map_or(Color::ByLayer, |c| c.color),
        lineweight: common.map_or(-1, |c| c.line_weight.value()),
        vendor_xdata: common.is_some_and(|c| c.extended_data.get_record("VENDOR").is_some()),
        xdictionary: common.and_then(|c| c.xdictionary_handle).map(|h| h.value()),
    }
}

/// The children (vertices, faces) and the SEQEND of the polyline in `doc`.
fn children(doc: &CadDocument) -> (Vec<Child>, Child) {
    for entity in doc.entities() {
        match entity {
            EntityType::Polyline2D(p) => {
                let vertices = p
                    .vertices
                    .iter()
                    .map(|v| {
                        let layer = if v.layer.is_empty() { "0" } else { &v.layer };
                        summary(v.common.as_deref(), v.handle, layer)
                    })
                    .collect();
                let s = p.seqend.as_deref().expect("seqend");
                return (vertices, summary(Some(s), s.handle, &s.layer));
            }
            EntityType::Polyline3D(p) => {
                let vertices = p
                    .vertices
                    .iter()
                    .map(|v| {
                        let layer = if v.layer.is_empty() { "0" } else { &v.layer };
                        summary(v.common.as_deref(), v.handle, layer)
                    })
                    .collect();
                let s = p.seqend.as_deref().expect("seqend");
                return (vertices, summary(Some(s), s.handle, &s.layer));
            }
            EntityType::PolygonMesh(m) => {
                let vertices = m
                    .vertices
                    .iter()
                    .map(|v| summary(Some(&v.common), v.common.handle, &v.common.layer))
                    .collect();
                let s = m.seqend.as_deref().expect("seqend");
                return (vertices, summary(Some(s), s.handle, &s.layer));
            }
            EntityType::PolyfaceMesh(m) => {
                let vertices = m
                    .vertices
                    .iter()
                    .map(|v| &v.common)
                    .chain(m.faces.iter().map(|f| &f.common))
                    .map(|c| summary(Some(c), c.handle, &c.layer))
                    .collect();
                let s = m.seqend.as_deref().expect("seqend");
                return (vertices, summary(Some(s), s.handle, &s.layer));
            }
            _ => {}
        }
    }
    panic!("no polyline");
}

fn wanted(kind: Kind) -> (Vec<Child>, Child) {
    let child = |handle, layer: &str, color, lineweight, vendor, xdictionary| Child {
        handle,
        layer: layer.to_string(),
        color,
        lineweight,
        vendor_xdata: vendor,
        xdictionary,
    };
    let mut vertices = vec![
        child(
            0x101,
            "VertexLayer",
            Color::Index(2),
            -1,
            true,
            Some(DICTIONARY),
        ),
        child(0x102, "0", Color::ByLayer, -1, false, None),
        child(0x103, "0", Color::Index(3), 35, false, None),
    ];
    match kind {
        Kind::PolygonMesh => vertices.push(child(0x104, "0", Color::ByLayer, -1, false, None)),
        Kind::PolyfaceMesh => vertices.push(child(0x104, "0", Color::Index(5), -1, false, None)),
        _ => {}
    }
    (
        vertices,
        child(SEQEND, "SeqLayer", Color::ByLayer, -1, true, None),
    )
}

/// The `340` values of the XRECORD of a DXF.
fn xrecord_links(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8(bytes.to_vec()).expect("utf8");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let start = lines
        .windows(2)
        .position(|w| w[0] == "0" && w[1] == "XRECORD")
        .expect("XRECORD");
    lines[start..]
        .chunks(2)
        .take_while(|p| p[0] != "0" || p[1] == "XRECORD")
        .filter(|p| p[0] == "340")
        .map(|p| p[1].to_string())
        .collect()
}

#[test]
fn the_dxf_reader_keeps_the_children() {
    for kind in KINDS {
        let doc = read(source(kind).into_bytes());
        assert_eq!(children(&doc), wanted(kind), "{kind:?}");
    }
}

#[test]
fn the_dxf_writer_keeps_the_children_in_bridge_mode() {
    for kind in KINDS {
        let doc = read(source(kind).into_bytes());
        let bytes = DxfWriter::new(&doc)
            .keep_heavy_polylines()
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf");
        assert_eq!(xrecord_links(&bytes), ["101", "190", "103"], "{kind:?}");
        assert_eq!(children(&read(bytes)), wanted(kind), "{kind:?}");
    }
}

#[test]
fn a_default_dxf_keeps_the_children_too() {
    // A 2D polyline whose children carry data is not down-saved to an LWPOLYLINE.
    for kind in KINDS {
        let doc = read(source(kind).into_bytes());
        let bytes = DxfWriter::new(&doc).write_to_vec().expect("write dxf");
        assert_eq!(children(&read(bytes)), wanted(kind), "{kind:?}");
    }
}

#[test]
fn a_version_is_kept() {
    // The source is an R2018 drawing; the children do not depend on it.
    let doc = read(source(Kind::Polyline2D).into_bytes());
    assert_eq!(doc.version, DxfVersion::AC1032);
}
