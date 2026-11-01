//! A DXF before R2013 holds ACIS data as ciphered SAT text in groups 1 and 3. Data read from a DWG of R2013 or later is SAB
//! only, so the groups convert it token for token; CADCraft writes them for its pre-2013 DXF and DWG saves.

use acadrust::entities::acis::primitives::build_planar_body;
use acadrust::entities::acis::SabWriter;
use acadrust::entities::AcisData;

#[test]
fn sab_only_data_gives_ciphered_sat_lines_that_end_the_acis_data() {
    let vertices = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    ];
    let faces = vec![
        vec![0, 3, 2, 1],
        vec![4, 5, 6, 7],
        vec![0, 1, 5, 4],
        vec![1, 2, 6, 5],
        vec![2, 3, 7, 6],
        vec![3, 0, 4, 7],
    ];
    let sab = SabWriter::write(&build_planar_body(&vertices, &faces).expect("cube builds"));
    let groups = AcisData::from_sab(sab).dxf_sat_groups(false);
    assert!(groups.len() > 3, "{groups:?}");
    assert!(groups
        .iter()
        .all(|(code, value)| (*code == 1 || *code == 3) && value.len() <= 2049));
    let text: Vec<String> = groups
        .iter()
        .filter(|(code, _)| *code == 1)
        .map(|(_, value)| AcisData::decode_sat(value))
        .collect();
    assert!(
        text.iter().all(|line| line.len() <= 255),
        "AutoCAD reads SAT lines of up to 255 characters"
    );
    assert!(text.iter().any(|line| line.contains("body ")), "{text:?}");
    assert_eq!(text.last().map(String::as_str), Some("End-of-ACIS-data"));
}

#[test]
fn no_data_gives_one_empty_line() {
    assert_eq!(AcisData::new().dxf_sat_groups(false), [(1, String::new())]);
}
