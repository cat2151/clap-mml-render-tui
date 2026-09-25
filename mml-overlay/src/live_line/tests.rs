use super::*;

const CHAIN_LINE: &str = r#"{"Surge XT patch": "patches_factory/Templates/Init Sine.fxp", "effects after instrument": [{"Surge XT Effects preset": "Reverb 1/Cathedral 2.srgfx"}]} o4c1"#;

#[test]
fn the_patch_and_the_chain_come_from_the_leading_json() {
    let line = live_line(CHAIN_LINE).unwrap();

    assert_eq!(
        line.patch.patch(),
        Some("patches_factory/Templates/Init Sine.fxp")
    );
    let chain: serde_json::Value = serde_json::from_str(line.patch.effect_chain()).unwrap();
    assert_eq!(
        chain,
        serde_json::json!([{"Surge XT Effects preset": "Reverb 1/Cathedral 2.srgfx"}])
    );
    assert!(!line.program.performance.is_silent());
    assert!(!line.program.repeat);
}

#[test]
fn a_line_without_a_chain_has_an_empty_chain() {
    assert_eq!(live_line("o4c1").unwrap().patch, LivePatch::new(None));
    assert_eq!(
        live_line(r#"{"Surge XT patch": "a.fxp"} o4c1"#)
            .unwrap()
            .patch,
        LivePatch::new(Some("a.fxp"))
    );
}

/// 行頭 JSON の他のキー（DAW の init セルが持つもの）は演奏に混ざらない。
#[test]
fn other_json_keys_do_not_leak_into_the_performance() {
    let with_keys =
        live_line(r#"{"Surge XT patch": "a.fxp", "generate from chord track": "close"} o4c1"#)
            .unwrap();
    let plain = live_line("o4c1").unwrap();
    assert_eq!(with_keys.program.performance, plain.program.performance);
}

#[test]
fn an_empty_line_is_rejected() {
    assert!(live_line("  ").is_err());
    assert!(live_line(r#"{"Surge XT patch": "a.fxp"}"#).is_err());
}
