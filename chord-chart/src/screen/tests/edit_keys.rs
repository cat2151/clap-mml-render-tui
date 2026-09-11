//! 編集まわりのキーの境目。廃止したキーが effect ゼロであること、`dd` の途中状態、
//! 直前のエラーの消え方、保存要求が出ない操作。

use super::*;

/// 廃止したキーが本当に 1 つも効かないこと。
///
/// カタログを注入してあるので、`a` が生き残っていれば section が増えて落ちる。
/// `[` `]` `<` `>` が生き残っていれば Key / BPM が動く。
#[test]
fn the_retired_keys_do_nothing_at_all() {
    let mut screen = three_section_screen();
    screen.set_chord_progression_source(std::sync::Arc::new(|| vec!["I-IV-V-I".to_string()]));
    let before = screen.song.clone();

    for (code, modifiers) in [
        (KeyCode::Char('a'), KeyModifiers::NONE),
        (KeyCode::Char('e'), KeyModifiers::NONE),
        (KeyCode::Char('x'), KeyModifiers::NONE),
        (KeyCode::Char('['), KeyModifiers::NONE),
        (KeyCode::Char(']'), KeyModifiers::NONE),
        // `<` `>` `+` は端末から SHIFT 付きで届く（旧 `song_edit` の実測）。
        (KeyCode::Char('<'), KeyModifiers::SHIFT),
        (KeyCode::Char('>'), KeyModifiers::SHIFT),
        (KeyCode::Char('+'), KeyModifiers::SHIFT),
        (KeyCode::Char('-'), KeyModifiers::NONE),
        (KeyCode::Char('J'), KeyModifiers::SHIFT),
        (KeyCode::Char('K'), KeyModifiers::SHIFT),
        // pane 移動は `Tab` のトグル 1 つだけ。`Shift+Tab`（`BackTab`）は割り当てない。
        (KeyCode::BackTab, KeyModifiers::SHIFT),
    ] {
        assert_eq!(
            screen.handle_key_event(KeyEvent::new(code, modifiers)),
            ChordChartAction::Continue,
            "{code:?} が曲を変えている"
        );
    }
    assert_eq!(
        screen.focus,
        Pane::Sections,
        "Shift+Tab が pane を動かしている"
    );

    assert_eq!(screen.song, before);
    // `e` が生き残っていると入力欄が開き、以降のキーを全部食う。
    assert!(!screen.line_input_open());
    assert_eq!(screen.error, None);
}

/// 右 pane でも同じ。`d`（複製）と `x`（削除）と `J` `K`（移動）は廃止済み。
#[test]
fn the_retired_keys_do_nothing_in_the_arrangement_pane_either() {
    let mut screen = three_section_screen();
    screen.focus = Pane::Arrangement;
    let before = screen.song.clone();

    for (code, modifiers) in [
        (KeyCode::Char('x'), KeyModifiers::NONE),
        (KeyCode::Char('J'), KeyModifiers::SHIFT),
        (KeyCode::Char('K'), KeyModifiers::SHIFT),
    ] {
        assert_eq!(
            screen.handle_key_event(KeyEvent::new(code, modifiers)),
            ChordChartAction::Continue
        );
    }
    // `d` は `dd` の 1 打目なので、単独では何も起きない（旧 `d` = 複製ではない）。
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('d'))),
        ChordChartAction::Continue
    );

    assert_eq!(screen.song, before);
}

/// `d` 単独では消えない。保留は次のキーで捨て、そのキーは通常どおり効く。
#[test]
fn a_lone_d_deletes_nothing_and_does_not_swallow_the_next_key() {
    let mut screen = three_section_screen();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('d'))),
        ChordChartAction::Continue
    );
    assert_eq!(screen.song.sections.len(), 3);

    // 捨てた保留の次のキー（`j`）は握り潰さない。
    screen.handle_key_event(key(KeyCode::Char('j')));
    assert_eq!(screen.clamped_section_cursor(), 1);
    assert_eq!(screen.song.sections.len(), 3);

    // 間に別のキーを挟んだ `d` … `d` は削除にならない。
    screen.handle_key_event(key(KeyCode::Char('d')));
    screen.handle_key_event(key(KeyCode::Char('k')));
    screen.handle_key_event(key(KeyCode::Char('d')));
    assert_eq!(screen.song.sections.len(), 3);
}

/// `dd` の 1 打目のあとに `Ctrl` / `Alt` 付きのキーが来ても、保留は捨てる。
/// （画面切替から戻ってきた `d` が 2 打目として効くと、押した覚えのない削除が起きる）
#[test]
fn a_pending_d_is_dropped_by_a_modifier_key() {
    let mut screen = three_section_screen();

    screen.handle_key_event(key(KeyCode::Char('d')));
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('d'))),
        ChordChartAction::Continue
    );

    assert_eq!(screen.song.sections.len(), 3);
}

/// `error` は「直前の操作ができなかった理由」なので、次の操作で消える。
#[test]
fn the_next_key_clears_a_stale_error() {
    let mut screen = three_section_screen();
    screen.error = Some("コード進行データがありません".into());

    screen.handle_key_event(key(KeyCode::Char('j')));

    assert_eq!(screen.error, None);
}

/// 移動系のキーは曲を変えないので、保存を要求しない。
#[test]
fn navigation_keys_do_not_ask_for_a_save() {
    let mut screen = three_section_screen();

    for code in [
        KeyCode::Tab,
        KeyCode::Char('h'),
        KeyCode::Char('l'),
        KeyCode::Char('j'),
        KeyCode::Char('k'),
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::Char('?'),
    ] {
        assert_eq!(
            screen.handle_key_event(key(code)),
            ChordChartAction::Continue
        );
    }
}
