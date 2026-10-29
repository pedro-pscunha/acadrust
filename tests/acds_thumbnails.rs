//! The layout thumbnails of the AcDs data store (`AcDb_Thumbnail_Schema`) survive a DWG round
//! trip and the DXF (`ACDSDATA`) between two DWGs.

use std::io::Cursor;

use acadrust::entities::solid3d::Solid3D;
use acadrust::entities::{AcisData, EntityType};
use acadrust::objects::ObjectType;
use acadrust::types::{DxfVersion, Handle};
use acadrust::{AcDsThumbnail, CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

/// Bytes that start as a PNG and are longer than one DXF binary chunk (127 bytes).
fn png(seed: u8, length: usize) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend((0..length).map(|index| (index as u8).wrapping_mul(7).wrapping_add(seed)));
    bytes
}

fn layouts(document: &CadDocument) -> Vec<Handle> {
    let mut handles: Vec<Handle> = document
        .objects
        .iter()
        .filter_map(|(handle, object)| matches!(object, ObjectType::Layout(_)).then_some(*handle))
        .collect();
    handles.sort_by_key(|handle| handle.value());
    handles
}

fn document(version: DxfVersion) -> CadDocument {
    let mut document = CadDocument::with_version(version);
    let handles = layouts(&document);
    assert!(handles.len() >= 2, "the model and the first layout");
    document.acds_thumbnails = vec![
        AcDsThumbnail {
            owner: handles[0],
            png: png(1, 2977),
        },
        AcDsThumbnail {
            owner: handles[1],
            png: png(9, 300),
        },
    ];
    document
}

fn through_dwg(document: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(document).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

#[test]
fn a_dwg_keeps_the_thumbnails() {
    for version in [DxfVersion::AC1027, DxfVersion::AC1032] {
        let source = document(version);
        let read = through_dwg(&source);
        assert_eq!(read.acds_thumbnails, source.acds_thumbnails, "{version:?}");
        // A second round trip reuses the section and keeps them.
        assert_eq!(
            through_dwg(&read).acds_thumbnails,
            source.acds_thumbnails,
            "{version:?}: second round trip"
        );
    }
}

#[test]
fn a_dwg_keeps_the_thumbnails_next_to_a_solid() {
    let sat = acadrust::entities::acis::primitives::build_planar_body(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        &[
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![3, 7, 6, 2],
            vec![1, 2, 6, 5],
            vec![0, 4, 7, 3],
        ],
    )
    .expect("box");
    let sab = acadrust::SabWriter::write(&sat);
    let mut source = document(DxfVersion::AC1032);
    let mut solid = Solid3D::new();
    solid.acis_data = AcisData::from_sab(sab.clone());
    let handle = source
        .add_entity(EntityType::Solid3D(solid))
        .expect("add solid");
    let read = through_dwg(&source);
    assert_eq!(read.acds_thumbnails, source.acds_thumbnails);
    let Some(EntityType::Solid3D(solid)) = read.get_entity(handle) else {
        panic!("the solid is missing");
    };
    assert_eq!(solid.acis_data.sab_data, sab, "the SAB stays beside them");
}

#[test]
fn the_thumbnails_survive_a_dxf_between_two_dwgs() {
    for bridge in [false, true] {
        let source = document(DxfVersion::AC1032);
        let read = through_dwg(&source);
        let writer = DxfWriter::new(&read);
        let writer = if bridge {
            writer.for_dwg_bridge()
        } else {
            writer
        };
        let dxf = writer.write_to_vec().expect("write dxf");
        let text = String::from_utf8(dxf.clone()).expect("utf8");
        assert!(text.contains("ACDSDATA"), "bridge {bridge}");
        assert!(text.contains("Thumbnail_Data"), "bridge {bridge}");
        let from_dxf = DxfReader::from_reader(Cursor::new(dxf))
            .expect("reader")
            .read()
            .expect("read dxf");
        assert_eq!(
            from_dxf.acds_thumbnails, source.acds_thumbnails,
            "bridge {bridge}: DXF"
        );
        assert_eq!(
            through_dwg(&from_dxf).acds_thumbnails,
            source.acds_thumbnails,
            "bridge {bridge}: DWG"
        );
    }
}

/// The layout AutoCAD writes for thumbnails alone (DXFOUT of a DWG without solids): five schemas,
/// numbered from 0, then one record per layout with the owner in group 320, the byte count in 94
/// and the PNG in groups 310.
#[test]
fn a_dxf_in_the_layout_of_autocad_gives_the_thumbnails() {
    let first = png(3, 200);
    let second = png(5, 40);
    let hex = |bytes: &[u8]| -> Vec<String> {
        bytes
            .chunks(127)
            .map(|chunk| chunk.iter().map(|byte| format!("{byte:02X}")).collect())
            .collect()
    };
    let mut pairs: Vec<(i32, String)> = vec![
        (0, "SECTION".into()),
        (2, "HEADER".into()),
        (9, "$ACADVER".into()),
        (1, "AC1027".into()),
        (0, "ENDSEC".into()),
        (0, "SECTION".into()),
        (2, "ACDSDATA".into()),
        (70, "2".into()),
        (71, "8".into()),
        (0, "ACDSSCHEMA".into()),
        (90, "0".into()),
        (1, "AcDb_Thumbnail_Schema".into()),
        (2, "AcDbDs::ID".into()),
        (280, "10".into()),
        (91, "8".into()),
        (2, "Thumbnail_Data".into()),
        (280, "15".into()),
        (91, "0".into()),
        (101, "ACDSRECORD".into()),
        (95, "0".into()),
        (90, "1".into()),
        (2, "AcDbDs::TreatedAsObjectData".into()),
        (280, "1".into()),
        (291, "1".into()),
        (0, "ACDSSCHEMA".into()),
        (90, "1".into()),
        (1, "AcDbDs::TreatedAsObjectDataSchema".into()),
        (2, "AcDbDs::TreatedAsObjectData".into()),
        (280, "1".into()),
        (91, "0".into()),
    ];
    for (owner, bytes) in [("1E", &first), ("22", &second)] {
        pairs.extend([
            (0, "ACDSRECORD".into()),
            (90, "0".into()),
            (2, "AcDbDs::ID".into()),
            (280, "10".into()),
            (320, owner.into()),
            (2, "Thumbnail_Data".into()),
            (280, "15".into()),
            (94, bytes.len().to_string()),
        ]);
        pairs.extend(hex(bytes).into_iter().map(|chunk| (310, chunk)));
    }
    pairs.extend([(0, "ENDSEC".into()), (0, "EOF".into())]);
    let text: String = pairs
        .into_iter()
        .map(|(code, value)| format!("{code}\n{value}\n"))
        .collect();
    let document = DxfReader::from_reader(Cursor::new(text.into_bytes()))
        .expect("reader")
        .read()
        .expect("read dxf");
    assert_eq!(
        document.acds_thumbnails,
        vec![
            AcDsThumbnail {
                owner: Handle::new(0x1E),
                png: first,
            },
            AcDsThumbnail {
                owner: Handle::new(0x22),
                png: second,
            },
        ]
    );
}

/// A layout the reader created by default can collide with a handle of the file and get a new
/// one; the thumbnail it owns follows it.
#[test]
fn the_owner_follows_a_layout_that_gets_a_new_handle() {
    let defaults = layouts(&CadDocument::new());
    let colliding = defaults[0];
    // A layer of the file holds the handle of the default layout.
    let text = format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1032\n0\nENDSEC\n\
         0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n70\n1\n\
         0\nLAYER\n5\n{h:X}\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n\
         2\nCLASH\n70\n0\n62\n7\n6\nCONTINUOUS\n0\nENDTAB\n0\nENDSEC\n\
         0\nSECTION\n2\nACDSDATA\n70\n2\n71\n2\n0\nACDSRECORD\n90\n0\n2\nAcDbDs::ID\n280\n10\n\
         320\n{h:X}\n2\nThumbnail_Data\n280\n15\n94\n9\n310\n89504E470D0A1A0A00\n0\nENDSEC\n0\nEOF\n",
        h = colliding.value()
    );
    let document = DxfReader::from_reader(Cursor::new(text.into_bytes()))
        .expect("reader")
        .read()
        .expect("read dxf");
    assert_eq!(document.acds_thumbnails.len(), 1);
    let owner = document.acds_thumbnails[0].owner;
    assert_ne!(owner, colliding, "the layout got a new handle");
    assert!(
        matches!(document.objects.get(&owner), Some(ObjectType::Layout(_))),
        "the thumbnail follows the layout"
    );
}
