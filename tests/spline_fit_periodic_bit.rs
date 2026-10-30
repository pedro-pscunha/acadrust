//! Bit 4 of the spline flags of a fit-point spline in an R2013 or later DWG is Periodic, not Closed: AutoCAD stores a closed spline
//! that is not periodic with bits 1 and 8 only (9), a closed periodic one with 13.

use std::io::Cursor;

use acadrust::entities::{EntityType, Spline};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter};

fn fit_spline(periodic: bool) -> Spline {
    let mut s = Spline::new();
    s.degree = 3;
    s.fit_points = vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 0.0),
        Vector3::new(10.0, 10.0, 0.0),
        Vector3::new(0.0, 10.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
    ];
    s.flags.closed = true;
    s.flags.periodic = periodic;
    s.dwg_flags1 = 1 | 8;
    s
}

fn roundtrip(version: DxfVersion, periodic: bool) -> Spline {
    let mut doc = CadDocument::with_version(version);
    doc.add_entity(EntityType::Spline(fit_spline(periodic)))
        .unwrap();
    let bytes = DwgWriter::write_to_vec(&doc).expect("DWG write failed");
    let read = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("DWG read failed");
    let spline = read.entities().find_map(|e| match e {
        EntityType::Spline(s) => Some(s.clone()),
        _ => None,
    });
    spline.expect("SPLINE missing after roundtrip")
}

#[test]
fn a_closed_fit_spline_that_is_not_periodic_has_no_bit_4() {
    let s = roundtrip(DxfVersion::AC1032, false);
    assert_eq!(s.dwg_flags1 & 4, 0, "flags1 {}", s.dwg_flags1);
    assert!(s.flags.closed, "closed by its equal end fit points");
    assert!(!s.flags.periodic);
}

#[test]
fn a_closed_periodic_fit_spline_has_bit_4() {
    let s = roundtrip(DxfVersion::AC1032, true);
    assert_eq!(s.dwg_flags1 & 4, 4, "flags1 {}", s.dwg_flags1);
    assert!(s.flags.closed && s.flags.periodic);
}
