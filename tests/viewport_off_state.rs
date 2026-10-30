use acadrust::entities::{Viewport, ViewportStatusFlags};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType};
use std::io::Cursor;

const ON: i32 = 0x8000;
const OFF: i32 = 0x20000;

fn drawing(bits: i32, off_screen: bool) -> CadDocument {
    let mut doc = CadDocument::new();
    let mut view = Viewport::new();
    view.id = 2;
    view.width = 160.;
    view.height = 100.;
    view.status = ViewportStatusFlags::from_bits(bits);
    view.off_screen = off_screen;
    doc.add_entity_to_layout(EntityType::Viewport(Box::new(view)), "Layout1")
        .unwrap();
    doc
}

fn viewport(doc: &CadDocument) -> &Viewport {
    doc.entities()
        .find_map(|entity| match entity {
            EntityType::Viewport(view) if view.width == 160. => Some(view),
            _ => None,
        })
        .unwrap()
}

#[test]
fn explicit_off_bit_is_independent_of_legacy_on_and_clipping() {
    for bits in [0, ON, OFF, ON | OFF, ON | OFF | 0x10000 | 0x4000] {
        assert_eq!(ViewportStatusFlags::from_bits(bits).to_bits(), bits);
    }
}

#[test]
fn viewport_visibility_helpers_respect_explicit_off() {
    let mut view = Viewport::new();
    view.status = ViewportStatusFlags::from_bits(ON | OFF);
    assert!(!view.is_on());
    view.turn_on();
    assert!(view.is_on());
    assert_eq!(view.status.to_bits() & (ON | OFF), ON);
    view.turn_off();
    assert!(!view.is_on());
    assert_eq!(view.status.to_bits() & (ON | OFF), OFF);
}

#[test]
fn dxf_status_group_68_respects_explicit_off_and_off_screen() {
    for bits in [0, ON, OFF, ON | OFF] {
        for off_screen in [false, true] {
            let doc = drawing(bits, off_screen);
            let text = String::from_utf8(DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
            let lines: Vec<_> = text.lines().collect();
            let fields: Vec<_> = lines.chunks_exact(2).collect();
            let start = fields
                .iter()
                .enumerate()
                .find_map(|(index, pair)| {
                    (pair[0].trim() == "0"
                        && pair[1].trim() == "VIEWPORT"
                        && fields[index + 1..]
                            .iter()
                            .take_while(|p| p[0].trim() != "0")
                            .any(|p| p[0].trim() == "40" && p[1].trim().parse::<f64>() == Ok(160.)))
                    .then_some(index)
                })
                .unwrap();
            let view_fields: Vec<_> = fields[start + 1..]
                .iter()
                .take_while(|p| p[0].trim() != "0")
                .collect();
            let group = |code: &str| -> i32 {
                view_fields.iter().find(|p| p[0].trim() == code).unwrap()[1]
                    .trim()
                    .parse()
                    .unwrap()
            };
            let expected = if bits & ON == 0 || bits & OFF != 0 {
                0
            } else if off_screen {
                -1
            } else {
                2
            };
            assert_eq!(
                group("68"),
                expected,
                "bits={bits:x}, off_screen={off_screen}"
            );
            assert_eq!(group("90") & (ON | OFF), bits & (ON | OFF));
        }
    }
}

#[test]
fn explicit_off_survives_dxf_and_dwg_readback() {
    for bits in [OFF, ON | OFF] {
        let doc = drawing(bits, false);
        let dxf = DxfReader::from_reader(Cursor::new(DxfWriter::new(&doc).write_to_vec().unwrap()))
            .unwrap()
            .read()
            .unwrap();
        let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        for restored in [dxf, dwg] {
            let view = viewport(&restored);
            assert_eq!(view.status.to_bits() & (ON | OFF), bits);
            assert!(!view.is_on());
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn old_serialized_status_defaults_explicit_off_to_false() {
    let mut value = serde_json::to_value(ViewportStatusFlags::default_on()).unwrap();
    value.as_object_mut().unwrap().remove("is_off");
    let restored: ViewportStatusFlags = serde_json::from_value(value).unwrap();
    assert_eq!(restored.to_bits() & (ON | OFF), ON);
}

/// The status (68) of the viewports of a layout, in order, as a DXF holds it.
fn statuses(doc: &CadDocument) -> Vec<i32> {
    let text = String::from_utf8(DxfWriter::new(doc).write_to_vec().unwrap()).unwrap();
    let lines: Vec<_> = text.lines().collect();
    let fields: Vec<_> = lines.chunks_exact(2).collect();
    let mut out = Vec::new();
    let mut in_viewport = false;
    for pair in fields {
        match (pair[0].trim(), pair[1].trim()) {
            ("0", kind) => in_viewport = kind == "VIEWPORT",
            ("68", value) if in_viewport => out.push(value.parse().unwrap()),
            _ => {}
        }
    }
    out
}

/// A DWG holds no status (68). A viewport read from one has it by AutoCAD's rule: 0 when off, its number when it is on and active,
/// -1 when it is on but past MAXACTVP.
#[test]
fn the_status_of_a_viewport_read_from_a_dwg_follows_maxactvp() {
    let mut doc = CadDocument::new();
    doc.header.max_active_viewports = 3;
    for (width, bits) in [(161., ON), (162., OFF), (163., ON), (164., ON), (165., ON)] {
        let mut view = Viewport::new();
        view.width = width;
        view.height = 100.;
        view.status = ViewportStatusFlags::from_bits(bits);
        doc.add_entity_to_layout(EntityType::Viewport(Box::new(view)), "Layout1")
            .unwrap();
    }
    let dwg = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    let all = statuses(&dwg);
    let ours = &all[all.len() - 5..];
    assert_eq!(ours, [1, 0, 3, 4, -1], "{all:?}");
}

/// A status a DXF holds is kept in a DXF, whatever the viewport's id.
#[test]
fn the_status_of_a_viewport_read_from_a_dxf_is_kept() {
    let mut doc = CadDocument::new();
    for (width, id, status) in [(161., 2, -1), (162., 2, 7), (163., 3, 3)] {
        let mut view = Viewport::new();
        view.width = width;
        view.height = 100.;
        view.id = id;
        view.dxf_status = Some(status);
        doc.add_entity_to_layout(EntityType::Viewport(Box::new(view)), "Layout1")
            .unwrap();
    }
    let dxf = DxfReader::from_reader(Cursor::new(DxfWriter::new(&doc).write_to_vec().unwrap()))
        .unwrap()
        .read()
        .unwrap();
    let all = statuses(&dxf);
    assert_eq!(&all[all.len() - 3..], [-1, 7, 3], "{all:?}");
}
