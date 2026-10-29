//! A VIEWPORT entity's render settings (DXF Reference, VIEWPORT groups 332 background, 333 shade
//! plot, 348 visual style, 361 sun hard-owner, 292 default lighting, 282 lighting type, 141
//! brightness, 142 contrast, 63 and 421 ambient color) survive DXF R2007 and later and the DXF to
//! DWG bridge. The DXF writer left them out and the reader had no arm for the handles, so a SUN
//! object a viewport owned lost its owner pointer and the viewport its visual style.

use std::io::Cursor;

use acadrust::entities::Viewport;
use acadrust::types::{Color, DxfVersion};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter, EntityType, Handle};

fn viewport() -> Viewport {
    let mut vp = Viewport::new();
    vp.id = 2;
    vp.width = 160.0;
    vp.height = 100.0;
    vp.background_handle = Handle::new(0x1A1);
    vp.shade_plot_handle = Handle::new(0x1A2);
    vp.visual_style_handle = Handle::new(0x1A3);
    vp.sun_handle = Handle::new(0x1A4);
    vp.default_lighting = false;
    vp.default_lighting_type = 2;
    vp.brightness = 0.25;
    vp.contrast = 0.5;
    vp.ambient_color = Color::Rgb {
        r: 10,
        g: 20,
        b: 30,
    };
    vp
}

fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    doc.add_entity_to_layout(EntityType::Viewport(Box::new(viewport())), "Layout1")
        .expect("add viewport");
    doc
}

fn viewports(doc: &CadDocument) -> Vec<&Viewport> {
    doc.entities()
        .filter_map(|e| match e {
            EntityType::Viewport(v) => Some(v.as_ref()),
            _ => None,
        })
        .collect()
}

fn check(doc: &CadDocument, what: &str) {
    let found = viewports(doc);
    let vp = found
        .iter()
        .find(|v| v.width == 160.0)
        .unwrap_or_else(|| panic!("{what}: viewport lost"));
    assert_eq!(
        vp.background_handle,
        Handle::new(0x1A1),
        "{what}: background"
    );
    assert_eq!(
        vp.shade_plot_handle,
        Handle::new(0x1A2),
        "{what}: shade plot"
    );
    assert_eq!(
        vp.visual_style_handle,
        Handle::new(0x1A3),
        "{what}: visual style"
    );
    assert_eq!(vp.sun_handle, Handle::new(0x1A4), "{what}: sun");
    assert!(!vp.default_lighting, "{what}: default lighting");
    assert_eq!(vp.default_lighting_type, 2, "{what}: lighting type");
    assert_eq!(
        (vp.brightness, vp.contrast),
        (0.25, 0.5),
        "{what}: brightness, contrast"
    );
    assert_eq!(
        vp.ambient_color,
        Color::Rgb {
            r: 10,
            g: 20,
            b: 30
        },
        "{what}: ambient color"
    );
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc).write_to_vec().expect("write dxf");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

#[test]
fn render_settings_survive_dxf() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1024, DxfVersion::AC1032] {
        check(&through_dxf(&document(version)), &format!("{version:?}"));
    }
}

#[test]
fn render_settings_survive_dxf_to_dwg() {
    for version in [DxfVersion::AC1021, DxfVersion::AC1032] {
        let bridged = through_dxf(&document(version));
        let bytes = DwgWriter::write_to_vec(&bridged).expect("write dwg");
        let back = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read dwg");
        check(&back, &format!("{version:?}: DWG"));
    }
}
