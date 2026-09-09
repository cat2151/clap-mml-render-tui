//! Chord Chart 画面と共有ランタイムの接続（`chord_chart_glue` / `runtime::screen`）。
//!
//! 画面そのものの挙動は `cmrt-chord-chart` crate 側のテストが見る。ここで見るのは
//! 「app に繋がっているか」だけ。

use super::*;
use crate::screen_switch::PrimaryScreen;

fn ctrl_g() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// 保存されたはずの曲を読み戻す。`None`（＝1 度も書かれていない）はここでは不合格。
fn saved_song() -> cmrt_chord_chart::Song {
    cmrt_chord_chart::load_song().expect("保存された曲を読めること")
}

#[test]
fn switching_to_the_chord_chart_and_back_leaves_the_other_screens_alone() {
    let mut app = TuiApp::new_for_test(test_config());

    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    assert_eq!(app.active_screen, PrimaryScreen::ChordChart);
    // 音を鳴らさない画面なので、入っても何も再生が始まらない。
    assert!(!app.grid_sequencer.state.is_running());

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    assert_eq!(app.active_screen, PrimaryScreen::Notepad);
}

/// 曲は画面をまたいでも保持される（load は起動時の 1 回だけ）。
#[test]
fn returning_to_the_chord_chart_keeps_the_song() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.chord_chart.song.push_section("B", "IIm-V-I-VIm");

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert_eq!(app.chord_chart.song.sections.len(), 2);
}

#[test]
fn ctrl_g_opens_the_menu_from_the_chord_chart_unless_the_help_is_open() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    assert!(app.try_open_screen_switch_menu(ctrl_g()));

    app.screen_switch_menu
        .handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.chord_chart.help_open = true;
    assert!(!app.try_open_screen_switch_menu(ctrl_g()));
}

/// 人間の手順（`Ctrl+G` → `C`）をそのまま辿る。menu の綴りを間違えても
/// `switch_to_primary_screen` 単体のテストは緑のままなので、ここで閉じておく。
#[test]
fn ctrl_g_then_c_opens_the_chord_chart() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(app.try_open_screen_switch_menu(ctrl_g()));

    let target = app.handle_screen_switch_menu_key(plain(KeyCode::Char('c')));

    assert_eq!(target, Some(PrimaryScreen::ChordChart));
    app.switch_to_primary_screen(target.unwrap(), None);
    assert_eq!(app.active_screen, PrimaryScreen::ChordChart);
    assert!(!app.screen_switch_menu.is_open());
}

/// 音を鳴らさずマウスも使わない画面。入力欄を開いていない間は notepad と同じまま。
#[test]
fn the_chord_chart_asks_for_neither_mouse_capture_nor_a_textarea_cursor() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert!(!app.uses_mouse_capture());
    assert!(!app.uses_textarea_cursor());
}

/// `i` / `n` の 1 行入力欄を開いている間だけ、端末カーソルを入力欄の形にする。
/// ここが false のままだと「打っている場所」が画面に出ない。
#[test]
fn the_textarea_cursor_follows_the_line_input() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('i')));
    assert!(app.chord_chart.line_input_open());
    assert!(app.uses_textarea_cursor());
    // マウスは相変わらず要らない。
    assert!(!app.uses_mouse_capture());

    app.handle_chord_chart_key_event(plain(KeyCode::Esc));
    assert!(!app.uses_textarea_cursor());
}

/// 入力中の `Ctrl+G` で画面が飛ぶと、打ちかけの文字列が消える。
#[test]
fn ctrl_g_does_not_leave_the_screen_while_the_line_input_is_open() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.handle_chord_chart_key_event(plain(KeyCode::Char('n')));

    assert!(!app.try_open_screen_switch_menu(ctrl_g()));

    app.handle_chord_chart_key_event(plain(KeyCode::Esc));
    assert!(app.try_open_screen_switch_menu(ctrl_g()));
}

/// 手入力の確定も「押したその場で保存」の対象（デバウンス禁止）。
#[test]
fn a_hand_typed_name_is_saved_as_soon_as_it_is_committed() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_rename_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('n')));
    // 既定値の `A` を消してから打ち直す。
    for _ in 0..8 {
        app.handle_chord_chart_key_event(plain(KeyCode::Backspace));
    }
    for ch in "Sabi".chars() {
        app.handle_chord_chart_key_event(plain(KeyCode::Char(ch)));
    }
    app.handle_chord_chart_key_event(plain(KeyCode::Enter));

    // 画面を離れていないのに、もう読み戻せる。
    let reloaded = saved_song();
    assert_eq!(
        reloaded
            .sections
            .iter()
            .map(|section| section.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Sabi"]
    );
    std::fs::remove_dir_all(&tmp).ok();
}

/// `b` の確定も「押したその場で保存」。prefix が実ファイルまで往復するか。
#[test]
fn the_prefix_typed_with_b_is_saved_immediately() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_prefix_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('b')));
    assert!(
        app.chord_chart.line_input_open(),
        "`b` が画面まで届いていない"
    );
    // 既定値 `Key=C BPM120` を消してから打ち直す。
    for _ in 0..24 {
        app.handle_chord_chart_key_event(plain(KeyCode::Backspace));
    }
    for ch in "Key=A BPM90".chars() {
        app.handle_chord_chart_key_event(plain(KeyCode::Char(ch)));
    }
    // 1 打目では保存されない（確定するまでは曲が変わっていない）。
    // この時点ではファイルすら書かれていないので、読めないのが正しい。
    assert_eq!(cmrt_chord_chart::load_song(), None);

    app.handle_chord_chart_key_event(plain(KeyCode::Enter));

    // 画面を離れていないのに、もう読み戻せる。
    assert_eq!(saved_song().prefix, "Key=A BPM90");
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn keys_reach_the_screen_through_the_glue() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.chord_chart.song.push_section("B", "IIm-V-I-VIm");

    app.handle_chord_chart_key_event(plain(KeyCode::Char('j')));
    assert_eq!(app.chord_chart.clamped_section_cursor(), 1);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('l')));
    assert_eq!(app.chord_chart.focus, cmrt_chord_chart::Pane::Arrangement);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('?')));
    assert!(app.chord_chart.help_open);
}

/// `q` は glue を抜けてランタイムまで届く（ここで握り潰されるとアプリが終了しない）。
#[test]
fn q_reaches_the_glue_as_a_quit() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert_eq!(
        app.handle_chord_chart_key_event(plain(KeyCode::Char('q'))),
        cmrt_chord_chart::ChordChartAction::Quit
    );
    // 移動キーは終了にならない（`q` だけが Quit）。
    assert_eq!(
        app.handle_chord_chart_key_event(plain(KeyCode::PageDown)),
        cmrt_chord_chart::ChordChartAction::Continue
    );
}

/// 画面を離れるときに書かないと、キーごとの保存を取りこぼしたぶんが消える。
#[test]
fn leaving_the_chord_chart_writes_the_song() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_save_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.chord_chart.song.push_section("Sabi", "IV-V-IIIm-VIm");

    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);

    // 保存先は `cmrt-history` が決める。読み戻せることまで見る。
    let reloaded = saved_song();
    assert_eq!(
        reloaded
            .sections
            .iter()
            .map(|section| section.name.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "Sabi"]
    );
    std::fs::remove_dir_all(&tmp).ok();
}

/// app の描画振り分けが chord chart を呼んでいるか（画面ごとの絵の中身は crate 側で見る）。
#[test]
fn the_root_draw_dispatches_to_the_chord_chart_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    let lines = crate::tui::ui::tests::render_lines(&mut app, 80, 24);
    let rendered = lines.join("\n");

    // どちらも ASCII なので、全角のセル 2 個問題（crate 側テストの `squeeze`）は起きない。
    assert!(rendered.contains("Chord Chart"), "{rendered}");
    assert!(rendered.contains("I-V-VIm-IV"), "{rendered}");
}

/// `g` のような曲を変えるキーは、押したその場でファイルへ書かれる（デバウンス禁止）。
/// 画面を離れる前に落ちても、押したぶんは残る。
#[test]
fn a_key_that_changes_the_song_is_saved_immediately() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_add_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.chord_chart
        .set_chord_progression_source(std::sync::Arc::new(|| vec!["I-IV-V-I".to_string()]));

    app.handle_chord_chart_key_event(plain(KeyCode::Char('g')));

    // 画面を離れていないのに、もう読み戻せる。
    let reloaded = saved_song();
    assert_eq!(
        reloaded
            .sections
            .iter()
            .map(|section| (section.name.as_str(), section.degrees.as_str()))
            .collect::<Vec<_>>(),
        vec![("A", "I-V-VIm-IV"), ("B", "I-IV-V-I")]
    );
    std::fs::remove_dir_all(&tmp).ok();
}

/// カタログの初回待ちは秒数をログへ 1 行だけ残す。体感の許容判断は人間だが、
/// **計測そのものは仕込み済み**であることをここで固定する。
#[test]
fn the_catalog_source_measures_and_logs_only_the_first_wait() {
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let counted = std::sync::Arc::clone(&calls);
    let collected = std::sync::Arc::clone(&lines);
    let source = crate::tui::chord_chart_glue::chord_chart_catalog_source(
        move || {
            counted.fetch_add(1, Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_millis(30));
            vec!["I-IV-V-I".to_string(), "I-V-VIm-IV".to_string()]
        },
        move |line| collected.lock().unwrap().push(line.to_string()),
    );
    // 組み立てただけでは引かない（起動時にネットワークを待たない）。
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert!(lines.lock().unwrap().is_empty());

    assert_eq!(source().len(), 2);
    assert_eq!(source().len(), 2);

    assert_eq!(calls.load(Ordering::Relaxed), 2);
    let logged = lines.lock().unwrap().clone();
    assert_eq!(logged.len(), 1, "{logged:?}");
    let line = &logged[0];
    assert!(line.contains("event=catalog-first-load"), "{line}");
    assert!(line.contains("count=2"), "{line}");
    let elapsed: u128 = line
        .split("elapsed_ms=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("{line}"));
    assert!(elapsed >= 5, "{line}");
}

/// `dd`（2 打）も「押したその場で保存」の経路に乗っているか。
///
/// glue は無改修だが、`Alt` 付きや保留キー方式のように**共有ランタイムを一度通る**
/// キーが `ChordChartAction::SongChanged` を返し損ねていないかをここで見る。
#[test]
fn the_two_stroke_delete_is_saved_immediately() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_song_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.chord_chart.song.push_section("B", "IIm-V-I-VIm");
    app.save_chord_chart();
    assert_eq!(saved_song().sections.len(), 2);

    // 1 打目では消えない＝保存もされない。
    app.handle_chord_chart_key_event(plain(KeyCode::Char('d')));
    assert_eq!(saved_song().sections.len(), 2);

    app.handle_chord_chart_key_event(plain(KeyCode::Char('d')));

    let reloaded = saved_song();
    assert_eq!(
        reloaded
            .sections
            .iter()
            .map(|section| section.name.as_str())
            .collect::<Vec<_>>(),
        vec!["B"]
    );
    std::fs::remove_dir_all(&tmp).ok();
}

/// `Alt+↑↓` が共有ランタイムに食われずに画面へ届くか。
///
/// 画面側の modifier ガードだけでなく、app の振り分けが `ALT` 付きを別扱いして
/// いないことまで見る（届かなければ並びは変わらない）。
#[test]
fn alt_arrow_keys_reach_the_screen_and_are_saved() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_move_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    app.chord_chart.song.push_section("B", "IIm-V-I-VIm");

    app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));

    let reloaded = saved_song();
    assert_eq!(
        reloaded
            .sections
            .iter()
            .map(|section| section.name.as_str())
            .collect::<Vec<_>>(),
        vec!["B", "A"]
    );
    std::fs::remove_dir_all(&tmp).ok();
}

/// 初回取得の**実際の秒数**を、画面を起動せずに実経路で測る。
///
/// 通常は skip する（ネットワークが要るため）。走らせ方:
///
/// ```text
/// cargo test -p clap-mml-render-tui the_cold_catalog_first_load -- --ignored --nocapture
/// ```
///
/// config ディレクトリを空の temp dir へ逃がすので、キャッシュが無い＝必ず「初回」になる。
/// 測るのは production と同じ [`crate::tui::chord_chart_glue::chord_chart_catalog_source_from`]。
#[test]
#[ignore = "ネットワークが要る（コード進行カタログの初回取得を実測する）"]
fn the_cold_catalog_first_load_is_measured_through_the_real_source() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_cold_catalog_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut cfg = test_config();
    // test_config() は空文字（＝取得しない）なので、実際の既定 URL へ戻す。
    cfg.chord_progression_source = cmrt_runtime::DEFAULT_CHORD_PROGRESSION_SOURCE.to_string();

    let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let collected = std::sync::Arc::clone(&lines);
    let catalog = crate::tui::chord_chart_glue::chord_chart_catalog_source_from(
        crate::chord_progression_source::ChordProgressionSource::spawn(&cfg),
        move |line| collected.lock().unwrap().push(line.to_string()),
    );

    let progressions = catalog();
    let logged = lines.lock().unwrap().clone();
    let line = logged
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("初回取得のログが出ていない: {logged:?}"));
    // 秒数そのものは人間が読む。ここは「測れていること」だけを固定する。
    eprintln!("{line}");
    assert!(line.contains("event=catalog-first-load"), "{line}");
    assert!(
        !progressions.is_empty(),
        "カタログを取得できていない: {line}"
    );

    std::fs::remove_dir_all(&tmp).ok();
}
