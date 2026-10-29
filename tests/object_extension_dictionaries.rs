//! MLEADERSTYLE, TABLESTYLE and SCALE objects keep their extension dictionary (and, for the two
//! styles, their XDATA) through DWG, DXF and the DXF to DWG bridge. AutoCAD hangs
//! ACAD_XREC_ROUNDTRIP records on mleader styles and the cell style map on table styles there;
//! the DXF writer left the extension dictionary pointer out of these three records and the DXF
//! reader did not keep it.

use std::io::Cursor;

use acadrust::objects::{ObjectType, Scale};
use acadrust::tables::AppId;
use acadrust::types::{DxfVersion, Handle};
use acadrust::xdata::{ExtendedDataRecord, XDataValue};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

const KEY: &str = "ACAD_XREC_ROUNDTRIP";

#[derive(Clone, Copy, Debug)]
enum Kind {
    MLeaderStyle,
    TableStyle,
    Scale,
}

/// A document whose object of `kind` has an extension dictionary holding an XRECORD under
/// ACAD_XREC_ROUNDTRIP and, when `xdata`, an XDATA record of the application ACME.
fn document(kind: Kind, xdata: bool) -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let handle = match kind {
        Kind::MLeaderStyle => doc
            .objects
            .iter()
            .find_map(|(h, o)| matches!(o, ObjectType::MultiLeaderStyle(_)).then_some(*h)),
        Kind::TableStyle => doc
            .objects
            .iter()
            .find_map(|(h, o)| matches!(o, ObjectType::TableStyle(_)).then_some(*h)),
        Kind::Scale => {
            let handle = doc.allocate_handle();
            let mut scale = Scale::new("1:7", 1.0, 7.0);
            scale.handle = handle;
            scale.owner_handle = doc.header.named_objects_dict_handle;
            doc.objects.insert(handle, ObjectType::Scale(scale));
            Some(handle)
        }
    }
    .unwrap_or_else(|| panic!("{kind:?}: no such object in a new drawing"));
    doc.ensure_xrecord(handle, KEY);
    if xdata {
        doc.app_ids.add(AppId::new("ACME")).expect("add appid");
        let mut record = ExtendedDataRecord::new("ACME");
        record.values = vec![XDataValue::String("kept".into()), XDataValue::Integer16(7)];
        doc.set_object_xdata_record(handle, record);
    }
    (doc, handle)
}

fn through_dwg(doc: &CadDocument) -> CadDocument {
    let bytes = DwgWriter::write_to_vec(doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc).write_to_vec().expect("write dxf");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("reader")
        .read()
        .expect("read dxf")
}

fn check(doc: &CadDocument, handle: Handle, xdata: bool, what: &str) {
    let dictionary = doc
        .extension_dictionary_handle(handle)
        .unwrap_or_else(|| panic!("{what}: no extension dictionary"));
    let Some(ObjectType::Dictionary(entries)) = doc.objects.get(&dictionary) else {
        panic!("{what}: the extension dictionary is not a dictionary")
    };
    let record = entries
        .get(KEY)
        .unwrap_or_else(|| panic!("{what}: no {KEY} entry"));
    assert!(
        matches!(doc.objects.get(&record), Some(ObjectType::XRecord(_))),
        "{what}: {KEY} is not an XRECORD"
    );
    if xdata {
        let data = doc
            .object_xdata(handle)
            .and_then(|x| x.get_record("ACME"))
            .unwrap_or_else(|| panic!("{what}: no ACME xdata"));
        assert_eq!(
            data.values,
            vec![XDataValue::String("kept".into()), XDataValue::Integer16(7)],
            "{what}"
        );
    }
}

fn run(kind: Kind, xdata: bool) {
    let (doc, handle) = document(kind, xdata);
    check(&doc, handle, xdata, &format!("{kind:?} source"));
    check(&through_dwg(&doc), handle, xdata, &format!("{kind:?} DWG"));
    check(&through_dxf(&doc), handle, xdata, &format!("{kind:?} DXF"));
    check(
        &through_dwg(&through_dxf(&doc)),
        handle,
        xdata,
        &format!("{kind:?} DXF to DWG"),
    );
}

#[test]
fn an_mleader_style_keeps_its_extension_dictionary_and_xdata() {
    run(Kind::MLeaderStyle, true);
}

#[test]
fn a_table_style_keeps_its_extension_dictionary_and_xdata() {
    run(Kind::TableStyle, true);
}

#[test]
fn a_scale_keeps_its_extension_dictionary() {
    run(Kind::Scale, false);
}

/// The groups of the DXF record with handle `handle`.
fn record(text: &str, handle: Handle) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let pairs: Vec<(String, String)> = lines
        .chunks(2)
        .map(|p| {
            (
                p[0].trim().to_string(),
                p.get(1).map_or(String::new(), |v| v.trim().to_string()),
            )
        })
        .collect();
    let wanted = format!("{:X}", handle.value());
    let at = pairs
        .iter()
        .position(|(c, v)| c == "5" && *v == wanted)
        .unwrap_or_else(|| panic!("no record {wanted}"));
    pairs[at..]
        .iter()
        .take_while(|(c, _)| c != "0")
        .cloned()
        .collect()
}

/// The record names its extension dictionary in a `{ACAD_XDICTIONARY` group, so a host that
/// reads only the DXF knows whose it is.
#[test]
fn the_dxf_records_name_their_extension_dictionaries() {
    for kind in [Kind::MLeaderStyle, Kind::TableStyle, Kind::Scale] {
        let (doc, handle) = document(kind, false);
        let dictionary = doc.extension_dictionary_handle(handle).expect("dictionary");
        let bytes = DxfWriter::new(&doc).write_to_vec().expect("write dxf");
        let groups = record(&String::from_utf8(bytes).expect("utf8"), handle);
        let at = groups
            .iter()
            .position(|(c, v)| c == "102" && v == "{ACAD_XDICTIONARY")
            .unwrap_or_else(|| panic!("{kind:?}: no xdictionary group in {groups:?}"));
        assert_eq!(
            groups[at + 1],
            ("360".to_string(), format!("{:X}", dictionary.value())),
            "{kind:?}"
        );
    }
}
