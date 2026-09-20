use super::*;
use serde_json::json;

#[test]
fn reads_an_empty_chain_when_the_key_is_missing_or_not_an_array() {
    assert!(init_cell_effect_chain("").is_empty());
    assert!(init_cell_effect_chain(r#"{"Surge XT patch":"Pad 1.fxp"}"#).is_empty());
    assert!(init_cell_effect_chain(r#"{"effects after instrument":"x"}"#).is_empty());
}

#[test]
fn reads_the_chain_in_order_without_interpreting_it() {
    let init = r#"{"Surge XT patch":"Pad 1.fxp","effects after instrument":[{"A preset":"1"},{"B preset":"2"}]}"#;

    assert_eq!(
        init_cell_effect_chain(init),
        vec![json!({"A preset": "1"}), json!({"B preset": "2"})]
    );
}

#[test]
fn writing_a_chain_keeps_the_other_keys_and_the_body() {
    let init = r#"{"Surge XT patch":"Pad 1.fxp"}v10"#;

    let written = init_cell_with_effect_chain(init, &[json!({"A preset": "1"})]);

    assert_eq!(
        written,
        r#"{"Surge XT patch":"Pad 1.fxp","effects after instrument":[{"A preset":"1"}]}v10"#
    );
    assert_eq!(
        init_cell_effect_chain(&written),
        vec![json!({"A preset": "1"})]
    );
}

#[test]
fn writing_an_empty_chain_removes_the_key() {
    let init = r#"{"Surge XT patch":"Pad 1.fxp","effects after instrument":[{"A preset":"1"}]}"#;

    assert_eq!(
        init_cell_with_effect_chain(init, &[]),
        r#"{"Surge XT patch":"Pad 1.fxp"}"#
    );
}

#[test]
fn writing_an_empty_chain_to_a_chain_only_cell_leaves_the_body() {
    assert_eq!(
        init_cell_with_effect_chain(r#"{"effects after instrument":[{"A preset":"1"}]}v10"#, &[]),
        "v10"
    );
    assert_eq!(
        init_cell_with_effect_chain(r#"{"effects after instrument":[{"A preset":"1"}]}"#, &[]),
        ""
    );
}

#[test]
fn stage_is_bypassed_reads_the_bypass_key() {
    assert!(!stage_is_bypassed(&json!({"A preset": "1"})));
    assert!(!stage_is_bypassed(
        &json!({"A preset": "1", "bypass": false})
    ));
    assert!(stage_is_bypassed(&json!({"A preset": "1", "bypass": true})));
}

#[test]
fn stage_with_bypass_true_adds_the_key_without_touching_the_plugin_key() {
    let stage = json!({"A preset": "1"});

    assert_eq!(
        stage_with_bypass(&stage, true),
        json!({"A preset": "1", "bypass": true})
    );
}

#[test]
fn stage_with_bypass_false_removes_the_key() {
    let stage = json!({"A preset": "1", "bypass": true});

    assert_eq!(stage_with_bypass(&stage, false), json!({"A preset": "1"}));
}
