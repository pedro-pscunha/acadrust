//! Header variables a DWG holds and the DXF reader reads, but the DXF writer left out of the
//! HEADER section, so a drawing that went DWG, DXF, DWG came back with their defaults:
//! the paper space extents, limits and insertion base, USERI1-5 and USERR1-5, ISOLINES, FACETRES,
//! the geographic location, MENU and the rest listed below.
//!
//! A DWG header holds more variables than AutoCAD writes to a DXF HEADER. The DXF Reference and
//! the HEADER of a DXF that AutoCAD 2018 saves have none of ISOLINES, FACETRES, TEXTQLTY,
//! PELLIPSE, DIMMZF, DIMMZS, DIMALTMZF and DIMALTMZS, so only the DWG bridge writer
//! (`DxfWriter::for_dwg_bridge`) writes them.

use std::io::Cursor;

use acadrust::types::{DxfVersion, Vector2, Vector3};
use acadrust::{CadDocument, DxfReader, DxfWriter};

/// Header variables of every DXF release this writer writes (AutoCAD 2000 and later).
fn set_common(doc: &mut CadDocument) {
    let h = &mut doc.header;
    h.paper_space_insertion_base = Vector3::new(1.5, 2.5, 3.5);
    h.paper_space_extents_min = Vector3::new(-10.0, -20.0, -30.0);
    h.paper_space_extents_max = Vector3::new(1367.25, 831.5, 8.0);
    h.paper_space_limits_min = Vector2::new(-0.05, 1.5);
    h.paper_space_limits_max = Vector2::new(1188.95, 841.0);
    h.user_int1 = 1000;
    h.user_int2 = 2;
    h.user_int3 = 3;
    h.user_int4 = 4;
    h.user_int5 = 5;
    h.user_real1 = 100.5;
    h.user_real2 = 2.5;
    h.user_real3 = 3.5;
    h.user_real4 = 4.5;
    h.user_real5 = 5.5;
    h.menu_name = "acad".to_string();
    h.isolines = 8;
    h.facet_resolution = 1.25;
    h.text_quality = 30;
    h.limit_check = true;
    h.multiline_justification = 1;
    h.multiline_scale = 2.5;
    h.chamfer_length = 1.5;
    h.chamfer_angle = 0.75;
    h.shade_edge = 2;
    h.shade_diffuse = 55;
    h.proxy_graphics = 0;
    h.tree_depth = 3030;
    h.sort_entities = 5;
    h.index_control = 2;
    h.xedit = false;
    h.extended_names = false;
    h.ole_startup = true;
    h.viewport_scale_factor = 0.5;
    h.associate_dimensions = false;
    h.update_dimensions_while_dragging = false;
    h.dim_user_positioned_text = true;
    h.dim_alt_zero_suppression = 3;
    h.dim_alt_rounding = 0.25;
    h.create_ellipse_as_polyline = true;
    h.user_timer = false;
    h.user_elapsed_time = 0.125;
}

fn check_common(doc: &CadDocument, what: &str) {
    let h = &doc.header;
    assert_eq!(
        h.paper_space_insertion_base,
        Vector3::new(1.5, 2.5, 3.5),
        "{what}: PINSBASE"
    );
    assert_eq!(
        h.paper_space_extents_min,
        Vector3::new(-10.0, -20.0, -30.0),
        "{what}: PEXTMIN"
    );
    assert_eq!(
        h.paper_space_extents_max,
        Vector3::new(1367.25, 831.5, 8.0),
        "{what}: PEXTMAX"
    );
    assert_eq!(
        h.paper_space_limits_min,
        Vector2::new(-0.05, 1.5),
        "{what}: PLIMMIN"
    );
    assert_eq!(
        h.paper_space_limits_max,
        Vector2::new(1188.95, 841.0),
        "{what}: PLIMMAX"
    );
    assert_eq!(
        [
            h.user_int1,
            h.user_int2,
            h.user_int3,
            h.user_int4,
            h.user_int5
        ],
        [1000, 2, 3, 4, 5],
        "{what}: USERI"
    );
    assert_eq!(
        [
            h.user_real1,
            h.user_real2,
            h.user_real3,
            h.user_real4,
            h.user_real5
        ],
        [100.5, 2.5, 3.5, 4.5, 5.5],
        "{what}: USERR"
    );
    assert_eq!(h.menu_name, "acad", "{what}: MENU");
    assert_eq!(h.isolines, 8, "{what}: ISOLINES");
    assert_eq!(h.facet_resolution, 1.25, "{what}: FACETRES");
    assert_eq!(h.text_quality, 30, "{what}: TEXTQLTY");
    assert!(h.limit_check, "{what}: LIMCHECK");
    assert_eq!(h.multiline_justification, 1, "{what}: CMLJUST");
    assert_eq!(h.multiline_scale, 2.5, "{what}: CMLSCALE");
    assert_eq!(
        (h.chamfer_length, h.chamfer_angle),
        (1.5, 0.75),
        "{what}: CHAMFERC/D"
    );
    assert_eq!(
        (h.shade_edge, h.shade_diffuse),
        (2, 55),
        "{what}: SHADEDGE/DIF"
    );
    assert_eq!(h.proxy_graphics, 0, "{what}: PROXYGRAPHICS");
    assert_eq!(h.tree_depth, 3030, "{what}: TREEDEPTH");
    assert_eq!(
        (h.sort_entities, h.index_control),
        (5, 2),
        "{what}: SORTENTS/INDEXCTL"
    );
    assert!(
        !h.xedit && !h.extended_names && h.ole_startup,
        "{what}: XEDIT/EXTNAMES/OLESTARTUP"
    );
    assert_eq!(h.viewport_scale_factor, 0.5, "{what}: PSVPSCALE");
    assert!(
        !h.associate_dimensions && !h.update_dimensions_while_dragging,
        "{what}: DIMASO/DIMSHO"
    );
    assert!(h.dim_user_positioned_text, "{what}: DIMUPT");
    assert_eq!(
        (h.dim_alt_zero_suppression, h.dim_alt_rounding),
        (3, 0.25),
        "{what}: DIMAZIN/DIMALTRND"
    );
    assert!(h.create_ellipse_as_polyline, "{what}: PELLIPSE");
    assert!(!h.user_timer, "{what}: USRTIMER");
    assert_eq!(h.user_elapsed_time, 0.125, "{what}: TDUSRTIMER");
}

/// Header variables of AutoCAD 2004 (dimension associativity, hidden text).
fn set_2004(doc: &mut CadDocument) {
    doc.header.dimension_associativity = 1;
    doc.header.hide_text = 0;
}

fn check_2004(doc: &CadDocument, what: &str) {
    assert_eq!(doc.header.dimension_associativity, 1, "{what}: DIMASSOC");
    assert_eq!(doc.header.hide_text, 0, "{what}: HIDETEXT");
}

/// Header variables of AutoCAD 2007 and later.
fn set_2007(doc: &mut CadDocument) {
    let h = &mut doc.header;
    h.halo_gap = 7;
    h.obscured_color = 5;
    h.obscured_linetype = 2;
    h.intersection_display = 1;
    h.intersection_color = 3;
    h.end_caps = 2;
    h.join_style = 1;
    h.cast_shadow = 2;
    h.shadow_plane_location = 12.5;
    h.loft_angle1 = 1.0;
    h.loft_angle2 = 1.25;
    h.loft_magnitude1 = 0.5;
    h.loft_magnitude2 = 0.75;
    h.loft_param = 7;
    h.loft_normals = 4;
    h.lens_length = 35.5;
    h.camera_height = 2.5;
    h.camera_display = true;
    h.latitude = 37.795;
    h.longitude = -122.394;
    h.north_direction = 0.5;
    h.timezone = -8000;
    h.step_size = 6.0;
    h.steps_per_second = 3.0;
}

fn check_2007(doc: &CadDocument, what: &str) {
    let h = &doc.header;
    assert_eq!(
        (h.halo_gap, h.obscured_color, h.obscured_linetype),
        (7, 5, 2),
        "{what}: HALOGAP/OBS"
    );
    assert_eq!(
        (h.intersection_display, h.intersection_color),
        (1, 3),
        "{what}: INTERSECTION"
    );
    assert_eq!(
        (h.end_caps, h.join_style, h.cast_shadow),
        (2, 1, 2),
        "{what}: ENDCAPS/JOINSTYLE/CSHADOW"
    );
    assert_eq!(h.shadow_plane_location, 12.5, "{what}: SHADOWPLANELOCATION");
    assert_eq!(
        [
            h.loft_angle1,
            h.loft_angle2,
            h.loft_magnitude1,
            h.loft_magnitude2
        ],
        [1.0, 1.25, 0.5, 0.75],
        "{what}: LOFT"
    );
    assert_eq!(
        (h.loft_param, h.loft_normals),
        (7, 4),
        "{what}: LOFTPARAM/NORMALS"
    );
    assert_eq!(
        (h.lens_length, h.camera_height, h.camera_display),
        (35.5, 2.5, true),
        "{what}: CAMERA"
    );
    assert_eq!(
        (h.latitude, h.longitude, h.north_direction),
        (37.795, -122.394, 0.5),
        "{what}: location"
    );
    assert_eq!(h.timezone, -8000, "{what}: TIMEZONE");
    assert_eq!(
        (h.step_size, h.steps_per_second),
        (6.0, 3.0),
        "{what}: STEPSIZE/STEPSPERSEC"
    );
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc)
        .for_dwg_bridge()
        .write_to_vec()
        .expect("write dxf");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn autocad_2000_variables_survive_dxf() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1015);
    set_common(&mut doc);
    check_common(&through_dxf(&doc), "AC1015");
}

#[test]
fn autocad_2004_variables_survive_dxf() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1018);
    set_common(&mut doc);
    set_2004(&mut doc);
    let back = through_dxf(&doc);
    check_common(&back, "AC1018");
    check_2004(&back, "AC1018");
}

#[test]
fn autocad_2007_and_later_variables_survive_dxf() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1024, DxfVersion::AC1032] {
        let mut doc = CadDocument::with_version(version);
        set_common(&mut doc);
        set_2004(&mut doc);
        set_2007(&mut doc);
        let back = through_dxf(&doc);
        let what = format!("{version:?}");
        check_common(&back, &what);
        check_2004(&back, &what);
        check_2007(&back, &what);
    }
}

#[test]
fn binary_dxf_carries_them_too() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    set_common(&mut doc);
    set_2004(&mut doc);
    set_2007(&mut doc);
    let mut writer = DxfWriter::new(&doc).for_dwg_bridge();
    writer.binary = true;
    let back = DxfReader::from_reader(Cursor::new(
        writer.write_to_vec().expect("write binary dxf"),
    ))
    .expect("reader")
    .read()
    .expect("read binary dxf");
    check_common(&back, "binary");
    check_2004(&back, "binary");
    check_2007(&back, "binary");
}

/// Variables of a DWG header that neither the DXF Reference nor an AutoCAD DXF has, with the first
/// release that has each.
const BRIDGE_ONLY: [(&str, DxfVersion); 8] = [
    ("$ISOLINES", DxfVersion::AC1015),
    ("$FACETRES", DxfVersion::AC1015),
    ("$TEXTQLTY", DxfVersion::AC1015),
    ("$PELLIPSE", DxfVersion::AC1015),
    ("$DIMMZF", DxfVersion::AC1024),
    ("$DIMMZS", DxfVersion::AC1024),
    ("$DIMALTMZF", DxfVersion::AC1024),
    ("$DIMALTMZS", DxfVersion::AC1024),
];

/// The names of the variables of the HEADER section.
fn header_names(bytes: Vec<u8>) -> Vec<String> {
    let text = String::from_utf8(bytes).expect("utf8");
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let end = lines
        .windows(2)
        .position(|w| w[0] == "0" && w[1] == "ENDSEC")
        .expect("end of the HEADER");
    lines[..end]
        .windows(2)
        .filter(|w| w[0] == "9")
        .map(|w| w[1].to_string())
        .collect()
}

#[test]
fn the_default_dxf_header_has_only_variables_autocad_writes() {
    for version in [
        DxfVersion::AC1015,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1032,
    ] {
        let mut doc = CadDocument::with_version(version);
        set_common(&mut doc);
        let names = header_names(DxfWriter::new(&doc).write_to_vec().expect("write dxf"));
        for (name, _) in BRIDGE_ONLY {
            assert!(!names.iter().any(|n| n == name), "{version:?}: {name}");
        }
    }
}

#[test]
fn the_bridge_dxf_header_has_them() {
    for version in [
        DxfVersion::AC1015,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1032,
    ] {
        let mut doc = CadDocument::with_version(version);
        set_common(&mut doc);
        let names = header_names(
            DxfWriter::new(&doc)
                .for_dwg_bridge()
                .write_to_vec()
                .expect("write dxf"),
        );
        for (name, since) in BRIDGE_ONLY {
            assert_eq!(
                names.iter().any(|n| n == name),
                version >= since,
                "{version:?}: {name}"
            );
        }
    }
}

/// The header variables of the DXF that AutoCAD saved (`tests/datatable/point_object_id.dxf`,
/// AC1032): a public DXF has no variable that AutoCAD does not write, as AutoCAD warns that it
/// ignores an unknown one (`$LASTSAVEDBY`) and refuses a malformed one.
#[test]
fn the_public_header_has_only_variables_autocad_writes() {
    let autocad = std::fs::read_to_string("tests/datatable/point_object_id.dxf").expect("fixture");
    let known: Vec<String> = autocad
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|w| w[0] == "9")
        .map(|w| w[1].to_string())
        .collect();
    assert!(known.len() > 200, "the fixture header is short");
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    set_common(&mut doc);
    set_2004(&mut doc);
    set_2007(&mut doc);
    let info = &mut doc.summary_info;
    info.title = "Title".into();
    info.last_saved_by = "Someone".into();
    info.revision_number = "2".into();
    info.hyperlink_base = "https://example.com/".into();
    info.custom_properties = vec![("Tag".into(), "Value".into())];
    let names = header_names(DxfWriter::new(&doc).write_to_vec().expect("write dxf"));
    for name in names {
        // $OSMODE is a variable of AutoCAD that its DXF leaves out for this drawing.
        assert!(
            name == "$OSMODE" || known.contains(&name),
            "{name} is not a header variable of AutoCAD"
        );
    }
}
