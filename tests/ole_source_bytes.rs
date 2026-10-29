//! An OLE object's compound file keeps the bytes it was read from while its content is
//! unchanged, as AutoCAD keeps an OLE object's bytes on save. Rebuilding it changed every
//! sector's place and, for a file whose last sector is shorter than a whole sector, lost the
//! stream bytes in that sector (Planeta Toulouse: 428 bytes of an Excel table's preview image).

use acadrust::compound_file::{
    BinaryRecord, CompoundEntry, CompoundStorage, CompoundStreamContent, StructuredStoragePayload,
};
use acadrust::entities::{Ole2Frame, OleFrameEnvelope};

const FREE: u32 = 0xFFFF_FFFF;
const END: u32 = 0xFFFF_FFFE;
const FAT_SECTOR: u32 = 0xFFFF_FFFD;

/// Synthetic bytes, not a real OLE object's.
fn pattern(n: usize, seed: u32) -> Vec<u8> {
    (0..n as u32)
        .map(|i| (i.wrapping_mul(2_654_435_761).wrapping_add(seed) >> 13) as u8)
        .collect()
}

fn put(buf: &mut [u8], at: usize, bytes: &[u8]) {
    buf[at..at + bytes.len()].copy_from_slice(bytes);
}

/// A version 3 compound file by hand: header, FAT in sector 0, directory in sector 1, and one
/// stream `CONTENTS` from sector 2 on. It ends right after the stream's last byte, so its last
/// sector is short, as other programs write them.
fn hand_made(stream: &[u8]) -> Vec<u8> {
    let sectors = stream.len().div_ceil(512) as u32;
    let mut header = vec![0u8; 512];
    put(
        &mut header,
        0,
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
    );
    put(&mut header, 24, &0x003Eu16.to_le_bytes());
    put(&mut header, 26, &3u16.to_le_bytes());
    put(&mut header, 28, &0xFFFEu16.to_le_bytes());
    put(&mut header, 30, &9u16.to_le_bytes());
    put(&mut header, 32, &6u16.to_le_bytes());
    put(&mut header, 44, &1u32.to_le_bytes());
    put(&mut header, 48, &1u32.to_le_bytes());
    put(&mut header, 56, &4096u32.to_le_bytes());
    put(&mut header, 60, &END.to_le_bytes());
    put(&mut header, 68, &END.to_le_bytes());
    put(&mut header, 76, &0u32.to_le_bytes());
    for i in 1..109 {
        put(&mut header, 76 + 4 * i, &FREE.to_le_bytes());
    }
    let mut fat = vec![0xFFu8; 512];
    put(&mut fat, 0, &FAT_SECTOR.to_le_bytes());
    put(&mut fat, 4, &END.to_le_bytes());
    for s in 2..2 + sectors {
        let next = if s == 1 + sectors { END } else { s + 1 };
        put(&mut fat, 4 * s as usize, &next.to_le_bytes());
    }
    let entry = |name: &str, kind: u8, child: u32, start: u32, size: u32| {
        let mut e = vec![0u8; 128];
        let utf16: Vec<u8> = name
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        put(&mut e, 0, &utf16);
        put(&mut e, 64, &(utf16.len() as u16).to_le_bytes());
        e[66] = kind;
        e[67] = 1;
        put(&mut e, 68, &FREE.to_le_bytes());
        put(&mut e, 72, &FREE.to_le_bytes());
        put(&mut e, 76, &child.to_le_bytes());
        put(&mut e, 116, &start.to_le_bytes());
        put(&mut e, 120, &size.to_le_bytes());
        e
    };
    let mut dir = entry("Root Entry", 5, 1, END, 0);
    dir.extend(entry("CONTENTS", 2, FREE, 2, stream.len() as u32));
    for _ in 0..2 {
        let mut empty = vec![0u8; 128];
        put(&mut empty, 68, &FREE.to_le_bytes());
        put(&mut empty, 72, &FREE.to_le_bytes());
        put(&mut empty, 76, &FREE.to_le_bytes());
        dir.extend(empty);
    }
    let mut out = header;
    out.extend(fat);
    out.extend(dir);
    out.extend_from_slice(stream);
    out
}

fn contents(storage: &CompoundStorage) -> Vec<u8> {
    storage
        .entries
        .iter()
        .find_map(|e| match e {
            CompoundEntry::Stream(s) if s.name == "CONTENTS" => match &s.content {
                CompoundStreamContent::Binary(records) => Some(BinaryRecord::join(records)),
                _ => None,
            },
            _ => None,
        })
        .expect("the CONTENTS stream")
}

#[test]
fn a_short_last_sector_keeps_its_stream_bytes() {
    let stream = pattern(4097, 7);
    let payload = StructuredStoragePayload::decode(&hand_made(&stream));
    let file = payload.compound_file.as_ref().expect("a compound file");
    assert_eq!(contents(&file.root), stream);
}

#[test]
fn an_unchanged_compound_file_keeps_its_bytes() {
    let source = hand_made(&pattern(4097, 7));
    let mut data = pattern(98, 1);
    data.extend(&source);
    data.extend(pattern(5, 3));
    assert_eq!(StructuredStoragePayload::decode(&data).encode(), data);
}

#[test]
fn a_changed_compound_file_is_written_from_its_content() {
    let source = hand_made(&pattern(4097, 7));
    let mut payload = StructuredStoragePayload::decode(&source);
    let changed = pattern(5000, 9);
    let file = payload.compound_file.as_mut().expect("a compound file");
    for e in &mut file.root.entries {
        if let CompoundEntry::Stream(s) = e {
            s.content = CompoundStreamContent::Binary(BinaryRecord::split(&changed, 4096));
        }
    }
    let written = payload.encode();
    assert_ne!(written, source);
    let back = StructuredStoragePayload::decode(&written);
    assert_eq!(
        contents(&back.compound_file.expect("a compound file").root),
        changed
    );
}

/// The DWG wrapper in front of the compound file (marker, the four corners, 30 more bytes)
/// ends with the compound file's length; a rebuilt file of another length gets its own.
#[test]
fn the_wrapper_names_the_written_compound_file_length() {
    let source = hand_made(&pattern(4097, 7));
    let mut extension = pattern(26, 5);
    extension.extend((source.len() as u32).to_le_bytes());
    let mut ole = Ole2Frame::new();
    ole.envelope = OleFrameEnvelope::Geometry {
        marker: 0x5580,
        extension_records: BinaryRecord::split(&extension, 4096),
    };
    ole.storage = StructuredStoragePayload::decode(&source);
    let unchanged = ole.encoded_payload();
    assert_eq!(
        &unchanged[128..],
        &source[..],
        "the source bytes, unchanged"
    );
    assert_eq!(&unchanged[124..128], &(source.len() as u32).to_le_bytes());
    let file = ole.storage.compound_file.as_mut().expect("a compound file");
    for e in &mut file.root.entries {
        if let CompoundEntry::Stream(s) = e {
            s.content = CompoundStreamContent::Binary(BinaryRecord::split(&pattern(5000, 9), 4096));
        }
    }
    let rebuilt = ole.encoded_payload();
    assert_eq!(
        &rebuilt[124..128],
        &((rebuilt.len() - 128) as u32).to_le_bytes()
    );
}
