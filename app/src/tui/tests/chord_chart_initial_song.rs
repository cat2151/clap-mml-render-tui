//! 保存ファイルが無い / 壊れているときの、最初の 1 曲の作られ方（app 側の配線）。
//!
//! 画面側の分岐は `cmrt-chord-chart` の `screen::tests` が見る。ここで見るのは
//! 「**画面へ入った時点で**走ること」と「その場で実ファイルへ書かれること」だけ。
//! 起動時（`TuiApp::new`）に走らせないのは、カタログの初回取得を待つのが
//! chord chart を開く人だけで済むようにするため。

use super::*;
use crate::screen_switch::PrimaryScreen;

/// 抽選の候補を 1 件に固定した画面。候補 1 件なら引けるものは 1 つしかない。
fn screen_that_could_not_load(progressions: &[&str]) -> cmrt_chord_chart::ChordChartScreen {
    let progressions: Vec<String> = progressions
        .iter()
        .map(|text| (*text).to_string())
        .collect();
    let mut screen = cmrt_chord_chart::ChordChartScreen::restored(None);
    screen.set_chord_progression_source(Arc::new(move || progressions.clone()));
    screen
}

/// 実ファイルを temp dir へ隔離する。戻り値の guard が落ちるまで有効。
fn temp_local_dirs(name: &str) -> (std::path::PathBuf, impl Sized) {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_chord_chart_{name}_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let guards = crate::test_utils::set_local_dir_envs(&tmp);
    (tmp, guards)
}

/// 保存ファイルが読めなかったときは、**画面へ入った時点で** 1 つ抽選して
/// その場で保存する。保存しないと、次の起動でまた別の進行が出る。
#[test]
fn entering_the_screen_picks_the_first_section_and_saves_it() {
    let (tmp, _guards) = temp_local_dirs("initial_pick");

    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart = screen_that_could_not_load(&["I-IV-V-I"]);
    // 入る前は、まだ 1 度も引いていない（＝起動しただけでは走らない）。
    assert!(app.chord_chart.song.sections.is_empty());
    assert_eq!(cmrt_chord_chart::load_song(), None);

    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    let saved = cmrt_chord_chart::load_song().expect("入った時点で保存されていること");
    assert_eq!(
        saved
            .sections
            .iter()
            .map(|section| (section.name.as_str(), section.degrees.as_str()))
            .collect::<Vec<_>>(),
        vec![("A", "I-IV-V-I")]
    );
    assert_eq!(saved.arrangement.len(), 1, "右 pane も 1 行から始まる");

    std::fs::remove_dir_all(&tmp).ok();
}

/// 前回 chord chart で終了していた場合は `switch_to_primary_screen` を通らない。
/// その経路でも抽選が走ること（`enter_restored_chord_chart`）。
#[test]
fn the_restored_startup_path_picks_the_first_section_too() {
    let (tmp, _guards) = temp_local_dirs("restored_pick");

    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = PrimaryScreen::ChordChart;
    app.chord_chart = screen_that_could_not_load(&["I-IV-V-I"]);

    app.enter_restored_chord_chart();

    let saved = cmrt_chord_chart::load_song().expect("入った時点で保存されていること");
    assert_eq!(saved.sections.len(), 1);

    std::fs::remove_dir_all(&tmp).ok();
}

/// 他の画面で終了していたなら、起動時には何も引かない
/// （カタログの初回取得を、開かない人にまで待たせない）。
#[test]
fn the_restored_startup_path_does_nothing_on_another_screen() {
    let (tmp, _guards) = temp_local_dirs("restored_other");

    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = PrimaryScreen::Notepad;
    app.chord_chart = screen_that_could_not_load(&["I-IV-V-I"]);

    app.enter_restored_chord_chart();

    assert!(app.chord_chart.song.sections.is_empty());
    assert_eq!(cmrt_chord_chart::load_song(), None);

    std::fs::remove_dir_all(&tmp).ok();
}

/// 保存した曲は次の起動でそのまま戻る（＝抽選し直さない）。
/// 実ファイルを 1 度書いてから読み直す、往復のテスト。
#[test]
fn the_generated_song_survives_a_restart_without_being_rerolled() {
    let (tmp, _guards) = temp_local_dirs("initial_roundtrip");

    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart = screen_that_could_not_load(&["I-IV-V-I"]);
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);
    let first = app.chord_chart.song.clone();

    // 次の起動。session が組み立てるのと同じ経路で読み直す。
    let mut restarted = cmrt_chord_chart::ChordChartScreen::restored(cmrt_chord_chart::load_song());
    restarted.set_chord_progression_source(Arc::new(|| vec!["IIm-V-I".to_string()]));
    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart = restarted;
    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert_eq!(app.chord_chart.song, first, "2 度目は引き直さない");

    std::fs::remove_dir_all(&tmp).ok();
}

/// カタログが無い（取得できていない / 未注入）ときは空のまま。
/// **進行をハードコードして埋め合わせない**し、panic も保存もしない。
#[test]
fn without_a_catalog_the_screen_opens_empty_without_panicking() {
    let (tmp, _guards) = temp_local_dirs("initial_no_catalog");

    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart = cmrt_chord_chart::ChordChartScreen::restored(None);

    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    assert!(app.chord_chart.song.sections.is_empty());
    assert!(app.chord_chart.song.arrangement.is_empty());
    // 曲が変わっていないので書きもしない（空の曲で上書きしない）。
    assert_eq!(cmrt_chord_chart::load_song(), None);
    // 無反応で終わらせず、理由を下段に出す。
    assert!(app.chord_chart.error.is_some());
    // 画面としては描ける（行が 0 でも落ちない）。
    let lines = crate::tui::ui::tests::render_lines(&mut app, 80, 24);
    assert!(lines.join("\n").contains("Chord Chart"), "{lines:?}");

    std::fs::remove_dir_all(&tmp).ok();
}

/// 壊れたファイルも「読めなかった」扱い。移行コードは書かないので、
/// 旧形式（`version` / `title` / `key` / `bpm`）は読めた形だけ拾って始まる。
#[test]
fn a_broken_file_is_replaced_by_one_freshly_picked_section() {
    let (tmp, _guards) = temp_local_dirs("initial_broken");
    let path = cmrt_history::chord_chart_file_path().expect("保存先が決まること");
    std::fs::create_dir_all(path.parent().expect("親ディレクトリがあること")).unwrap();
    std::fs::write(&path, "{ not json").unwrap();

    let mut app = TuiApp::new_for_test(test_config());
    app.chord_chart = cmrt_chord_chart::ChordChartScreen::restored(cmrt_chord_chart::load_song());
    app.chord_chart
        .set_chord_progression_source(Arc::new(|| vec!["I-IV-V-I".to_string()]));

    app.switch_to_primary_screen(PrimaryScreen::ChordChart, None);

    let saved = cmrt_chord_chart::load_song().expect("入った時点で保存されていること");
    assert_eq!(saved.sections.len(), 1);
    assert_eq!(saved.sections[0].degrees, "I-IV-V-I");

    std::fs::remove_dir_all(&tmp).ok();
}
