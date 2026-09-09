//! 曲がいつディスクへ落ちるか。**変更のたびに即書き**（デバウンス禁止）なので、
//! 「押した直後にファイルを読み戻して見える」ことを 1 操作ずつ固定する。
//!
//! 画面から出るときの書き込みもここ。保存先は `set_local_dir_envs` で temp へ隔離する。

use super::*;
use crate::screen_switch::PrimaryScreen;

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// 保存されたはずの曲を読み戻す。`None`（＝1 度も書かれていない）はここでは不合格。
fn saved_song() -> cmrt_chord_chart::Song {
    cmrt_chord_chart::load_song().expect("保存された曲を読めること")
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
