//! 手動キー `Shift+P` / `Space` のトグル（3.3）。
//!
//! 鳴っているかどうかを知っているのは app 側だけなので、「鳴っている」状態は
//! glue と同じ [`ChordChartScreen::set_preview_sounding`] で作る。

use super::*;

/// `Shift+P` は SHIFT 付きで届く。**テストも実際の来方で押す。**
fn shift_p() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT)
}

fn space() -> KeyEvent {
    key(KeyCode::Char(' '))
}

/// 止まっているときに押すと、カーソル行を鳴らす要求が出る。
#[test]
fn the_toggle_asks_to_play_the_row_under_the_cursor_while_silent() {
    for press in [shift_p(), space()] {
        let mut screen = entered();
        assert!(!screen.preview_sounding());

        screen.handle_key_event(press);

        assert_eq!(
            taken(&mut screen),
            PreviewRequest {
                name: "A".to_string(),
                degrees: "I-V-VIm-IV".to_string(),
            },
            "{press:?} で鳴る要求が出ること"
        );
    }
}

/// 鳴っているときに押すと、止める要求が出る。
///
/// 鳴っているかどうかを知っているのは app 側だけなので、テストも glue と同じ
/// [`ChordChartScreen::set_preview_sounding`] で状態を作る。
#[test]
fn the_toggle_asks_for_silence_while_the_preview_is_sounding() {
    for press in [shift_p(), space()] {
        let mut screen = entered();
        screen.set_preview_sounding(true);

        screen.handle_key_event(press);

        assert_eq!(
            taken(&mut screen),
            PreviewRequest::silent(),
            "{press:?} で止める要求が出ること"
        );
    }
}

/// カーソルは動かさない。トグルは「いまいる行」を鳴らし直すキー。
#[test]
fn the_toggle_does_not_move_the_cursor() {
    let mut screen = entered();
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.take_preview();

    screen.handle_key_event(space());

    assert_eq!(screen.section_cursor, 1);
    assert_eq!(taken(&mut screen).name, "B");
}

/// カーソル移動と違い、**同じ行で連打しても毎回要求が出る**（鳴らし直しキーなので、
/// 「行が変わらなかったから鳴らさない」を持ち込まない）。
#[test]
fn pressing_the_toggle_twice_while_silent_asks_twice() {
    let mut screen = entered();

    screen.handle_key_event(space());
    assert_eq!(taken(&mut screen).name, "A");

    screen.handle_key_event(space());
    assert_eq!(taken(&mut screen).name, "A");
}

/// 右 pane でも同じキーが効く（両 pane 共通キー）。対象は**参照先** section。
#[test]
fn the_toggle_works_in_the_arrangement_pane_too() {
    let mut screen = entered();
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.take_preview();

    screen.handle_key_event(shift_p());

    assert_eq!(screen.focus, Pane::Arrangement);
    assert_eq!(taken(&mut screen).name, "Sabi");
}

/// 1 行入力欄が開いている間、`Space` は**文字**として入る。
#[test]
fn a_space_typed_into_the_line_input_stays_a_character() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::Char('i')));
    for _ in 0..64 {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    for code in ['I', ' ', 'V'] {
        screen.handle_key_event(key(KeyCode::Char(code)));
    }
    assert_eq!(
        screen.take_preview(),
        None,
        "入力中のキーは preview を起こさないこと"
    );

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Enter)),
        ChordChartAction::SongChanged
    );
    assert_eq!(screen.song.sections[0].degrees, "I V");
}

/// `Shift+P` も入力欄では文字。
#[test]
fn a_shift_p_typed_into_the_line_input_stays_a_character() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::Char('n')));
    for _ in 0..64 {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    screen.handle_key_event(shift_p());
    screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(screen.song.sections[0].name, "P");
    assert_eq!(screen.take_preview(), None);
}

/// ヘルプ overlay を開いている間は握り潰す（既存の作法）。
#[test]
fn the_toggle_does_nothing_while_the_help_overlay_is_open() {
    for press in [shift_p(), space()] {
        let mut screen = entered();
        screen.handle_key_event(key(KeyCode::Char('?')));
        assert!(screen.help_open);

        screen.handle_key_event(press);

        assert!(screen.help_open, "{press:?} でヘルプが閉じないこと");
        assert_eq!(screen.take_preview(), None);
    }
}

/// 小文字の `p` は割り当てていない（3.3 は `Shift+P` と `Space` の 2 つだけ）。
#[test]
fn the_lowercase_p_is_not_a_preview_key() {
    let mut screen = entered();

    screen.handle_key_event(key(KeyCode::Char('p')));

    assert_eq!(screen.take_preview(), None);
}
