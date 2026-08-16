use super::*;

const PAD: &str = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#;

#[test]
fn strips_nothing_when_there_is_no_json() {
    let stripped = strip_patch_json("cde");
    assert_eq!(stripped.mml, "cde");
    assert_eq!(stripped.offset_chars, 0);
}

#[test]
fn strips_the_leading_json_and_reports_where_the_mml_starts() {
    let text = format!("{PAD} cde");
    let stripped = strip_patch_json(&text);
    assert_eq!(stripped.mml, "cde");
    assert_eq!(stripped.offset_chars, PAD.chars().count() + 1);
}

#[test]
fn keeps_an_unfinished_json_as_mml() {
    // カーソルが JSON の途中にあるときの prefix はこの形になる。
    let stripped = strip_patch_json(r#"{"Surge XT patch": "Pads"#);
    assert_eq!(stripped.mml, r#"{"Surge XT patch": "Pads"#);
    assert_eq!(stripped.offset_chars, 0);
}

#[test]
fn reads_the_patch_name_back() {
    assert_eq!(
        patch_name(&format!("{PAD} cde")).as_deref(),
        Some("Pads/Pad 1.fxp")
    );
    assert_eq!(patch_name("cde"), None);
    assert_eq!(patch_name(r#"{"other": 1} cde"#), None);
}

#[test]
fn inserts_the_json_in_front_of_a_bare_mml() {
    let (text, delta) = set_patch_name("cde", "Pads/Pad 1.fxp");
    assert_eq!(text, format!("{PAD} cde"));
    assert_eq!(delta, PAD.chars().count() as isize + 1);
}

#[test]
fn overwrites_an_existing_json() {
    let (text, delta) = set_patch_name(&format!("{PAD} cde"), "Leads/Lead 1.fxp");
    assert_eq!(text, r#"{"Surge XT patch": "Leads/Lead 1.fxp"} cde"#);
    assert_eq!(delta, 2);
}

#[test]
fn escapes_a_patch_name_that_contains_a_quote() {
    let (text, _) = set_patch_name("c", r#"Pads/a"b.fxp"#);
    assert_eq!(patch_name(&text).as_deref(), Some(r#"Pads/a"b.fxp"#));
}
