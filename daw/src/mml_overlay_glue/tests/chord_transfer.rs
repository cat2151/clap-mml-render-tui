//! MML overlay で打った文字列がコード表記だったときの、chord 行への移送。
//!
//! 「打ちかけの文字列を捨てない」のがこの機能の要なので、
//! **移送先に何が入ったか**と**元のセルが触られていないか**を毎回両方見る。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::super::{DawApp, DawMode, CHORD_TRACK, FIRST_PLAYABLE_TRACK};
use super::{ctrl, key, log_lines, plain};
use crate::input::tests::build_test_app;

/// 演奏 track の meas1 を開いた状態にする。
fn opened_on_a_playable_cell() -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK;
    app.editor.cursor_measure = 1;
    assert!(app.try_open_mml_overlay(ctrl('p')));
    (app, cache_rx)
}

/// `I` `-` `I` `V` と打つ。overlay の 1 行モードはカーソルが行末にある。
fn type_a_chord(app: &mut DawApp) {
    for code in "I-IV".chars() {
        app.handle_mml_overlay_key_event(plain(code));
    }
}

#[test]
fn a_chord_notation_is_moved_to_the_chord_row_instead_of_the_cell() {
    let (mut app, _cache_rx) = opened_on_a_playable_cell();
    type_a_chord(&mut app);

    // 1 回目の Enter で確認ダイアログ、2 回目で既定の「移送」。
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    assert_eq!(app.mode, DawMode::MmlOverlay, "まだ閉じない");
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(app.editor.data[CHORD_TRACK][1], "I-IV");
    assert_eq!(
        app.editor.data[FIRST_PLAYABLE_TRACK][1], "",
        "編集していたセルへは書かない（生成に任せる）"
    );
    assert_eq!(app.editor.cursor_track, CHORD_TRACK, "chord 行へ移る");
    assert_eq!(app.editor.cursor_measure, 1, "小節は動かない");
    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
}

/// 移送のあとの `C` は、移送元の track へ戻る（`C` と同じ跳び方を通しているため）。
#[test]
fn pressing_c_after_a_transfer_returns_to_the_track_it_came_from() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK + 1;
    app.editor.cursor_measure = 1;
    assert!(app.try_open_mml_overlay(ctrl('p')));
    type_a_chord(&mut app);
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    assert_eq!(app.editor.cursor_track, CHORD_TRACK);

    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));

    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK + 1);
}

/// 「このまま MML として確定」を選べば、ダイアログが無かった場合と同じ確定。
#[test]
fn choosing_to_keep_it_as_mml_writes_the_cell_exactly_like_before() {
    let (mut app, _cache_rx) = opened_on_a_playable_cell();
    type_a_chord(&mut app);

    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    app.handle_mml_overlay_key_event(key(KeyCode::Down));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(app.editor.data[FIRST_PLAYABLE_TRACK][1], "I-IV");
    assert_eq!(app.editor.data[CHORD_TRACK][1], "");
    assert_eq!(app.editor.cursor_track, FIRST_PLAYABLE_TRACK);
    // `Enter` の確定なので次の meas を開いたまま（従来どおり）。
    assert_eq!(app.editor.cursor_measure, 2);
    assert!(app.mml_overlay.is_open());
}

/// ダイアログの `Esc` は確定そのものの取り消し。どちらのセルも書かれない。
#[test]
fn cancelling_the_dialog_writes_nothing_at_all() {
    let (mut app, _cache_rx) = opened_on_a_playable_cell();
    type_a_chord(&mut app);

    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));

    assert_eq!(app.editor.data[FIRST_PLAYABLE_TRACK][1], "");
    assert_eq!(app.editor.data[CHORD_TRACK][1], "");
    assert!(app.mml_overlay.is_open());
    assert_eq!(app.mml_overlay.value(), "I-IV");
}

/// 普通の MML はダイアログを通らない（従来の確定のまま）。
#[test]
fn plain_mml_still_commits_in_one_enter() {
    let (mut app, _cache_rx) = opened_on_a_playable_cell();
    for code in "cde".chars() {
        app.handle_mml_overlay_key_event(plain(code));
    }

    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(app.editor.data[FIRST_PLAYABLE_TRACK][1], "cde");
    assert_eq!(app.editor.data[CHORD_TRACK][1], "");
    assert_eq!(app.editor.cursor_measure, 2);
}

/// 生成対象でない track から移しても、そのままでは音が変わらない。
/// 直し方をログへ出す（出さないと「移したのに無音」で行き止まる）。
#[test]
fn transferring_from_a_track_that_does_not_generate_says_how_to_fix_it() {
    let (mut app, _cache_rx) = opened_on_a_playable_cell();
    type_a_chord(&mut app);
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    let logs = log_lines(&app).join("\n");
    assert!(logs.contains("chord 行の meas1 へ移しました"), "{logs}");
    assert!(
        logs.contains(crate::mml::chord_generation::GENERATE_FROM_CHORD_TRACK_KEY),
        "{logs}"
    );
}

/// 既に生成対象なら、直し方の案内は出さない（移した時点で鳴るため）。
#[test]
fn transferring_from_a_generating_track_does_not_nag() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK;
    app.editor.cursor_measure = 1;
    app.editor.data[FIRST_PLAYABLE_TRACK][0] =
        r#"{"generate from chord track": "close"}"#.to_string();
    assert!(app.try_open_mml_overlay(ctrl('p')));
    type_a_chord(&mut app);
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    let logs = log_lines(&app).join("\n");
    assert!(logs.contains("chord 行の meas1 へ移しました"), "{logs}");
    assert!(
        !logs.contains("chord 行から生成されません"),
        "生成対象なのに案内が出ている:\n{logs}"
    );
}

/// 2 節のバグと、直ったことの対比。**同じ `Cm7` が、置き場所で音の数が変わる。**
///
/// 期待値はローカルの実バイナリから取った（推測で書かない）。
///
/// ```text
/// $ chord2mml.exe "close | Cm7 |"        → v11/*|*/'c1d+ga+'/*|*/
/// $ mmlabc-to-smf.exe --no-play -o x.mid "t120v11/*|*/'c1d+ga+'/*|*/"
///                                        → NoteOn 4 個: 60 63 67 70
/// $ mmlabc-to-smf.exe --no-play -o y.mid "t120Cm7"
///                                        → NoteOn 1 個: 60（`m7` は黙って捨てられる）
/// ```
#[test]
fn the_same_cm7_is_one_wrong_note_in_the_cell_but_a_chord_from_the_chord_row() {
    // (a) 「このまま MML として確定」を選んだ場合。セルに `Cm7` が残る。
    let (mut app, _cache_rx) = opened_on_a_generating_cell();
    type_cm7(&mut app);
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    app.handle_mml_overlay_key_event(key(KeyCode::Down));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    let as_mml = generated_note_ons(&app);

    // (b) 既定の「chord 行へ移す」を選んだ場合。セルは空のまま chord 行から生成される。
    let (mut app, _cache_rx) = opened_on_a_generating_cell();
    type_cm7(&mut app);
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    let from_chord_row = generated_note_ons(&app);

    assert_eq!(
        as_mml,
        vec![(0, 60)],
        "MML なら単音の C になる（2 節のバグ）"
    );
    assert_eq!(
        from_chord_row,
        vec![(0, 60), (0, 63), (0, 67), (0, 70)],
        "chord 行からなら Cm7 が和音で鳴る"
    );
}

/// 生成対象 track（`"generate from chord track": "close"`）の meas1 を開いた状態にする。
fn opened_on_a_generating_cell() -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    app.editor.cursor_track = FIRST_PLAYABLE_TRACK;
    app.editor.cursor_measure = 1;
    app.editor.data[FIRST_PLAYABLE_TRACK][0] =
        r#"{"generate from chord track": "close"}"#.to_string();
    assert!(app.try_open_mml_overlay(ctrl('p')));
    (app, cache_rx)
}

fn type_cm7(app: &mut DawApp) {
    for code in "Cm7".chars() {
        app.handle_mml_overlay_key_event(plain(code));
    }
}

/// カーソル行ではなく**移送元の track**の meas1 を、演奏と同じ経路で SMF にして数える。
fn generated_note_ons(app: &DawApp) -> Vec<(u32, u8)> {
    crate::mml::tests::chord_note_counts::note_ons(&crate::mml::build_cell_mml_from_data(
        &app.editor.data,
        app.editor.measures,
        FIRST_PLAYABLE_TRACK,
        1,
    ))
}
