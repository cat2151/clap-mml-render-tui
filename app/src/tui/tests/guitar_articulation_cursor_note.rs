//! Guitar Articulation 画面で、カーソル列の 1 音だけを鳴らす操作（`h` / `l` の移動・1 音モード）が
//! どの messages・どの音色で `play_line` へ渡るか。

use super::guitar_articulation::{
    app_with_mml, app_with_mml_and_effects, attach_recording_sender, first_candidate_chain,
    flat_events, messages, plain, sent_messages, wait_until,
};
use super::*;
use crate::tui::guitar_articulation::{convert, RuleTable, Take, PATCH};
use cmrt_mml_overlay::{LivePatch, RecordingSink};

#[test]
fn h_and_l_play_the_cursor_column_note_without_the_note_mode() {
    let (mut app, sink) = app_with_mml();
    assert!(!app.guitar_articulation.note_preview());

    // 続けて送ると sender が後勝ちでまとめるので、1 キーごとに積まれるのを待つ。
    for (count, code) in ['l', 'l', 'h'].into_iter().enumerate() {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char(code)));
        wait_until("1 音", || sink.timelines() > count);
    }

    assert_eq!(app.guitar_articulation.cursor(), 1);
    let note = app.guitar_articulation.column_events(Take::Converted);
    let sent = sink.timeline_events().len();
    assert_eq!(sent_messages(&sink, sent - note.len()), messages(&note));
    assert_eq!(sink.timelines(), 3);
}

/// 1 音モードにして `l` で 2 列目へ移り、`l` が鳴らした 1 音が積まれるのを待つ。積まれたイベント数を返す。
fn move_to_the_second_column(app: &mut TuiApp<'_>, sink: &RecordingSink) -> usize {
    for code in ['n', 'l'] {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char(code)));
    }
    wait_until("l の 1 音", || sink.timelines() >= 1);
    sink.timeline_events().len()
}

fn is_keyswitch_note_on(message: &[u8; 3]) -> bool {
    message[0] & 0xF0 == 0x90
        && message[2] != 0
        && crate::tui::guitar_articulation::Articulation::from_keyswitch(message[1]).is_some()
}

#[test]
fn in_note_mode_toggling_a_rule_plays_only_the_cursor_column() {
    let (mut app, sink) = app_with_mml();

    let moved = move_to_the_second_column(&mut app, &sink);
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('a')));
    let note = app.guitar_articulation.column_events(Take::Converted);
    wait_until("1 音が積まれる", || {
        sink.timeline_events().len() >= moved + note.len()
    });

    assert_eq!(sent_messages(&sink, moved), messages(&note));
    assert!(sent_messages(&sink, moved).contains(&[0x90, 26, 127]));
    let whole = convert(&flat_events(), app.guitar_articulation.rules());
    assert_ne!(sent_messages(&sink, moved), messages(&whole));
    assert_eq!(sink.timelines(), 2);
}

#[test]
fn in_note_mode_b_plays_the_cursor_note_without_keyswitches() {
    let (mut app, sink) = app_with_mml();

    let moved = move_to_the_second_column(&mut app, &sink);
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('b')));
    let note = app.guitar_articulation.column_events(Take::Plain);
    wait_until("1 音が積まれる", || {
        sink.timeline_events().len() >= moved + note.len()
    });

    let sent = sent_messages(&sink, moved);
    assert_eq!(sent, messages(&note));
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert!(!sent.iter().any(is_keyswitch_note_on), "{sent:?}");
}

/// 1 音の演奏も全体と同じ音色・chain で送るので、sender は音色を読み直さない。
#[test]
fn a_note_is_sent_to_the_same_patch_and_chain_as_the_whole_take() {
    let (mut app, _preview_sink) = app_with_mml_and_effects();
    for code in [
        KeyCode::Char('x'),
        KeyCode::Char('a'),
        KeyCode::Enter,
        KeyCode::Enter,
    ] {
        app.handle_guitar_articulation_key_event(plain(code));
    }
    let sink = attach_recording_sender(&mut app);
    let whole = convert(&flat_events(), &RuleTable::default());

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));
    wait_until("全体", || sink.timeline_events().len() >= whole.len());
    // l は 1 音モードに依らず移動先の 1 音を鳴らす。
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('l')));
    let note = app.guitar_articulation.column_events(Take::Converted);
    wait_until("1 音", || {
        sink.timeline_events().len() >= whole.len() + note.len()
    });

    assert_eq!(sent_messages(&sink, whole.len()), messages(&note));
    assert_eq!(sink.timelines(), 2);
    assert_eq!(
        sink.prepared(),
        vec![LivePatch::with_effect_chain(
            Some(PATCH),
            &first_candidate_chain()
        )]
    );
}
