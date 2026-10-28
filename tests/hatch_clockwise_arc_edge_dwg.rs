//! A clockwise hatch arc edge read from an AutoCAD DWG means the same arc as in DXF: AutoCAD stores
//! 360 minus each real angle in both formats. The DWG reader used to keep the stored angles, so the
//! arc of `tests/fixtures/hatch/README.md` started at (20,10) instead of (20,-10), and a DXF written
//! from it made CAD programs draw a near-full circle (Eztec Firenze hatch 5190D, radius 1991).

use acadrust::entities::hatch::BoundaryEdge;
use acadrust::entities::EntityType;
use acadrust::DwgReader;

#[test]
fn a_clockwise_arc_edge_from_autocad_starts_where_the_previous_edge_ends() {
    let doc = DwgReader::from_file("tests/fixtures/hatch/clockwise-arc-edge.dwg")
        .unwrap()
        .read()
        .unwrap();
    let hatch = doc
        .entities()
        .find_map(|e| {
            if let EntityType::Hatch(h) = e {
                Some(h)
            } else {
                None
            }
        })
        .expect("the fixture has one hatch");
    let edges = &hatch.paths[0].edges;
    let (i, arc) = edges
        .iter()
        .enumerate()
        .find_map(|(i, e)| {
            if let BoundaryEdge::CircularArc(a) = e {
                Some((i, a))
            } else {
                None
            }
        })
        .expect("the bite is an arc edge");
    assert!(!arc.counter_clockwise, "AutoCAD stores the bite clockwise");
    let at = |a: f64| {
        (
            arc.center.x + arc.radius * a.cos(),
            arc.center.y + arc.radius * a.sin(),
        )
    };
    let close = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() < 1e-9 && (p.1 - q.1).abs() < 1e-9;
    let BoundaryEdge::Line(before) = &edges[(i + edges.len() - 1) % edges.len()] else {
        panic!("a line precedes the bite")
    };
    assert!(
        close(at(arc.start_angle), (before.end.x, before.end.y)),
        "arc starts at {:?}, previous edge ends at {:?}",
        at(arc.start_angle),
        before.end
    );
    assert!(close(at(arc.start_angle), (20.0, -10.0)));
    assert!(close(at(arc.end_angle), (20.0, 10.0)));
}

#[test]
fn a_clockwise_arc_edge_keeps_autocads_dxf_angles() {
    let doc = DwgReader::from_file("tests/fixtures/hatch/clockwise-arc-edge.dwg")
        .unwrap()
        .read()
        .unwrap();
    let mut out = Vec::new();
    acadrust::DxfWriter::new(&doc)
        .write_to_writer(&mut out)
        .unwrap();
    let text = String::from_utf8_lossy(&out).replace('\r', "");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let at = lines
        .windows(2)
        .position(|w| w == ["72", "2"])
        .expect("an arc edge in the DXF");
    let value = |code: &str| {
        lines[at..]
            .chunks(2)
            .find(|p| p[0] == code)
            .and_then(|p| p[1].parse::<f64>().ok())
            .unwrap()
    };
    assert!((value("50") - 90.0).abs() < 1e-9, "50 = {}", value("50"));
    assert!((value("51") - 270.0).abs() < 1e-9, "51 = {}", value("51"));
    assert_eq!(value("73"), 0.0);
}

#[test]
fn a_clockwise_arc_edge_survives_a_dwg_round_trip() {
    let doc = DwgReader::from_file("tests/fixtures/hatch/clockwise-arc-edge.dwg")
        .unwrap()
        .read()
        .unwrap();
    let bytes = acadrust::DwgWriter::write_to_vec(&doc).unwrap();
    let again = DwgReader::from_stream(std::io::Cursor::new(bytes))
        .read()
        .unwrap();
    let arc = |d: &acadrust::CadDocument| {
        d.entities()
            .find_map(|e| {
                if let EntityType::Hatch(h) = e {
                    Some(h)
                } else {
                    None
                }
            })
            .and_then(|h| {
                h.paths[0].edges.iter().find_map(|e| {
                    if let BoundaryEdge::CircularArc(a) = e {
                        Some(a.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap()
    };
    let (a, b) = (arc(&doc), arc(&again));
    assert!(
        (a.start_angle - b.start_angle).abs() < 1e-12 && (a.end_angle - b.end_angle).abs() < 1e-12,
        "{a:?} != {b:?}"
    );
}
