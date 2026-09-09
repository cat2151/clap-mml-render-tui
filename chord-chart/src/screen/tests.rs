use super::*;

use crossterm::event::KeyEvent;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// section 3 つ・arrangement 2 行の曲。カーソルが両 pane で別々に動くことを見るため、
/// 行数をわざと変えてある。
fn three_section_screen() -> ChordChartScreen {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    let b = song.push_section("B", "IIm-V-I-VIm");
    song.push_section("Sabi", "IV-V-IIIm-VIm");
    song.arrangement = vec![a, b];
    ChordChartScreen::new(song)
}

/// section 25 個・arrangement 25 行の曲。`PgDn` / `PgUp` の 10 行を、端で丸められずに
/// 2 回ぶん見るために 21 行以上要る。
fn twenty_five_section_screen() -> ChordChartScreen {
    let mut song = Song::empty();
    let ids: Vec<_> = (0..25)
        .map(|index| song.push_section(format!("S{index}"), "I-V-VIm-IV"))
        .collect();
    song.arrangement = ids;
    ChordChartScreen::new(song)
}

/// section 1 つ（`A`）を 1 回だけ並べた曲。
fn one_section_song() -> Song {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    song.arrangement = vec![a];
    song
}

/// カタログを注入した画面。`progressions` が抽選の全候補。
fn with_catalog(mut screen: ChordChartScreen, progressions: &[&str]) -> ChordChartScreen {
    let progressions: Vec<String> = progressions
        .iter()
        .map(|text| (*text).to_string())
        .collect();
    screen.set_chord_progression_source(std::sync::Arc::new(move || progressions.clone()));
    screen
}

#[test]
fn the_screen_starts_on_the_sections_pane() {
    let screen = ChordChartScreen::new(one_section_song());

    assert_eq!(screen.focus, Pane::Sections);
    assert_eq!(screen.song.sections.len(), 1);
    assert_eq!(
        screen.selected_section().map(|s| s.name.as_str()),
        Some("A")
    );
}

/// 保存ファイルが読めた（`Some`）なら、画面へ入っても**何も足さない**。
#[test]
fn a_restored_song_is_left_exactly_as_it_was_loaded() {
    let saved = one_section_song();
    let mut screen = with_catalog(
        ChordChartScreen::restored(Some(saved.clone())),
        &["I-IV-V-I"],
    );

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert_eq!(screen.song, saved);
}

/// section を全部消して保存した曲（`Some(空)`）も、そのまま空で開く。
/// ここで抽選すると、消したはずの section が再起動のたびに復活する。
#[test]
fn a_restored_song_with_no_sections_stays_empty() {
    let mut screen = with_catalog(
        ChordChartScreen::restored(Some(Song::empty())),
        &["I-IV-V-I"],
    );

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert!(screen.song.sections.is_empty());
    assert!(screen.song.arrangement.is_empty());
}

/// 保存ファイルが読めなかった（`None`）ときだけ、`g` と同じ抽選が 1 回走る。
/// **進行はカタログから来る**（この crate はコード進行を 1 つも持たない）。
#[test]
fn a_song_that_could_not_be_loaded_generates_one_section_from_the_catalog() {
    let mut screen = with_catalog(ChordChartScreen::restored(None), &["I-IV-V-I"]);

    assert_eq!(screen.enter(), ChordChartAction::SongChanged);

    assert_eq!(screen.song.sections.len(), 1);
    assert_eq!(screen.song.sections[0].name, "A");
    assert_eq!(screen.song.sections[0].degrees, "I-IV-V-I");
    // 右 pane も 1 行から始まる（section だけだと並びが空のままになる）。
    assert_eq!(screen.song.arrangement, vec![screen.song.sections[0].id]);
    assert_eq!(screen.error, None);
}

/// 自動抽選は**入るたびには走らない**。画面を出入りするたびに section が増えると、
/// 曲が勝手に伸びていく。
#[test]
fn the_automatic_pick_happens_only_on_the_first_enter() {
    let mut screen = with_catalog(ChordChartScreen::restored(None), &["I-IV-V-I"]);

    assert_eq!(screen.enter(), ChordChartAction::SongChanged);
    assert_eq!(screen.enter(), ChordChartAction::Continue);
    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert_eq!(screen.song.sections.len(), 1);
}

/// カタログが無ければ空のまま。**進行をハードコードして埋め合わせない**。
/// 理由は下段に出す（無反応で終わらせない）。
#[test]
fn a_failed_automatic_pick_leaves_the_song_empty_with_a_reason() {
    let mut screen = ChordChartScreen::restored(None);

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert!(screen.song.sections.is_empty());
    assert!(screen.song.arrangement.is_empty());
    assert_eq!(
        screen.error.as_deref(),
        Some(crate::catalog::NO_CATALOG_MESSAGE)
    );
    // 失敗しても「1 回だけ」は使い切る（開き直すたびに取得を待たされない）。
    assert_eq!(screen.enter(), ChordChartAction::Continue);
}

/// 空カタログ（取得はできたが 0 件）も同じ扱い。
#[test]
fn an_empty_catalog_is_treated_like_a_missing_one() {
    let mut screen = with_catalog(ChordChartScreen::restored(None), &[]);

    assert_eq!(screen.enter(), ChordChartAction::Continue);

    assert!(screen.song.sections.is_empty());
    assert_eq!(
        screen.error.as_deref(),
        Some(crate::catalog::NO_CATALOG_MESSAGE)
    );
}

/// 行が減ったあとに保持した index が溢れても、描画側が範囲外を引かない。
#[test]
fn a_cursor_past_the_end_is_clamped_to_the_last_row() {
    let screen = ChordChartScreen {
        section_cursor: 99,
        arrangement_cursor: 99,
        ..ChordChartScreen::new(one_section_song())
    };

    assert_eq!(screen.clamped_section_cursor(), 0);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
    assert!(screen.selected_section().is_some());
}

#[test]
fn an_empty_song_has_no_selected_section_and_a_zero_cursor() {
    let mut screen = ChordChartScreen::new(Song::empty());
    screen.section_cursor = 3;

    assert_eq!(screen.clamped_section_cursor(), 0);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
    assert!(screen.selected_section().is_none());
}

/// `h` / `l` は左右そのもの（トグルではない）。同じキーを 2 回押しても戻らない。
#[test]
fn h_and_l_move_the_focus_to_the_named_pane() {
    let mut screen = three_section_screen();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('l'))),
        ChordChartAction::Continue
    );
    assert_eq!(screen.focus, Pane::Arrangement);

    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(screen.focus, Pane::Arrangement);

    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(screen.focus, Pane::Sections);

    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(screen.focus, Pane::Sections);
}

/// `q` はどちらの pane からでもアプリ終了。曲は変えない。
#[test]
fn q_asks_the_runtime_to_quit_from_either_pane() {
    let mut screen = three_section_screen();
    let before = screen.song.clone();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        ChordChartAction::Quit
    );

    screen.focus = Pane::Arrangement;
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        ChordChartAction::Quit
    );
    assert_eq!(screen.song, before);
}

/// `PgDn` / `PgUp` は 10 行。端で止まり、行き過ぎても反対側へは回らない。
#[test]
fn the_page_keys_move_ten_rows_and_stop_at_the_ends() {
    let mut screen = twenty_five_section_screen();

    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(screen.clamped_section_cursor(), 10);

    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(screen.clamped_section_cursor(), 20);

    // 3 回目は末尾（24 行目）で止まる。
    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(screen.clamped_section_cursor(), 24);

    screen.handle_key_event(key(KeyCode::PageUp));
    assert_eq!(screen.clamped_section_cursor(), 14);

    for _ in 0..5 {
        screen.handle_key_event(key(KeyCode::PageUp));
    }
    assert_eq!(screen.clamped_section_cursor(), 0);
}

/// `PgDn` / `PgUp` もフォーカスしている pane だけを動かす。
#[test]
fn the_page_keys_follow_the_focused_pane_only() {
    let mut screen = twenty_five_section_screen();

    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::PageDown));

    assert_eq!(screen.clamped_arrangement_cursor(), 10);
    assert_eq!(screen.clamped_section_cursor(), 0);
}

/// 行が 10 未満でも溢れない（arrangement は 0 行にもできる）。
#[test]
fn the_page_keys_do_nothing_in_a_short_or_empty_pane() {
    let mut screen = three_section_screen();

    screen.handle_key_event(key(KeyCode::PageDown));
    assert_eq!(screen.clamped_section_cursor(), 2);

    let mut empty = ChordChartScreen::new(Song::empty());
    empty.handle_key_event(key(KeyCode::PageDown));
    empty.handle_key_event(key(KeyCode::PageUp));
    assert_eq!(empty.section_cursor, 0);
}

#[test]
fn j_and_k_move_the_cursor_of_the_focused_pane_only() {
    let mut screen = three_section_screen();

    screen.handle_key_event(key(KeyCode::Char('j')));
    assert_eq!(screen.clamped_section_cursor(), 1);
    assert_eq!(screen.clamped_arrangement_cursor(), 0);

    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('j')));
    assert_eq!(screen.clamped_arrangement_cursor(), 1);
    assert_eq!(screen.clamped_section_cursor(), 1);

    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.clamped_arrangement_cursor(), 0);
}

#[test]
fn the_arrow_keys_do_the_same_as_j_and_k() {
    let mut screen = three_section_screen();

    screen.handle_key_event(key(KeyCode::Down));
    screen.handle_key_event(key(KeyCode::Down));
    assert_eq!(screen.clamped_section_cursor(), 2);

    screen.handle_key_event(key(KeyCode::Up));
    assert_eq!(screen.clamped_section_cursor(), 1);
}

/// 押しっぱなしで溢れさせたあと、`k` 1 回で必ず 1 行戻ること。
/// 丸めずに足し引きすると、ここで何十回も無反応になる。
#[test]
fn the_cursor_stops_at_the_ends_instead_of_running_away() {
    let mut screen = three_section_screen();

    for _ in 0..20 {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    assert_eq!(screen.section_cursor, 2);

    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.section_cursor, 1);

    for _ in 0..20 {
        screen.handle_key_event(key(KeyCode::Char('k')));
    }
    assert_eq!(screen.section_cursor, 0);
}

/// 行が 0 のときにカーソルキーを押しても panic しない（arrangement は空にできる）。
#[test]
fn moving_the_cursor_in_an_empty_pane_does_nothing() {
    let mut screen = ChordChartScreen::new(Song::empty());

    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('j')));

    assert_eq!(screen.section_cursor, 0);
    assert_eq!(screen.arrangement_cursor, 0);
}

#[test]
fn question_mark_toggles_the_help_and_escape_closes_it() {
    let mut screen = three_section_screen();

    // `?` は端末によって SHIFT 付きで来る。どちらでも開くこと。
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT));
    assert!(screen.help_open);

    screen.handle_key_event(key(KeyCode::Char('?')));
    assert!(!screen.help_open);

    screen.handle_key_event(key(KeyCode::Char('?')));
    screen.handle_key_event(key(KeyCode::Esc));
    assert!(!screen.help_open);
}

/// ヘルプを閉じたときに裏の画面が動いていると、何が起きたのか分からなくなる。
#[test]
fn the_help_overlay_swallows_every_other_key() {
    let mut screen = three_section_screen();
    screen.handle_key_event(key(KeyCode::Char('?')));

    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::PageDown));
    // ヘルプを読んでいる途中の `q` でアプリが落ちない（閉じるのは `?` / `Esc`）。
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        ChordChartAction::Continue
    );

    assert!(screen.help_open);
    assert_eq!(screen.section_cursor, 0);
    assert_eq!(screen.focus, Pane::Sections);
}

/// `Ctrl+G`（画面切替）は共有ランタイムのもの。取りこぼしても画面が動かないこと。
/// ALT はこの画面が `Alt+↑` `Alt+↓` にだけ使うので、それ以外の ALT 付きも同じ扱い。
#[test]
fn control_and_alt_combinations_are_left_to_the_shared_runtime() {
    let mut screen = three_section_screen();

    screen.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT));
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT));

    assert_eq!(screen.section_cursor, 0);
    assert_eq!(screen.focus, Pane::Sections);
}

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
        // pane 移動は `h` / `l` になったので、`Tab` はもう何もしない。
        (KeyCode::Tab, KeyModifiers::NONE),
        (KeyCode::BackTab, KeyModifiers::SHIFT),
    ] {
        assert_eq!(
            screen.handle_key_event(KeyEvent::new(code, modifiers)),
            ChordChartAction::Continue,
            "{code:?} が曲を変えている"
        );
    }
    assert_eq!(screen.focus, Pane::Sections, "Tab が pane を動かしている");

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
