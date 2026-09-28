use super::*;

use cmrt_core::EFFECT_STAGE_BYPASS_JSON_KEY;
use serde_json::json;

use crate::auto_reverb::tests::test_catalog;

fn hall() -> Value {
    json!({"Dragonfly Hall Reverb preset": "Medium Clear Hall"})
}

fn room() -> Value {
    json!({"Dragonfly Room Reverb preset": "Small Drum Room"})
}

fn distortion() -> Value {
    json!({"Distortion preset": "Crunch"})
}

fn delay() -> Value {
    json!({"Surge XT Effects preset": "Delay/Echo"})
}

fn head(chain: Vec<Value>, recorded: Option<Value>) -> Value {
    let mut json = json!({ "Surge XT patch": "Pads/Pad 1.fxp", EFFECT_CHAIN_JSON_KEY: chain });
    if let Some(recorded) = recorded {
        json[AUTO_REVERB_JSON_KEY] = recorded;
    }
    json
}

fn from(json: &Value) -> HostChain {
    HostChain::from_json(Some(json), Some(&test_catalog()))
}

#[test]
fn only_the_recorded_stage_is_replaced_and_the_others_keep_their_places() {
    let chain = from(&head(vec![distortion(), hall(), delay()], Some(hall())));

    assert!(!chain.is_manual_reverb());
    assert_eq!(
        chain.with_auto_reverb(Some(&room())),
        vec![distortion(), room(), delay()]
    );
    assert_eq!(chain.with_auto_reverb(None), vec![distortion(), delay()]);
}

#[test]
fn a_chain_without_a_reverb_gets_the_auto_reverb_at_the_end() {
    let chain = from(&head(vec![distortion(), delay()], None));
    assert!(!chain.is_manual_reverb());
    assert_eq!(
        chain.with_auto_reverb(Some(&hall())),
        vec![distortion(), delay(), hall()]
    );
    assert_eq!(
        HostChain::from_json(None, None).with_auto_reverb(Some(&hall())),
        vec![hall()]
    );
}

#[test]
fn a_reverb_that_differs_from_the_record_or_has_no_record_is_manual() {
    let bypassed_hall = json!({
        "Dragonfly Hall Reverb preset": "Medium Clear Hall",
        EFFECT_STAGE_BYPASS_JSON_KEY: true,
    });
    for json in [
        // 控えと違う reverb に差し替えた。
        head(vec![distortion(), room()], Some(hall())),
        // 控えの段を bypass した。
        head(vec![bypassed_hall], Some(hall())),
        // 控えの段を消した。
        head(vec![distortion(), delay()], Some(hall())),
        // 控えが無いのに reverb がある。
        head(vec![delay(), hall()], None),
    ] {
        let chain = from(&json);
        assert!(chain.newly_detected_manual_reverb(), "{json}");
        assert_eq!(
            chain.with_auto_reverb(Some(&hall())),
            json[EFFECT_CHAIN_JSON_KEY].as_array().unwrap().clone(),
            "{json}"
        );
    }
}

#[test]
fn the_manual_mark_keeps_the_chain_manual_without_detecting_it_again() {
    let mut json = head(vec![delay()], None);
    json[MANUAL_REVERB_JSON_KEY] = json!(true);

    let chain = from(&json);
    assert!(chain.is_manual_reverb());
    assert!(!chain.newly_detected_manual_reverb());
    assert_eq!(chain.with_auto_reverb(Some(&hall())), vec![delay()]);
}

#[test]
fn write_into_replaces_the_record_with_the_manual_mark() {
    let json = head(vec![distortion(), room()], Some(hall()));
    let mut object = json.as_object().cloned().unwrap();

    from(&json).write_into(&mut object, None);

    assert!(object.get(AUTO_REVERB_JSON_KEY).is_none());
    assert_eq!(object[MANUAL_REVERB_JSON_KEY], json!(true));
    assert_eq!(object[EFFECT_CHAIN_JSON_KEY], json!([distortion(), room()]));
}

#[test]
fn write_into_sets_the_chain_and_the_record_and_removes_them_when_dry() {
    let json = head(vec![hall()], Some(hall()));
    let chain = from(&json);
    let mut object = json.as_object().cloned().unwrap();

    chain.write_into(&mut object, Some(&room()));
    assert_eq!(object[EFFECT_CHAIN_JSON_KEY], json!([room()]));
    assert_eq!(object[AUTO_REVERB_JSON_KEY], room());

    chain.write_into(&mut object, None);
    assert!(object.get(EFFECT_CHAIN_JSON_KEY).is_none());
    assert!(object.get(AUTO_REVERB_JSON_KEY).is_none());
    assert!(object.get(MANUAL_REVERB_JSON_KEY).is_none());
    assert_eq!(object["Surge XT patch"], "Pads/Pad 1.fxp");
}

#[test]
fn reverb_labels_name_the_manual_reverbs_and_mark_the_bypassed_ones() {
    let json = head(
        vec![
            distortion(),
            room(),
            json!({
                "Dragonfly Hall Reverb preset": "Medium Clear Hall",
                EFFECT_STAGE_BYPASS_JSON_KEY: true,
            }),
        ],
        None,
    );
    assert_eq!(
        from(&json).reverb_labels(Some(&test_catalog())),
        vec![
            "Dragonfly Room Reverb: Small Drum Room".to_string(),
            "Dragonfly Hall Reverb: Medium Clear Hall (bypass)".to_string(),
        ]
    );
    assert!(from(&head(vec![delay()], None))
        .reverb_labels(Some(&test_catalog()))
        .is_empty());
}
