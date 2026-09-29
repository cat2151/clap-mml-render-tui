use super::*;
use serde_json::json;

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

#[test]
fn an_unknown_stage_is_labelled_with_its_json_and_bypass_mark() {
    let stage = json!({"A preset": "1", "bypass": true});

    assert_eq!(
        stage_label(&stage, None),
        r#"[bypass] {"A preset":"1","bypass":true}"#
    );
}

#[test]
fn chain_json_is_empty_for_an_empty_chain_and_an_array_otherwise() {
    assert_eq!(chain_json(&[]), "");
    assert_eq!(
        chain_json(&[json!({"A preset": "1"}), json!({"B preset": "2"})]),
        r#"[{"A preset":"1"},{"B preset":"2"}]"#
    );
}
