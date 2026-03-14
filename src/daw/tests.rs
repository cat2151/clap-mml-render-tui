use super::parse_tempo_bpm;

#[test]
fn parse_tempo_bpm_basic() {
    assert_eq!(parse_tempo_bpm("t120cde"), Some(120.0));
}

#[test]
fn parse_tempo_bpm_at_start() {
    assert_eq!(parse_tempo_bpm("t80"), Some(80.0));
}

#[test]
fn parse_tempo_bpm_no_tempo() {
    assert_eq!(parse_tempo_bpm("cde"), None);
}

#[test]
fn parse_tempo_bpm_empty() {
    assert_eq!(parse_tempo_bpm(""), None);
}

#[test]
fn parse_tempo_bpm_after_json() {
    // JSON除去後の残りMMLにt120が含まれる場合
    assert_eq!(parse_tempo_bpm("t200efg"), Some(200.0));
}
