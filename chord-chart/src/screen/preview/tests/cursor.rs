//! カーソルが動いたときに preview 要求が立つ / 立たない境目（3.2）。

use super::*;

#[test]
fn moving_the_cursor_down_asks_to_play_the_new_row() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::Char('j')));

    assert_eq!(
        taken(&mut screen),
        PreviewRequest {
            name: "B".to_string(),
            degrees: "IIm-V-I-VIm".to_string(),
            chord_index: None,
        }
    );
}

/// `↓` は `j` と同じキー。片方だけ配線を忘れても気づけるように、両方押す。
#[test]
fn the_arrow_keys_ask_for_the_same_preview_as_hjkl() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::Down));
    assert_eq!(taken(&mut screen).name, "B");

    screen.handle_key_event(key(KeyCode::Up));
    assert_eq!(taken(&mut screen).name, "A");
}

/// 端で止まって行が変わらなかったら鳴らさない。連打で同じ音が鳴り直さないため。
#[test]
fn a_cursor_that_did_not_move_asks_for_nothing() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::Char('k')));

    assert_eq!(screen.take_preview(), None);
}

#[test]
fn paging_asks_for_the_row_it_landed_on() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(taken(&mut screen).name, "Sabi");

    screen.handle_key_event(key(KeyCode::PageUp));
    assert_eq!(taken(&mut screen).name, "A");
}

/// 端に貼り付いた `PgDn` も「行が変わらなかった」ので鳴らさない。
#[test]
fn paging_against_the_end_asks_for_nothing() {
    let mut screen = entered();
    screen.handle_key_event(key(KeyCode::PageDown));
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::PageDown));

    assert_eq!(screen.take_preview(), None);
}

/// `Tab` で右 pane へ移ったら、移った先の行が**参照している** section を鳴らす。
/// 右 pane の行番号は左 pane の行番号ではない（1 行目の参照先は `A`）。
#[test]
fn moving_to_the_arrangement_pane_asks_for_the_referenced_section() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::Tab));

    assert_eq!(screen.focus, Pane::Arrangement);
    assert_eq!(
        taken(&mut screen),
        PreviewRequest {
            name: "A".to_string(),
            degrees: "I-V-VIm-IV".to_string(),
            chord_index: None,
        }
    );
}

/// 右 pane の 2 行目は左 pane の 2 行目（`B`）ではなく、参照先の `Sabi`。
#[test]
fn the_arrangement_pane_follows_the_reference_not_the_row_number() {
    let mut screen = entered();
    screen.handle_key_event(key(KeyCode::Tab));
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Char('j')));

    assert_eq!(taken(&mut screen).name, "Sabi");
}

/// 右から左へ戻ると、左のカーソル行を鳴らし直す（右で聴いていた音のままにしない）。
#[test]
fn moving_back_to_the_sections_pane_asks_for_the_row_it_left() {
    let mut screen = entered();
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Tab));
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Tab));

    assert_eq!(screen.focus, Pane::Sections);
    assert_eq!(taken(&mut screen).name, "B");
}

/// 画面を開いた直後に 1 回。行があるので無音ではない。
#[test]
fn entering_the_screen_asks_for_the_row_under_the_cursor() {
    let mut screen = screen();

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert_eq!(taken(&mut screen).name, "A");
}

/// 初回抽選が走ったときは、**抽選で作られた section** が対象になる
/// （抽選より先に要求を立てると、空の曲を鳴らす要求になってしまう）。
#[test]
fn entering_after_the_initial_pick_asks_for_the_section_it_just_made() {
    let mut screen = ChordChartScreen::restored(None);
    screen.set_chord_progression_source(std::sync::Arc::new(|| vec!["I-IV-V-I".to_string()]));

    assert_eq!(screen.enter(), ChordChartAction::SongChanged);

    assert_eq!(
        taken(&mut screen),
        PreviewRequest {
            name: "A".to_string(),
            degrees: "I-IV-V-I".to_string(),
            chord_index: None,
        }
    );
}

/// 行が 1 つも無い曲では「止めろ」を立てる。`None` にすると、前の section が
/// 鳴りっぱなしのまま空の画面を見ることになる。
#[test]
fn a_song_without_rows_asks_for_silence_rather_than_nothing() {
    let mut screen = ChordChartScreen::new(Song::empty());

    screen.enter();

    let request = taken(&mut screen);
    assert!(request.is_silent());
    assert_eq!(request, PreviewRequest::silent());
}

/// degrees が空 / 空白だけの section も無音（名前は残す。ログで区別できるように）。
#[test]
fn a_section_with_no_degrees_asks_for_silence() {
    let mut song = Song::empty();
    song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "   ");
    let mut screen = ChordChartScreen::new(song);
    screen.enter();
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Char('j')));

    let request = taken(&mut screen);
    assert!(request.is_silent());
    assert_eq!(request.name, "B");
}

/// 参照が壊れている arrangement 行（section を消したのに残った id）でも無音。
#[test]
fn a_dangling_arrangement_row_asks_for_silence() {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    let ghost = song.issue_section_id();
    song.arrangement = vec![a, ghost];
    let mut screen = ChordChartScreen::new(song);
    screen.handle_key_event(key(KeyCode::Tab));
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Char('j')));

    assert!(taken(&mut screen).is_silent());
}

/// 編集キーは preview を起こさない。中身が変わっただけでは鳴らさない
/// （聴きたければ手動キーを押す＝Stage 3）。
#[test]
fn the_editing_keys_do_not_ask_for_a_preview() {
    let editing_keys = [
        KeyCode::Char('g'),
        KeyCode::Char('r'),
        KeyCode::Char('i'),
        KeyCode::Char('n'),
        KeyCode::Char('b'),
        KeyCode::Char('1'),
        KeyCode::Char('9'),
    ];
    for code in editing_keys {
        let mut screen = entered();
        screen.set_chord_progression_source(std::sync::Arc::new(|| vec!["I-IV-V-I".to_string()]));

        screen.handle_key_event(key(code));

        assert_eq!(
            screen.take_preview(),
            None,
            "{code:?} は preview を起こさないこと"
        );
    }
}

/// `dd`（削除）も同じ。カーソル行の中身が別の section に変わっても鳴らさない。
#[test]
fn deleting_a_row_does_not_ask_for_a_preview() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::Char('d')));
    screen.handle_key_event(key(KeyCode::Char('d')));

    assert_eq!(screen.song.sections.len(), 2);
    assert_eq!(screen.take_preview(), None);
}

/// `1`..`9` の挿入で右 pane のカーソルが動いても鳴らさない（編集キーだから）。
#[test]
fn inserting_into_the_arrangement_does_not_ask_for_a_preview() {
    let mut screen = entered();
    screen.handle_key_event(key(KeyCode::Tab));
    screen.take_preview();

    screen.handle_key_event(key(KeyCode::Char('2')));

    assert_eq!(screen.song.arrangement.len(), 3);
    assert_eq!(screen.take_preview(), None);
}

/// 取り出したら消える。1 回のキーで 2 回鳴らないため。
#[test]
fn taking_the_request_leaves_nothing_behind() {
    let mut screen = entered();
    screen.handle_key_event(key(KeyCode::Char('j')));

    assert!(screen.take_preview().is_some());
    assert_eq!(screen.take_preview(), None);
}
