//! A DWG goes through DXF as a bridge (DwgReader, DxfWriter, a host, DxfReader, DwgWriter). Objects
//! that exist only as DWG bytes (an AutoCAD Architecture display configuration, a class acadrust
//! does not decode) have no DXF form, and the DYNAMICBLOCKPURGEPREVENTER record is one BricsCAD's
//! audit skips. `DxfWriter::for_dwg_bridge` writes them as private records that the DXF reader
//! reads back, so the DWG writer writes them again; a normal DXF never gets them.

use std::io::Cursor;

use acadrust::classes::arch::ArchObject;
use acadrust::classes::DxfClass;
use acadrust::io::dwg::dwg_stream_readers::object_reader::DwgObjectReader;
use acadrust::io::dwg::dwg_stream_writers::DwgMergedWriter;
use acadrust::io::dwg::{DwgReferenceType, DwgVersion};
use acadrust::objects::{
    BlockRepresentationData, Dictionary, DynamicBlockData, DynamicBlockObject, ObjectType,
};
use acadrust::types::{DxfVersion, Handle};
use acadrust::xdata::XDataValue;
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const AEC: &str = "AECS_DISP_REP_MEMBER_MODEL_DETAIL";
const UNKNOWN: &str = "TAGROOTDATA";

/// The merged-stream bytes of an object record of class `class_number`: the common header and
/// a small opaque body, like the objects a DWG reader keeps without decoding.
fn raw_record(
    doc: &CadDocument,
    class_number: i16,
    handle: Handle,
    owner: Handle,
    xdictionary: Option<Handle>,
    target: Handle,
    eed: &[(Handle, Vec<u8>)],
) -> (Vec<u8>, i64) {
    let mut writer = DwgMergedWriter::new(
        DwgVersion::from_dxf_version(doc.version).unwrap(),
        doc.version,
    );
    writer.write_object_type(class_number);
    writer.main_mut().write_handle_undefined(handle.value());
    for (application, bytes) in eed {
        writer.write_bit_short(bytes.len() as i16);
        writer
            .main_mut()
            .write_handle(DwgReferenceType::HardPointer, application.value());
        writer.write_bytes(bytes);
    }
    writer.write_bit_short(0); // EED terminator.
    writer.write_handle(DwgReferenceType::SoftPointer, owner.value());
    writer.write_bit_long(1); // One reactor.
    writer.write_handle(DwgReferenceType::SoftPointer, owner.value());
    writer.write_bit(xdictionary.is_none());
    if let Some(x) = xdictionary {
        writer.write_handle(DwgReferenceType::HardOwnership, x.value());
    }
    writer.write_bit(false); // No AcDs payload.
    writer.write_raw_long(0x1234_5678);
    writer.write_bit_double(123.75);
    writer.write_variable_text("opaque class state");
    writer.write_handle(DwgReferenceType::HardPointer, target.value());
    let raw = writer.merge();
    let handle_bits = raw.len() as i64 * 8 - writer.handle_start_bits();
    (raw, handle_bits)
}

struct Fixture {
    doc: CadDocument,
    arch: Handle,
    unknown: Handle,
    xdictionary: Handle,
    root: Handle,
    /// The object the class data of both records refers to.
    target: Handle,
}

fn fixture() -> Fixture {
    let mut doc = CadDocument::new();
    doc.version = DxfVersion::AC1032;
    doc.classes
        .add_or_update(DxfClass::new(AEC, "AecsDbDispRepMemberModelDetail"));
    doc.classes
        .add_or_update(DxfClass::new(UNKNOWN, "AcDbTagRootData"));
    let root = doc.header.named_objects_dict_handle;
    let arch = doc.allocate_handle();
    let unknown = doc.allocate_handle();
    let xdictionary = doc.allocate_handle();
    let target = doc.allocate_handle();
    let mut dictionary = Dictionary::new();
    dictionary.handle = xdictionary;
    dictionary.owner = arch;
    doc.objects
        .insert(xdictionary, ObjectType::Dictionary(dictionary));
    let mut referred = Dictionary::new();
    referred.handle = target;
    referred.owner = root;
    doc.objects.insert(target, ObjectType::Dictionary(referred));

    let number = doc.classes.get_by_name(AEC).unwrap().class_number;
    let (raw, bits) = raw_record(&doc, number, arch, root, Some(xdictionary), target, &[]);
    let mut object = ArchObject::new(AEC, arch, root);
    object.reactors = vec![root];
    object.xdictionary_handle = Some(xdictionary);
    object.raw_dwg_data = Some(raw);
    object.raw_dwg_handle_bits = bits;
    object.raw_dwg_version = Some(doc.version);
    doc.objects.insert(arch, ObjectType::Arch(object));

    let number = doc.classes.get_by_name(UNKNOWN).unwrap().class_number;
    let (raw, bits) = raw_record(&doc, number, unknown, root, None, target, &[]);
    doc.objects.insert(
        unknown,
        ObjectType::Unknown {
            type_name: UNKNOWN.to_string(),
            handle: unknown,
            owner: root,
            raw_dxf_codes: None,
            raw_dwg_data: Some(raw),
            raw_dwg_handle_bits: bits,
            raw_dwg_version: Some(doc.version),
        },
    );
    let Some(ObjectType::Dictionary(dictionary)) = doc.objects.get_mut(&root) else {
        panic!()
    };
    dictionary.add_entry("AEC test", arch);
    dictionary.add_entry("TAG test", unknown);
    dictionary.add_entry("referred", target);
    Fixture {
        doc,
        arch,
        unknown,
        xdictionary,
        root,
        target,
    }
}

fn text(writer: DxfWriter) -> String {
    String::from_utf8(writer.write_to_vec().expect("write dxf")).expect("utf8")
}

/// A record's bytes with the size and handle-bit prefix a reader takes.
fn framed_record(raw: &[u8], handle_bits: i64) -> Vec<u8> {
    assert!(raw.len() < 0x8000);
    let mut framed = (raw.len() as u16).to_le_bytes().to_vec();
    let mut remaining = handle_bits as u64;
    loop {
        let byte = (remaining & 0x7f) as u8;
        remaining >>= 7;
        framed.push(byte | if remaining != 0 { 0x80 } else { 0 });
        if remaining == 0 {
            break;
        }
    }
    framed.extend_from_slice(raw);
    framed
}

fn read_dxf(bytes: Vec<u8>) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn bridged(doc: &CadDocument) -> CadDocument {
    read_dxf(
        DxfWriter::new(doc)
            .for_dwg_bridge()
            .write_to_vec()
            .expect("write dxf"),
    )
}

/// The DXF names of the CLASS records.
fn class_names(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut names = Vec::new();
    for (i, pair) in lines.chunks(2).enumerate() {
        if pair[0] == "0" && pair.get(1) == Some(&"CLASS") {
            if let Some(next) = lines.chunks(2).nth(i + 1) {
                names.push(next.get(1).copied().unwrap_or("").to_string());
            }
        }
    }
    names
}

fn records(text: &str, name: &str) -> usize {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    lines
        .chunks(2)
        .filter(|p| p[0] == "0" && p.get(1) == Some(&name))
        .count()
}

#[test]
fn a_normal_dxf_has_neither_object() {
    let f = fixture();
    let dxf = text(DxfWriter::new(&f.doc));
    assert_eq!((records(&dxf, AEC), records(&dxf, UNKNOWN)), (0, 0));
    assert_eq!(
        DxfWriter::new(&f.doc).omitted_architecture_records().len(),
        1
    );
}

#[test]
fn the_bridge_writes_them_under_their_class_names() {
    let f = fixture();
    let dxf = text(DxfWriter::new(&f.doc).for_dwg_bridge());
    assert_eq!((records(&dxf, AEC), records(&dxf, UNKNOWN)), (1, 1));
    assert!(DxfWriter::new(&f.doc)
        .for_dwg_bridge()
        .omitted_architecture_records()
        .is_empty());
    // The CLASSES section keeps their classes, for a host that keeps classes by name.
    let classes = class_names(&dxf);
    assert!(
        classes.iter().any(|c| c == AEC) && classes.iter().any(|c| c == UNKNOWN),
        "{classes:?}"
    );
}

#[test]
fn the_dxf_reader_gives_back_the_raw_form() {
    let f = fixture();
    let back = bridged(&f.doc);
    let ObjectType::Arch(was) = f.doc.objects.get(&f.arch).unwrap() else {
        panic!()
    };
    let Some(ObjectType::Arch(now)) = back.objects.get(&f.arch) else {
        panic!("Arch object lost")
    };
    assert_eq!(now.dxf_name, was.dxf_name);
    assert_eq!(now.cpp_class_name, was.cpp_class_name);
    assert_eq!(
        (now.owner, &now.reactors, now.xdictionary_handle),
        (f.root, &vec![f.root], Some(f.xdictionary))
    );
    assert_eq!(now.raw_dwg_data, was.raw_dwg_data);
    assert_eq!(now.raw_dwg_handle_bits, was.raw_dwg_handle_bits);
    assert_eq!(now.raw_dwg_version, Some(DxfVersion::AC1032));
    assert!(now.raw_dxf_codes.is_none());
    let references = now.raw_dwg_references.as_ref().expect("references");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].handle, f.target);

    // A class that is not an Arch class gets the same form, with its handles and owner.
    let Some(ObjectType::Arch(now)) = back.objects.get(&f.unknown) else {
        panic!("TAGROOTDATA lost")
    };
    let ObjectType::Unknown {
        raw_dwg_data,
        raw_dwg_handle_bits,
        ..
    } = f.doc.objects.get(&f.unknown).unwrap()
    else {
        panic!()
    };
    assert_eq!((now.dxf_name.as_str(), now.owner), (UNKNOWN, f.root));
    assert_eq!(&now.raw_dwg_data, raw_dwg_data);
    assert_eq!(&now.raw_dwg_handle_bits, raw_dwg_handle_bits);
    assert_eq!(now.raw_dwg_version, Some(DxfVersion::AC1032));
}

/// The DXF with every handle group that names `from` renamed to `to`,
/// as a host that renumbers handles would (the target's own record included).
fn renumbered(text: &str, from: Handle, to: Handle) -> String {
    let (from, to) = (format!("{:X}", from.value()), format!("{:X}", to.value()));
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for pair in lines.chunks(2) {
        let code: i32 = pair[0].trim().parse().unwrap_or(0);
        let value = pair.get(1).map_or("", |v| v.trim());
        out.push(pair[0].to_string());
        if let Some(v) = pair.get(1) {
            let pointer = matches!(code, 5 | 330 | 340 | 350 | 360 | 390);
            out.push(if pointer && value == from {
                to.clone()
            } else {
                v.to_string()
            });
        }
    }
    out.join("\r\n")
}

#[test]
fn a_host_that_renumbers_handles_is_followed_into_the_class_data() {
    let f = fixture();
    let dxf = text(DxfWriter::new(&f.doc).for_dwg_bridge());
    let moved = Handle::new(f.target.value() + 0x4000);
    let back = read_dxf(renumbered(&dxf, f.target, moved).into_bytes());
    let Some(ObjectType::Arch(now)) = back.objects.get(&f.arch) else {
        panic!("Arch object lost")
    };
    assert_eq!(now.raw_dwg_references.as_ref().unwrap()[0].handle, moved);
    // The DWG holds the new handle in the class data's handle stream.
    let bytes = DwgWriter::write_to_vec(&back).expect("write dwg");
    let again = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let Some(ObjectType::Arch(arch)) = again.objects.get(&f.arch) else {
        panic!()
    };
    let raw = arch.raw_dwg_data.as_ref().unwrap();
    let objects = DwgObjectReader::new(
        framed_record(raw, arch.raw_dwg_handle_bits),
        again.version,
        Default::default(),
    )
    .unwrap();
    let (type_code, mut reader) = objects.read_record_at(0).unwrap();
    let common = objects.read_common_non_entity_data(&mut reader, type_code);
    assert_eq!(common.common.handle, f.arch.value());
    assert_eq!(reader.read_raw_long(), 0x1234_5678);
    assert_eq!(reader.read_bit_double(), 123.75);
    assert_eq!(reader.read_variable_text(), "opaque class state");
    let (target, kind) = reader.read_typed_handle();
    assert_eq!(target, moved.value());
    assert_eq!(kind, DwgReferenceType::HardPointer);
}

#[test]
fn the_dwg_writer_writes_them_again() {
    let f = fixture();
    let bytes = DwgWriter::write_to_vec(&bridged(&f.doc)).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let ObjectType::Arch(arch) = back
        .objects
        .get(&f.arch)
        .expect("Arch object lost in the DWG")
    else {
        panic!()
    };
    assert_eq!(
        (arch.owner, arch.xdictionary_handle),
        (f.root, Some(f.xdictionary))
    );
    let was = match f.doc.objects.get(&f.arch).unwrap() {
        ObjectType::Arch(a) => a,
        _ => panic!(),
    };
    // The body after the common header is the one that went in.
    assert_eq!(
        arch.raw_dwg_data.as_ref().map(Vec::len),
        was.raw_dwg_data.as_ref().map(Vec::len)
    );
    // A DWG reader reads a class that is not an Arch class as an unknown object again.
    assert!(matches!(
        back.objects.get(&f.unknown),
        Some(ObjectType::Unknown {
            raw_dwg_data: Some(_),
            ..
        })
    ));
    let ObjectType::Dictionary(root) = back.objects.get(&f.root).unwrap() else {
        panic!()
    };
    assert_eq!(
        (root.get("AEC test"), root.get("TAG test")),
        (Some(f.arch), Some(f.unknown))
    );
}

// ── EED of a raw record ─────────────────────────────────────────────────

const APPLICATION: &str = "AECEED";

/// An EED block (R2007+ encoding) of a string and a short.
fn eed_bytes() -> Vec<u8> {
    let units: Vec<u16> = "kept text".encode_utf16().collect();
    let mut bytes = vec![0];
    bytes.extend_from_slice(&(units.len() as u16).to_le_bytes());
    for unit in units {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes.push(70);
    bytes.extend_from_slice(&7i16.to_le_bytes());
    bytes
}

/// The fixture with an APPID (a handle of its own) and an EED block of that application in the
/// common header of the Arch record; with `registered` false the block names a handle no APPID
/// has.
fn with_eed(registered: bool) -> (Fixture, Handle) {
    let mut f = fixture();
    let application = f.doc.allocate_handle();
    if registered {
        let mut app = acadrust::tables::AppId::new(APPLICATION);
        app.handle = application;
        f.doc.app_ids.add(app).expect("add appid");
    }
    let number = f.doc.classes.get_by_name(AEC).unwrap().class_number;
    let (raw, bits) = raw_record(
        &f.doc,
        number,
        f.arch,
        f.root,
        Some(f.xdictionary),
        f.target,
        &[(application, eed_bytes())],
    );
    let Some(ObjectType::Arch(object)) = f.doc.objects.get_mut(&f.arch) else {
        panic!()
    };
    object.raw_dwg_data = Some(raw);
    object.raw_dwg_handle_bits = bits;
    (f, application)
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

/// The APPID of `name` in `doc` and the values of the object's EED record of that application.
fn eed_of(doc: &CadDocument, object: Handle, name: &str) -> Option<(Handle, Vec<XDataValue>)> {
    let record = doc.object_xdata.get(&object)?.get_record(name)?;
    Some((doc.app_ids.get(name)?.handle, record.values.clone()))
}

fn eed_values() -> Vec<XDataValue> {
    vec![
        XDataValue::String("kept text".to_string()),
        XDataValue::Integer16(7),
    ]
}

#[test]
fn a_dwg_keeps_the_eed_of_a_raw_record() {
    let (f, application) = with_eed(true);
    let back = through_dwg(&f.doc);
    assert_eq!(
        eed_of(&back, f.arch, APPLICATION),
        Some((application, eed_values()))
    );
}

#[test]
fn the_eed_of_a_raw_record_follows_a_renumbered_appid() {
    let (f, application) = with_eed(true);
    let dxf = text(DxfWriter::new(&f.doc).for_dwg_bridge());
    // The EED travels as XDATA of the record, which names the application by name.
    assert!(
        dxf.contains("1001\n") || dxf.contains("1001\r\n"),
        "no XDATA"
    );
    let moved = Handle::new(application.value() + 0x5000);
    let back = read_dxf(renumbered(&dxf, application, moved).into_bytes());
    assert_eq!(back.app_ids.get(APPLICATION).unwrap().handle, moved);
    let again = through_dwg(&back);
    // The DWG names the application the host renumbered, not the stale handle.
    assert_eq!(
        eed_of(&again, f.arch, APPLICATION),
        Some((moved, eed_values()))
    );
    assert!(again.app_ids.iter().all(|app| app.handle != application));
}

/// A host that gives the stale handle of the application to another APPID must not move the
/// block to that application.
#[test]
fn the_eed_of_a_raw_record_does_not_move_to_the_appid_that_takes_the_old_handle() {
    let (mut f, application) = with_eed(true);
    let other = f.doc.allocate_handle();
    let mut app = acadrust::tables::AppId::new("OTHERAPP");
    app.handle = other;
    f.doc.app_ids.add(app).expect("add appid");
    let dxf = text(DxfWriter::new(&f.doc).for_dwg_bridge());
    let moved = Handle::new(application.value() + 0x5000);
    let swapped = renumbered(&renumbered(&dxf, application, moved), other, application);
    let back = read_dxf(swapped.into_bytes());
    assert_eq!(back.app_ids.get("OTHERAPP").unwrap().handle, application);
    let again = through_dwg(&back);
    assert_eq!(
        eed_of(&again, f.arch, APPLICATION),
        Some((moved, eed_values()))
    );
    assert_eq!(eed_of(&again, f.arch, "OTHERAPP"), None);
}

#[test]
fn an_eed_block_of_an_unknown_application_is_dropped_and_reported() {
    let (f, application) = with_eed(false);
    let writer = DxfWriter::new(&f.doc).for_dwg_bridge();
    let omitted = writer.omitted_architecture_records();
    assert_eq!(omitted.len(), 1, "{omitted:?}");
    assert_eq!(omitted[0].handle, f.arch);
    assert!(
        omitted[0]
            .reason
            .contains(&format!("{:X}", application.value())),
        "{}",
        omitted[0].reason
    );
    let back = through_dwg(&bridged(&f.doc));
    // The record is there, with no EED naming an application the file does not hold.
    assert!(matches!(
        back.objects.get(&f.arch),
        Some(ObjectType::Arch(_))
    ));
    assert!(back
        .object_xdata
        .get(&f.arch)
        .is_none_or(|data| data.records().is_empty()));
}

// ── DYNAMICBLOCKPURGEPREVENTER ──────────────────────────────────────────

const PREVENTER: &str = "ACDB_DYNAMICBLOCKPURGEPREVENTER_VERSION";

fn with_preventer() -> (CadDocument, Handle, Handle) {
    let mut doc = CadDocument::new();
    doc.version = DxfVersion::AC1032;
    assert!(
        doc.classes.get_by_name(PREVENTER).is_some(),
        "the default classes have it"
    );
    let block = doc.block_records.get("*Model_Space").unwrap().handle;
    let handle = doc.allocate_handle();
    let mut object = DynamicBlockObject::new(PREVENTER, "AcDbDynamicBlockPurgePreventer");
    object.handle = handle;
    object.owner = doc.header.named_objects_dict_handle;
    object.data = DynamicBlockData::Representation(BlockRepresentationData { flags: 1, block });
    doc.objects.insert(handle, ObjectType::DynamicBlock(object));
    let Some(ObjectType::Dictionary(root)) =
        doc.objects.get_mut(&doc.header.named_objects_dict_handle)
    else {
        panic!()
    };
    root.add_entry("AcDbDynamicBlockPurgePreventer", handle);
    (doc, handle, block)
}

#[test]
fn a_normal_dxf_still_leaves_the_purge_preventer_out() {
    let (doc, _, _) = with_preventer();
    assert_eq!(records(&text(DxfWriter::new(&doc)), PREVENTER), 0);
}

#[test]
fn the_bridge_keeps_the_purge_preventer_to_the_dwg() {
    let (doc, handle, block) = with_preventer();
    let dxf = text(DxfWriter::new(&doc).for_dwg_bridge());
    assert_eq!(records(&dxf, PREVENTER), 1);
    let bridged = read_dxf(dxf.into_bytes());
    let bytes = DwgWriter::write_to_vec(&bridged).expect("write dwg");
    let back = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let Some(ObjectType::DynamicBlock(object)) = back.objects.get(&handle) else {
        panic!("the purge preventer was lost")
    };
    assert_eq!(object.dxf_name, PREVENTER);
    assert_eq!(
        object.data,
        DynamicBlockData::Representation(BlockRepresentationData { flags: 1, block })
    );
}
