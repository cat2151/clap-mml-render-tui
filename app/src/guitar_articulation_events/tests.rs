use super::*;
use cmrt_guitar_articulation::{convert, RowRule, Rule, FULL_PATCH, PATCH};

fn flat_events() -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance("o3 l8 e f+ g")
        .unwrap()
        .events
}

fn rules() -> RuleTable {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle_row(RowRule::EconomyPicking);
    rules
}

#[test]
fn the_play_log_line_names_the_take_the_keyswitches_the_patch_and_the_inputs() {
    let flat = flat_events();
    let converted = convert(&flat, &RuleTable::default());

    assert_eq!(
        play_log_line(
            FULL_PATCH,
            Take::Converted,
            &converted,
            "o3 l8 e f+ g",
            &RuleTable::default()
        ),
        format!(
            "guitar-articulation: event=play take=converted events=8 keyswitch_note_ons=1 seconds={:.3} patch={FULL_PATCH:?} mml=\"o3 l8 e f+ g\" rules={{\"columns\":{{}},\"rows\":[]}}",
            converted.last().unwrap().seconds
        )
    );
    assert!(
        play_log_line(PATCH, Take::Plain, &flat, "", &RuleTable::default())
            .contains("take=plain events=6 keyswitch_note_ons=0")
    );
}

#[test]
fn the_last_played_line_gives_back_the_mml_and_the_rules() {
    let mml = r#"o3 l16 "quoted" \ e f+ g rules= mml="#;
    let first = play_log_line(
        PATCH,
        Take::Converted,
        &flat_events(),
        "c",
        &RuleTable::default(),
    );
    let last = play_log_line(PATCH, Take::Converted, &flat_events(), mml, &rules());
    // raw の演奏はルール表を使わないので飛ばす。
    let plain = play_log_line(
        PATCH,
        Take::Plain,
        &flat_events(),
        "d",
        &RuleTable::default(),
    );
    let log = format!("[t1] {first}\n[t2] {last}\n[t3] {plain}\n[t4] other: event=x\n");

    let play = last_played(&log).unwrap();

    assert_eq!(play.line, format!("[t2] {last}"));
    assert_eq!(play.mml, mml);
    assert_eq!(play.rules, rules());
}

#[test]
fn the_previous_play_is_the_latest_one_with_the_same_mml_and_other_rules() {
    let line = |mml: &str, rules: &RuleTable| {
        play_log_line(PATCH, Take::Converted, &flat_events(), mml, rules)
    };
    let older = line("cde", &RuleTable::default());
    let other_mml = line("efg", &RuleTable::default());
    let replayed = line("cde", &rules());
    let last = line("cde", &rules());
    let log = format!("{older}\n{other_mml}\n{replayed}\n{last}\n");

    let (previous, latest) = last_two_distinct(&log).unwrap();

    assert_eq!(previous.line, older);
    assert_eq!(latest.rules, rules());
    assert!(last_two_distinct(&format!("{replayed}\n{last}\n")).is_err());
}

#[test]
fn lines_written_before_the_mml_was_logged_are_skipped() {
    let old = "[t0] guitar-articulation: event=play take=converted events=8 keyswitch_note_ons=1 seconds=0.750 patch=\"x\"";
    let new = play_log_line(
        PATCH,
        Take::Converted,
        &flat_events(),
        "cde",
        &RuleTable::default(),
    );

    assert_eq!(last_played(&format!("{new}\n{old}\n")).unwrap().mml, "cde");
    assert!(last_played(old).is_err());
}

#[test]
fn an_mml_with_rules_json_reports_the_converted_events() {
    let text = report(&GuitarArticulationEventsRequest {
        last_played: false,
        compare_previous: false,
        rules: Some(rules().to_json()),
        mml: Some("o3 l8 e f+ g".to_string()),
    })
    .unwrap();

    assert!(text.contains("KS Hammer-On"), "{text}");
    assert!(report(&GuitarArticulationEventsRequest {
        last_played: false,
        compare_previous: false,
        rules: Some("{".to_string()),
        mml: Some("c".to_string()),
    })
    .is_err());
}

#[test]
fn a_note_play_after_the_whole_take_does_not_replace_the_last_played() {
    let flat = flat_events();
    let whole = play_log_line(
        PATCH,
        Take::Converted,
        &convert(&flat, &rules()),
        "o3 l8 e f+ g",
        &rules(),
    );
    let note = play_note_log_line(PATCH, Take::Converted, 1, &flat[2..4]);
    assert!(
        note.starts_with("guitar-articulation: event=play-note column=1 take=converted events=2 "),
        "{note}"
    );
    let log = format!("[t1] {whole}\n[t2] {note}\n");

    let play = last_played(&log).unwrap();

    assert_eq!(play.line, format!("[t1] {whole}"));
    assert_eq!(play.mml, "o3 l8 e f+ g");
    assert_eq!(play.rules, rules());
}

#[test]
fn a_sample_midi_play_is_logged_but_not_taken_as_the_last_played() {
    let mml_line = play_log_line(PATCH, Take::Converted, &flat_events(), "cde", &rules());
    let events = flat_events();
    let whole = play_sample_midi_log_line("CC22_Mute_Control.mid", None, &events, PATCH);
    let note = play_sample_midi_log_line("CC22_Mute_Control.mid", Some(3), &events, PATCH);
    let log = format!(
        "{mml_line}
{whole}
{note}
"
    );

    assert_eq!(
        whole,
        format!(
            "guitar-articulation: event=play-sample-midi file=\"CC22_Mute_Control.mid\" note=all events=6 seconds={:.3} patch={PATCH:?}",
            events.last().unwrap().seconds
        )
    );
    assert!(note.contains(" note=3 events=6 "), "{note}");
    assert_eq!(last_played(&log).unwrap().line, mml_line);
}
