//! Every group of an MLEADERSTYLE survives DXF -> DWG -> DXF. Each field of the style holds a
//! non-default value, so a group the DXF reader or writer drops, or maps to the wrong field,
//! comes back as the default or as another value.

use std::io::Cursor;

use acadrust::objects::{
    BlockContentConnectionType, LeaderContentType, LeaderDrawOrderType, MultiLeaderDrawOrderType,
    MultiLeaderPathType, MultiLeaderStyle, ObjectType, TextAlignmentType, TextAngleType,
    TextAttachmentDirectionType, TextAttachmentType,
};
use acadrust::types::{Color, DxfVersion, Handle, LineWeight};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const DESCRIPTION: &str = "Custom style description";

/// A style whose every field differs from `MultiLeaderStyle::new`.
fn style(document: &CadDocument, handle: Handle, owner: Handle) -> MultiLeaderStyle {
    let handle_of = |name: &str| document.block_records.get(name).expect("block").handle;
    let mut style = MultiLeaderStyle::new("Custom");
    style.handle = handle;
    style.owner_handle = owner;
    style.description = DESCRIPTION.to_string();
    style.content_type = LeaderContentType::Block;
    style.multileader_draw_order = MultiLeaderDrawOrderType::LeaderFirst;
    style.leader_draw_order = LeaderDrawOrderType::LeaderTailFirst;
    style.max_leader_points = 5;
    style.first_segment_angle = 0.5;
    style.second_segment_angle = 0.75;
    style.path_type = MultiLeaderPathType::Spline;
    style.line_color = Color::Rgb {
        r: 10,
        g: 20,
        b: 30,
    };
    style.line_type_handle = document
        .line_types
        .get("Continuous")
        .map(|line_type| line_type.handle);
    style.line_weight = LineWeight::from_value(50);
    style.enable_landing = false;
    style.landing_gap = 0.3;
    style.enable_dogleg = false;
    style.landing_distance = 0.7;
    style.arrowhead_handle = Some(handle_of("*Model_Space"));
    style.arrowhead_size = 0.4;
    style.text_style_handle = document.text_styles.get("Standard").map(|s| s.handle);
    style.text_left_attachment = TextAttachmentType::MiddleOfText;
    style.text_right_attachment = TextAttachmentType::BottomLine;
    style.text_top_attachment = TextAttachmentType::CenterOfText;
    style.text_bottom_attachment = TextAttachmentType::MiddleOfBottomLine;
    style.text_attachment_direction = TextAttachmentDirectionType::Vertical;
    style.text_angle_type = TextAngleType::Optimized;
    style.text_alignment = TextAlignmentType::Right;
    style.text_color = Color::Index(3);
    style.text_height = 0.6;
    style.text_frame = true;
    style.text_always_left = true;
    style.default_text = "Default text".to_string();
    style.block_content_handle = Some(handle_of("*Paper_Space"));
    style.block_content_color = Color::Rgb {
        r: 200,
        g: 100,
        b: 50,
    };
    style.block_content_connection = BlockContentConnectionType::BasePoint;
    style.block_content_rotation = 0.25;
    style.block_content_scale_x = 2.0;
    style.block_content_scale_y = 3.0;
    style.block_content_scale_z = 4.0;
    style.enable_block_scale = true;
    style.enable_block_rotation = true;
    style.scale_factor = 1.5;
    style.align_space = 0.9;
    style.break_gap_size = 0.2;
    style.is_annotative = true;
    style.property_changed = true;
    style.unknown_flag_298 = true;
    style
}

/// Every field the test sets is not the default one.
fn assert_all_non_default(style: &MultiLeaderStyle) {
    let d = MultiLeaderStyle::new("Custom");
    let checks = [
        ("description", style.description != d.description),
        ("content_type", style.content_type != d.content_type),
        (
            "multileader_draw_order",
            style.multileader_draw_order != d.multileader_draw_order,
        ),
        (
            "leader_draw_order",
            style.leader_draw_order != d.leader_draw_order,
        ),
        (
            "max_leader_points",
            style.max_leader_points != d.max_leader_points,
        ),
        (
            "first_segment_angle",
            style.first_segment_angle != d.first_segment_angle,
        ),
        (
            "second_segment_angle",
            style.second_segment_angle != d.second_segment_angle,
        ),
        ("path_type", style.path_type != d.path_type),
        ("line_color", style.line_color != d.line_color),
        (
            "line_type_handle",
            style.line_type_handle != d.line_type_handle,
        ),
        ("line_weight", style.line_weight != d.line_weight),
        ("enable_landing", style.enable_landing != d.enable_landing),
        ("landing_gap", style.landing_gap != d.landing_gap),
        ("enable_dogleg", style.enable_dogleg != d.enable_dogleg),
        (
            "landing_distance",
            style.landing_distance != d.landing_distance,
        ),
        (
            "arrowhead_handle",
            style.arrowhead_handle != d.arrowhead_handle,
        ),
        ("arrowhead_size", style.arrowhead_size != d.arrowhead_size),
        (
            "text_style_handle",
            style.text_style_handle != d.text_style_handle,
        ),
        (
            "text_left_attachment",
            style.text_left_attachment != d.text_left_attachment,
        ),
        (
            "text_right_attachment",
            style.text_right_attachment != d.text_right_attachment,
        ),
        (
            "text_top_attachment",
            style.text_top_attachment != d.text_top_attachment,
        ),
        (
            "text_bottom_attachment",
            style.text_bottom_attachment != d.text_bottom_attachment,
        ),
        (
            "text_attachment_direction",
            style.text_attachment_direction != d.text_attachment_direction,
        ),
        (
            "text_angle_type",
            style.text_angle_type != d.text_angle_type,
        ),
        ("text_alignment", style.text_alignment != d.text_alignment),
        ("text_color", style.text_color != d.text_color),
        ("text_height", style.text_height != d.text_height),
        ("text_frame", style.text_frame != d.text_frame),
        (
            "text_always_left",
            style.text_always_left != d.text_always_left,
        ),
        ("default_text", style.default_text != d.default_text),
        (
            "block_content_handle",
            style.block_content_handle != d.block_content_handle,
        ),
        (
            "block_content_color",
            style.block_content_color != d.block_content_color,
        ),
        (
            "block_content_connection",
            style.block_content_connection != d.block_content_connection,
        ),
        (
            "block_content_rotation",
            style.block_content_rotation != d.block_content_rotation,
        ),
        (
            "block_content_scale_x",
            style.block_content_scale_x != d.block_content_scale_x,
        ),
        (
            "block_content_scale_y",
            style.block_content_scale_y != d.block_content_scale_y,
        ),
        (
            "block_content_scale_z",
            style.block_content_scale_z != d.block_content_scale_z,
        ),
        (
            "enable_block_scale",
            style.enable_block_scale != d.enable_block_scale,
        ),
        (
            "enable_block_rotation",
            style.enable_block_rotation != d.enable_block_rotation,
        ),
        ("scale_factor", style.scale_factor != d.scale_factor),
        ("align_space", style.align_space != d.align_space),
        ("break_gap_size", style.break_gap_size != d.break_gap_size),
        ("is_annotative", style.is_annotative != d.is_annotative),
        (
            "property_changed",
            style.property_changed != d.property_changed,
        ),
        (
            "unknown_flag_298",
            style.unknown_flag_298 != d.unknown_flag_298,
        ),
    ];
    for (field, differs) in checks {
        assert!(
            differs,
            "{field} still has its default value in the test style"
        );
    }
}

fn document(version: DxfVersion) -> CadDocument {
    let mut document = CadDocument::with_version(version);
    let dictionary = document
        .objects
        .iter()
        .find_map(|(handle, object)| match object {
            ObjectType::MultiLeaderStyle(style) => {
                Some(style.owner_handle).filter(|h| *h != *handle)
            }
            _ => None,
        })
        .expect("the ACAD_MLEADERSTYLE dictionary");
    let handle = document.allocate_handle();
    let custom = style(&document, handle, dictionary);
    assert_all_non_default(&custom);
    if let Some(ObjectType::Dictionary(dictionary)) = document.objects.get_mut(&dictionary) {
        dictionary.add_entry("Custom", handle);
    }
    document
        .objects
        .insert(handle, ObjectType::MultiLeaderStyle(custom));
    document
}

/// The groups of the custom MLEADERSTYLE record of a DXF, without the handle and owner groups.
fn record(dxf: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = dxf.lines().map(str::trim).collect();
    let mut at = 0;
    while at + 1 < lines.len() {
        if lines[at] == "0" && lines[at + 1] == "MLEADERSTYLE" {
            let mut groups = Vec::new();
            let mut index = at + 2;
            while index + 1 < lines.len() && lines[index] != "0" {
                if !["5", "330", "102"].contains(&lines[index]) {
                    groups.push((lines[index].to_string(), lines[index + 1].to_string()));
                }
                index += 2;
            }
            if groups.iter().any(|g| g.0 == "3" && g.1 == DESCRIPTION) {
                return groups;
            }
            at = index;
        } else {
            at += 1;
        }
    }
    panic!("the custom MLEADERSTYLE is not in the DXF");
}

fn dxf_of(document: &CadDocument) -> String {
    String::from_utf8(DxfWriter::new(document).write_to_vec().expect("write dxf")).expect("utf8")
}

#[test]
fn every_group_survives_dxf_dwg_dxf() {
    for version in [DxfVersion::AC1024, DxfVersion::AC1032] {
        let first = dxf_of(&document(version));
        let wanted = record(&first);
        let from_dxf = DxfReader::from_reader(Cursor::new(first.into_bytes()))
            .expect("reader")
            .read()
            .expect("read dxf");
        let from_dwg = DwgReader::from_stream(Cursor::new(
            DwgWriter::write_to_vec(&from_dxf).expect("write dwg"),
        ))
        .read()
        .expect("read dwg");
        let second = record(&dxf_of(&from_dwg));
        assert_eq!(second, wanted, "{version:?}");
        // The groups the test sets are all there.
        for code in [
            "3", "90", "40", "41", "42", "43", "44", "45", "46", "47", "49", "91", "92", "93",
            "94", "140", "141", "142", "143", "170", "171", "172", "173", "174", "175", "176",
            "177", "178", "271", "272", "273", "290", "291", "292", "293", "294", "295", "296",
            "297", "300", "340", "341", "342", "343",
        ] {
            assert!(
                wanted.iter().any(|g| g.0 == code),
                "group {code} is not written"
            );
        }
    }
}

/// What the DXF reader makes of the record, field by field.
#[test]
fn the_dxf_reader_takes_every_group() {
    let version = DxfVersion::AC1032;
    let source = document(version);
    let wanted = source
        .objects
        .values()
        .find_map(|object| match object {
            ObjectType::MultiLeaderStyle(style) if style.description == DESCRIPTION => {
                Some(style.clone())
            }
            _ => None,
        })
        .expect("the custom style");
    let read = DxfReader::from_reader(Cursor::new(dxf_of(&source).into_bytes()))
        .expect("reader")
        .read()
        .expect("read dxf");
    let style = read
        .objects
        .values()
        .find_map(|object| match object {
            ObjectType::MultiLeaderStyle(style) if style.description == DESCRIPTION => {
                Some(style.clone())
            }
            _ => None,
        })
        .expect("the custom style after the DXF read");
    // The name is not in the record: it is the key of the dictionary entry.
    let mut style = style;
    style.name = wanted.name.clone();
    assert_eq!(style, wanted);
}
