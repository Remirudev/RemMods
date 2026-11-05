//! Text tables and the item list.
#![allow(clippy::expect_used)]

use remmods_unpack::fixture::{item_list, text_table};
use remmods_unpack::{UnpackError, parse_item_list, parse_text_table};

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(key, text)| ((*key).to_owned(), (*text).to_owned()))
        .collect()
}

#[test]
fn text_records_are_found_between_other_bytes() {
    let records = [
        ("TEXT_UI_000_cmn_bdu_f_blouse", "Rüschenbluse"),
        ("TEXT_UI_001_rat_har_long", "Langes Haar"),
    ];
    assert_eq!(parse_text_table(&text_table(&records)), pairs(&records));
}

#[test]
fn a_table_without_records_is_empty() {
    assert!(parse_text_table(b"nothing to see").is_empty());
    assert!(parse_text_table(&[]).is_empty());
}

#[test]
fn a_tag_with_wild_lengths_is_skipped_and_the_rest_still_reads() {
    let mut bytes = b"text\x10\0\0\0".to_vec();
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&text_table(&[("KEY", "Value")]));
    assert_eq!(parse_text_table(&bytes), pairs(&[("KEY", "Value")]));
}

#[test]
fn a_cut_off_record_is_dropped() {
    let bytes = text_table(&[("KEY", "Value"), ("SECOND", "Text")]);
    let cut = bytes.len() - b"package end".len() - 3;
    assert_eq!(parse_text_table(&bytes[..cut]), pairs(&[("KEY", "Value")]));
}

#[test]
fn the_item_list_pairs_prefabs_with_text_keys() {
    let entries = [
        ("IP_rat_bdu_f_blouse", "TEXT_UI_000_rat_bdu_f_blouse"),
        ("IP_cmn_sho_f_barefoot", "TEXT_UI_001_cmn_sho_f_barefoot"),
    ];
    assert_eq!(
        parse_item_list(&item_list(&entries)).expect("list"),
        pairs(&entries)
    );
}

#[test]
fn an_empty_item_list_is_fine() {
    assert!(parse_item_list(&item_list(&[])).expect("list").is_empty());
}

#[test]
fn a_file_that_is_no_item_list_is_refused() {
    assert!(matches!(
        parse_item_list(&[]),
        Err(UnpackError::ItemList(_))
    ));
    assert!(matches!(
        parse_item_list(&[0xFF; 64]),
        Err(UnpackError::ItemList(_))
    ));
}

#[test]
fn garbled_tables_never_panic() {
    let table = text_table(&[("KEY", "Value"), ("SECOND", "Text")]);
    let list = item_list(&[("IP_rat_bdu_f_blouse", "KEY"), ("IP_b", "SECOND")]);
    let mut seed = 0xDEAD_BEEF_u32;
    for _ in 0..5000 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let value = u8::try_from(seed >> 24).unwrap_or(0);
        let mut bytes = table.clone();
        let at = usize::try_from(seed).unwrap_or(0) % bytes.len();
        bytes[at] = value;
        let _ = parse_text_table(&bytes);
        let mut bytes = list.clone();
        let at = usize::try_from(seed).unwrap_or(0) % bytes.len();
        bytes[at] = value;
        let _ = parse_item_list(&bytes);
    }
}
