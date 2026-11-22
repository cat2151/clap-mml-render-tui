use super::*;

#[test]
fn dexed_state_accepts_juce_blob_and_checks_midi_mapping() {
    let state = |wheel, mapping| {
        format!(
            r#"<dexedState wheelMod="{wheel}"><dexedBlob base64:sysex="data" base64:program="data"/><midiCC>{mapping}</midiCC></dexedState>"#
        )
    };
    assert_eq!(
        wheel_without_cc1(&state("0 0 0 0", r#"<mapping cc="267"/>"#)).unwrap(),
        "0 0 0 0"
    );
    assert!(wheel_without_cc1(&state("1 1 0 0", "")).is_err());
    for mapping in [r#"<mapping cc="1"/>"#, r#"<mapping cc="257"/>"#] {
        assert!(wheel_without_cc1(&state("0 0 0 0", mapping)).is_err());
    }
}
