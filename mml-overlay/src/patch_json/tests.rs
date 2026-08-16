use super::*;

const PAD: &str = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#;

#[test]
fn strips_nothing_when_there_is_no_json() {
    assert_eq!(strip_patch_json("cde"), "cde");
}

#[test]
fn strips_the_leading_json_and_leaves_the_mml() {
    assert_eq!(strip_patch_json(&format!("{PAD} cde")), "cde");
}

#[test]
fn keeps_an_unfinished_json_as_mml() {
    let text = r#"{"Surge XT patch": "Pads"#;
    assert_eq!(strip_patch_json(text), text);
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
